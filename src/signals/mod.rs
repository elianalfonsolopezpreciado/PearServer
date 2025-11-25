// Unix signal handling for graceful shutdown
// Captures SIGTERM and SIGINT to allow clean resource cleanup

use anyhow::Result;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook_tokio::Signals;
use futures::StreamExt;
use tracing::{info, debug};

/// Create a future that resolves when a shutdown signal is received
/// Listens for SIGTERM and SIGINT (Ctrl+C)
pub fn create_shutdown_listener() -> Result<impl std::future::Future<Output = ()>> {
    let signals = Signals::new(&[SIGTERM, SIGINT])?;
    
    Ok(async move {
        let mut signals = signals;
        
        while let Some(signal) = signals.next().await {
            match signal {
                SIGTERM => {
                    info!("Received SIGTERM - graceful shutdown initiated");
                    break;
                }
                SIGINT => {
                    info!("Received SIGINT (Ctrl+C) - graceful shutdown initiated");
                    break;
                }
                _ => {
                    debug!("Received unexpected signal: {}", signal);
                }
            }
        }
    })
}

/// Signal-safe shutdown coordinator
/// Ensures all subsystems are notified and have time to cleanup
pub struct ShutdownCoordinator {
    /// Broadcast channel for notifying subsystems
    tx: tokio::sync::broadcast::Sender<()>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(16);
        Self { tx }
    }

    /// Subscribe to shutdown notifications
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    /// Trigger shutdown across all subsystems
    pub fn trigger(&self) {
        let _ = self.tx.send(());
        info!("Shutdown signal broadcast to all subsystems");
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_coordinator() {
        let coordinator = ShutdownCoordinator::new();
        let mut rx = coordinator.subscribe();
        
        coordinator.trigger();
        
        // Should receive shutdown signal
        assert!(rx.try_recv().is_ok());
    }
}
