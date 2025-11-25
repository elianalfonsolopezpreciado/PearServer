// Canary Deployment Module
// Advanced deployment workflow with safety mechanisms

pub mod rollout;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use dashmap::DashMap;
use uuid::Uuid;
use anyhow::{Result, Context};
use tracing::{info, warn};

/// Canary deployment manager
pub struct CanaryManager {
    /// Active canary deployments
    canaries: Arc<DashMap<String, CanaryDeployment>>,
    
    /// Error rate tracker
    error_tracker: Arc<ErrorRateTracker>,
}

/// Canary deployment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryDeployment {
    pub site_id: String,
    pub canary_id: Uuid,
    pub beta_secret: String,
    pub created_at: Instant,
    pub traffic_percentage: f64,
    pub status: CanaryStatus,
    pub wasm_module: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanaryStatus {
    Testing,
    RollingOut,
    Completed,
    RolledBack,
}

/// Error rate tracker for canary deployments
pub struct ErrorRateTracker {
    /// Error counts per deployment
    errors: DashMap<Uuid, ErrorStats>,
}

#[derive(Debug, Clone)]
struct ErrorStats {
    total_requests: u64,
    error_count: u64,
    started_at: Instant,
}

impl CanaryManager {
    /// Create a new canary manager
    pub fn new() -> Self {
        info!("Initializing Canary Deployment Manager");
        
        Self {
            canaries: Arc::new(DashMap::new()),
            error_tracker: Arc::new(ErrorRateTracker::new()),
        }
    }

    /// Create a canary deployment
    pub fn create_canary(
        &self,
        site_id: String,
        wasm_module: Vec<u8>,
    ) -> Result<CanaryInfo> {
        let canary_id = Uuid::new_v4();
        let beta_secret = Self::generate_beta_secret();
        
        let canary = CanaryDeployment {
            site_id: site_id.clone(),
            canary_id,
            beta_secret: beta_secret.clone(),
            created_at: Instant::now(),
            traffic_percentage: 0.0,  // Start at 0%
            status: CanaryStatus::Testing,
            wasm_module,
        };
        
        self.canaries.insert(site_id.clone(), canary);
        self.error_tracker.init_tracking(canary_id);
        
        info!(
            site_id = %site_id,
            canary_id = %canary_id,
            "Canary deployment created"
        );
        
        Ok(CanaryInfo {
            canary_id,
            beta_secret: beta_secret.clone(),
            test_url: format!("https://example.com/?beta={}", beta_secret),
            cookie_header: format!("X-Pear-Beta: {}", beta_secret),
        })
    }

    /// Check if request should go to canary
    pub fn should_route_to_canary(
        &self,
        site_id: &str,
        beta_cookie: Option<&str>,
        beta_query: Option<&str>,
    ) -> bool {
        if let Some(canary) = self.canaries.get(site_id) {
            // Check for explicit beta access
            if let Some(cookie) = beta_cookie {
                if cookie == canary.beta_secret {
                    return true;
                }
            }
            
            if let Some(query) = beta_query {
                if query == canary.beta_secret {
                    return true;
                }
            }
            
            // Check traffic percentage for gradual rollout
            if canary.status == CanaryStatus::RollingOut {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let roll: f64 = rng.gen();
                return roll < canary.traffic_percentage;
            }
        }
        
        false
    }

    /// Promote canary to production (start rolling update)
    pub fn promote_to_production(&self, site_id: &str) -> Result<()> {
        let mut canary_entry = self.canaries.get_mut(site_id)
            .context("Canary not found")?;
        
        let canary = canary_entry.value_mut();
        
        if canary.status != CanaryStatus::Testing {
            anyhow::bail!("Canary must be in Testing status to promote");
        }
        
        canary.status = CanaryStatus::RollingOut;
        canary.traffic_percentage = 0.10; // Start with 10%
        
        info!(site_id = %site_id, "Starting canary rollout at 10% traffic");
        
        Ok(())
    }

    /// Increase traffic to canary
    pub fn increase_traffic(&self, site_id: &str, percentage: f64) -> Result<()> {
        let mut canary_entry = self.canaries.get_mut(site_id)
            .context("Canary not found")?;
        
        let canary = canary_entry.value_mut();
        canary.traffic_percentage = percentage.min(1.0);
        
        info!(
            site_id = %site_id,
            traffic_pct = canary.traffic_percentage * 100.0,
            "Canary traffic increased"
        );
        
        Ok(())
    }

