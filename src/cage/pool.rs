// Cage Pool - Redundant WebAssembly instance management
// Manages multiple Cage instances for a single site to ensure high availability

use super::{Cage, CageState, CageConfig, create_engine};
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error, instrument};

/// Pool of redundant Cage instances for a single site
pub struct CagePool {
    /// Site identifier
    site_id: String,
    
    /// All Cage instances in this pool
    cages: Arc<RwLock<Vec<Arc<Cage>>>>,
    
    /// Configuration for Cages in this pool
    config: CageConfig,
    
    /// Target number of replicas (default: 3)
    target_replicas: usize,
    
    /// Next Cage ID for spawning new instances
    next_cage_id: Arc<std::sync::atomic::AtomicU64>,
    
    /// Round-robin index for load balancing
    round_robin_index: Arc<std::sync::atomic::AtomicUsize>,
}

impl CagePool {
    /// Create a new CagePool
    #[instrument(skip(wasm_bytes))]
    pub async fn new(
        site_id: String,
        wasm_bytes: Vec<u8>,
        config: CageConfig,
        target_replicas: usize,
    ) -> Result<Self> {
        info!(site_id = %site_id, replicas = target_replicas, "Creating CagePool");

        let pool = Self {
            site_id: site_id.clone(),
            cages: Arc::new(RwLock::new(Vec::new())),
            config,
            target_replicas,
            next_cage_id: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            round_robin_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };

        // Spawn initial Cages
        for _ in 0..target_replicas {
            pool.spawn_cage(&wasm_bytes).await?;
        }

        info!(
            site_id = %site_id,
            active_cages = target_replicas,
            "CagePool initialized"
        );

        Ok(pool)
    }

    /// Spawn a new Cage instance
    #[instrument(skip(self, wasm_bytes))]
    async fn spawn_cage(&self, wasm_bytes: &[u8]) -> Result<Arc<Cage>> {
        let cage_id = self.next_cage_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cage_name = format!("{}-cage-{}", self.site_id, cage_id);

        debug!(site_id = %self.site_id, cage_id = cage_id, "Spawning new Cage");

        // Create engine (in production, this would be shared across pools)
        let engine = create_engine()?;

        // Create the Cage
        let cage = Cage::new(
            cage_id,
            cage_name,
            engine,
            wasm_bytes,
            self.config.clone(),
        )?;

        // Initialize the Cage
        cage.initialize().await
            .context("Failed to initialize Cage")?;

        let cage_arc = Arc::new(cage);

        // Add to pool
        let mut cages = self.cages.write().await;
        cages.push(cage_arc.clone());

        info!(
            site_id = %self.site_id,
            cage_id = cage_id,
            pool_size = cages.len(),
            "Cage spawned successfully"
        );

        Ok(cage_arc)
    }

    /// Get a healthy Cage for request execution (round-robin)
    #[instrument(skip(self))]
    pub async fn get_cage_round_robin(&self) -> Option<Arc<Cage>> {
        let cages = self.cages.read().await;
        
        if cages.is_empty() {
            warn!(site_id = %self.site_id, "No Cages available in pool");
            return None;
        }

        let start_index = self.round_robin_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % cages.len();
        
        // Try each Cage starting from the round-robin index
        for offset in 0..cages.len() {
            let index = (start_index + offset) % cages.len();
            let cage = &cages[index];
            
            if cage.is_healthy() {
                debug!(
                    site_id = %self.site_id,
                    cage_id = cage.id(),
                    strategy = "round-robin",
                    "Selected Cage for request"
                );
                return Some(cage.clone());
            }
        }

        warn!(site_id = %self.site_id, "No healthy Cages available");
        None
    }

    /// Get a healthy Cage with the least active requests
    #[instrument(skip(self))]
    pub async fn get_cage_least_connected(&self) -> Option<Arc<Cage>> {
        let cages = self.cages.read().await;
        
        if cages.is_empty() {
            warn!(site_id = %self.site_id, "No Cages available in pool");
            return None;
        }

        // Find the Cage with the least active requests
        let mut best_cage: Option<Arc<Cage>> = None;
        let mut min_requests = u64::MAX;

        for cage in cages.iter() {
            if cage.is_healthy() {
                let active = cage.active_request_count();
                if active < min_requests {
                    min_requests = active;
                    best_cage = Some(cage.clone());
                }
            }
        }

        if let Some(ref cage) = best_cage {
            debug!(
                site_id = %self.site_id,
                cage_id = cage.id(),
                active_requests = min_requests,
                strategy = "least-connected",
                "Selected Cage for request"
            );
        } else {
            warn!(site_id = %self.site_id, "No healthy Cages available");
        }

        best_cage
    }

