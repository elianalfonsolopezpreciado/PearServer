// Performance Baseline Monitor
// Tracks latency and detects anomalies using statistical analysis

use std::collections::VecDeque;
use std::time::Duration;
use tracing::{warn, info, debug};

/// Performance baseline monitor
pub struct PerformanceMonitor {
    /// Request latency samples (rolling window)
    latency_samples: VecDeque<f64>,
    
    /// Database query latency samples
    db_latency_samples: VecDeque<f64>,
    
    /// Maximum samples to keep
    max_samples: usize,
    
    /// Standard deviation multiplier for alerts
    std_dev_threshold: f64,
    
    /// Calculated baseline statistics
    baseline: Option<BaselineStats>,
}

#[derive(Debug, Clone)]
struct BaselineStats {
    mean: f64,
    std_dev: f64,
    last_updated: std::time::Instant,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(max_samples: usize, std_dev_threshold: f64) -> Self {
        info!("Initializing Performance Baseline Monitor");
        
        Self {
            latency_samples: VecDeque::with_capacity(max_samples),
            db_latency_samples: VecDeque::with_capacity(max_samples),
            max_samples,
            std_dev_threshold,
            baseline: None,
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(1000, 2.0) // 1000 samples, 2 std deviations
    }

    /// Record request latency
    pub fn record_latency(&mut self, latency: Duration) -> PerformanceAlert {
        let latency_ms = latency.as_secs_f64() * 1000.0;
        
        // Add to samples
        if self.latency_samples.len() >= self.max_samples {
            self.latency_samples.pop_front();
        }
        self.latency_samples.push_back(latency_ms);

        // Update baseline if enough samples
        if self.latency_samples.len() >= 30 {
            self.update_baseline();
        }

        // Check for anomaly
        self.check_anomaly(latency_ms, &self.latency_samples, "Request Latency")
    }

    /// Record database query latency
    pub fn record_db_latency(&mut self, latency: Duration) -> PerformanceAlert {
        let latency_ms = latency.as_secs_f64() * 1000.0;
        
        // Add to samples
        if self.db_latency_samples.len() >= self.max_samples {
            self.db_latency_samples.pop_front();
        }
        self.db_latency_samples.push_back(latency_ms);

        // Check for anomaly
        self.check_anomaly(latency_ms, &self.db_latency_samples, "Database Query")
    }

    /// Update baseline statistics
    fn update_baseline(&mut self) {
        if self.latency_samples.len() < 30 {
            return;
        }

        let mean = self.calculate_mean(&self.latency_samples);
        let std_dev = self.calculate_std_dev(&self.latency_samples, mean);

        self.baseline = Some(BaselineStats {
            mean,
            std_dev,
            last_updated: std::time::Instant::now(),
        });

        debug!(
            mean_ms = mean,
            std_dev = std_dev,
            "Baseline updated"
        );
    }

    /// Calculate mean
    fn calculate_mean(&self, samples: &VecDeque<f64>) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        samples.iter().sum::<f64>() / samples.len() as f64
    }

    /// Calculate standard deviation
    fn calculate_std_dev(&self, samples: &VecDeque<f64>, mean: f64) -> f64 {
        if samples.len() < 2 {
            return 0.0;
        }

        let variance = samples.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (samples.len() - 1) as f64;

        variance.sqrt()
    }

    /// Check if value is anomalous
    fn check_anomaly(&self, value: f64, samples: &VecDeque<f64>, metric_name: &str) -> PerformanceAlert {
        if samples.len() < 30 {
            return PerformanceAlert::Normal;
        }

        let mean = self.calculate_mean(samples);
        let std_dev = self.calculate_std_dev(samples, mean);

        if std_dev == 0.0 {
            return PerformanceAlert::Normal;
        }

        let z_score = (value - mean).abs() / std_dev;

        if z_score > self.std_dev_threshold {
            warn!(
                metric = metric_name,
                value_ms = value,
                mean_ms = mean,
                std_dev = std_dev,
                z_score = z_score,
                "Performance anomaly detected"
            );

            PerformanceAlert::Anomaly {
                metric: metric_name.to_string(),
                value_ms: value,
                baseline_ms: mean,
                deviation: z_score,
            }
        } else {
            PerformanceAlert::Normal
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> PerformanceStats {
        let latency_mean = self.calculate_mean(&self.latency_samples);
        let latency_std_dev = self.calculate_std_dev(&self.latency_samples, latency_mean);

        let db_mean = self.calculate_mean(&self.db_latency_samples);
        let db_std_dev = self.calculate_std_dev(&self.db_latency_samples, db_mean);

        PerformanceStats {
            request_latency_mean_ms: latency_mean,
            request_latency_std_dev: latency_std_dev,
            db_latency_mean_ms: db_mean,
            db_latency_std_dev: db_std_dev,
            sample_count: self.latency_samples.len(),
        }
    }

    /// Get percentiles
    pub fn percentiles(&self) -> Percentiles {
        let mut sorted: Vec<f64> = self.latency_samples.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if sorted.is_empty() {
            return Percentiles::default();
        }

        Percentiles {
            p50: Self::percentile(&sorted, 0.50),
            p90: Self::percentile(&sorted, 0.90),
            p95: Self::percentile(&sorted, 0.95),
            p99: Self::percentile(&sorted, 0.99),
        }
    }

    fn percentile(sorted: &[f64], p: f64) -> f64 {
        let index = (sorted.len() as f64 * p) as usize;
        sorted.get(index.min(sorted.len() - 1)).copied().unwrap_or(0.0)
    }
}

/// Performance alert
#[derive(Debug, Clone)]
pub enum PerformanceAlert {
    Normal,
    Anomaly {
        metric: String,
        value_ms: f64,
        baseline_ms: f64,
        deviation: f64,
    },
}

impl PerformanceAlert {
    pub fn is_anomaly(&self) -> bool {
        matches!(self, PerformanceAlert::Anomaly { .. })
    }
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub request_latency_mean_ms: f64,
    pub request_latency_std_dev: f64,
    pub db_latency_mean_ms: f64,
    pub db_latency_std_dev: f64,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Percentiles {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::default_config();
        assert_eq!(monitor.max_samples, 1000);
    }

    #[test]
    fn test_baseline_calculation() {
        let mut monitor = PerformanceMonitor::new(100, 2.0);

        // Add normal samples (around 100ms)
        for _ in 0..50 {
            monitor.record_latency(Duration::from_millis(100));
        }

        let stats = monitor.stats();
        assert!(stats.request_latency_mean_ms > 90.0);
        assert!(stats.request_latency_mean_ms < 110.0);
    }

    #[test]
    fn test_anomaly_detection() {
        let mut monitor = PerformanceMonitor::new(100, 2.0);

        // Establish baseline
        for _ in 0..50 {
            monitor.record_latency(Duration::from_millis(100));
        }

        // Record anomalous value
        let alert = monitor.record_latency(Duration::from_millis(500));
        assert!(alert.is_anomaly());
    }
}
