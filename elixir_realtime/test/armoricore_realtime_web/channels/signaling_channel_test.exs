defmodule ArmoricoreRealtimeWeb.SignalingChannelTest do
  use ExUnit.Case, async: true
  use Phoenix.ChannelTest

  @endpoint ArmoricoreRealtimeWeb.Endpoint

  alias ArmoricoreRealtimeWeb.UserSocket
  alias ArmoricoreRealtimeWeb.SignalingChannel
  alias ArmoricoreRealtime.JWT
  alias ArmoricoreRealtime.CallManager

  @secret "test-secret-key"
  @caller_id "123e4567-e89b-12d3-a456-426614174000"
  @callee_id "223e4567-e89b-12d3-a456-426614174001"
  @call_id "call-123"

  setup do
    Application.put_env(:armoricore_realtime, :jwt, secret: @secret)
    
    # Start CallManager if not already started
    start_supervised!(CallManager)
    
    # Create valid token for caller
    signer = Joken.Signer.create("HS256", @secret)
    caller_claims = %{
      "user_id" => @caller_id,
      "exp" => System.system_time(:second) + 3600
    }
    {:ok, caller_token, _} = Joken.encode_and_sign(caller_claims, signer)
    
    # Create valid token for callee
    callee_claims = %{
      "user_id" => @callee_id,
      "exp" => System.system_time(:second) + 3600
    }
    {:ok, callee_token, _} = Joken.encode_and_sign(callee_claims, signer)
    
    on_exit(fn ->
      Application.delete_env(:armoricore_realtime, :jwt)
    end)

    {:ok, 
     caller_token: caller_token, 
     callee_token: callee_token,
     caller_id: @caller_id,
     callee_id: @callee_id,
     call_id: @call_id}
  end

  describe "join/3" do
    test "allows caller to join signaling channel", %{caller_token: token, caller_id: caller_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{caller_id}", %{user_id: caller_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => @callee_id
        })

      assert socket.assigns.call_id == call_id
      assert socket.assigns.user_id == caller_id
      assert socket.assigns.caller_id == caller_id
      assert socket.assigns.callee_id == @callee_id
    end

    test "allows callee to join signaling channel", %{callee_id: callee_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{callee_id}", %{user_id: callee_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => @caller_id,
          "callee_id" => callee_id
        })

      assert socket.assigns.call_id == call_id
      assert socket.assigns.user_id == callee_id
    end

    test "rejects unauthorized user", %{call_id: call_id} do
      unauthorized_id = "999e4567-e89b-12d3-a456-426614174999"
      
      assert {:error, %{reason: "unauthorized"}} =
        socket(UserSocket, "user_socket:#{unauthorized_id}", %{user_id: unauthorized_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => @caller_id,
          "callee_id" => @callee_id
        })
    end
  end

  describe "handle_in call_initiate" do
    test "initiates a call successfully", %{caller_id: caller_id, callee_id: callee_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{caller_id}", %{user_id: caller_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      ref = push(socket, "call_initiate", %{
        "callee_id" => callee_id,
        "call_type" => "video"
      })

      assert_reply ref, :ok, call_metadata
      assert call_metadata.caller_id == caller_id
      assert call_metadata.callee_id == callee_id
      assert call_metadata.call_type == "video"
      assert call_metadata.state == "ringing"
      assert Map.has_key?(call_metadata, :initiated_at)

      assert_broadcast "call_initiated", ^call_metadata
    end

    test "initiates voice call", %{caller_id: caller_id, callee_id: callee_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{caller_id}", %{user_id: caller_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      ref = push(socket, "call_initiate", %{
        "callee_id" => callee_id,
        "call_type" => "voice"
      })

      assert_reply ref, :ok, call_metadata
      assert call_metadata.call_type == "voice"
    end
  end

  describe "handle_in call_offer" do
    test "sends SDP offer successfully", %{caller_id: caller_id, callee_id: callee_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{caller_id}", %{user_id: caller_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      sdp_offer = "v=0\r\no=- 1234567890 1234567890 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n"
      
      ref = push(socket, "call_offer", %{
        "sdp" => sdp_offer,
        "type" => "offer"
      })

      assert_reply ref, :ok, offer
      assert offer.sdp == sdp_offer
      assert offer.type == "offer"
      assert offer.call_id == call_id

      assert_broadcast "call_offer", ^offer
    end
  end

  describe "handle_in call_answer" do
    test "sends SDP answer successfully", %{callee_id: callee_id, caller_id: caller_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{callee_id}", %{user_id: callee_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      sdp_answer = "v=0\r\no=- 9876543210 9876543210 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n"
      
      ref = push(socket, "call_answer", %{
        "sdp" => sdp_answer,
        "type" => "answer"
      })

      assert_reply ref, :ok, answer
      assert answer.sdp == sdp_answer
      assert answer.type == "answer"
      assert answer.call_id == call_id

      assert_broadcast "call_answer", ^answer
    end
  end

  describe "handle_in ice_candidate" do
    test "exchanges ICE candidate successfully", %{caller_id: caller_id, callee_id: callee_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{caller_id}", %{user_id: caller_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      candidate = %{
        "candidate" => "candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host",
        "sdpMLineIndex" => 0,
        "sdpMid" => "0"
      }
      
      ref = push(socket, "ice_candidate", candidate)

      assert_reply ref, :ok, result
      assert result.call_id == call_id

      assert_broadcast "ice_candidate", broadcasted_candidate
      assert broadcasted_candidate.candidate == candidate["candidate"]
    end
  end

  describe "handle_in call_end" do
    test "ends call successfully", %{caller_id: caller_id, callee_id: callee_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{caller_id}", %{user_id: caller_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      ref = push(socket, "call_end", %{"reason" => "user_hangup"})

      assert_reply ref, :ok, result
      assert result.call_id == call_id
      assert result.reason == "user_hangup"

      assert_broadcast "call_ended", ended_call
      assert ended_call.call_id == call_id
    end
  end

  describe "handle_in call_reject" do
    test "rejects call successfully", %{callee_id: callee_id, caller_id: caller_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{callee_id}", %{user_id: callee_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      ref = push(socket, "call_reject", %{"reason" => "busy"})

      assert_reply ref, :ok, result
      assert result.call_id == call_id
      assert result.reason == "busy"

      assert_broadcast "call_rejected", rejected_call
      assert rejected_call.call_id == call_id
    end
  end

  describe "error handling" do
    test "handles missing required fields in call_initiate", %{caller_id: caller_id, callee_id: callee_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{caller_id}", %{user_id: caller_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      ref = push(socket, "call_initiate", %{})

      assert_reply ref, :error, %{reason: _}
    end

    test "handles invalid SDP in call_offer", %{caller_id: caller_id, callee_id: callee_id, call_id: call_id} do
      {:ok, _, socket} =
        socket(UserSocket, "user_socket:#{caller_id}", %{user_id: caller_id})
        |> subscribe_and_join(SignalingChannel, "signaling:call:#{call_id}", %{
          "caller_id" => caller_id,
          "callee_id" => callee_id
        })

      ref = push(socket, "call_offer", %{"sdp" => "", "type" => "offer"})

      assert_reply ref, :error, %{reason: _}
    end
  end
end

