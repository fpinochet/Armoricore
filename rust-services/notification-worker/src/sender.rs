//! Notification Sender
//!
//! Handles sending push notifications (FCM/APNS) and emails (SMTP).
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


use crate::database::{DevicePlatform, DeviceTokenDb};
use armoricore_types::schemas::NotificationType;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use lettre::{
    message::{header::ContentType, Message, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use pem::parse;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Cached APNS JWT token with expiration
struct ApnsJwtToken {
    token: String,
    expires_at: u64,
}

/// Notification sender that handles different notification types
pub struct NotificationSender {
    http_client: Client,
    device_token_db: Option<Arc<DeviceTokenDb>>,
    fcm_api_key: Option<String>,
    apns_key_id: Option<String>,
    apns_team_id: Option<String>,
    apns_bundle_id: Option<String>,
    apns_key_path: Option<String>,
    apns_key_contents: Option<Vec<u8>>, // Cached key contents
    apns_jwt_token: Arc<Mutex<Option<ApnsJwtToken>>>, // Cached JWT token
    apns_use_sandbox: bool, // Use sandbox endpoint for development
    smtp_relay: Option<String>,
    smtp_username: Option<String>,
    smtp_password: Option<String>,
    smtp_from: Option<String>,
}

impl NotificationSender {
    /// Create a new notification sender (using environment variables)
    pub fn new() -> Self {
        let apns_key_path = env::var("APNS_KEY_PATH").ok();
        let apns_key_contents = apns_key_path
            .as_ref()
            .and_then(|path| fs::read(path).ok());
        
        let apns_use_sandbox = env::var("APNS_USE_SANDBOX")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        Self {
            http_client: Client::new(),
            device_token_db: None, // Will be set via set_device_token_db
            fcm_api_key: env::var("FCM_API_KEY").ok(),
            apns_key_id: env::var("APNS_KEY_ID").ok(),
            apns_team_id: env::var("APNS_TEAM_ID").ok(),
            apns_bundle_id: env::var("APNS_BUNDLE_ID").ok(),
            apns_key_path,
            apns_key_contents,
            apns_jwt_token: Arc::new(Mutex::new(None)),
            apns_use_sandbox,
            smtp_relay: env::var("SMTP_RELAY").ok(),
            smtp_username: env::var("SMTP_USERNAME").ok(),
            smtp_password: env::var("SMTP_PASSWORD").ok(),
            smtp_from: env::var("SMTP_FROM").ok(),
        }
    }

    /// Create a new notification sender with keys from key store
    pub async fn with_key_store(
        key_store: &armoricore_keys::KeyStore,
    ) -> anyhow::Result<Self> {
        use armoricore_keys::service_integration::*;

        let apns_key_path = env::var("APNS_KEY_PATH").ok();
        let apns_key_contents = if let Some(path) = &apns_key_path {
            fs::read(path).ok()
        } else {
            // Try to load from key store if path not provided
            // Note: Key store typically stores secrets, not file paths
            // For now, we'll still require APNS_KEY_PATH for the .p8 file
            None
        };

        let apns_use_sandbox = env::var("APNS_USE_SANDBOX")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        Ok(Self {
            http_client: Client::new(),
            device_token_db: None, // Will be set via set_device_token_db
            fcm_api_key: get_fcm_api_key(key_store).await,
            apns_key_id: get_apns_key_id(key_store).await,
            apns_team_id: get_apns_team_id(key_store).await,
            apns_bundle_id: get_apns_bundle_id(key_store).await,
            apns_key_path, // File path, not a secret
            apns_key_contents,
            apns_jwt_token: Arc::new(Mutex::new(None)),
            apns_use_sandbox,
            smtp_relay: env::var("SMTP_RELAY").ok(), // Not a secret
            smtp_username: get_smtp_username(key_store).await,
            smtp_password: get_smtp_password(key_store).await,
            smtp_from: env::var("SMTP_FROM").ok(), // Not a secret
        })
    }

    /// Set the device token database (optional)
    pub fn with_device_token_db(mut self, db: Arc<DeviceTokenDb>) -> Self {
        self.device_token_db = Some(db);
        self
    }

    /// Send a notification
    pub async fn send_notification(
        &self,
        user_id: &Uuid,
        notification_type: &NotificationType,
        title: &str,
        body: &str,
        data: &Value,
    ) -> anyhow::Result<()> {
        info!(
            user_id = %user_id,
            notification_type = ?notification_type,
            title = title,
            "Sending notification"
        );

        match notification_type {
            NotificationType::Push => {
                self.send_push_notification(user_id, title, body, data).await
            }
            NotificationType::Email => self.send_email(user_id, title, body, data).await,
        }
    }

    /// Send a push notification (FCM for Android, APNS for iOS)
    async fn send_push_notification(
        &self,
        user_id: &Uuid,
        title: &str,
        body: &str,
        data: &Value,
    ) -> anyhow::Result<()> {
        let mut errors = Vec::new();
        let mut sent_count = 0;

        // Try to get device tokens from database, fall back to event payload
        let android_tokens = if let Some(ref db) = self.device_token_db {
            // Use database if available
            db.get_user_tokens_by_platform(user_id, DevicePlatform::Android)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.device_token)
                .collect::<Vec<_>>()
        } else {
            // Fall back to event payload
            data.get("device_token")
                .and_then(|v| v.as_str())
                .map(|t| vec![t.to_string()])
                .unwrap_or_default()
        };

        let ios_tokens = if let Some(ref db) = self.device_token_db {
            // Use database if available
            db.get_user_tokens_by_platform(user_id, DevicePlatform::Ios)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.device_token)
                .collect::<Vec<_>>()
        } else {
            // Fall back to event payload
            data.get("device_token")
                .and_then(|v| v.as_str())
                .map(|t| vec![t.to_string()])
                .unwrap_or_default()
        };

        // Send to Android devices (FCM)
        if let Some(ref fcm_key) = self.fcm_api_key {
            if android_tokens.is_empty() {
                warn!("No Android device tokens found for user");
            } else {
                for token in android_tokens {
                    let mut token_data = data.clone();
                    token_data["device_token"] = json!(token);
                    if let Err(e) = self.send_fcm_notification(user_id, title, body, &token_data, fcm_key).await {
                        warn!(error = %e, "Failed to send FCM notification");
                        errors.push(format!("FCM: {}", e));
                    } else {
                        sent_count += 1;
                    }
                }
            }
        } else {
            warn!("FCM_API_KEY not configured, skipping Android push");
        }

        // Send to iOS devices (APNS)
        if self.apns_key_id.is_some() && self.apns_team_id.is_some() {
            if ios_tokens.is_empty() {
                warn!("No iOS device tokens found for user");
            } else {
                for token in ios_tokens {
                    let mut token_data = data.clone();
                    token_data["device_token"] = json!(token);
                    if let Err(e) = self.send_apns_notification(user_id, title, body, &token_data).await {
                        warn!(error = %e, "Failed to send APNS notification");
                        errors.push(format!("APNS: {}", e));
                    } else {
                        sent_count += 1;
                    }
                }
            }
        } else {
            warn!("APNS credentials not configured, skipping iOS push");
        }

        if sent_count == 0 && self.fcm_api_key.is_none() && self.apns_key_id.is_none() {
            return Err(anyhow::anyhow!(
                "No push notification service configured. Set FCM_API_KEY or APNS_* variables."
            ));
        }

        if sent_count == 0 {
            return Err(anyhow::anyhow!(
                "No device tokens found for user. Configure DATABASE_URL or provide device_token in event data."
            ));
        }

        if !errors.is_empty() {
            warn!(
                sent = sent_count,
                errors = errors.len(),
                "Some notifications failed, but {} succeeded",
                sent_count
            );
            // Don't fail completely if some succeeded
        }

        info!(
            user_id = %user_id,
            sent = sent_count,
            "Push notifications sent"
        );

        Ok(())
    }

    /// Send FCM (Firebase Cloud Messaging) notification
    async fn send_fcm_notification(
        &self,
        user_id: &Uuid,
        title: &str,
        body: &str,
        data: &Value,
        api_key: &str,
    ) -> anyhow::Result<()> {
        // TODO: Get device token from database based on user_id
        // For now, this is a placeholder that shows the structure
        
        let device_token = data
            .get("device_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("device_token not provided in data"))?;

        let payload = json!({
            "to": device_token,
            "notification": {
                "title": title,
                "body": body
            },
            "data": data
        });

        let response = self
            .http_client
            .post("https://fcm.googleapis.com/fcm/send")
            .header("Authorization", format!("key={}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "FCM API error: {} - {}",
                status,
                error_text
            ));
        }

        info!(
            user_id = %user_id,
            device_token = device_token,
            "FCM notification sent successfully"
        );

        Ok(())
    }

    /// Send APNS (Apple Push Notification Service) notification
    async fn send_apns_notification(
        &self,
        user_id: &Uuid,
        title: &str,
        body: &str,
        data: &Value,
    ) -> anyhow::Result<()> {
        // Get required APNS configuration
        let key_id = self
            .apns_key_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("APNS_KEY_ID not configured"))?;
        let team_id = self
            .apns_team_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("APNS_TEAM_ID not configured"))?;
        let bundle_id = self
            .apns_bundle_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("APNS_BUNDLE_ID not configured"))?;
        let key_path = self
            .apns_key_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("APNS_KEY_PATH not configured"))?;

        // Get device token from data
        let device_token = data
            .get("device_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("device_token not provided in data"))?;

        // Get or generate JWT token for APNS authentication (with caching)
        let jwt_token = self.get_or_generate_apns_jwt(key_id, team_id, key_path).await?;

        // Build APNS payload
        let payload = json!({
            "aps": {
                "alert": {
                    "title": title,
                    "body": body
                },
                "sound": "default"
            },
            "data": data
        });

        // Determine APNS endpoint (production or sandbox)
        let apns_base = if self.apns_use_sandbox {
            "https://api.sandbox.push.apple.com"
        } else {
            "https://api.push.apple.com"
        };
        let apns_url = format!("{}/3/device/{}", apns_base, device_token);

        info!(
            user_id = %user_id,
            device_token = device_token,
            bundle_id = bundle_id,
            "Sending APNS notification"
        );

        // Send to APNS
        let response = self
            .http_client
            .post(&apns_url)
            .header("Authorization", format!("Bearer {}", jwt_token))
            .header("apns-topic", bundle_id)
            .header("apns-priority", "10")
            .header("apns-push-type", "alert")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!(
                user_id = %user_id,
                status = %status,
                error = error_text,
                "APNS API error"
            );
            return Err(anyhow::anyhow!(
                "APNS API error: {} - {}",
                status,
                error_text
            ));
        }

        info!(
            user_id = %user_id,
            device_token = device_token,
            "APNS notification sent successfully"
        );

        Ok(())
    }

    /// Get cached JWT token or generate a new one if expired/missing
    async fn get_or_generate_apns_jwt(
        &self,
        key_id: &str,
        team_id: &str,
        key_path: &str,
    ) -> anyhow::Result<String> {
        // Check if we have a valid cached token
        {
            let cached = self.apns_jwt_token.lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock APNS JWT token cache: mutex poisoned - {}", e))?;
            if let Some(ref token_data) = *cached {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| anyhow::anyhow!("Failed to get current time: {}", e))?
                    .as_secs();
                
                // Token is valid for 1 hour (3600 seconds)
                // Refresh if less than 5 minutes remaining (300 seconds buffer)
                if token_data.expires_at > now + 300 {
                    info!(
                        "Using cached APNS JWT token (expires in {} seconds)",
                        token_data.expires_at.saturating_sub(now)
                    );
                    return Ok(token_data.token.clone());
                } else {
                    info!("Cached APNS JWT token expired or expiring soon, generating new one");
                }
            }
        }

        // Generate new token
        let new_token = self.generate_apns_jwt(key_id, team_id, key_path)?;
        
        // Cache the new token
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("Failed to get current time: {}", e))?
            .as_secs();
        let expires_at = now + 3600; // APNS tokens are valid for 1 hour
        
        {
            let mut cached = self.apns_jwt_token.lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock APNS JWT token cache: mutex poisoned - {}", e))?;
            *cached = Some(ApnsJwtToken {
                token: new_token.clone(),
                expires_at,
            });
        }

        info!("Generated and cached new APNS JWT token (valid for 1 hour)");
        Ok(new_token)
    }

    /// Generate a new JWT token for APNS authentication
    fn generate_apns_jwt(
        &self,
        key_id: &str,
        team_id: &str,
        key_path: &str,
    ) -> anyhow::Result<String> {
        // Load private key from file or use cached contents
        let key_contents = if let Some(ref cached) = self.apns_key_contents {
            String::from_utf8(cached.clone())
                .map_err(|e| anyhow::anyhow!("Failed to convert key contents to string: {}", e))?
        } else {
            // Fall back to reading from file
            fs::read_to_string(key_path)
                .map_err(|e| anyhow::anyhow!("Failed to read APNS key file: {}", e))?
        };

        // Parse PEM file
        let pem = parse(&key_contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse APNS key file: {}", e))?;

        // Create JWT header with ES256 algorithm and key ID
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(key_id.to_string());
        header.typ = Some("JWT".to_string());

        // Create JWT claims
        // APNS requires:
        // - iss: Team ID (issuer)
        // - iat: Issued at time (Unix timestamp)
        // Note: APNS tokens don't have an expiration claim, but they're valid for 1 hour
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("Failed to get current time: {}", e))?
            .as_secs();

        #[derive(Serialize)]
        struct Claims {
            iss: String, // Team ID
            iat: u64,    // Issued at
        }

        let claims = Claims {
            iss: team_id.to_string(),
            iat: now,
        };

        // Create encoding key from PEM
        // APNS uses ES256 (ECDSA P-256 with SHA-256)
        let encoding_key = EncodingKey::from_ec_pem(pem.contents())
            .map_err(|e| anyhow::anyhow!("Failed to create ES256 encoding key from PEM: {}. Make sure the key is a valid EC private key in PEM format.", e))?;

        // Encode JWT
        let token = encode(&header, &claims, &encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT token: {}. Check that the key ID and team ID are correct.", e))?;

        info!(
            key_id = key_id,
            team_id = team_id,
            "Generated new APNS JWT token"
        );

        Ok(token)
    }

    /// Send an email via SMTP
    async fn send_email(
        &self,
        user_id: &Uuid,
        title: &str,
        body: &str,
        data: &Value,
    ) -> anyhow::Result<()> {
        let smtp_relay = self
            .smtp_relay
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP_RELAY not configured"))?;

        // Get recipient email from data or use a placeholder
        let recipient = data
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("email not provided in data"))?;

        let from_email = self
            .smtp_from
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP_FROM not configured"))?;

        info!(
            user_id = %user_id,
            to = recipient,
            subject = title,
            "Sending email"
        );

        // Build email message
        let email = Message::builder()
            .from(from_email.parse()?)
            .to(recipient.parse()?)
            .subject(title)
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(body.to_string()),
            )?;

        // Create SMTP transport
        let creds = if let (Some(ref username), Some(ref password)) =
            (&self.smtp_username, &self.smtp_password)
        {
            Some(Credentials::new(username.clone(), password.clone()))
        } else {
            None
        };

        let mailer = if let Some(creds) = creds {
            AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_relay)?
                .credentials(creds)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_relay)?.build()
        };

        // Send email
        mailer.send(email).await?;

        info!(
            user_id = %user_id,
            to = recipient,
            "Email sent successfully"
        );

        Ok(())
    }
}

impl Default for NotificationSender {
    fn default() -> Self {
        Self::new()
    }
}
