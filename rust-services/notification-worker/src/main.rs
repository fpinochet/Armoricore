//! Notification Worker
//!
//! Consumes `notification.requested` events from the message bus and sends
//! push notifications or emails. Publishes `notification.sent` or `notification.failed` events.
// Copyright 2025 Francisco F. Pinochet
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.


mod database;
mod dead_letter_queue;
mod health;
mod rate_limiter;
mod retry;
mod sender;
mod worker;

use anyhow::Result;
use armoricore_config::AppConfig;
use armoricore_logging::init_console_logging;
use message_bus_client::nats::NatsClient;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_console_logging("notification-worker", "info");

    info!("Starting Notification Worker");

    // Load configuration
    let config = AppConfig::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

    info!(
        message_bus_url = config.message_bus_url(),
        "Configuration loaded"
    );

    // Start health check server in background
    let health_port = std::env::var("HEALTH_CHECK_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8081);
    
    let health_server = health::HealthServer::new(health_port);
    let health_handle = tokio::spawn(async move {
        if let Err(e) = health_server.start().await {
            error!(error = %e, "Health check server error");
        }
    });

    // Connect to message bus
    let message_bus = NatsClient::new(
        config.message_bus_url(),
        config.message_bus.stream_name.clone(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to connect to message bus: {}", e))?;

    info!("Connected to message bus");

    // Create worker
    let worker = worker::NotificationWorker::new(Arc::new(message_bus))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create worker: {}", e))?;

    // Start processing events
    info!("Starting event processing");
    let worker_handle = tokio::spawn(async move {
        if let Err(e) = worker.run().await {
            error!(error = %e, "Worker error");
        }
    });

    // Wait for shutdown signal
    info!("Notification Worker running. Press Ctrl+C to stop.");
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received, initiating graceful shutdown");
        }
        Err(err) => {
            error!(error = %err, "Unable to listen for shutdown signal");
        }
    }

    // Graceful shutdown: wait for worker to finish current operations
    info!("Waiting for in-flight operations to complete...");
    tokio::select! {
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
            warn!("Shutdown timeout reached, forcing shutdown");
        }
        _ = worker_handle => {
            info!("Worker completed gracefully");
        }
    }

    // Cancel health check server
    health_handle.abort();
    info!("Notification Worker stopped");

    Ok(())
}
