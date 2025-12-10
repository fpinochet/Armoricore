defmodule ArmoricoreRealtime.SecurityTest do
  use ExUnit.Case, async: true
  use ArmoricoreRealtimeWeb.ConnCase

  alias ArmoricoreRealtime.Security
  alias ArmoricoreRealtime.Security.RevokedToken
  alias ArmoricoreRealtime.Repo

  setup do
    # Clean up test data
    Repo.delete_all(RevokedToken)
    :ok
  end

  describe "revoke_token/5" do
    test "revokes a token successfully" do
      jti = Ecto.UUID.generate()
      user_id = Ecto.UUID.generate()
      expires_at = DateTime.utc_now() |> DateTime.add(3600, :second)

      assert {:ok, %RevokedToken{}} = Security.revoke_token(jti, user_id, "access", expires_at, "test")
    end

    test "prevents revoked token from being used" do
      jti = Ecto.UUID.generate()
      user_id = Ecto.UUID.generate()
      expires_at = DateTime.utc_now() |> DateTime.add(3600, :second)

      Security.revoke_token(jti, user_id, "access", expires_at, "test")
      assert Security.is_token_revoked?(jti) == true
    end

    test "non-revoked token is not revoked" do
      jti = Ecto.UUID.generate()
      assert Security.is_token_revoked?(jti) == false
    end
  end

  describe "is_token_revoked?/1" do
    test "returns true for revoked token" do
      jti = Ecto.UUID.generate()
      user_id = Ecto.UUID.generate()
      expires_at = DateTime.utc_now() |> DateTime.add(3600, :second)

      Security.revoke_token(jti, user_id, "access", expires_at, "test")
      assert Security.is_token_revoked?(jti) == true
    end

    test "returns false for non-revoked token" do
      jti = Ecto.UUID.generate()
      assert Security.is_token_revoked?(jti) == false
    end

    test "returns false for expired revoked token" do
      jti = Ecto.UUID.generate()
      user_id = Ecto.UUID.generate()
      expires_at = DateTime.utc_now() |> DateTime.add(-3600, :second)  # Expired 1 hour ago

      Security.revoke_token(jti, user_id, "access", expires_at, "test")
      # Cleanup expired tokens
      Security.cleanup_expired_tokens()
      assert Security.is_token_revoked?(jti) == false
    end
  end

  describe "cleanup_expired_tokens/0" do
    test "removes expired tokens" do
      jti1 = Ecto.UUID.generate()
      jti2 = Ecto.UUID.generate()
      user_id = Ecto.UUID.generate()

      # Create expired token
      expires_at1 = DateTime.utc_now() |> DateTime.add(-3600, :second)
      Security.revoke_token(jti1, user_id, "access", expires_at1, "test")

      # Create valid token
      expires_at2 = DateTime.utc_now() |> DateTime.add(3600, :second)
      Security.revoke_token(jti2, user_id, "access", expires_at2, "test")

      # Cleanup
      Security.cleanup_expired_tokens()

      # Expired token should be gone
      assert Security.is_token_revoked?(jti1) == false
      # Valid token should still be revoked
      assert Security.is_token_revoked?(jti2) == true
    end
  end
end

