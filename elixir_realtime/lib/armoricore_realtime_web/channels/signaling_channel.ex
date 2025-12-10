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

defmodule ArmoricoreRealtimeWeb.SignalingChannel do
  @moduledoc """
  Signaling Channel for WebRTC voice/video calls.
  
  Handles:
  - Call initiation
  - SDP offer/answer exchange
  - ICE candidate exchange
  - Call state management
  - Call termination
  """

  use ArmoricoreRealtimeWeb, :channel
  require Logger

  # Call states
  @call_state_initiating "initiating"
  @call_state_ringing "ringing"
  @call_state_connected "connected"
  @call_state_ended "ended"

  @impl true
  def join("signaling:call:" <> call_id, payload, socket) do
    user_id = socket.assigns.user_id
    caller_id = payload["caller_id"]
    callee_id = payload["callee_id"]

    # Verify user is part of the call
    if user_id == caller_id || user_id == callee_id do
      Logger.info("User #{user_id} joining signaling channel for call #{call_id}")

      # Subscribe to call-specific PubSub topic
      Phoenix.PubSub.subscribe(ArmoricoreRealtime.PubSub, "signaling:call:#{call_id}")

      {:ok, socket
           |> assign(:call_id, call_id)
           |> assign(:user_id, user_id)
           |> assign(:caller_id, caller_id)
           |> assign(:callee_id, callee_id)
           |> assign(:call_state, @call_state_initiating)}
    else
      Logger.warning("User #{user_id} attempted to join call #{call_id} they're not part of")
      {:error, %{reason: "unauthorized"}}
    end
  end

  @doc """
  Initiates a call.
  
  Payload:
  {
    "callee_id": "user-uuid",
    "call_type": "voice" | "video"
  }
  """
  @impl true
  def handle_in("call_initiate", %{"callee_id" => callee_id, "call_type" => call_type}, socket) do
    caller_id = socket.assigns.user_id
    call_id = socket.assigns.call_id

    Logger.info("Call #{call_id} initiated by #{caller_id} to #{callee_id} (#{call_type})")

    # Create call metadata
    call_metadata = %{
      call_id: call_id,
      caller_id: caller_id,
      callee_id: callee_id,
      call_type: call_type,
      state: @call_state_ringing,
      initiated_at: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    # Broadcast call initiation to both participants
    broadcast(socket, "call_initiated", call_metadata)

    # Publish to message bus for analytics/moderation
    ArmoricoreRealtimeWeb.ChannelHelpers.publish_call_event(call_metadata)

    {:reply, {:ok, call_metadata}, assign(socket, :call_state, @call_state_ringing)}
  end

  # Sends SDP offer.
  # Payload: {"sdp": "v=0\\no=-...", "type": "offer"}
  @impl true
  def handle_in("call_offer", %{"sdp" => sdp, "type" => "offer"} = _payload, socket) do
    user_id = socket.assigns.user_id
    call_id = socket.assigns.call_id

    Logger.debug("SDP offer received from #{user_id} for call #{call_id}")

    # Create offer message
    offer = %{
      call_id: call_id,
      from: user_id,
      sdp: sdp,
      type: "offer",
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    # Broadcast offer to other participant (not sender)
    broadcast_from(socket, "call_offer", offer)

    {:reply, {:ok, %{status: "offer_sent"}}, socket}
  end

  # Sends SDP answer.
  # Payload: {"sdp": "v=0\\no=-...", "type": "answer"}
  @impl true
  def handle_in("call_answer", %{"sdp" => sdp, "type" => "answer"} = _payload, socket) do
    user_id = socket.assigns.user_id
    call_id = socket.assigns.call_id

    Logger.debug("SDP answer received from #{user_id} for call #{call_id}")

    # Create answer message
    answer = %{
      call_id: call_id,
      from: user_id,
      sdp: sdp,
      type: "answer",
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    # Update call state to connected
    call_metadata = %{
      call_id: call_id,
      state: @call_state_connected,
      connected_at: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    # Broadcast answer to other participant (not sender)
    broadcast_from(socket, "call_answer", answer)

    # Broadcast call connected state
    broadcast(socket, "call_connected", call_metadata)

    {:reply, {:ok, %{status: "answer_sent"}}, assign(socket, :call_state, @call_state_connected)}
  end

  # Exchanges ICE candidate.
  # Payload: {"candidate": "candidate:...", "sdp_mid": "0", "sdp_m_line_index": 0}
  @impl true
  def handle_in("ice_candidate", %{"candidate" => candidate} = payload, socket) do
    user_id = socket.assigns.user_id
    call_id = socket.assigns.call_id

    Logger.debug("ICE candidate received from #{user_id} for call #{call_id}")

    # Create ICE candidate message
    ice_candidate = %{
      call_id: call_id,
      from: user_id,
      candidate: candidate,
      sdp_mid: Map.get(payload, "sdp_mid"),
      sdp_m_line_index: Map.get(payload, "sdp_m_line_index"),
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    # Broadcast ICE candidate to other participant (not sender)
    broadcast_from(socket, "ice_candidate", ice_candidate)

    {:reply, {:ok, %{status: "ice_candidate_sent"}}, socket}
  end

  # Ends the call.
  # Payload: {"reason": "user_hangup" | "rejected" | "timeout" | "error"}
  @impl true
  def handle_in("call_end", %{"reason" => reason} = _payload, socket) do
    user_id = socket.assigns.user_id
    call_id = socket.assigns.call_id

    Logger.info("Call #{call_id} ended by #{user_id}, reason: #{reason}")

    # Create end call message
    end_call = %{
      call_id: call_id,
      ended_by: user_id,
      reason: reason,
      ended_at: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    # Broadcast call end to all participants
    broadcast(socket, "call_ended", end_call)

    # Publish to message bus
    ArmoricoreRealtimeWeb.ChannelHelpers.publish_call_event(end_call)

    {:reply, {:ok, %{status: "call_ended"}}, assign(socket, :call_state, @call_state_ended)}
  end

  # Rejects an incoming call.
  @impl true
  def handle_in("call_reject", _payload, socket) do
    user_id = socket.assigns.user_id
    call_id = socket.assigns.call_id

    Logger.info("Call #{call_id} rejected by #{user_id}")

    # Create reject message
    reject_call = %{
      call_id: call_id,
      rejected_by: user_id,
      reason: "rejected",
      rejected_at: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    # Broadcast call rejection
    broadcast(socket, "call_rejected", reject_call)

    # Publish to message bus
    ArmoricoreRealtimeWeb.ChannelHelpers.publish_call_event(reject_call)

    {:reply, {:ok, %{status: "call_rejected"}}, assign(socket, :call_state, @call_state_ended)}
  end

  # Handles ping for connection keepalive.
  @impl true
  def handle_in("ping", _payload, socket) do
    {:reply, {:ok, %{ping: "pong"}}, socket}
  end

  @impl true
  def terminate(_reason, socket) do
    # If call is still active, end it
    call_state = socket.assigns[:call_state]
    if call_state && call_state != @call_state_ended do
      user_id = socket.assigns[:user_id]
      call_id = socket.assigns[:call_id]

      Logger.info("User #{user_id} disconnected from call #{call_id}, ending call")

      # Broadcast call end due to disconnect
      end_call = %{
        call_id: call_id,
        ended_by: user_id,
        reason: "disconnected",
        ended_at: DateTime.utc_now() |> DateTime.to_iso8601()
      }

      Phoenix.PubSub.broadcast(
        ArmoricoreRealtime.PubSub,
        "signaling:call:#{call_id}",
        %Phoenix.Socket.Broadcast{
          topic: "signaling:call:#{call_id}",
          event: "call_ended",
          payload: end_call
        }
      )
    end

    :ok
  end
end
