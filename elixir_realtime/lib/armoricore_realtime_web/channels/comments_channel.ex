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

defmodule ArmoricoreRealtimeWeb.CommentsChannel do
  @moduledoc """
  Live Comments Channel for streaming comments.
  
  Handles:
  - High-frequency comment broadcasting
  - Rate limiting
  - Stream-specific comments
  """

  use ArmoricoreRealtimeWeb, :channel
  require Logger

  @impl true
  def join("comments:stream:" <> stream_id, _payload, socket) do
    user_id = socket.assigns.user_id

    Logger.info("User #{user_id} joining comments for stream #{stream_id}")

    {:ok, socket
         |> assign(:stream_id, stream_id)
         |> assign(:user_id, user_id)
         |> assign(:last_comment_time, 0)}
  end

  @impl true
  def handle_in("new_comment", %{"content" => content} = payload, socket) do
    user_id = socket.assigns.user_id
    stream_id = socket.assigns.stream_id

    # Rate limiting: prevent spam (max 1 comment per second)
    current_time = System.system_time(:second)
    last_time = socket.assigns.last_comment_time

    if current_time - last_time < 1 do
      {:reply, {:error, %{reason: "rate_limit"}}, socket}
    else
      # Generate comment ID
      comment_id = UUID.uuid4()
      timestamp = Map.get(payload, "timestamp", current_time)

      # Create comment payload
      comment = %{
        id: comment_id,
        content: content,
        user_id: user_id,
        stream_id: stream_id,
        timestamp: timestamp
      }

      Logger.info("New comment from user #{user_id} on stream #{stream_id}")

      # Broadcast to all subscribers
      broadcast(socket, "new_comment", comment)

      # Publish to message bus for analytics/moderation
      ArmoricoreRealtimeWeb.ChannelHelpers.publish_comment_event(comment)

      {:reply, {:ok, comment}, assign(socket, :last_comment_time, current_time)}
    end
  end

  @impl true
  def handle_in("ping", _payload, socket) do
    {:reply, {:ok, %{ping: "pong"}}, socket}
  end
end
