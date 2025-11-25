// Traffic Router Module
// Intelligent request distribution across Cage instances

pub mod strategies;
pub mod health;

use crate::cage::pool::CagePool;
use anyhow::{Result, Context};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, debug, warn, error, instrument};
use hyper::{Request, Response, StatusCode};
use hyper::body::{Incoming, Bytes};
use http_body_util::Full;

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    /// Distribute requests in round-robin fashion
    RoundRobin,
    
    /// Route to Cage with fewest active connections
    LeastConnected,
}

/// Router configuration
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Load balancing strategy
    pub strategy: LoadBalancingStrategy,
    
    /// Enable health checking
    pub health_check_enabled: bool,
    
    /// Health check interval in seconds
    pub health_check_interval_secs: u64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::RoundRobin,
            health_check_enabled: true,
            health_check_interval_secs: 5,
        }
    }
}

/// Traffic Router coordinating request distribution
pub struct Router {
    /// Map of site ID to CagePool
    pools: Arc<DashMap<String, Arc<CagePool>>>,
    
    /// Router configuration
    config: RouterConfig,
    
    /// Request counter for metrics
    total_requests: Arc<std::sync::atomic::AtomicU64>,
    
    /// Success counter
    successful_requests: Arc<std::sync::atomic::AtomicU64>,
    
    /// Failed requests counter
    failed_requests: Arc<std::sync::atomic::AtomicU64>,
}

impl Router {
    /// Create a new Router
    pub fn new(config: RouterConfig) -> Self {
        info!("Initializing Traffic Router");
        
        Self {
            pools: Arc::new(DashMap::new()),
            config,
            total_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            successful_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            failed_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Register a CagePool for a site
    pub fn register_pool(&self, site_id: String, pool: Arc<CagePool>) {
        info!(site_id = %site_id, "Registering CagePool with Router");
        self.pools.insert(site_id, pool);
    }

    /// Unregister a CagePool
    pub fn unregister_pool(&self, site_id: &str) {
        info!(site_id = %site_id, "Unregistering CagePool from Router");
        self.pools.remove(site_id);
    }

    /// Route an HTTP request to the appropriate Cage
    #[instrument(skip(self, req), fields(method = %req.method(), uri = %req.uri()))]
    pub async fn route_request(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let start = std::time::Instant::now();
        
        // Extract site ID from request (simplified - in production, use Host header)
        let site_id = self.extract_site_id(&req);
        
        debug!(site_id = %site_id, "Routing request to site");

        // Get the CagePool for this site
        let pool = match self.pools.get(&site_id) {
            Some(pool) => pool.clone(),
            None => {
                warn!(site_id = %site_id, "No CagePool found for site");
                self.failed_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Ok(self.error_response(
                    StatusCode::NOT_FOUND,
                    "Site not found",
                ));
            }
        };

        // Select a Cage based on load balancing strategy
        let cage = match self.config.strategy {
            LoadBalancingStrategy::RoundRobin => pool.get_cage_round_robin().await,
            LoadBalancingStrategy::LeastConnected => pool.get_cage_least_connected().await,
        };

        let cage = match cage {
            Some(cage) => cage,
            None => {
                error!(site_id = %site_id, "No healthy Cages available");
                self.failed_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Ok(self.error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "No healthy instances available",
                ));
            }
        };

        // Execute request in the selected Cage
        let request_data = self.serialize_request(&req).await;
        
        match cage.execute_request(&request_data).await {
            Ok(response_data) => {
                self.successful_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                let duration = start.elapsed();
                debug!(
                    site_id = %site_id,
                    cage_id = cage.id(),
                    duration_ms = duration.as_millis(),
                    "Request routed successfully"
                );

                Ok(self.build_response(response_data))
            }
            Err(e) => {
                error!(
                    site_id = %site_id,
                    cage_id = cage.id(),
                    error = %e,
                    "Cage execution failed"
                );
                self.failed_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                Ok(self.error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Request execution failed",
                ))
            }
        }
    }

    /// Extract site ID from request (simplified)
    fn extract_site_id(&self, req: &Request<Incoming>) -> String {
        // In production, extract from Host header
        // For Phase 2, use a default site
        req.headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("default-site")
            .to_string()
    }

    /// Serialize request for Cage execution
    async fn serialize_request(&self, req: &Request<Incoming>) -> Vec<u8> {
        // Simplified serialization for Phase 2
        // In production, serialize full HTTP request
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        
        format!("{{\"method\":\"{}\",\"uri\":\"{}\"}}", method, uri).into_bytes()
    }

    /// Build HTTP response from Cage output
    fn build_response(&self, data: Vec<u8>) -> Response<Full<Bytes>> {
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("X-Powered-By", "Pear-Server/0.2.0")
            .header("X-Routed-By", "Cage-Router")
            .body(Full::new(Bytes::from(data)))
            .unwrap()
    }

    /// Build error response
    fn error_response(&self, status: StatusCode, message: &str) -> Response<Full<Bytes>> {
        let body = serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap()
    }

    /// Get router statistics
    pub fn stats(&self) -> RouterStats {
        RouterStats {
            total_requests: self.total_requests.load(std::sync::atomic::Ordering::Relaxed),
            successful_requests: self.successful_requests.load(std::sync::atomic::Ordering::Relaxed),
            failed_requests: self.failed_requests.load(std::sync::atomic::Ordering::Relaxed),
            active_pools: self.pools.len(),
        }
    }

    /// Start health checking loop
    pub async fn start_health_checks(&self) {
        if !self.config.health_check_enabled {
            return;
        }

        let pools = self.pools.clone();
        let interval_secs = self.config.health_check_interval_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            
            loop {
                interval.tick().await;
                
                for entry in pools.iter() {
                    let site_id = entry.key();
                    let pool = entry.value();
                    
                    let stats = pool.health_stats().await;
                    
                    if !stats.is_healthy() {
                        warn!(
                            site_id = %site_id,
                            healthy = stats.healthy_cages,
                            total = stats.total_cages,
                            "Pool health degraded"
                        );
                    }
                }
            }
        });

        info!("Health check loop started");
    }

    /// Get number of registered pools
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

/// Router statistics
#[derive(Debug, Clone)]
pub struct RouterStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub active_pools: usize,
}

impl RouterStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let config = RouterConfig::default();
        let router = Router::new(config);
        assert_eq!(router.pool_count(), 0);
    }

    #[test]
    fn test_router_stats() {
        let router = Router::new(RouterConfig::default());
        let stats = router.stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.success_rate(), 0.0);
    }
}
