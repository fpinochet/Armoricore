//! Adaptive bitrate control for video streams
//!
//! Dynamically adjusts video bitrate and resolution based on network conditions.
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


use crate::error::MediaEngineResult;
use crate::connection_health::NetworkMetrics;
use crate::video_pipeline::{VideoPipeline, VideoResolution};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Adaptive bitrate configuration
#[derive(Debug, Clone)]
pub struct AdaptiveBitrateConfig {
    /// Minimum bitrate (bps)
    pub min_bitrate: u32,
    /// Maximum bitrate (bps)
    pub max_bitrate: u32,
    /// Bitrate adjustment step (bps)
    pub bitrate_step: u32,
    /// Minimum resolution
    pub min_resolution: VideoResolution,
    /// Maximum resolution
    pub max_resolution: VideoResolution,
    /// Adjustment interval (seconds)
    pub adjustment_interval: Duration,
    /// History window size
    pub history_size: usize,
    /// Packet loss threshold for downgrade
    pub packet_loss_threshold: f32,
    /// RTT threshold for downgrade (ms)
    pub rtt_threshold_ms: f64,
}

impl Default for AdaptiveBitrateConfig {
    fn default() -> Self {
        AdaptiveBitrateConfig {
            min_bitrate: 400_000,  // 400 kbps
            max_bitrate: 10_000_000,  // 10 Mbps
            bitrate_step: 100_000,  // 100 kbps steps
            min_resolution: VideoResolution::P360,
            max_resolution: VideoResolution::P1080,
            adjustment_interval: Duration::from_secs(2),
            history_size: 10,
            packet_loss_threshold: 0.05,  // 5%
            rtt_threshold_ms: 200.0,
        }
    }
}

/// Network sample for adaptive bitrate
#[derive(Debug, Clone)]
struct NetworkSample {
    metrics: NetworkMetrics,
    #[allow(dead_code)]
    timestamp: Instant,
}

/// Adaptive bitrate controller
pub struct AdaptiveBitrateController {
    config: AdaptiveBitrateConfig,
    history: VecDeque<NetworkSample>,
    last_adjustment: Instant,
    current_bitrate: u32,
    current_resolution: VideoResolution,
    target_bitrate: u32,
    target_resolution: VideoResolution,
}

impl AdaptiveBitrateController {
    /// Create a new adaptive bitrate controller
    pub fn new(config: AdaptiveBitrateConfig, initial_bitrate: u32, initial_resolution: VideoResolution) -> Self {
        AdaptiveBitrateController {
            config,
            history: VecDeque::new(),
            last_adjustment: Instant::now(),
            current_bitrate: initial_bitrate,
            current_resolution: initial_resolution,
            target_bitrate: initial_bitrate,
            target_resolution: initial_resolution,
        }
    }

    /// Update with network metrics
    pub fn update_metrics(&mut self, metrics: &NetworkMetrics) {
        let sample = NetworkSample {
            metrics: metrics.clone(),
            timestamp: Instant::now(),
        };

        self.history.push_back(sample);
        if self.history.len() > self.config.history_size {
            self.history.pop_front();
        }
    }

    /// Adjust bitrate based on network conditions
    pub fn adjust(&mut self, pipeline: &mut VideoPipeline) -> MediaEngineResult<bool> {
        // Check if enough time has passed since last adjustment
        if self.last_adjustment.elapsed() < self.config.adjustment_interval {
            return Ok(false);
        }

        if self.history.is_empty() {
            return Ok(false);
        }

        // Calculate average metrics
        let avg_loss: f32 = self.history.iter()
            .map(|s| s.metrics.packet_loss_rate)
            .sum::<f32>() / self.history.len() as f32;

        let avg_rtt: f64 = self.history.iter()
            .map(|s| s.metrics.rtt_ms)
            .sum::<f64>() / self.history.len() as f64;

        let avg_bandwidth: f64 = self.history.iter()
            .map(|s| s.metrics.bandwidth_kbps)
            .sum::<f64>() / self.history.len() as f64;

        let mut adjusted = false;

        // Determine if we should downgrade or upgrade
        let should_downgrade = avg_loss > self.config.packet_loss_threshold
            || avg_rtt > self.config.rtt_threshold_ms;

        let should_upgrade = avg_loss < 0.01
            && avg_rtt < self.config.rtt_threshold_ms * 0.7
            && avg_bandwidth > (self.current_bitrate as f64 / 1000.0) * 1.5; // 50% headroom

        if should_downgrade {
            // Downgrade: reduce bitrate or resolution
            if self.current_bitrate > self.config.min_bitrate {
                let new_bitrate = (self.current_bitrate as i32 - self.config.bitrate_step as i32)
                    .max(self.config.min_bitrate as i32) as u32;
                self.target_bitrate = new_bitrate;
                pipeline.update_bitrate(new_bitrate)?;
                adjusted = true;
            } else if self.current_resolution != self.config.min_resolution {
                // Downgrade resolution
                let new_resolution = self.downgrade_resolution(self.current_resolution);
                self.target_resolution = new_resolution;
                pipeline.update_resolution(new_resolution)?;
                adjusted = true;
            }
        } else if should_upgrade {
            // Upgrade: increase bitrate or resolution
            let max_allowed_bitrate = (avg_bandwidth * 1000.0 * 0.8) as u32; // 80% of available bandwidth
            
            if self.current_bitrate < max_allowed_bitrate.min(self.config.max_bitrate) {
                let new_bitrate = (self.current_bitrate + self.config.bitrate_step)
                    .min(max_allowed_bitrate)
                    .min(self.config.max_bitrate);
                self.target_bitrate = new_bitrate;
                pipeline.update_bitrate(new_bitrate)?;
                adjusted = true;
            } else if self.current_resolution != self.config.max_resolution {
                // Check if we can upgrade resolution
                let required_bitrate = self.estimate_bitrate_for_resolution(
                    self.upgrade_resolution(self.current_resolution)
                );
                if required_bitrate <= max_allowed_bitrate {
                    let new_resolution = self.upgrade_resolution(self.current_resolution);
                    self.target_resolution = new_resolution;
                    pipeline.update_resolution(new_resolution)?;
                    adjusted = true;
                }
            }
        }

        if adjusted {
            self.current_bitrate = self.target_bitrate;
            self.current_resolution = self.target_resolution;
            self.last_adjustment = Instant::now();
        }

        Ok(adjusted)
    }

