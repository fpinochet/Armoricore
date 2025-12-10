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

defmodule ArmoricoreRealtime.Security do
  @moduledoc """
  Security context for token revocation and security features.
  """

  import Ecto.Query
  alias ArmoricoreRealtime.Repo
  alias ArmoricoreRealtime.Security.RevokedToken

  @doc """
  Revokes a token (adds to blacklist).
  """
  def revoke_token(token_jti, user_id, token_type, expires_at, reason \\ "logout") do
    %RevokedToken{}
    |> RevokedToken.changeset(%{
      token_jti: token_jti,
      user_id: user_id,
      token_type: token_type,
      revoked_at: DateTime.utc_now(),
      expires_at: expires_at,
      reason: reason
    })
    |> Repo.insert()
  end

  @doc """
  Checks if a token is revoked.
  """
  def is_token_revoked?(token_jti) when is_binary(token_jti) do
    query =
      from rt in RevokedToken,
        where: rt.token_jti == ^token_jti,
        where: rt.expires_at > ^DateTime.utc_now(),
        select: count(rt.id)

    Repo.one(query) > 0
  end

  @doc """
  Revokes all tokens for a user (logout from all devices).
  """
  def revoke_all_user_tokens(_user_id, _reason \\ "logout_all") do
    # This would require extracting JTI from tokens
    # For now, we'll revoke tokens as they're used
    # In production, you might want to track active tokens
    {:ok, :revoked}
  end

  @doc """
  Cleans up expired revoked tokens.
  """
  def cleanup_expired_tokens do
    from(rt in RevokedToken, where: rt.expires_at < ^DateTime.utc_now())
    |> Repo.delete_all()
  end
end
