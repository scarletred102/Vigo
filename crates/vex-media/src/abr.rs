// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Adaptive Bitrate (ABR) selection algorithm.
//!
//! Monitors download throughput and buffer level to choose the best
//! quality level. Uses hysteresis to prevent oscillation — quality
//! switches are rate-limited to at most once every 10 seconds.

use std::time::{Duration, Instant};

/// Minimum time between quality switches (hysteresis).
const SWITCH_COOLDOWN: Duration = Duration::from_secs(10);

/// Buffer threshold below which we switch down immediately.
const LOW_BUFFER_SECS: f64 = 5.0;

/// Buffer threshold above which we consider switching up.
const HEALTHY_BUFFER_SECS: f64 = 15.0;

/// A quality level with its bitrate.
#[derive(Debug, Clone)]
pub struct QualityLevel {
    /// Level index (0 = lowest quality).
    pub index: usize,
    /// Bitrate in bits per second.
    pub bandwidth: u64,
    /// Human-readable label (e.g., "720p").
    pub label: String,
}

/// Throughput sample from a segment download.
#[derive(Debug, Clone, Copy)]
pub struct ThroughputSample {
    /// Download throughput in bits per second.
    pub bps: u64,
    /// When this sample was recorded.
    pub timestamp: Instant,
}

/// ABR decision — which quality level to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbrDecision {
    /// Stay at the current level.
    Hold,
    /// Switch to the given level index.
    Switch(usize),
}

/// Adaptive bitrate controller.
#[derive(Debug)]
pub struct AbrController {
    /// Available quality levels, sorted by bandwidth ascending.
    levels: Vec<QualityLevel>,
    /// Current quality level index.
    current_level: usize,
    /// Recent throughput samples (sliding window).
    throughput_history: Vec<ThroughputSample>,
    /// Maximum history entries to keep.
    max_history: usize,
    /// Time of the last quality switch.
    last_switch: Instant,
    /// Safety factor (0.0–1.0). We target `factor * estimated_throughput`.
    safety_factor: f64,
}

impl AbrController {
    /// Create a new ABR controller with the given quality levels.
    ///
    /// Levels are sorted by bandwidth internally. Starts at the lowest quality.
    #[must_use]
    pub fn new(mut levels: Vec<QualityLevel>) -> Self {
        levels.sort_by_key(|l| l.bandwidth);
        // Re-index after sorting
        for (i, level) in levels.iter_mut().enumerate() {
            level.index = i;
        }
        Self {
            levels,
            current_level: 0,
            throughput_history: Vec::new(),
            max_history: 20,
            last_switch: Instant::now() - SWITCH_COOLDOWN, // allow immediate first switch
            safety_factor: 0.8,
        }
    }

    /// Current quality level index.
    #[must_use]
    pub fn current_level(&self) -> usize {
        self.current_level
    }

    /// Number of available levels.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Get quality level info by index.
    #[must_use]
    pub fn level(&self, index: usize) -> Option<&QualityLevel> {
        self.levels.get(index)
    }

    /// Record a throughput sample from a completed segment download.
    pub fn record_throughput(&mut self, bps: u64) {
        self.throughput_history.push(ThroughputSample {
            bps,
            timestamp: Instant::now(),
        });

        // Trim old samples
        if self.throughput_history.len() > self.max_history {
            let excess = self.throughput_history.len() - self.max_history;
            self.throughput_history.drain(..excess);
        }
    }

    /// Record a throughput sample with a specific timestamp (for testing).
    pub fn record_throughput_at(&mut self, bps: u64, timestamp: Instant) {
        self.throughput_history
            .push(ThroughputSample { bps, timestamp });

        if self.throughput_history.len() > self.max_history {
            let excess = self.throughput_history.len() - self.max_history;
            self.throughput_history.drain(..excess);
        }
    }

    /// Estimated throughput based on recent samples (exponentially weighted).
    #[must_use]
    pub fn estimated_throughput(&self) -> u64 {
        if self.throughput_history.is_empty() {
            return 0;
        }

        // Use exponentially weighted moving average (EWMA)
        // Recent samples weigh more
        let alpha = 0.7;
        let mut ewma = self.throughput_history[0].bps as f64;
        for sample in self.throughput_history.iter().skip(1) {
            ewma = alpha * sample.bps as f64 + (1.0 - alpha) * ewma;
        }

        ewma as u64
    }

    /// Make an ABR decision given the current buffer level.
    ///
    /// `buffer_secs` is how many seconds of buffered content are available.
    #[must_use]
    pub fn decide(&mut self, buffer_secs: f64) -> AbrDecision {
        if self.levels.is_empty() {
            return AbrDecision::Hold;
        }

        let estimated = self.estimated_throughput();
        let safe_throughput = (estimated as f64 * self.safety_factor) as u64;
        let cooldown_elapsed = self.last_switch.elapsed() >= SWITCH_COOLDOWN;

        // Emergency: low buffer → switch down immediately (ignore cooldown)
        if buffer_secs < LOW_BUFFER_SECS && self.current_level > 0 {
            let target = find_highest_fitting(&self.levels, safe_throughput);
            let target = target.min(self.current_level.saturating_sub(1));
            if target != self.current_level {
                self.current_level = target;
                self.last_switch = Instant::now();
                return AbrDecision::Switch(target);
            }
        }

        // Normal: only switch if cooldown has elapsed
        if !cooldown_elapsed {
            return AbrDecision::Hold;
        }

        let target = find_highest_fitting(&self.levels, safe_throughput);

        // Switch up only if buffer is healthy
        if target > self.current_level && buffer_secs < HEALTHY_BUFFER_SECS {
            return AbrDecision::Hold;
        }

        // Switch down if we can't sustain current level
        if target < self.current_level {
            self.current_level = target;
            self.last_switch = Instant::now();
            return AbrDecision::Switch(target);
        }

        // Switch up
        if target > self.current_level {
            self.current_level = target;
            self.last_switch = Instant::now();
            return AbrDecision::Switch(target);
        }

        AbrDecision::Hold
    }

