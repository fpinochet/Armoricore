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

defmodule ArmoricoreRealtime.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    # Initialize ETS table for rate limiting
    :ets.new(:rate_limits, [:named_table, :public, :set])
    
    children = [
      ArmoricoreRealtimeWeb.Telemetry,
      {DNSCluster, query: Application.get_env(:armoricore_realtime, :dns_cluster_query) || :ignore},
      # Database
      ArmoricoreRealtime.Repo,
      {Phoenix.PubSub, name: ArmoricoreRealtime.PubSub},
      # Key management
      {ArmoricoreRealtime.KeyManager, storage_path: "priv/keys"},
      # Call manager
      ArmoricoreRealtime.CallManager,
      # Media Engine gRPC client
      ArmoricoreRealtime.MediaEngineClient,
      # Message bus subscriber
      ArmoricoreRealtime.MessageBus,
      # Media processing optimizations
      ArmoricoreRealtime.Media.ProcessorPool,
      ArmoricoreRealtime.Media.PriorityQueue,
      ArmoricoreRealtime.Media.Pipeline,
      ArmoricoreRealtime.Media.Consumer,
      ArmoricoreRealtime.Media.Distributed,
      # Start to serve requests, typically the last entry
      ArmoricoreRealtimeWeb.Endpoint
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: ArmoricoreRealtime.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    ArmoricoreRealtimeWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
