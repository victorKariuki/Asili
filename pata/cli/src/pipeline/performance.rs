//! Performance profiling and latency tracking

use std::time::{Duration, Instant};
use std::collections::BTreeMap;

/// Performance metrics for command execution
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Phase name -> duration map
    pub phases: BTreeMap<String, Duration>,
    /// Total execution time
    pub total_duration: Duration,
    /// Threshold for SLO violation warnings in milliseconds
    pub slo_threshold_ms: u64,
}

impl PerformanceMetrics {
    /// Create a new performance tracker
    pub fn new(slo_threshold_ms: u64) -> Self {
        Self {
            phases: BTreeMap::new(),
            total_duration: Duration::from_secs(0),
            slo_threshold_ms,
        }
    }

    /// Record a phase's execution time
    pub fn record_phase(&mut self, name: &str, duration: Duration) {
        self.phases.insert(name.to_string(), duration);
    }

    /// Set total execution time
    pub fn set_total(&mut self, duration: Duration) {
        self.total_duration = duration;
    }

    /// Check if execution violates SLO
    pub fn exceeds_slo(&self) -> bool {
        self.total_duration.as_millis() as u64 > self.slo_threshold_ms
    }

    /// Format as performance report
    pub fn report(&self) -> String {
        let mut report = format!(
            "Uendeshaji: {:.2}s",
            self.total_duration.as_secs_f64()
        );

        if !self.phases.is_empty() {
            report.push_str("\n\nAwamu:\n");
            for (phase, duration) in &self.phases {
                let ms = duration.as_millis();
                report.push_str(&format!("  {}: {:.2}ms\n", phase, ms));
            }
        }

        if self.exceeds_slo() {
            report.push_str(&format!(
                "\nOnyo: Kuzidi SLO ({} ms)\n",
                self.slo_threshold_ms
            ));
        }

        report
    }
}

/// Scoped timer for automatic phase tracking
pub struct ScopedTimer {
    name: String,
    start: Instant,
}

impl ScopedTimer {
    /// Create a new scoped timer
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
        }
    }

    /// Get elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Get name
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn performance_metrics_tracks_phases() {
        let mut metrics = PerformanceMetrics::new(5000);
        metrics.record_phase("parse", Duration::from_millis(100));
        metrics.record_phase("compile", Duration::from_millis(200));
        metrics.set_total(Duration::from_millis(300));

        assert_eq!(metrics.phases.len(), 2);
        assert!(!metrics.exceeds_slo());
    }

    #[test]
    fn performance_metrics_detects_slo_violation() {
        let mut metrics = PerformanceMetrics::new(100);
        metrics.set_total(Duration::from_millis(150));

        assert!(metrics.exceeds_slo());
    }

    #[test]
    fn scoped_timer_tracks_elapsed() {
        let timer = ScopedTimer::new("test_phase");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed();

        assert!(elapsed.as_millis() >= 10);
        assert_eq!(timer.name(), "test_phase");
    }
}