    /// Get pool health statistics
    pub async fn health_stats(&self) -> PoolHealthStats {
        let cages = self.cages.read().await;
        
        let total = cages.len();
        let mut healthy = 0;
        let mut crashed = 0;
        let mut initializing = 0;

        for cage in cages.iter() {
            match cage.state().await {
                CageState::Running if cage.is_healthy() => healthy += 1,
                CageState::Crashed => crashed += 1,
                CageState::Initializing => initializing += 1,
                _ => {}
            }
        }

        PoolHealthStats {
            site_id: self.site_id.clone(),
            total_cages: total,
            healthy_cages: healthy,
            crashed_cages: crashed,
            initializing_cages: initializing,
        }
    }

    /// Remove crashed Cages from the pool
    #[instrument(skip(self))]
    pub async fn remove_crashed_cages(&self) -> usize {
        let mut cages = self.cages.write().await;
        let original_len = cages.len();
        
        cages.retain(|cage| {
            let state = futures::executor::block_on(cage.state());
            state != CageState::Crashed && state != CageState::Terminated
        });

        let removed = original_len - cages.len();
        
        if removed > 0 {
            info!(
                site_id = %self.site_id,
                removed = removed,
                remaining = cages.len(),
                "Removed crashed Cages from pool"
            );
        }

        removed
    }

    /// Ensure pool has the target number of healthy replicas
    #[instrument(skip(self, wasm_bytes))]
    pub async fn maintain_replicas(&self, wasm_bytes: &[u8]) -> Result<()> {
        // Remove crashed Cages
        self.remove_crashed_cages().await;

        let current_count = {
            let cages = self.cages.read().await;
            cages.len()
        };

        // Spawn new Cages if below target
        if current_count < self.target_replicas {
            let to_spawn = self.target_replicas - current_count;
            
            info!(
                site_id = %self.site_id,
                current = current_count,
                target = self.target_replicas,
                spawning = to_spawn,
                "Spawning additional Cages to meet target"
            );

            for _ in 0..to_spawn {
                if let Err(e) = self.spawn_cage(wasm_bytes).await {
                    error!(
                        site_id = %self.site_id,
                        error = %e,
                        "Failed to spawn Cage during maintenance"
                    );
                }
            }
        }

        Ok(())
    }

    /// Get site ID
    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    /// Get current number of Cages in the pool
    pub async fn size(&self) -> usize {
        self.cages.read().await.len()
    }
}

/// Health statistics for a CagePool
#[derive(Debug, Clone)]
pub struct PoolHealthStats {
    pub site_id: String,
    pub total_cages: usize,
    pub healthy_cages: usize,
    pub crashed_cages: usize,
    pub initializing_cages: usize,
}

impl PoolHealthStats {
    pub fn is_healthy(&self) -> bool {
        self.healthy_cages > 0
    }

    pub fn health_percentage(&self) -> f64 {
        if self.total_cages == 0 {
            0.0
        } else {
            (self.healthy_cages as f64 / self.total_cages as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_creation() {
        let wat = r#"(module)"#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let config = CageConfig::default();
        
        let pool = CagePool::new(
            "test-site".to_string(),
            wasm_bytes,
            config,
            3,
        ).await;
        
        assert!(pool.is_ok());
        let pool = pool.unwrap();
        assert_eq!(pool.size().await, 3);
    }

    #[tokio::test]
    async fn test_round_robin_selection() {
        let wat = r#"(module)"#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let config = CageConfig::default();
        
        let pool = CagePool::new(
            "test-site".to_string(),
            wasm_bytes,
            config,
            3,
        ).await.unwrap();

        // Should be able to get a Cage
        let cage1 = pool.get_cage_round_robin().await;
        assert!(cage1.is_some());
        
        let cage2 = pool.get_cage_round_robin().await;
        assert!(cage2.is_some());
    }

    #[tokio::test]
    async fn test_health_stats() {
        let wat = r#"(module)"#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let config = CageConfig::default();
        
        let pool = CagePool::new(
            "test-site".to_string(),
            wasm_bytes,
            config,
            3,
        ).await.unwrap();

        let stats = pool.health_stats().await;
        assert_eq!(stats.total_cages, 3);
        assert!(stats.is_healthy());
        assert!(stats.health_percentage() > 0.0);
    }
}
