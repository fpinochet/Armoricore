ExUnit.start()

# Start Ecto repository for database tests
Ecto.Adapters.SQL.Sandbox.mode(ArmoricoreRealtime.Repo, :manual)
