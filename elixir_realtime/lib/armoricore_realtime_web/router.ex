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

defmodule ArmoricoreRealtimeWeb.Router do
  use ArmoricoreRealtimeWeb, :router

  pipeline :browser do
    plug :accepts, ["html"]
    plug :fetch_session
    plug :fetch_live_flash
    plug :put_root_layout, html: {ArmoricoreRealtimeWeb.Layouts, :root}
    plug :protect_from_forgery
    plug :put_secure_browser_headers
  end

  pipeline :api do
    plug :accepts, ["json"]
    plug ArmoricoreRealtimeWeb.Plugs.InputValidator
    plug ArmoricoreRealtimeWeb.Plugs.RateLimiter
  end

  pipeline :api_auth do
    plug :accepts, ["json"]
    plug ArmoricoreRealtimeWeb.Plugs.Authenticate
  end

  scope "/", ArmoricoreRealtimeWeb do
    pipe_through :browser

    get "/", PageController, :home
  end

  # LiveDashboard for monitoring
  import Phoenix.LiveDashboard.Router

  scope "/" do
    pipe_through :browser
    live_dashboard "/dashboard", metrics: ArmoricoreRealtimeWeb.Telemetry
  end

  # API routes
  scope "/api", ArmoricoreRealtimeWeb do
    pipe_through :api

    # Health check (public)
    get "/health", HealthController, :check

    # Authentication endpoints (public)
    post "/auth/login", AuthController, :login
    post "/auth/refresh", AuthController, :refresh
    post "/auth/logout", AuthController, :logout
    get "/auth/verify", AuthController, :verify

    # Protected routes (require authentication)
    scope "/", ArmoricoreRealtimeWeb do
      pipe_through :api_auth

      # Add protected routes here
      # Example:
      # get "/users/me", UserController, :me
    end
  end
end
