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

defmodule ArmoricoreRealtime.E2EE do
  @moduledoc """
  End-to-End Encryption (E2EE) Module for Text Messages
  
  This module provides optional E2EE functionality for text-only messages.
  Messages are encrypted on the client side and can only be decrypted by
  authorized recipients. The server never has access to plaintext.
  
  Features:
  - AES-256-GCM encryption for messages
  - Per-message encryption keys (derived from shared secret)
  - Key exchange support (ECDH)
  - Message authentication (GCM tag)
  - Optional E2EE for channels
  
  Note: This is a server-side utility module. Actual encryption/decryption
  should be performed on the client side for true E2EE. This module provides
  key management and validation.
  """

  alias ArmoricoreRealtime.E2EE.Key
  alias ArmoricoreRealtime.Repo
  import Ecto.Query

  @doc """
  Encrypts a text message using AES-256-GCM.
  
  This is a server-side utility. In true E2EE, encryption happens on the client.
  This function is provided for server-side processing if needed.
  
  ## Parameters
  - `plaintext` - The message text to encrypt
  - `key` - 32-byte encryption key (binary)
  - `nonce` - 12-byte nonce (binary, optional - will be generated if not provided)
  
  ## Returns
  - `{:ok, %{ciphertext: binary, nonce: binary, tag: binary}}` on success
  - `{:error, reason}` on failure
  """
  @spec encrypt_message(String.t(), binary(), binary() | nil) ::
          {:ok, %{ciphertext: binary(), nonce: binary(), tag: binary()}} | {:error, String.t()}
  def encrypt_message(plaintext, key, nonce \\ nil) when is_binary(plaintext) and is_binary(key) do
    # Validate key size (32 bytes for AES-256)
    if byte_size(key) != 32 do
      {:error, "Encryption key must be exactly 32 bytes"}
    else
      # Generate nonce if not provided
      nonce = nonce || :crypto.strong_rand_bytes(12)

      # Encrypt using AES-256-GCM
      try do
        {ciphertext, tag} =
          :crypto.crypto_one_time_aead(:aes_256_gcm, key, nonce, plaintext, "", true)

        {:ok, %{ciphertext: ciphertext, nonce: nonce, tag: tag}}
      rescue
        e -> {:error, "Encryption failed: #{inspect(e)}"}
      end
    end
  end

  @doc """
  Decrypts an encrypted message using AES-256-GCM.
  
  This is a server-side utility. In true E2EE, decryption happens on the client.
  This function is provided for server-side processing if needed.
  
  ## Parameters
  - `ciphertext` - The encrypted message (binary)
  - `key` - 32-byte decryption key (binary)
  - `nonce` - 12-byte nonce (binary)
  - `tag` - 16-byte authentication tag (binary)
  
  ## Returns
  - `{:ok, plaintext}` on success
  - `{:error, reason}` on failure
  """
  @spec decrypt_message(binary(), binary(), binary(), binary()) ::
          {:ok, String.t()} | {:error, String.t()}
  def decrypt_message(ciphertext, key, nonce, tag)
      when is_binary(ciphertext) and is_binary(key) and is_binary(nonce) and is_binary(tag) do
    # Validate key size (32 bytes for AES-256)
    if byte_size(key) != 32 do
      {:error, "Decryption key must be exactly 32 bytes"}
    else
      # Validate nonce size (12 bytes for GCM)
      if byte_size(nonce) != 12 do
        {:error, "Nonce must be exactly 12 bytes"}
      else
        # Validate tag size (16 bytes for GCM)
        if byte_size(tag) != 16 do
          {:error, "Tag must be exactly 16 bytes"}
        else
          # Decrypt using AES-256-GCM
          try do
            {plaintext, _tag} =
              :crypto.crypto_one_time_aead(:aes_256_gcm, key, nonce, ciphertext, tag, false)

            {:ok, plaintext}
          rescue
            e -> {:error, "Decryption failed: #{inspect(e)}"}
          end
        end
      end
    end
  end

  @doc """
  Generates a random encryption key (32 bytes for AES-256).
  
  ## Returns
  - Random 32-byte binary key
  """
  @spec generate_key() :: binary()
  def generate_key do
    :crypto.strong_rand_bytes(32)
  end

  @doc """
  Generates a random nonce (12 bytes for AES-256-GCM).
  
  ## Returns
  - Random 12-byte binary nonce
  """
  @spec generate_nonce() :: binary()
  def generate_nonce do
    :crypto.strong_rand_bytes(12)
  end

  @doc """
  Derives an encryption key from a shared secret using HKDF.
  
  This is useful for deriving per-message keys from a shared secret
  established via key exchange (e.g., ECDH).
  
  ## Parameters
  - `shared_secret` - The shared secret (binary)
  - `salt` - Optional salt (binary, defaults to empty)
  - `info` - Optional context info (binary, defaults to "e2ee")
  
  ## Returns
  - 32-byte derived key (binary)
  """
  @spec derive_key(binary(), binary() | nil, binary() | nil) :: binary()
  def derive_key(shared_secret, salt \\ nil, info \\ "e2ee") do
    salt = salt || ""
    # Use HKDF-SHA256 to derive a 32-byte key
    # Simplified HKDF: HMAC-SHA256(shared_secret, salt || info)
    derived = :crypto.mac(:hmac, :sha256, shared_secret, salt <> info)
    # Ensure we have at least 32 bytes (SHA256 produces 32 bytes, so this is safe)
    binary_part(derived, 0, min(32, byte_size(derived)))
  end

  @doc """
  Validates that a message is properly encrypted (has required fields).
  
  ## Parameters
  - `encrypted_message` - Map with encrypted message data
  
  ## Returns
  - `true` if message has all required fields
  - `false` otherwise
  """
  @spec is_valid_encrypted_message(map()) :: boolean()
  def is_valid_encrypted_message(%{ciphertext: c, nonce: n, tag: t})
      when is_binary(c) and is_binary(n) and is_binary(t) do
    byte_size(n) == 12 and byte_size(t) == 16
  end

  def is_valid_encrypted_message(_), do: false

  @doc """
  Formats an encrypted message for transmission.
  
  Encodes the encrypted message components (ciphertext, nonce, tag) into
  a format suitable for JSON transmission.
  
  ## Parameters
  - `encrypted_data` - Map with ciphertext, nonce, and tag
  
  ## Returns
  - Map with base64-encoded fields
  """
  @spec format_encrypted_message(map()) :: map()
  def format_encrypted_message(%{ciphertext: c, nonce: n, tag: t}) do
    %{
      ciphertext: Base.encode64(c),
      nonce: Base.encode64(n),
      tag: Base.encode64(t),
      encrypted: true
    }
  end

  @doc """
  Parses an encrypted message from transmission format.
  
  Decodes base64-encoded encrypted message components.
  
  ## Parameters
  - `formatted_message` - Map with base64-encoded fields
  
  ## Returns
  - `{:ok, map}` with binary fields on success
  - `{:error, reason}` on failure
  """
  @spec parse_encrypted_message(map()) ::
          {:ok, map()} | {:error, String.t()}
  def parse_encrypted_message(%{ciphertext: c, nonce: n, tag: t}) do
    try do
      {:ok,
       %{
         ciphertext: Base.decode64!(c),
         nonce: Base.decode64!(n),
         tag: Base.decode64!(t)
       }}
    rescue
      e -> {:error, "Failed to decode encrypted message: #{inspect(e)}"}
    end
  end

  def parse_encrypted_message(_), do: {:error, "Invalid encrypted message format"}

  # Key Management Functions

  @doc """
  Stores a public key for a user (for key exchange).
  
  ## Parameters
  - `user_id` - The user ID
  - `public_key` - The public key (binary)
  - `channel_id` - Optional channel ID for channel-specific keys
  - `key_type` - Key type (default: "ecdh_p256")
  
  ## Returns
  - `{:ok, %Key{}}` on success
  - `{:error, changeset}` on failure
  """
  @spec store_public_key(binary(), binary(), String.t() | nil, String.t()) ::
          {:ok, Key.t()} | {:error, Ecto.Changeset.t()}
  def store_public_key(user_id, public_key, channel_id \\ nil, key_type \\ "ecdh_p256") do
    # Deactivate old keys for this user/channel
    if channel_id do
      Repo.update_all(
        from(k in Key, where: k.user_id == ^user_id and k.channel_id == ^channel_id),
        set: [is_active: false]
      )
    end

    %Key{}
    |> Key.changeset(%{
      user_id: user_id,
      channel_id: channel_id,
      public_key: public_key,
      key_type: key_type,
      is_active: true
    })
    |> Repo.insert()
  end

  @doc """
  Gets the active public key for a user.
  
  ## Parameters
  - `user_id` - The user ID
  - `channel_id` - Optional channel ID
  
  ## Returns
  - `{:ok, %Key{}}` if found
  - `{:error, :not_found}` if not found
  """
  @spec get_public_key(binary(), String.t() | nil) ::
          {:ok, Key.t()} | {:error, :not_found}
  def get_public_key(user_id, channel_id \\ nil) do
    query =
      from(k in Key,
        where: k.user_id == ^user_id and k.is_active == true,
        order_by: [desc: k.inserted_at],
        limit: 1
      )

    query = 
      if channel_id do
        from(k in query, where: k.channel_id == ^channel_id)
      else
        query
      end

    case Repo.one(query) do
      nil -> {:error, :not_found}
      key -> {:ok, key}
    end
  end

  @doc """
  Gets all active public keys for a channel (for group E2EE).
  
  ## Parameters
  - `channel_id` - The channel ID
  
  ## Returns
  - List of active keys
  """
  @spec get_channel_keys(String.t()) :: [Key.t()]
  def get_channel_keys(channel_id) do
    from(k in Key,
      where: k.channel_id == ^channel_id and k.is_active == true,
      order_by: [desc: k.inserted_at]
    )
    |> Repo.all()
  end

  @doc """
  Revokes (deactivates) a user's public key.
  
  ## Parameters
  - `user_id` - The user ID
  - `channel_id` - Optional channel ID
  
  ## Returns
  - `:ok`
  """
  @spec revoke_key(binary(), String.t() | nil) :: :ok
  def revoke_key(user_id, channel_id \\ nil) do
    query = from(k in Key, where: k.user_id == ^user_id and k.is_active == true)

    query = 
      if channel_id do
        from(k in query, where: k.channel_id == ^channel_id)
      else
        query
      end

    Repo.update_all(query, set: [is_active: false])
    :ok
  end
end
