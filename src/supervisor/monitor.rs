// Monitoring subsystem for Supervisor
// Resource usage tracking and anomaly detection

use tracing::{debug, warn};

/// Resource usage metrics for a Cage
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub memory_bytes: u64,
    pub cpu_percent: f64,
    pub active_requests: u64,
    pub total_requests: u64,
    pub uptime_secs: u64,
}

/// Resource monitor for tracking Cage usage
pub struct ResourceMonitor {
    /// Warning threshold for memory usage (percentage)
    memory_warning_threshold: f64,
    
    /// Warning threshold for CPU usage (percentage)
    cpu_warning_threshold: f64,
}

impl ResourceMonitor {
    pub fn new(memory_threshold: f64, cpu_threshold: f64) -> Self {
        Self {
            memory_warning_threshold: memory_threshold,
            cpu_warning_threshold: cpu_threshold,
        }
    }

    /// Create with default thresholds
    pub fn default_config() -> Self {
        Self {
            memory_warning_threshold: 80.0,  // 80% memory usage
            cpu_warning_threshold: 90.0,      // 90% CPU usage
        }
    }

    /// Analyze resource metrics and return warnings
    pub fn analyze(&self, metrics: &ResourceMetrics, cage_id: u64) -> Vec<ResourceWarning> {
        let mut warnings = Vec::new();

        // Check memory usage (simplified - in production, calculate percentage)
        let memory_mb = metrics.memory_bytes as f64 / (1024.0 * 1024.0);
        if memory_mb > 100.0 {  // More than 100MB
            warnings.push(ResourceWarning::HighMemory {
                cage_id,
                usage_mb: memory_mb,
            });
            warn!(cage_id = cage_id, memory_mb = memory_mb, "High memory usage detected");
        }

        // Check CPU usage
        if metrics.cpu_percent > self.cpu_warning_threshold {
            warnings.push(ResourceWarning::HighCpu {
                cage_id,
                usage_percent: metrics.cpu_percent,
            });
            warn!(cage_id = cage_id, cpu_percent = metrics.cpu_percent, "High CPU usage detected");
        }

        // Check for request buildup
        if metrics.active_requests > 50 {
            warnings.push(ResourceWarning::RequestBacklog {
                cage_id,
                active_requests: metrics.active_requests,
            });
            warn!(cage_id = cage_id, active_requests = metrics.active_requests, "Request backlog detected");
        }

        if !warnings.is_empty() {
            debug!(cage_id = cage_id, warnings = warnings.len(), "Resource warnings generated");
        }

        warnings
    }
}

/// Resource warning types
#[derive(Debug, Clone)]
pub enum ResourceWarning {
    HighMemory {
        cage_id: u64,
        usage_mb: f64,
    },
    HighCpu {
        cage_id: u64,
        usage_percent: f64,
    },
    RequestBacklog {
        cage_id: u64,
        active_requests: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let monitor = ResourceMonitor::default_config();
        assert!(monitor.memory_warning_threshold > 0.0);
    }

    #[test]
    fn test_high_cpu_warning() {
        let monitor = ResourceMonitor::new(80.0, 90.0);
        let metrics = ResourceMetrics {
            memory_bytes: 50 * 1024 * 1024,
            cpu_percent: 95.0,
            active_requests: 10,
            total_requests: 1000,
            uptime_secs: 3600,
        };

        let warnings = monitor.analyze(&metrics, 1);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_no_warnings() {
        let monitor = ResourceMonitor::default_config();
        let metrics = ResourceMetrics {
            memory_bytes: 10 * 1024 * 1024,  // 10MB
            cpu_percent: 30.0,
            active_requests: 5,
            total_requests: 100,
            uptime_secs: 60,
        };

        let warnings = monitor.analyze(&metrics, 1);
        assert!(warnings.is_empty());
    }
}