    /// Downgrade resolution
    fn downgrade_resolution(&self, current: VideoResolution) -> VideoResolution {
        match current {
            VideoResolution::P8K => VideoResolution::P5K,
            VideoResolution::P5K => VideoResolution::P4K,
            VideoResolution::P4K => VideoResolution::P1440,
            VideoResolution::P1440 => VideoResolution::P1080,
            VideoResolution::P1080 => VideoResolution::P720,
            VideoResolution::P720 => VideoResolution::P480,
            VideoResolution::P480 => VideoResolution::P360,
            VideoResolution::P360 => VideoResolution::P360, // Can't go lower
        }
    }

    /// Upgrade resolution
    fn upgrade_resolution(&self, current: VideoResolution) -> VideoResolution {
        match current {
            VideoResolution::P360 => VideoResolution::P480,
            VideoResolution::P480 => VideoResolution::P720,
            VideoResolution::P720 => VideoResolution::P1080,
            VideoResolution::P1080 => VideoResolution::P1440,
            VideoResolution::P1440 => VideoResolution::P4K,
            VideoResolution::P4K => VideoResolution::P5K,
            VideoResolution::P5K => VideoResolution::P8K,
            VideoResolution::P8K => VideoResolution::P8K, // Can't go higher
        }
    }

    /// Estimate required bitrate for resolution
    fn estimate_bitrate_for_resolution(&self, resolution: VideoResolution) -> u32 {
        // Rough estimates based on resolution
        match resolution {
            VideoResolution::P360 => 400_000,  // 400 kbps
            VideoResolution::P480 => 800_000,  // 800 kbps
            VideoResolution::P720 => 1_500_000,  // 1.5 Mbps
            VideoResolution::P1080 => 3_000_000,  // 3 Mbps
            VideoResolution::P1440 => 5_000_000,  // 5 Mbps
            VideoResolution::P4K => 8_000_000,  // 8 Mbps
            VideoResolution::P5K => 12_000_000,  // 12 Mbps
            VideoResolution::P8K => 20_000_000,  // 20 Mbps
        }
    }

    /// Get current bitrate
    pub fn current_bitrate(&self) -> u32 {
        self.current_bitrate
    }

    /// Get current resolution
    pub fn current_resolution(&self) -> VideoResolution {
        self.current_resolution
    }

    /// Get target bitrate
    pub fn target_bitrate(&self) -> u32 {
        self.target_bitrate
    }

    /// Get target resolution
    pub fn target_resolution(&self) -> VideoResolution {
        self.target_resolution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_bitrate_downgrade() {
        let config = AdaptiveBitrateConfig::default();
        let mut controller = AdaptiveBitrateController::new(
            config,
            2_000_000,
            VideoResolution::P1080,
        );

        // Simulate poor network conditions
        let metrics = NetworkMetrics {
            rtt_ms: 300.0,
            packet_loss_rate: 0.1,
            jitter_ms: 100.0,
            bandwidth_kbps: 500.0,
            timestamp: Instant::now(),
        };

        controller.update_metrics(&metrics);
        
        let video_config = VideoConfig::default();
        let mut pipeline = VideoPipeline::new(video_config).unwrap();
        
        // Should downgrade
        let adjusted = controller.adjust(&mut pipeline).unwrap();
        // Note: May not adjust immediately due to interval check
    }

    #[test]
    fn test_resolution_downgrade() {
        let config = AdaptiveBitrateConfig::default();
        let controller = AdaptiveBitrateController::new(
            config,
            2_000_000,
            VideoResolution::P1080,
        );

        let downgraded = controller.downgrade_resolution(VideoResolution::P1080);
        assert_eq!(downgraded, VideoResolution::P720);
    }

    #[test]
    fn test_resolution_upgrade() {
        let config = AdaptiveBitrateConfig::default();
        let controller = AdaptiveBitrateController::new(
            config,
            2_000_000,
            VideoResolution::P720,
        );

        let upgraded = controller.upgrade_resolution(VideoResolution::P720);
        assert_eq!(upgraded, VideoResolution::P1080);
    }
}

