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

defmodule ArmoricoreRealtime.MediaEngineClient do
  @moduledoc """
  gRPC client for connecting to the Rust Realtime Media Engine.
  
  This module provides functions to interact with the Rust media engine
  running on port 50051, including stream management, audio encoding/decoding,
  and packet routing.
  """

  use GenServer
  require Logger

  @grpc_server_url Application.compile_env(:armoricore_realtime, :media_engine_grpc_url, "http://localhost:50051")

  ## Client API

  @doc """
  Starts the MediaEngineClient GenServer.
  """
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Creates a new media stream.
  
  ## Parameters
  - `user_id` - User ID for the stream
  - `media_type` - `:audio` or `:video`
  - `ssrc` - Synchronization source identifier
  - `codec` - Codec name (e.g., "opus", "h264")
  - `bitrate` - Bitrate in bps
  - `encryption_enabled` - Whether encryption is enabled
  
  ## Returns
  - `{:ok, stream_id}` on success
  - `{:error, reason}` on failure
  """
  def create_stream(user_id, media_type, ssrc, codec, bitrate, encryption_enabled \\ true) do
    GenServer.call(__MODULE__, {:create_stream, user_id, media_type, ssrc, codec, bitrate, encryption_enabled})
  end

  @doc """
  Stops a media stream.
  
  ## Parameters
  - `stream_id` - Stream ID to stop
  
  ## Returns
  - `:ok` on success
  - `{:error, reason}` on failure
  """
  def stop_stream(stream_id) do
    GenServer.call(__MODULE__, {:stop_stream, stream_id})
  end

  @doc """
  Gets stream information.
  
  ## Parameters
  - `stream_id` - Stream ID to query
  
  ## Returns
  - `{:ok, stream_info}` on success
  - `{:error, reason}` on failure
  """
  def get_stream(stream_id) do
    GenServer.call(__MODULE__, {:get_stream, stream_id})
  end

  @doc """
  Routes an RTP packet.
  
  ## Parameters
  - `stream_id` - Stream ID
  - `rtp_packet` - RTP packet bytes
  
  ## Returns
  - `{:ok, destination}` on success
  - `{:error, reason}` on failure
  """
  def route_packet(stream_id, rtp_packet) do
    GenServer.call(__MODULE__, {:route_packet, stream_id, rtp_packet})
  end

  @doc """
  Encodes audio samples.
  
  ## Parameters
  - `stream_id` - Stream ID
  - `samples` - List of float PCM samples
  - `sample_rate` - Sample rate in Hz
  - `channels` - Number of channels
  - `timestamp` - RTP timestamp
  
  ## Returns
  - `{:ok, encoded_data}` on success
  - `{:error, reason}` on failure
  """
  def encode_audio(stream_id, samples, sample_rate, channels, timestamp) do
    GenServer.call(__MODULE__, {:encode_audio, stream_id, samples, sample_rate, channels, timestamp})
  end

  @doc """
  Decodes audio data.
  
  ## Parameters
  - `stream_id` - Stream ID
  - `encoded_data` - Encoded audio bytes
  - `timestamp` - RTP timestamp
  
  ## Returns
  - `{:ok, {samples, sample_rate, channels}}` on success
  - `{:error, reason}` on failure
  """
  def decode_audio(stream_id, encoded_data, timestamp) do
    GenServer.call(__MODULE__, {:decode_audio, stream_id, encoded_data, timestamp})
  end

  @doc """
  Updates stream state.
  
  ## Parameters
  - `stream_id` - Stream ID
  - `state` - New state (`:initializing`, `:active`, `:paused`, `:stopped`, `:error`)
  
  ## Returns
  - `:ok` on success
  - `{:error, reason}` on failure
  """
  def update_stream_state(stream_id, state) do
    GenServer.call(__MODULE__, {:update_stream_state, stream_id, state})
  end

  @doc """
  Gets stream statistics.
  
  ## Parameters
  - `stream_id` - Stream ID
  
  ## Returns
  - `{:ok, stats}` on success
  - `{:error, reason}` on failure
  """
  def get_stream_stats(stream_id) do
    GenServer.call(__MODULE__, {:get_stream_stats, stream_id})
  end

  ## GenServer Callbacks

  @impl true
  def init(_opts) do
    # For now, we'll use a simple HTTP/2 client approach
    # In production, this would use the grpc library with proper channel management
    Logger.info("MediaEngineClient started, connecting to #{@grpc_server_url}")
    {:ok, %{grpc_url: @grpc_server_url, channel: nil}}
  end

  @impl true
  def handle_call({:create_stream, user_id, media_type, ssrc, codec, bitrate, encryption_enabled}, _from, state) do
    # Convert media_type atom to protobuf enum
    media_type_enum = case media_type do
      :audio -> 0  # AUDIO
      :video -> 1  # VIDEO
      _ -> 0
    end

    # Build request (simplified - in production would use proper protobuf encoding)
    request = %{
      config: %{
        user_id: user_id,
        media_type: media_type_enum,
        ssrc: ssrc,
        payload_type: 96,  # Default payload type
        codec: codec,
        bitrate: bitrate,
        encryption_enabled: encryption_enabled
      }
    }

    # Call gRPC service (placeholder - will be implemented with actual gRPC client)
    result = call_grpc_service("CreateStream", request, state)
    
    case result do
      {:ok, %{stream_id: stream_id, success: true}} ->
        Logger.info("Stream created: #{stream_id}")
        {:reply, {:ok, stream_id}, state}
      {:error, reason} ->
        Logger.error("gRPC call failed: #{inspect(reason)}")
        {:reply, {:error, "gRPC connection error: #{inspect(reason)}"}, state}
      other ->
        Logger.error("Unexpected response: #{inspect(other)}")
        {:reply, {:error, "Unexpected response from gRPC service"}, state}
    end
  end

  @impl true
  def handle_call({:stop_stream, stream_id}, _from, state) do
    request = %{stream_id: stream_id}
    result = call_grpc_service("StopStream", request, state)
    
    case result do
      {:ok, %{success: true}} ->
        Logger.info("Stream stopped: #{stream_id}")
        {:reply, :ok, state}
      {:error, reason} ->
        {:reply, {:error, "gRPC connection error: #{inspect(reason)}"}, state}
      other ->
        Logger.error("Unexpected response: #{inspect(other)}")
        {:reply, {:error, "Unexpected response from gRPC service"}, state}
    end
  end

  @impl true
  def handle_call({:get_stream, stream_id}, _from, state) do
    request = %{stream_id: stream_id}
    result = call_grpc_service("GetStream", request, state)
    
    case result do
      {:ok, response} ->
        {:reply, {:ok, response}, state}
      {:error, reason} ->
        {:reply, {:error, "gRPC connection error: #{inspect(reason)}"}, state}
    end
  end

  @impl true
  def handle_call({:route_packet, stream_id, rtp_packet}, _from, state) do
    request = %{
      stream_id: stream_id,
      rtp_packet: rtp_packet
    }
    result = call_grpc_service("RoutePacket", request, state)
    
    case result do
      {:ok, %{success: true, destination: destination}} ->
        {:reply, {:ok, destination}, state}
      {:error, reason} ->
        {:reply, {:error, "gRPC connection error: #{inspect(reason)}"}, state}
      other ->
        Logger.error("Unexpected response: #{inspect(other)}")
        {:reply, {:error, "Unexpected response from gRPC service"}, state}
    end
  end

  @impl true
  def handle_call({:encode_audio, stream_id, samples, sample_rate, channels, timestamp}, _from, state) do
    request = %{
      stream_id: stream_id,
      samples: samples,
      sample_rate: sample_rate,
      channels: channels,
      timestamp: timestamp
    }
    result = call_grpc_service("EncodeAudio", request, state)
    
    case result do
      {:ok, %{success: true, encoded_data: encoded_data}} ->
        {:reply, {:ok, encoded_data}, state}
      {:error, reason} ->
        {:reply, {:error, "gRPC connection error: #{inspect(reason)}"}, state}
      other ->
        Logger.error("Unexpected response: #{inspect(other)}")
        {:reply, {:error, "Unexpected response from gRPC service"}, state}
    end
  end

  @impl true
  def handle_call({:decode_audio, stream_id, encoded_data, timestamp}, _from, state) do
    request = %{
      stream_id: stream_id,
      encoded_data: encoded_data,
      timestamp: timestamp
    }
    result = call_grpc_service("DecodeAudio", request, state)
    
    case result do
      {:ok, %{success: true, samples: samples, sample_rate: sample_rate, channels: channels}} ->
        {:reply, {:ok, {samples, sample_rate, channels}}, state}
      {:error, reason} ->
        {:reply, {:error, "gRPC connection error: #{inspect(reason)}"}, state}
      other ->
        Logger.error("Unexpected response: #{inspect(other)}")
        {:reply, {:error, "Unexpected response from gRPC service"}, state}
    end
  end

  @impl true
  def handle_call({:update_stream_state, stream_id, state}, _from, server_state) do
    # Convert atom to protobuf enum
    state_enum = case state do
      :initializing -> 0
      :active -> 1
      :paused -> 2
      :stopped -> 3
      :error -> 4
      _ -> 0
    end

    request = %{
      stream_id: stream_id,
      new_state: state_enum
    }
    result = call_grpc_service("UpdateStreamState", request, server_state)
    
    case result do
      {:ok, %{success: true}} ->
        {:reply, :ok, server_state}
      {:error, _reason} ->
        {:reply, {:error, "gRPC connection error"}, server_state}
      other ->
        Logger.error("Unexpected response: #{inspect(other)}")
        {:reply, {:error, "Unexpected response from gRPC service"}, server_state}
    end
  end

  @impl true
  def handle_call({:get_stream_stats, stream_id}, _from, state) do
    request = %{stream_id: stream_id}
    result = call_grpc_service("GetStreamStats", request, state)
    
    case result do
      # Note: When real gRPC is implemented, add pattern for {:ok, %{exists: true, stats: stats}}
      # Currently mock always returns exists: false
      {:ok, %{exists: false}} ->
        {:reply, {:error, "Stream not found"}, state}
      {:error, reason} ->
        {:reply, {:error, "gRPC connection error: #{inspect(reason)}"}, state}
      other ->
        Logger.error("Unexpected response: #{inspect(other)}")
        {:reply, {:error, "Unexpected response from gRPC service"}, state}
    end
  end

  ## Private Functions

  # Placeholder for actual gRPC call
  # In production, this would use the grpc library to make actual gRPC calls
  defp call_grpc_service(method, _request, _state) do
    # TODO: Implement actual gRPC call using grpc library
    # For now, return a mock response to show the interface works
    Logger.warning("gRPC call to #{method} - using placeholder (actual implementation needed)")
    
    # Mock response for development
    case method do
      "CreateStream" ->
        {:ok, %{stream_id: UUID.uuid4(), success: true, error: ""}}
      "StopStream" ->
        {:ok, %{success: true, error: ""}}
      "GetStream" ->
        {:ok, %{exists: false, config: nil, state: 0}}
      "RoutePacket" ->
        {:ok, %{success: true, destination: nil, error: ""}}
      "EncodeAudio" ->
        {:ok, %{success: true, encoded_data: <<>>, error: ""}}
      "DecodeAudio" ->
        {:ok, %{success: true, samples: [], sample_rate: 16000, channels: 1, error: ""}}
      "UpdateStreamState" ->
        {:ok, %{success: true, error: ""}}
      "GetStreamStats" ->
        {:ok, %{exists: false, stats: nil}}
      _ ->
        {:error, "Unknown method: #{method}"}
    end
  end
end
