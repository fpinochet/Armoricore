//! Retry logic with exponential backoff
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


use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay in seconds
    pub initial_delay_secs: u64,
    /// Maximum delay in seconds (cap for exponential backoff)
    pub max_delay_secs: u64,
    /// Multiplier for exponential backoff (e.g., 2.0 for doubling)
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_secs: 1,
            max_delay_secs: 60,
            multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_retries: u32, initial_delay_secs: u64, max_delay_secs: u64, multiplier: f64) -> Self {
        Self {
            max_retries,
            initial_delay_secs,
            max_delay_secs,
            multiplier,
        }
    }

    /// Get delay for a specific retry attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_secs(0);
        }

        // Calculate exponential backoff: initial_delay * (multiplier ^ (attempt - 1))
        let delay_secs = (self.initial_delay_secs as f64) * self.multiplier.powi((attempt - 1) as i32);
        let delay_secs = delay_secs.min(self.max_delay_secs as f64) as u64;

        Duration::from_secs(delay_secs)
    }
}

/// Retry a function with exponential backoff
pub async fn retry_with_backoff<F, T, E>(
    config: &RetryConfig,
    mut f: F,
) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Display,
{
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        match f().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!(
                        attempt = attempt,
                        total_attempts = attempt + 1,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);

                if attempt < config.max_retries {
                    let delay = config.delay_for_attempt(attempt + 1);
                    warn!(
                        attempt = attempt + 1,
                        max_retries = config.max_retries,
                        delay_secs = delay.as_secs(),
                        error = %last_error.as_ref().unwrap(),
                        "Operation failed, retrying with exponential backoff"
                    );
                    sleep(delay).await;
                } else {
                    warn!(
                        attempt = attempt + 1,
                        max_retries = config.max_retries,
                        error = %last_error.as_ref().unwrap(),
                        "Operation failed after all retries"
                    );
                }
            }
        }
    }

    match last_error {
        Some(e) => Err(e),
        None => {
            // This should never happen - we only reach here if all retries failed
            // but no error was recorded, which indicates a logic error
            panic!("Retry logic error: no errors recorded but operation failed after {} retries", config.max_retries);
        }
    }
}

/// Check if an error is retryable (transient)
pub fn is_retryable_error(error: &anyhow::Error) -> bool {
    let error_str = error.to_string().to_lowercase();
    
    // Network errors are usually retryable
    if error_str.contains("network") 
        || error_str.contains("timeout")
        || error_str.contains("connection")
        || error_str.contains("temporary")
        || error_str.contains("unavailable")
        || error_str.contains("rate limit")
        || error_str.contains("429") // HTTP 429 Too Many Requests
        || error_str.contains("503") // HTTP 503 Service Unavailable
        || error_str.contains("502") // HTTP 502 Bad Gateway
    {
        return true;
    }

    // Authentication errors are usually not retryable
    if error_str.contains("unauthorized")
        || error_str.contains("forbidden")
        || error_str.contains("401")
        || error_str.contains("403")
        || error_str.contains("invalid")
        || error_str.contains("malformed")
    {
        return false;
    }

    // Default: assume retryable for unknown errors
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_delays() {
        let config = RetryConfig::default();
        
        assert_eq!(config.delay_for_attempt(0), Duration::from_secs(0));
        assert_eq!(config.delay_for_attempt(1), Duration::from_secs(1));
        assert_eq!(config.delay_for_attempt(2), Duration::from_secs(2));
        assert_eq!(config.delay_for_attempt(3), Duration::from_secs(4));
    }

    #[test]
    fn test_retry_config_max_delay() {
        let config = RetryConfig::new(5, 1, 10, 2.0);
        
        // Should cap at max_delay_secs
        assert!(config.delay_for_attempt(10).as_secs() <= 10);
    }

    #[tokio::test]
    async fn test_retry_succeeds_on_first_attempt() {
        let config = RetryConfig::new(3, 1, 10, 2.0);
        let mut attempts = 0;

        let result = retry_with_backoff(&config, || {
            attempts += 1;
            Box::pin(async move { Ok::<i32, String>(42) })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_retries() {
        let config = RetryConfig::new(3, 0, 10, 2.0); // 0 delay for faster tests
        let mut attempts = 0;

        let result = retry_with_backoff(&config, || {
            attempts += 1;
            Box::pin(async move {
                if attempts < 3 {
                    Err::<i32, String>("temporary error".to_string())
                } else {
                    Ok(42)
                }
            })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_fails_after_max_retries() {
        let config = RetryConfig::new(2, 0, 10, 2.0); // 0 delay for faster tests
        let mut attempts = 0;

        let result = retry_with_backoff(&config, || {
            attempts += 1;
            Box::pin(async move { Err::<i32, String>("persistent error".to_string()) })
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 3); // initial + 2 retries
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error(&anyhow::anyhow!("network error")));
        assert!(is_retryable_error(&anyhow::anyhow!("timeout occurred")));
        assert!(is_retryable_error(&anyhow::anyhow!("HTTP 429 Too Many Requests")));
        assert!(is_retryable_error(&anyhow::anyhow!("HTTP 503 Service Unavailable")));
        
        assert!(!is_retryable_error(&anyhow::anyhow!("unauthorized access")));
        assert!(!is_retryable_error(&anyhow::anyhow!("HTTP 401 Unauthorized")));
        assert!(!is_retryable_error(&anyhow::anyhow!("invalid token")));
    }
}

