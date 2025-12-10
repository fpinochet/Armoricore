//! Device Token Database
//!
//! Handles storage and retrieval of device tokens for push notifications.
//! Supports both database-backed storage and fallback to event payloads.
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


use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use tokio_postgres::{Client, NoTls};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Device platform type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevicePlatform {
    #[serde(rename = "ios")]
    Ios,
    #[serde(rename = "android")]
    Android,
}

impl DevicePlatform {
    fn as_str(&self) -> &'static str {
        match self {
            DevicePlatform::Ios => "ios",
            DevicePlatform::Android => "android",
        }
    }
}

/// Device token record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToken {
    pub user_id: Uuid,
    pub device_token: String,
    pub platform: DevicePlatform,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Device token database client
pub struct DeviceTokenDb {
    client: Option<Client>,
}

impl DeviceTokenDb {
    /// Create a new device token database client
    pub async fn new() -> Result<Self> {
        // Check if database URL is configured
        let database_url = match env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                warn!("DATABASE_URL not configured, device token database disabled");
                return Ok(Self { client: None });
            }
        };

        info!("Connecting to device token database");

        // Parse connection string and connect
        let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

        // Spawn connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!(error = %e, "Database connection error");
            }
        });

        // Run migrations
        Self::run_migrations(&client).await?;

        info!("Device token database connected and initialized");

        Ok(Self {
            client: Some(client),
        })
    }

    /// Run database migrations
    async fn run_migrations(client: &Client) -> Result<()> {
        client
            .execute(
                r#"
                CREATE TABLE IF NOT EXISTS device_tokens (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    user_id UUID NOT NULL,
                    device_token TEXT NOT NULL,
                    platform VARCHAR(10) NOT NULL CHECK (platform IN ('ios', 'android')),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    UNIQUE(user_id, device_token)
                )
                "#,
                &[],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create device_tokens table: {}", e))?;

        // Create index on user_id for fast lookups
        client
            .execute(
                r#"
                CREATE INDEX IF NOT EXISTS idx_device_tokens_user_id 
                ON device_tokens(user_id)
                "#,
                &[],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create index: {}", e))?;

        // Create index on platform for filtering
        client
            .execute(
                r#"
                CREATE INDEX IF NOT EXISTS idx_device_tokens_platform 
                ON device_tokens(platform)
                "#,
                &[],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create index: {}", e))?;

        Ok(())
    }

    /// Check if database is available
    pub fn is_available(&self) -> bool {
        self.client.is_some()
    }

    /// Get all device tokens for a user
    pub async fn get_user_tokens(&self, user_id: &Uuid) -> Result<Vec<DeviceToken>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not configured"))?;

        let rows = client
            .query(
                r#"
                SELECT user_id, device_token, platform, created_at, updated_at
                FROM device_tokens
                WHERE user_id = $1
                ORDER BY updated_at DESC
                "#,
                &[user_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query device tokens: {}", e))?;

        let mut tokens = Vec::new();
        for row in rows {
            let platform_str: String = row.get(2);
            let platform = match platform_str.as_str() {
                "ios" => DevicePlatform::Ios,
                "android" => DevicePlatform::Android,
                _ => {
                    warn!("Unknown platform: {}, skipping", platform_str);
                    continue;
                }
            };

            tokens.push(DeviceToken {
                user_id: row.get(0),
                device_token: row.get(1),
                platform,
                created_at: row.get(3),
                updated_at: row.get(4),
            });
        }

        Ok(tokens)
    }

    /// Get device tokens for a user filtered by platform
    pub async fn get_user_tokens_by_platform(
        &self,
        user_id: &Uuid,
        platform: DevicePlatform,
    ) -> Result<Vec<DeviceToken>> {
        let all_tokens = self.get_user_tokens(user_id).await?;
        Ok(all_tokens
            .into_iter()
            .filter(|t| t.platform == platform)
            .collect())
    }

    /// Store or update a device token
    pub async fn store_token(
        &self,
        user_id: &Uuid,
        device_token: &str,
        platform: DevicePlatform,
    ) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not configured"))?;

        client
            .execute(
                r#"
                INSERT INTO device_tokens (user_id, device_token, platform, created_at, updated_at)
                VALUES ($1, $2, $3, NOW(), NOW())
                ON CONFLICT (user_id, device_token)
                DO UPDATE SET 
                    platform = EXCLUDED.platform,
                    updated_at = NOW()
                "#,
                &[user_id, &device_token, &platform.as_str()],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store device token: {}", e))?;

        info!(
            user_id = %user_id,
            platform = ?platform,
            "Device token stored"
        );

        Ok(())
    }

    /// Remove a device token
    pub async fn remove_token(&self, user_id: &Uuid, device_token: &str) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not configured"))?;

        client
            .execute(
                r#"
                DELETE FROM device_tokens
                WHERE user_id = $1 AND device_token = $2
                "#,
                &[user_id, &device_token],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to remove device token: {}", e))?;

        info!(
            user_id = %user_id,
            "Device token removed"
        );

        Ok(())
    }

    /// Remove all tokens for a user
    pub async fn remove_user_tokens(&self, user_id: &Uuid) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not configured"))?;

        client
            .execute(
                r#"
                DELETE FROM device_tokens
                WHERE user_id = $1
                "#,
                &[user_id],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to remove user tokens: {}", e))?;

        info!(
            user_id = %user_id,
            "All device tokens removed for user"
        );

        Ok(())
    }
}