    /// Complete canary deployment
    pub fn complete_deployment(&self, site_id: &str) -> Result<()> {
        let mut canary_entry = self.canaries.get_mut(site_id)
            .context("Canary not found")?;
        
        let canary = canary_entry.value_mut();
        canary.status = CanaryStatus::Completed;
        canary.traffic_percentage = 1.0;
        
        info!(site_id = %site_id, "Canary deployment completed");
        
        Ok(())
    }

    /// Rollback canary deployment
    pub fn rollback(&self, site_id: &str, reason: String) -> Result<()> {
        let mut canary_entry = self.canaries.get_mut(site_id)
            .context("Canary not found")?;
        
        let canary = canary_entry.value_mut();
        canary.status = CanaryStatus::RolledBack;
        canary.traffic_percentage = 0.0;
        
        warn!(
            site_id = %site_id,
            reason = %reason,
            "Canary deployment rolled back"
        );
        
        Ok(())
    }

    /// Record request result
    pub fn record_request(&self, canary_id: Uuid, is_error: bool) {
        self.error_tracker.record(canary_id, is_error);
    }

    /// Check if error rate is too high (automatic rollback trigger)
    pub fn check_error_rate(&self, site_id: &str) -> Result<bool> {
        if let Some(canary) = self.canaries.get(site_id) {
            let error_rate = self.error_tracker.error_rate(canary.canary_id);
            
            // Rollback if error rate > 5%
            if error_rate > 0.05 {
                warn!(
                    site_id = %site_id,
                    error_rate = error_rate * 100.0,
                    "High error rate detected - triggering rollback"
                );
                
                drop(canary); // Release lock before calling rollback
                self.rollback(site_id, "High error rate detected".to_string())?;
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Generate random beta secret
    fn generate_beta_secret() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        
        (0..16)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

impl Default for CanaryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorRateTracker {
    fn new() -> Self {
        Self {
            errors: DashMap::new(),
        }
    }

    fn init_tracking(&self, canary_id: Uuid) {
        self.errors.insert(canary_id, ErrorStats {
            total_requests: 0,
            error_count: 0,
            started_at: Instant::now(),
        });
    }

    fn record(&self, canary_id: Uuid, is_error: bool) {
        if let Some(mut stats) = self.errors.get_mut(&canary_id) {
            stats.total_requests += 1;
            if is_error {
                stats.error_count += 1;
            }
        }
    }

    fn error_rate(&self, canary_id: Uuid) -> f64 {
        if let Some(stats) = self.errors.get(&canary_id) {
            if stats.total_requests == 0 {
                return 0.0;
            }
            stats.error_count as f64 / stats.total_requests as f64
        } else {
            0.0
        }
    }
}

/// Canary information for user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryInfo {
    pub canary_id: Uuid,
    pub beta_secret: String,
    pub test_url: String,
    pub cookie_header: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_creation() {
        let manager = CanaryManager::new();
        
        let info = manager.create_canary(
            "test-site".to_string(),
            vec![0, 1, 2, 3],
        ).unwrap();
        
        assert!(!info.beta_secret.is_empty());
        assert!(info.test_url.contains(&info.beta_secret));
    }

    #[test]
    fn test_canary_routing() {
        let manager = CanaryManager::new();
        
        let info = manager.create_canary(
            "test-site".to_string(),
            vec![],
        ).unwrap();
        
        // Should route with correct secret
        assert!(manager.should_route_to_canary(
            "test-site",
            Some(&info.beta_secret),
            None
        ));
        
        // Should not route with wrong secret
        assert!(!manager.should_route_to_canary(
            "test-site",
            Some("wrong-secret"),
            None
        ));
    }

    #[test]
    fn test_canary_promotion() {
        let manager = CanaryManager::new();
        
        manager.create_canary("test-site".to_string(), vec![]).unwrap();
        
        manager.promote_to_production("test-site").unwrap();
        
        let canary = manager.canaries.get("test-site").unwrap();
        assert_eq!(canary.status, CanaryStatus::RollingOut);
        assert_eq!(canary.traffic_percentage, 0.10);
    }
}
