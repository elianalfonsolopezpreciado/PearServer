// Self-Healing Supervisor Module
// Automatic failure detection and recovery system

pub mod monitor;

use crate::cage::pool::{CagePool, PoolHealthStats};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Interval};
use tracing::{info, warn, error, debug, instrument};
use dashmap::DashMap;

/// Supervisor configuration
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Monitoring interval in seconds
    pub monitoring_interval_secs: u64,
    
    /// Minimum time between respawn attempts (exponential backoff base)
    pub min_respawn_delay_ms: u64,
    
    /// Maximum respawn delay (exponential backoff cap)
    pub max_respawn_delay_ms: u64,
    
    /// Maximum respawn attempts before giving up
    pub max_respawn_attempts: u32,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            monitoring_interval_secs: 5,
            min_respawn_delay_ms: 1000,      // 1 second
            max_respawn_delay_ms: 60000,     // 1 minute
            max_respawn_attempts: 5,
        }
    }
}

/// Self-Healing Supervisor
/// Monitors Cage health and automatically respawns failed instances
pub struct Supervisor {
    /// Configuration
    config: SupervisorConfig,
    
    /// Map of site ID to CagePool
    pools: Arc<DashMap<String, SupervisedPool>>,
    
    /// Total healing events counter
    healing_events: Arc<std::sync::atomic::AtomicU64>,
    
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
}

/// Supervised pool with Wasm bytes for respawning
struct SupervisedPool {
    pool: Arc<CagePool>,
    wasm_bytes: Vec<u8>,
    respawn_attempts: Arc<std::sync::atomic::AtomicU32>,
    last_respawn: Arc<RwLock<Option<std::time::Instant>>>,
}

impl Supervisor {
    /// Create a new Supervisor
    pub fn new(config: SupervisorConfig) -> Self {
        info!("Initializing Self-Healing Supervisor");
        
        Self {
            config,
            pools: Arc::new(DashMap::new()),
            healing_events: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Register a CagePool for supervision
    pub fn register_pool(&self, site_id: String, pool: Arc<CagePool>, wasm_bytes: Vec<u8>) {
        info!(site_id = %site_id, "Registering pool with Supervisor");
        
        let supervised = SupervisedPool {
            pool,
            wasm_bytes,
            respawn_attempts: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            last_respawn: Arc::new(RwLock::new(None)),
        };
        
        self.pools.insert(site_id, supervised);
    }

    /// Unregister a pool
    pub fn unregister_pool(&self, site_id: &str) {
        info!(site_id = %site_id, "Unregistering pool from Supervisor");
        self.pools.remove(site_id);
    }

    /// Start the supervision loop
    #[instrument(skip(self))]
    pub async fn start(&self) {
        if self.running.swap(true, std::sync::atomic::Ordering::Relaxed) {
            warn!("Supervisor already running");
            return;
        }

        info!("Starting Self-Healing Supervisor loop");

        let pools = self.pools.clone();
        let config = self.config.clone();
        let healing_events = self.healing_events.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(config.monitoring_interval_secs)
            );

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                interval.tick().await;

                for entry in pools.iter() {
                    let site_id = entry.key();
                    let supervised = entry.value();

                    // Check pool health
                    let stats = supervised.pool.health_stats().await;

                    debug!(
                        site_id = %site_id,
                        total = stats.total_cages,
                        healthy = stats.healthy_cages,
                        crashed = stats.crashed_cages,
                        "Pool health check"
                    );

                    // If pool is unhealthy, attempt healing
                    if stats.crashed_cages > 0 || stats.healthy_cages == 0 {
                        warn!(
                            site_id = %site_id,
                            crashed = stats.crashed_cages,
                            healthy = stats.healthy_cages,
                            "Pool requires healing"
                        );

                        if let Err(e) = Self::heal_pool(
                            &supervised,
                            site_id,
                            &config,
                            &healing_events,
                        ).await {
                            error!(
                                site_id = %site_id,
                                error = %e,
                                "Failed to heal pool"
                            );
                        }
                    }
                }
            }

            info!("Supervisor loop stopped");
        });
    }

    /// Stop the supervision loop
    pub fn stop(&self) {
        info!("Stopping Supervisor");
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// Heal a pool by respawning failed Cages
    #[instrument(skip(supervised, config, healing_events))]
    async fn heal_pool(
        supervised: &SupervisedPool,
        site_id: &str,
        config: &SupervisorConfig,
        healing_events: &Arc<std::sync::atomic::AtomicU64>,
    ) -> anyhow::Result<()> {
        // Check respawn attempts
        let attempts = supervised.respawn_attempts.load(std::sync::atomic::Ordering::Relaxed);
        
        if attempts >= config.max_respawn_attempts {
            error!(
                site_id = %site_id,
                attempts = attempts,
                "Maximum respawn attempts reached - giving up"
            );
            return Ok(());
        }

        // Calculate backoff delay
        let delay_ms = Self::calculate_backoff(
            attempts,
            config.min_respawn_delay_ms,
            config.max_respawn_delay_ms,
        );

        // Check if enough time has passed since last respawn
        let mut last_respawn = supervised.last_respawn.write().await;
        if let Some(last) = *last_respawn {
            let elapsed = last.elapsed();
            if elapsed < Duration::from_millis(delay_ms) {
                debug!(
                    site_id = %site_id,
                    elapsed_ms = elapsed.as_millis(),
                    required_ms = delay_ms,
                    "Waiting for backoff period"
                );
                return Ok(());
            }
        }

        info!(
            site_id = %site_id,
            attempt = attempts + 1,
            delay_ms = delay_ms,
            "Attempting to heal pool"
        );

        // Maintain replicas (removes crashed and spawns new)
        supervised.pool.maintain_replicas(&supervised.wasm_bytes).await?;

        // Update respawn tracking
        supervised.respawn_attempts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *last_respawn = Some(std::time::Instant::now());
        healing_events.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        info!(site_id = %site_id, "Pool healing complete");

        Ok(())
    }

    /// Calculate exponential backoff delay
    fn calculate_backoff(attempts: u32, min_ms: u64, max_ms: u64) -> u64 {
        let delay = min_ms * 2u64.pow(attempts);
        delay.min(max_ms)
    }

    /// Get supervisor statistics
    pub fn stats(&self) -> SupervisorStats {
        SupervisorStats {
            supervised_pools: self.pools.len(),
            healing_events: self.healing_events.load(std::sync::atomic::Ordering::Relaxed),
            is_running: self.running.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// Supervisor statistics
#[derive(Debug, Clone)]
pub struct SupervisorStats {
    pub supervised_pools: usize,
    pub healing_events: u64,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supervisor_creation() {
        let config = SupervisorConfig::default();
        let supervisor = Supervisor::new(config);
        
        let stats = supervisor.stats();
        assert_eq!(stats.supervised_pools, 0);
        assert_eq!(stats.healing_events, 0);
    }

    #[test]
    fn test_backoff_calculation() {
        let delay0 = Supervisor::calculate_backoff(0, 1000, 60000);
        let delay1 = Supervisor::calculate_backoff(1, 1000, 60000);
        let delay2 = Supervisor::calculate_backoff(2, 1000, 60000);
        
        assert_eq!(delay0, 1000);
        assert_eq!(delay1, 2000);
        assert_eq!(delay2, 4000);
    }

    #[test]
    fn test_backoff_cap() {
        let delay = Supervisor::calculate_backoff(10, 1000, 10000);
        assert_eq!(delay, 10000); // Should be capped
    }
}
