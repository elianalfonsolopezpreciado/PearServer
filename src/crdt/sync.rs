// CRDT Synchronization Protocol
// Manages state propagation between Cage instances

use super::CrdtStateManager;
use std::sync::Arc;
use tokio::time::{Duration, Interval};
use tracing::{info, debug, error, instrument};

/// Synchronization configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Sync interval in milliseconds
    pub sync_interval_ms: u64,
    
    /// Enable batched synchronization
    pub enable_batching: bool,
    
    /// Batch size (number of changes before forcing sync)
    pub batch_size: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_ms: 100,  // Sync every 100ms
            enable_batching: true,
            batch_size: 10,
        }
    }
}

/// Synchronization coordinator
/// Manages periodic state sync between Cages in a pool
pub struct SyncCoordinator {
    config: SyncConfig,
    running: Arc<std::sync::atomic::AtomicBool>,
    sync_count: Arc<std::sync::atomic::AtomicU64>,
}

impl SyncCoordinator {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            sync_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Start synchronization loop for a set of state managers
    #[instrument(skip(self, managers))]
    pub async fn start(&self, managers: Vec<Arc<CrdtStateManager>>) {
        if self.running.swap(true, std::sync::atomic::Ordering::Relaxed) {
            info!("Sync coordinator already running");
            return;
        }

        info!(
            interval_ms = self.config.sync_interval_ms,
            num_managers = managers.len(),
            "Starting CRDT synchronization loop"
        );

        let config = self.config.clone();
        let running = self.running.clone();
        let sync_count = self.sync_count.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_millis(config.sync_interval_ms)
            );

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                interval.tick().await;

                // Synchronize all managers
                if let Err(e) = Self::sync_all(&managers).await {
                    error!(error = %e, "Synchronization failed");
                } else {
                    sync_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    debug!(sync_count = sync_count.load(std::sync::atomic::Ordering::Relaxed), "Sync completed");
                }
            }

            info!("Synchronization loop stopped");
        });
    }

    /// Synchronize all managers (delta-based)
    async fn sync_all(managers: &[Arc<CrdtStateManager>]) -> anyhow::Result<()> {
        if managers.len() < 2 {
            return Ok(());
        }

        // Collect changes from all managers
        let mut all_changes = Vec::new();
        
        for manager in managers {
            let changes = manager.get_changes().await?;
            all_changes.push(changes);
        }

        // Apply all changes to all managers (excluding self)
        for (i, manager) in managers.iter().enumerate() {
            for (j, changes) in all_changes.iter().enumerate() {
                if i != j {
                    manager.apply_changes(changes).await?;
                }
            }
        }

        Ok(())
    }

    /// Stop synchronization
    pub fn stop(&self) {
        info!("Stopping sync coordinator");
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get sync statistics
    pub fn stats(&self) -> SyncStats {
        SyncStats {
            total_syncs: self.sync_count.load(std::sync::atomic::Ordering::Relaxed),
            is_running: self.running.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// Synchronization statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    pub total_syncs: u64,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::CrdtStateManager;

    #[tokio::test]
    async fn test_sync_coordinator_creation() {
        let config = SyncConfig::default();
        let coordinator = SyncCoordinator::new(config);
        
        let stats = coordinator.stats();
        assert_eq!(stats.total_syncs, 0);
        assert!(!stats.is_running);
    }

    #[tokio::test]
    async fn test_multi_manager_sync() {
        let manager1 = Arc::new(CrdtStateManager::new("site1".to_string()));
        let manager2 = Arc::new(CrdtStateManager::new("site2".to_string()));
        
        // Set different values
        manager1.set("key1", serde_json::json!("value1")).await.unwrap();
        manager2.set("key2", serde_json::json!("value2")).await.unwrap();
        
        // Sync
        SyncCoordinator::sync_all(&[manager1.clone(), manager2.clone()]).await.unwrap();
        
        // Both should have both keys
        assert!(manager1.get("key2").await.unwrap().is_some());
        assert!(manager2.get("key1").await.unwrap().is_some());
    }
}
