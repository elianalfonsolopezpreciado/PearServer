// Global state management with safe concurrency patterns
// Foundation for future CRDT databases and WebAssembly runtimes

pub mod shared_memory;

use dashmap::DashMap;
use std::sync::Arc;
use bytes::Bytes;

/// Global state manager for the Pear Server
/// Uses Arc for shared ownership across async tasks
/// This is the foundation that will later host:
/// - CRDT-based state synchronization
/// - WebAssembly runtime pools
/// - Internal traffic routing tables
#[derive(Clone)]
pub struct GlobalState {
    /// Connection metadata indexed by connection ID
    /// DashMap provides lock-free concurrent access
    connections: Arc<DashMap<u64, ConnectionMetadata>>,
    
    /// Shared configuration that can be updated without locks
    /// Uses arc-swap for wait-free reads
    config: Arc<arc_swap::ArcSwap<ServerConfig>>,
    
    /// Request counter for generating unique IDs
    request_counter: Arc<std::sync::atomic::AtomicU64>,

    /// Shared memory pool for zero-copy operations
    memory_pool: Arc<shared_memory::MemoryPool>,
}

impl GlobalState {
    /// Create a new global state manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            config: Arc::new(arc_swap::ArcSwap::from_pointee(ServerConfig::default())),
            request_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            memory_pool: Arc::new(shared_memory::MemoryPool::new()),
        }
    }

    /// Generate a unique request ID
    pub fn next_request_id(&self) -> u64 {
        self.request_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Register a new connection
    pub fn register_connection(&self, conn_id: u64, metadata: ConnectionMetadata) {
        self.connections.insert(conn_id, metadata);
    }

    /// Remove a connection
    pub fn remove_connection(&self, conn_id: u64) {
        self.connections.remove(&conn_id);
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Get the current server configuration (wait-free read)
    pub fn config(&self) -> arc_swap::Guard<Arc<ServerConfig>> {
        self.config.load()
    }

    /// Acquire a buffer from the memory pool
    pub fn acquire_buffer(&self, size: usize) -> Bytes {
        self.memory_pool.acquire(size)
    }

    /// Get memory pool statistics
    pub fn memory_pool_stats(&self) -> shared_memory::PoolStats {
        self.memory_pool.stats()
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata for active connections
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    pub protocol: Protocol,
    pub remote_addr: String,
    pub connected_at: std::time::Instant,
    pub request_count: u64,
}

/// Protocol type
#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    Http2,
    Http3,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Http2 => write!(f, "HTTP/2"),
            Protocol::Http3 => write!(f, "HTTP/3"),
        }
    }
}

/// Server configuration (can be hot-reloaded)
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub max_connections: usize,
    pub request_timeout_ms: u64,
    pub enable_compression: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1_000_000,
            request_timeout_ms: 30_000,
            enable_compression: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_state_creation() {
        let state = GlobalState::new();
        assert_eq!(state.connection_count(), 0);
    }

    #[test]
    fn test_request_id_generation() {
        let state = GlobalState::new();
        let id1 = state.next_request_id();
        let id2 = state.next_request_id();
        assert!(id2 > id1);
    }

    #[test]
    fn test_connection_management() {
        let state = GlobalState::new();
        
        let metadata = ConnectionMetadata {
            protocol: Protocol::Http2,
            remote_addr: "127.0.0.1:1234".to_string(),
            connected_at: std::time::Instant::now(),
            request_count: 0,
        };
        
        state.register_connection(1, metadata);
        assert_eq!(state.connection_count(), 1);
        
        state.remove_connection(&1);
        assert_eq!(state.connection_count(), 0);
    }
}
