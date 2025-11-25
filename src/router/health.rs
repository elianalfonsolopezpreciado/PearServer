// Health checking subsystem for Cages
// Implements periodic health probes and circuit breaker pattern

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::time::{Duration, Interval};
use tracing::{debug, warn, info};

/// Health status for a Cage or Pool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health checker for individual Cages
pub struct HealthChecker {
    /// Interval between health checks
    check_interval: Duration,
    
    /// Failure threshold before marking as unhealthy
    failure_threshold: u32,
    
    /// Success threshold to mark as healthy again
    success_threshold: u32,
}

impl HealthChecker {
    pub fn new(
        check_interval: Duration,
        failure_threshold: u32,
        success_threshold: u32,
    ) -> Self {
        Self {
            check_interval,
            failure_threshold,
            success_threshold,
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 2,
        }
    }

    /// Get check interval
    pub fn interval(&self) -> Duration {
        self.check_interval
    }
}

/// Circuit breaker for Cage health management
/// Prevents routing to known-bad Cages
pub struct CircuitBreaker {
    /// Current state
    state: Arc<parking_lot::RwLock<CircuitState>>,
    
    /// Consecutive failures
    consecutive_failures: Arc<AtomicU64>,
    
    /// Consecutive successes
    consecutive_successes: Arc<AtomicU64>,
    
    /// Failure threshold
    failure_threshold: u64,
    
    /// Success threshold to close circuit
    success_threshold: u64,
    
    /// Half-open retry timeout
    retry_timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if recovered
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u64, success_threshold: u64, retry_timeout: Duration) -> Self {
        Self {
            state: Arc::new(parking_lot::RwLock::new(CircuitState::Closed)),
            consecutive_failures: Arc::new(AtomicU64::new(0)),
            consecutive_successes: Arc::new(AtomicU64::new(0)),
            failure_threshold,
            success_threshold,
            retry_timeout,
        }
    }

    /// Record a successful health check
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
        let successes = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;

        let mut state = self.state.write();
        
        match *state {
            CircuitState::HalfOpen => {
                if successes >= self.success_threshold {
                    info!("Circuit breaker closing - Cage recovered");
                    *state = CircuitState::Closed;
                    self.consecutive_successes.store(0, Ordering::Relaxed);
                }
            }
            CircuitState::Open => {
                // Should not happen, but transition to half-open
                *state = CircuitState::HalfOpen;
            }
            CircuitState::Closed => {
                // Already good
            }
        }
    }

    /// Record a failed health check
    pub fn record_failure(&self) {
        self.consecutive_successes.store(0, Ordering::Relaxed);
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;

        if failures >= self.failure_threshold {
            let mut state = self.state.write();
            
            if *state != CircuitState::Open {
                warn!(
                    failures = failures,
                    threshold = self.failure_threshold,
                    "Circuit breaker opening - Cage unhealthy"
                );
                *state = CircuitState::Open;
            }
        }
    }

    /// Check if request should be allowed
    pub fn should_allow_request(&self) -> bool {
        let state = self.state.read();
        
        match *state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => {
                // Allow some requests in half-open to test recovery
                true
            }
            CircuitState::Open => false,
        }
    }

    /// Attempt to transition from Open to HalfOpen
    pub fn try_half_open(&self) {
        let mut state = self.state.write();
        
        if *state == CircuitState::Open {
            debug!("Circuit breaker transitioning to half-open for retry");
            *state = CircuitState::HalfOpen;
            self.consecutive_successes.store(0, Ordering::Relaxed);
        }
    }

    /// Get current health status
    pub fn health_status(&self) -> HealthStatus {
        let state = self.state.read();
        
        match *state {
            CircuitState::Closed => HealthStatus::Healthy,
            CircuitState::HalfOpen => HealthStatus::Degraded,
            CircuitState::Open => HealthStatus::Unhealthy,
        }
    }
}

/// Health probe result
#[derive(Debug)]
pub struct ProbeResult {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub error: Option<String>,
}

impl ProbeResult {
    pub fn success(response_time_ms: u64) -> Self {
        Self {
            healthy: true,
            response_time_ms,
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            healthy: false,
            response_time_ms: 0,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_secs(10));
        
        assert!(breaker.should_allow_request());
        
        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();
        
        // Should be open now
        assert!(!breaker.should_allow_request());
        assert_eq!(breaker.health_status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_circuit_breaker_closes_after_recovery() {
        let breaker = CircuitBreaker::new(2, 2, Duration::from_secs(10));
        
        // Open the circuit
        breaker.record_failure();
        breaker.record_failure();
        
        // Try to recover
        breaker.try_half_open();
        assert_eq!(breaker.health_status(), HealthStatus::Degraded);
        
        // Record successes
        breaker.record_success();
        breaker.record_success();
        
        // Should be closed now
        assert_eq!(breaker.health_status(), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_checker_creation() {
        let checker = HealthChecker::default_config();
        assert!(checker.interval().as_secs() > 0);
    }
}
