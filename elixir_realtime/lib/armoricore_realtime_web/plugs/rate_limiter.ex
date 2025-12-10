# Copyright 2025 Francisco F. Pinochet
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

defmodule ArmoricoreRealtimeWeb.Plugs.RateLimiter do
  @moduledoc """
  Rate limiting plug for API endpoints.
  
  Uses a token bucket algorithm to limit requests per IP address.
  """
  
  import Plug.Conn
  require Logger

  @behaviour Plug

  # Default rate limits (requests per window)
  @default_limit 100
  @default_window 60_000 # 1 minute in milliseconds
  
  # Rate limits by endpoint type
  @limits %{
    "/api/auth/login" => {10, 60_000},      # 10 requests per minute
    "/api/auth/register" => {5, 60_000},    # 5 requests per minute
    "/api/auth/refresh" => {20, 60_000},    # 20 requests per minute
    "/api/media/upload" => {50, 60_000},    # 50 requests per minute
    "/api/notifications" => {100, 60_000},  # 100 requests per minute
    default: {@default_limit, @default_window}
  }

  def init(opts), do: opts

  def call(conn, _opts) do
    client_ip = get_client_ip(conn)
    path = conn.request_path
    
    {limit, window} = get_limit_for_path(path)
    
    case check_rate_limit(client_ip, path, limit, window) do
      :ok ->
        conn
        |> put_resp_header("x-ratelimit-limit", Integer.to_string(limit))
        |> put_resp_header("x-ratelimit-remaining", Integer.to_string(limit - 1))
        |> put_resp_header("x-ratelimit-reset", Integer.to_string(get_reset_time(window)))
      
      {:error, :rate_limit_exceeded} ->
        Logger.warning("Rate limit exceeded for IP: #{client_ip}, path: #{path}")
        conn
        |> put_status(429)
        |> put_resp_header("x-ratelimit-limit", Integer.to_string(limit))
        |> put_resp_header("x-ratelimit-remaining", "0")
        |> put_resp_header("x-ratelimit-reset", Integer.to_string(get_reset_time(window)))
        |> put_resp_content_type("application/json")
        |> send_resp(429, Jason.encode!(%{error: "Rate limit exceeded. Please try again later."}))
        |> halt()
    end
  end

  defp get_client_ip(conn) do
    case get_req_header(conn, "x-forwarded-for") do
      [ip | _] -> ip
      [] -> to_string(:inet_parse.ntoa(conn.remote_ip))
    end
  end

  defp get_limit_for_path(path) do
    Enum.find_value(@limits, @limits.default, fn
      {key, value} when key != :default ->
        if String.starts_with?(path, key), do: value
      _ -> false
    end)
  end

  defp check_rate_limit(ip, path, limit, window) do
    # Ensure ETS table exists (create if it doesn't)
    ensure_ets_table()
    
    key = {:rate_limit, ip, path}
    
    # Use ETS (Erlang Term Storage) for in-memory rate limiting
    # In production, consider using Redis for distributed rate limiting
    now = System.system_time(:millisecond)
    
    case :ets.lookup(:rate_limits, key) do
      [] ->
        # First request, create entry
        reset_time = now + window
        :ets.insert(:rate_limits, {key, 1, reset_time})
        :ok
      
      [{^key, count, reset_time}] ->
        if now > reset_time do
          # Window expired, reset
          new_reset_time = now + window
          :ets.insert(:rate_limits, {key, 1, new_reset_time})
          :ok
        else
          if count >= limit do
            {:error, :rate_limit_exceeded}
          else
            # Increment counter atomically
            :ets.update_counter(:rate_limits, key, {2, 1})
            :ok
          end
        end
    end
  end

  defp get_reset_time(window) do
    # Return Unix timestamp in seconds
    div(System.system_time(:millisecond) + window, 1000)
  end

  defp ensure_ets_table do
    case :ets.whereis(:rate_limits) do
      :undefined ->
        # Table doesn't exist, create it
        :ets.new(:rate_limits, [:named_table, :public, :set])
      _ ->
        # Table exists, do nothing
        :ok
    end
  end
end
