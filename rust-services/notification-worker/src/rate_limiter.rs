//! Rate limiting for notification sending
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


use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    /// Maximum number of tokens (requests) allowed
    capacity: u32,
    /// Number of tokens added per refill period
    refill_amount: u32,
    /// Duration between refills
    refill_period: Duration,
    /// Current number of available tokens
    tokens: Arc<Mutex<u32>>,
    /// Last refill time
    last_refill: Arc<Mutex<Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of tokens (burst capacity)
    /// * `refill_amount` - Number of tokens added per refill period
    /// * `refill_period` - Duration between refills
    ///
    /// # Example
    /// ```
    /// use notification_worker::rate_limiter::RateLimiter;
    /// use std::time::Duration;
    /// 
    /// // Allow 100 requests per minute
    /// let limiter = RateLimiter::new(100, 100, Duration::from_secs(60));
    /// ```
    pub fn new(capacity: u32, refill_amount: u32, refill_period: Duration) -> Self {
        Self {
            capacity,
            refill_amount,
            refill_period,
            tokens: Arc::new(Mutex::new(capacity)),
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Create a rate limiter from requests per second
    pub fn from_requests_per_second(requests_per_second: u32) -> Self {
        Self::new(requests_per_second, requests_per_second, Duration::from_secs(1))
    }

    /// Create a rate limiter from requests per minute
    pub fn from_requests_per_minute(requests_per_minute: u32) -> Self {
        Self::new(requests_per_minute, requests_per_minute, Duration::from_secs(60))
    }

    /// Try to acquire a token (non-blocking)
    /// Returns true if token was acquired, false if rate limit exceeded
    pub async fn try_acquire(&self) -> bool {
        let mut tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;

        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);

        if elapsed >= self.refill_period {
            // Calculate how many refill periods have passed
            let periods = elapsed.as_secs_f64() / self.refill_period.as_secs_f64();
            let tokens_to_add = (periods * self.refill_amount as f64) as u32;

            *tokens = (*tokens + tokens_to_add).min(self.capacity);
            *last_refill = now;

            debug!(
                tokens_added = tokens_to_add,
                current_tokens = *tokens,
                "Rate limiter tokens refilled"
            );
        }

        // Try to consume a token
        if *tokens > 0 {
            *tokens -= 1;
            debug!(remaining_tokens = *tokens, "Token acquired");
            true
        } else {
            warn!("Rate limit exceeded, token not acquired");
            false
        }
    }

    /// Wait until a token is available (blocking)
    pub async fn acquire(&self) {
        loop {
            if self.try_acquire().await {
                return;
            }

            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get current number of available tokens
    pub async fn available_tokens(&self) -> u32 {
        let tokens = self.tokens.lock().await;
        *tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_acquires_tokens() {
        let limiter = RateLimiter::new(5, 5, Duration::from_secs(1));

        // Should be able to acquire 5 tokens
        for _ in 0..5 {
            assert!(limiter.try_acquire().await);
        }

        // 6th attempt should fail
        assert!(!limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refills_tokens() {
        let limiter = RateLimiter::new(5, 5, Duration::from_millis(100));

        // Exhaust tokens
        for _ in 0..5 {
            assert!(limiter.try_acquire().await);
        }
        assert!(!limiter.try_acquire().await);

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be able to acquire tokens again
        assert!(limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_from_requests_per_second() {
        let limiter = RateLimiter::from_requests_per_second(10);

        // Should be able to acquire 10 tokens
        for _ in 0..10 {
            assert!(limiter.try_acquire().await);
        }

        // 11th attempt should fail
        assert!(!limiter.try_acquire().await);
    }
}

