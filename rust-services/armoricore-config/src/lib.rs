//! Configuration management for Armoricore services

use serde::Deserialize;
use std::env;

/// Message bus configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MessageBusConfig {
    pub url: String,
    pub stream_name: Option<String>,
    pub subject_prefix: Option<String>,
}

/// Object storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ObjectStorageConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: Option<String>,
}

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub message_bus: MessageBusConfig,
    pub object_storage: Option<ObjectStorageConfig>,
    pub log_level: Option<String>,
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, config::ConfigError> {
        // Load .env file if it exists
        let _ = dotenvy::dotenv();

        let message_bus_url = env::var("MESSAGE_BUS_URL")
            .unwrap_or_else(|_| "nats://localhost:4222".to_string());
        
        let stream_name = env::var("MESSAGE_BUS_STREAM_NAME").ok();
        
        let log_level = env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());

        // Build object storage config if all required vars are present
        let object_storage = if let (Ok(endpoint), Ok(access_key), Ok(secret_key), Ok(bucket)) = (
            env::var("OBJECT_STORAGE_ENDPOINT"),
            env::var("OBJECT_STORAGE_ACCESS_KEY"),
            env::var("OBJECT_STORAGE_SECRET_KEY"),
            env::var("OBJECT_STORAGE_BUCKET"),
        ) {
            Some(ObjectStorageConfig {
                endpoint,
                access_key,
                secret_key,
                bucket,
                region: env::var("OBJECT_STORAGE_REGION").ok(),
            })
        } else {
            None
        };

        Ok(Self {
            message_bus: MessageBusConfig {
                url: message_bus_url,
                stream_name,
                subject_prefix: Some("armoricore".to_string()),
            },
            object_storage,
            log_level: Some(log_level),
        })
    }

    /// Get message bus URL
    pub fn message_bus_url(&self) -> &str {
        &self.message_bus.url
    }

    /// Get log level, defaulting to "info"
    pub fn log_level(&self) -> &str {
        self.log_level.as_deref().unwrap_or("info")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // This test would require mocking environment variables
        // For now, just verify the struct compiles
        let _config = MessageBusConfig {
            url: "nats://localhost:4222".to_string(),
            stream_name: None,
            subject_prefix: None,
        };
    }
}

