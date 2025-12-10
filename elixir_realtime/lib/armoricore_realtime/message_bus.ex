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

defmodule ArmoricoreRealtime.MessageBus do
  @moduledoc """
  Message bus integration for NATS.
  
  Subscribes to events from the message bus and handles them.
  """

  use GenServer
  require Logger

  @doc """
  Starts the message bus subscriber.
  """
  def start_link(_opts) do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  @impl true
  def init(_opts) do
    # Get NATS URL from config
    nats_url = Application.get_env(:armoricore_realtime, :message_bus_url, "nats://localhost:4222")

    Logger.info("Connecting to message bus: #{nats_url}")

    case Gnat.start_link(%{connection_name: :armoricore_nats, url: nats_url}) do
      {:ok, gnat} ->
        # Subscribe to events we care about
        subscribe_to_events(gnat)
        {:ok, %{gnat: gnat}}

      {:error, reason} ->
        Logger.error("Failed to connect to message bus: #{inspect(reason)}")
        {:stop, reason}
    end
  end

  @impl true
  def handle_info({:msg, %{topic: topic, body: body}}, state) do
    Logger.info("Received message on topic: #{topic}")

    case Jason.decode(body) do
      {:ok, event} ->
        handle_event(topic, event)
        {:noreply, state}

      {:error, reason} ->
        Logger.error("Failed to decode event: #{inspect(reason)}")
        {:noreply, state}
    end
  end

  def handle_info(msg, state) do
    Logger.debug("Received unhandled message: #{inspect(msg)}")
    {:noreply, state}
  end

  @impl true
  def handle_call(:get_gnat, _from, state) do
    {:reply, {:ok, state.gnat}, state}
  end

  # Subscribe to events we care about
  defp subscribe_to_events(gnat) do
    # Subscribe to media.ready events
    Gnat.sub(gnat, self(), "armoricore.media_ready", queue_group: "armoricore-realtime")

    # Subscribe to notification.sent events
    Gnat.sub(gnat, self(), "armoricore.notification_sent", queue_group: "armoricore-realtime")

    # Subscribe to transcription.complete events
    Gnat.sub(gnat, self(), "armoricore.transcription_complete", queue_group: "armoricore-realtime")

    Logger.info("Subscribed to message bus events")
  end

  # Handle different event types
  defp handle_event("armoricore.media_ready", event) do
    Logger.info("Media ready event: #{inspect(event)}")
    
    # Extract media_id and user_id from payload
    media_id = get_in(event, ["payload", "media_id"])
    user_id = get_in(event, ["payload", "user_id"])
    
    if media_id do
      # Broadcast to media-specific topic
      Phoenix.PubSub.broadcast(
        ArmoricoreRealtime.PubSub,
        "media:#{media_id}",
        {:media_ready, event}
      )
      
      # Also broadcast to user's personal topic if user_id is available
      if user_id do
        Phoenix.PubSub.broadcast(
          ArmoricoreRealtime.PubSub,
          "user:#{user_id}",
          {:media_ready, event}
        )
      end
      
      Logger.info("Broadcasted media_ready event for media_id: #{media_id}")
    else
      Logger.warning("Media ready event missing media_id: #{inspect(event)}")
    end
  end

  defp handle_event("armoricore.notification_sent", event) do
    Logger.info("Notification sent event: #{inspect(event)}")
    
    # Extract user_id from payload
    user_id = get_in(event, ["payload", "user_id"])
    
    if user_id do
      # Broadcast to user's personal topic
      Phoenix.PubSub.broadcast(
        ArmoricoreRealtime.PubSub,
        "user:#{user_id}",
        {:notification_sent, event}
      )
      
      Logger.info("Broadcasted notification_sent event for user_id: #{user_id}")
    else
      Logger.warning("Notification sent event missing user_id: #{inspect(event)}")
    end
  end

  defp handle_event("armoricore.transcription_complete", event) do
    Logger.info("Transcription complete event: #{inspect(event)}")
    
    # Extract media_id from payload
    media_id = get_in(event, ["payload", "media_id"])
    user_id = get_in(event, ["payload", "user_id"])
    
    if media_id do
      # Broadcast to media-specific topic
      Phoenix.PubSub.broadcast(
        ArmoricoreRealtime.PubSub,
        "media:#{media_id}",
        {:transcription_complete, event}
      )
      
      # Also broadcast to user's personal topic if user_id is available
      if user_id do
        Phoenix.PubSub.broadcast(
          ArmoricoreRealtime.PubSub,
          "user:#{user_id}",
          {:transcription_complete, event}
        )
      end
      
      Logger.info("Broadcasted transcription_complete event for media_id: #{media_id}")
    else
      Logger.warning("Transcription complete event missing media_id: #{inspect(event)}")
    end
  end

  defp handle_event(topic, event) do
    Logger.warning("Unhandled event topic: #{topic}, event: #{inspect(event)}")
  end
end
