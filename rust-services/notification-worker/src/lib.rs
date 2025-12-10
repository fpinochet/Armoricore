//! Notification Worker Library
//!
//! This library provides notification sending functionality including:
//! - Push notifications (FCM, APNS)
//! - Email sending (SMTP)
//! - Device token database
//! - Retry logic with exponential backoff
//! - Rate limiting
//! - Dead letter queue
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


pub mod database;
pub mod dead_letter_queue;
pub mod rate_limiter;
pub mod retry;
pub mod sender;
pub mod worker;

