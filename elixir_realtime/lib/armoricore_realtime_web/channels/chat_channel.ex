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

defmodule ArmoricoreRealtimeWeb.ChatChannel do
  @moduledoc """
  Chat Channel for real-time messaging.
  
  Handles:
  - Joining chat rooms
  - Sending messages
  - Receiving messages
  - Typing indicators
  - Publishing to message bus for moderation
  """

  use ArmoricoreRealtimeWeb, :channel
  require Logger

  # Typing indicator timeout (3 seconds of inactivity)
  @typing_timeout 3_000

  @impl true
  def join("chat:room:" <> room_id, _payload, socket) do
    user_id = socket.assigns.user_id

    Logger.info("User #{user_id} joining chat room #{room_id}")

    # Subscribe to presence for this room
    Phoenix.PubSub.subscribe(ArmoricoreRealtime.PubSub, "presence:room:#{room_id}")

    # Authorize room access (in production, check permissions)
    {:ok, socket
         |> assign(:room_id, room_id)
         |> assign(:user_id, user_id)
         |> assign(:typing_timer, nil)}
  end

  @impl true
  def handle_in("new_message", %{"content" => content} = payload, socket) do
    # Check if message is encrypted (E2EE)
    encrypted? = Map.get(payload, "encrypted", false)
    
    # Validate encrypted message format if E2EE is enabled
    if encrypted? do
      # Check if message has required encrypted fields
      has_encrypted_fields = 
        Map.has_key?(payload, "ciphertext") and
        Map.has_key?(payload, "nonce") and
        Map.has_key?(payload, "tag")
      
      if has_encrypted_fields do
        # Message is properly encrypted, proceed with broadcast
        handle_encrypted_message(payload, socket)
      else
        {:reply, {:error, %{reason: "Invalid encrypted message format"}}, socket}
      end
    else
      # Regular (non-encrypted) message
      handle_plaintext_message(content, payload, socket)
    end
  end

  @impl true
  def handle_in("typing_start", _payload, socket) do
    user_id = socket.assigns.user_id
    room_id = socket.assigns.room_id

    Logger.debug("User #{user_id} started typing in room #{room_id}")

    # Cancel any existing typing timer
    cancel_typing_timer(socket)

    # Broadcast typing start to all users in the room (except sender)
    broadcast_from(socket, "user_typing", %{
      user_id: user_id,
      room_id: room_id,
      is_typing: true
    })

    # Set a timer to auto-stop typing after timeout
    timer_ref = Process.send_after(self(), {:typing_timeout, user_id}, @typing_timeout)

    {:reply, {:ok, %{status: "typing"}}, assign(socket, :typing_timer, timer_ref)}
  end

  @impl true
  def handle_in("typing_stop", _payload, socket) do
    user_id = socket.assigns.user_id
    room_id = socket.assigns.room_id

    Logger.debug("User #{user_id} stopped typing in room #{room_id}")

    # Cancel typing timer
    cancel_typing_timer(socket)

    # Broadcast typing stop to all users in the room (except sender)
    broadcast_from(socket, "user_typing", %{
      user_id: user_id,
      room_id: room_id,
      is_typing: false
    })

    {:reply, {:ok, %{status: "stopped"}}, assign(socket, :typing_timer, nil)}
  end

  @impl true
  def handle_in("ping", _payload, socket) do
    {:reply, {:ok, %{ping: "pong"}}, socket}
  end

  # It is also common to receive messages from the client and
  # broadcast to everyone in the current topic (chat:room:lobby).
  @impl true
  def handle_in("shout", payload, socket) do
    broadcast(socket, "shout", payload)
    {:noreply, socket}
  end

  # Helper functions
  defp handle_encrypted_message(payload, socket) do
    user_id = socket.assigns.user_id
    room_id = socket.assigns.room_id

    # Generate message ID
    message_id = UUID.uuid4()

    # Create message payload (server doesn't decrypt)
    message = %{
      id: message_id,
      content: payload, # Send encrypted payload as-is
      user_id: user_id,
      room_id: room_id,
      encrypted: true,
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    Logger.info("New encrypted message from user #{user_id} in room #{room_id}")

    # Broadcast to all subscribers in the room
    broadcast(socket, "new_message", message)

    # Note: Don't publish encrypted messages to message bus for moderation
    # (server can't read them)

    {:reply, {:ok, message}, socket}
  end

  defp handle_plaintext_message(content, _payload, socket) do
    user_id = socket.assigns.user_id
    room_id = socket.assigns.room_id

    # Generate message ID
    message_id = UUID.uuid4()

    # Create message payload
    message = %{
      id: message_id,
      content: content,
      user_id: user_id,
      room_id: room_id,
      encrypted: false,
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    Logger.info("New message from user #{user_id} in room #{room_id}")

    # Broadcast to all subscribers in the room
    broadcast(socket, "new_message", message)

    # Publish to message bus for moderation workflows
    ArmoricoreRealtimeWeb.ChannelHelpers.publish_chat_message(message)

    {:reply, {:ok, message}, socket}
  end

  @impl true
  def handle_info({:typing_timeout, user_id}, socket) do
    room_id = socket.assigns.room_id

    Logger.debug("Typing timeout for user #{user_id} in room #{room_id}")

    # Broadcast typing stop due to timeout
    broadcast_from(socket, "user_typing", %{
      user_id: user_id,
      room_id: room_id,
      is_typing: false
    })

    {:noreply, assign(socket, :typing_timer, nil)}
  end

  # Helper to cancel typing timer
  defp cancel_typing_timer(socket) do
    case socket.assigns.typing_timer do
      nil -> :ok
      timer_ref -> Process.cancel_timer(timer_ref)
    end
  end

  @impl true
  def terminate(_reason, socket) do
    # Clean up: stop typing indicator when user disconnects
    user_id = socket.assigns[:user_id]
    room_id = socket.assigns[:room_id]

    if user_id && room_id do
      # Broadcast typing stop to all users in the room
      # Note: Using broadcast instead of broadcast_from since socket may be closing
      Phoenix.PubSub.broadcast(
        ArmoricoreRealtime.PubSub,
        "chat:room:#{room_id}",
        %Phoenix.Socket.Broadcast{
          topic: "chat:room:#{room_id}",
          event: "user_typing",
          payload: %{
            user_id: user_id,
            room_id: room_id,
            is_typing: false
          }
        }
      )
    end

    cancel_typing_timer(socket)
    :ok
  end

end