    /// Force switch to a specific level (manual override).
    pub fn force_level(&mut self, level: usize) {
        if level < self.levels.len() {
            self.current_level = level;
            self.last_switch = Instant::now();
        }
    }

    /// Set the safety factor (0.0–1.0).
    pub fn set_safety_factor(&mut self, factor: f64) {
        self.safety_factor = factor.clamp(0.1, 1.0);
    }

    /// Reset cooldown timer (allows immediate next switch, for testing).
    pub fn reset_cooldown(&mut self) {
        self.last_switch = Instant::now() - SWITCH_COOLDOWN;
    }
}

/// Find the highest quality level whose bandwidth fits within the throughput.
fn find_highest_fitting(levels: &[QualityLevel], throughput: u64) -> usize {
    let mut best = 0;
    for level in levels {
        if level.bandwidth <= throughput {
            best = level.index;
        } else {
            break; // levels are sorted ascending
        }
    }
    best
}

/// Helper to create quality levels for testing.
#[cfg(test)]
fn make_levels(bandwidths: &[(u64, &str)]) -> Vec<QualityLevel> {
    bandwidths
        .iter()
        .enumerate()
        .map(|(i, (bw, label))| QualityLevel {
            index: i,
            bandwidth: *bw,
            label: label.to_string(),
        })
        .collect()
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_levels() -> Vec<QualityLevel> {
        make_levels(&[
            (500_000, "360p"),
            (1_000_000, "480p"),
            (2_500_000, "720p"),
            (5_000_000, "1080p"),
        ])
    }

    #[test]
    fn test_starts_at_lowest() {
        let abr = AbrController::new(test_levels());
        assert_eq!(abr.current_level(), 0);
    }

    #[test]
    fn test_no_throughput_holds() {
        let mut abr = AbrController::new(test_levels());
        assert_eq!(abr.decide(20.0), AbrDecision::Hold);
    }

    #[test]
    fn test_switch_up_with_high_throughput() {
        let mut abr = AbrController::new(test_levels());
        abr.reset_cooldown();

        // Record high throughput
        for _ in 0..5 {
            abr.record_throughput(6_000_000);
        }

        // High buffer → should switch up
        let decision = abr.decide(HEALTHY_BUFFER_SECS + 1.0);
        match decision {
            AbrDecision::Switch(level) => assert!(level > 0, "should switch up from 0"),
            AbrDecision::Hold => panic!("expected switch up"),
        }
    }

    #[test]
    fn test_switch_down_on_low_buffer() {
        let mut abr = AbrController::new(test_levels());

        // Start at highest level
        abr.force_level(3);
        assert_eq!(abr.current_level(), 3);

        // Record low throughput
        for _ in 0..5 {
            abr.record_throughput(400_000);
        }

        // Low buffer → emergency switch down
        let decision = abr.decide(LOW_BUFFER_SECS - 1.0);
        match decision {
            AbrDecision::Switch(level) => assert!(level < 3, "should switch down from 3"),
            AbrDecision::Hold => panic!("expected switch down"),
        }
    }

    #[test]
    fn test_hysteresis_prevents_rapid_switch() {
        let mut abr = AbrController::new(test_levels());
        abr.reset_cooldown();

        // Record enough throughput for level 2
        for _ in 0..5 {
            abr.record_throughput(3_000_000);
        }

        // First switch should happen
        let d1 = abr.decide(HEALTHY_BUFFER_SECS + 1.0);
        assert!(matches!(d1, AbrDecision::Switch(_)));

        // Immediately try again — should hold due to cooldown
        let d2 = abr.decide(HEALTHY_BUFFER_SECS + 1.0);
        assert_eq!(d2, AbrDecision::Hold);
    }

    #[test]
    fn test_estimated_throughput_ewma() {
        let mut abr = AbrController::new(test_levels());

        abr.record_throughput(1_000_000);
        abr.record_throughput(1_000_000);
        abr.record_throughput(3_000_000); // spike

        let est = abr.estimated_throughput();
        // EWMA should be pulled toward 3M but not all the way
        assert!(est > 1_000_000);
        assert!(est < 3_000_000);
    }

    #[test]
    fn test_force_level() {
        let mut abr = AbrController::new(test_levels());
        abr.force_level(2);
        assert_eq!(abr.current_level(), 2);

        // Out of range — no change
        abr.force_level(99);
        assert_eq!(abr.current_level(), 2);
    }

    #[test]
    fn test_no_switch_up_with_low_buffer() {
        let mut abr = AbrController::new(test_levels());
        abr.reset_cooldown();

        // High throughput but low buffer
        for _ in 0..5 {
            abr.record_throughput(6_000_000);
        }

        // Buffer is between LOW and HEALTHY — should not switch up
        let decision = abr.decide(HEALTHY_BUFFER_SECS - 1.0);
        assert_eq!(decision, AbrDecision::Hold);
    }

    #[test]
    fn test_empty_levels() {
        let mut abr = AbrController::new(vec![]);
        assert_eq!(abr.decide(20.0), AbrDecision::Hold);
    }
}
