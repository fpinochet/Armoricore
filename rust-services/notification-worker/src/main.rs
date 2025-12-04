//! Notification Worker
//!
//! Consumes `notification.requested` events from the message bus and sends
//! push notifications or emails. Publishes `notification.sent` or `notification.failed` events.

mod worker;
mod sender;

use anyhow::Result;
use armoricore_config::AppConfig;
use armoricore_logging::init_console_logging;
use message_bus_client::nats::NatsClient;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

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

    // Connect to message bus
    let message_bus = NatsClient::new(
        config.message_bus_url(),
        config.message_bus.stream_name.clone(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to connect to message bus: {}", e))?;

    info!("Connected to message bus");

    // Create worker
    let worker = worker::NotificationWorker::new(Arc::new(message_bus));

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
            info!("Shutdown signal received");
        }
        Err(err) => {
            error!(error = %err, "Unable to listen for shutdown signal");
        }
    }

    // Cancel the worker
    worker_handle.abort();
    info!("Notification Worker stopped");

    Ok(())
}
