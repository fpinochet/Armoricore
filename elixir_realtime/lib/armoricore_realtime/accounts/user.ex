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

defmodule ArmoricoreRealtime.Accounts.User do
  @moduledoc """
  User schema for accounts and profiles.
  """

  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  schema "users" do
    field :email, :string
    field :password_hash, :string
    field :password, :string, virtual: true, redact: true
    field :password_confirmation, :string, virtual: true, redact: true
    field :username, :string
    field :first_name, :string
    field :last_name, :string
    field :avatar_url, :string
    field :is_active, :boolean, default: true
    field :is_verified, :boolean, default: false
    field :last_login_at, :utc_datetime
    field :metadata, :map, default: %{}

    timestamps(type: :utc_datetime)
  end

  @doc false
  def changeset(user, attrs) do
    user
    |> cast(attrs, [:email, :username, :first_name, :last_name, :avatar_url, :is_active, :is_verified, :metadata])
    |> validate_required([:email])
    |> validate_format(:email, ~r/^[^\s]+@[^\s]+$/, message: "must have the @ sign and no spaces")
    |> validate_length(:email, max: 255)
    |> validate_length(:username, max: 100)
    |> unique_constraint(:email)
    |> unique_constraint(:username)
  end

  @doc """
  Changeset for user registration (includes password).
  """
  def registration_changeset(user, attrs, opts \\ []) do
    user
    |> changeset(attrs)
    |> cast(attrs, [:password])
    |> validate_required([:password])
    |> validate_length(:password, min: 8, max: 72)
    |> validate_confirmation(:password, required: true)
    |> maybe_hash_password(opts)
  end

  @doc """
  Changeset for password updates.
  """
  def password_changeset(user, attrs, opts \\ []) do
    user
    |> cast(attrs, [:password])
    |> validate_confirmation(:password, required: true)
    |> validate_length(:password, min: 8, max: 72)
    |> maybe_hash_password(opts)
  end

  defp maybe_hash_password(changeset, opts) do
    hash_password? = Keyword.get(opts, :hash_password, true)
    password = get_change(changeset, :password)

    if hash_password? && password && changeset.valid? do
      changeset
      |> validate_length(:password, min: 8, max: 72)
      |> put_change(:password_hash, Bcrypt.hash_pwd_salt(password))
      |> delete_change(:password)
    else
      changeset
    end
  end

  @doc """
  Verifies the password.
  """
  def valid_password?(%ArmoricoreRealtime.Accounts.User{password_hash: hash}, password)
      when is_binary(hash) and byte_size(password) > 0 do
    Bcrypt.verify_pass(password, hash)
  end

  def valid_password?(_, _) do
    Bcrypt.no_user_verify()
    false
  end

  @doc """
  Updates the last login timestamp.
  """
  def update_last_login_changeset(user) do
    change(user, last_login_at: DateTime.utc_now())
  end
end
