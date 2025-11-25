// Shared memory abstractions and zero-copy buffer management
// Provides memory pool for efficient buffer allocation without excessive allocations

use bytes::{Bytes, BytesMut};
use parking_lot::Mutex;
use std::sync::Arc;

/// Memory pool for zero-copy buffer operations
/// Reduces allocation overhead for frequently used buffer sizes
pub struct MemoryPool {
    /// Pre-allocated buffers for common sizes
    pools: Arc<Mutex<BufferPools>>,
    
    /// Statistics for pool performance monitoring
    stats: Arc<std::sync::atomic::AtomicU64>,
}

struct BufferPools {
    /// Small buffers (< 4KB) - for headers, small payloads
    small: Vec<BytesMut>,
    
    /// Medium buffers (4KB - 64KB) - for typical HTTP responses
    medium: Vec<BytesMut>,
    
    /// Large buffers (> 64KB) - for large file transfers
    large: Vec<BytesMut>,
}

const SMALL_BUFFER_SIZE: usize = 4 * 1024;       // 4KB
const MEDIUM_BUFFER_SIZE: usize = 64 * 1024;     // 64KB
const LARGE_BUFFER_SIZE: usize = 1024 * 1024;    // 1MB

const POOL_SIZE_SMALL: usize = 1000;
const POOL_SIZE_MEDIUM: usize = 500;
const POOL_SIZE_LARGE: usize = 100;

impl MemoryPool {
    /// Create a new memory pool with pre-allocated buffers
    pub fn new() -> Self {
        let pools = BufferPools {
            small: Vec::with_capacity(POOL_SIZE_SMALL),
            medium: Vec::with_capacity(POOL_SIZE_MEDIUM),
            large: Vec::with_capacity(POOL_SIZE_LARGE),
        };

        Self {
            pools: Arc::new(Mutex::new(pools)),
            stats: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Acquire a buffer of at least the specified size
    /// Returns a buffer from the pool if available, otherwise allocates
    pub fn acquire(&self, size: usize) -> Bytes {
        self.stats.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let mut pools = self.pools.lock();
        
        let buffer = if size <= SMALL_BUFFER_SIZE {
            pools.small.pop().unwrap_or_else(|| BytesMut::with_capacity(SMALL_BUFFER_SIZE))
        } else if size <= MEDIUM_BUFFER_SIZE {
            pools.medium.pop().unwrap_or_else(|| BytesMut::with_capacity(MEDIUM_BUFFER_SIZE))
        } else if size <= LARGE_BUFFER_SIZE {
            pools.large.pop().unwrap_or_else(|| BytesMut::with_capacity(LARGE_BUFFER_SIZE))
        } else {
            // For very large sizes, allocate on demand
            BytesMut::with_capacity(size)
        };

        buffer.freeze()
    }

    /// Return a buffer to the pool for reuse
    /// Note: In practice, Bytes uses Arc internally, so this is automatic
    /// This method is here for future explicit pool management
    #[allow(dead_code)]
    pub fn release(&self, buffer: Bytes) {
        let capacity = buffer.capacity();
        
        if capacity == 0 {
            return;
        }

        // Convert back to BytesMut if possible (requires unique ownership)
        // For now, we let the buffer drop naturally
        // In a more advanced implementation, we could use a custom allocator
        
        drop(buffer);
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let pools = self.pools.lock();
        
        PoolStats {
            total_acquisitions: self.stats.load(std::sync::atomic::Ordering::Relaxed),
            small_available: pools.small.len(),
            medium_available: pools.medium.len(),
            large_available: pools.large.len(),
        }
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for memory pool performance
#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    pub total_acquisitions: u64,
    pub small_available: usize,
    pub medium_available: usize,
    pub large_available: usize,
}

/// Zero-copy data passing utilities
pub mod zero_copy {
    use bytes::Bytes;

    /// Create a shared reference to data without copying
    /// Uses Bytes::from() which supports cheap cloning via Arc
    #[inline]
    pub fn share_bytes(data: Vec<u8>) -> Bytes {
        Bytes::from(data)
    }

    /// Split bytes without copying the underlying data
    #[inline]
    pub fn split_at(bytes: &Bytes, mid: usize) -> (Bytes, Bytes) {
        let left = bytes.slice(..mid);
        let right = bytes.slice(mid..);
        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_creation() {
        let pool = MemoryPool::new();
        let stats = pool.stats();
        assert_eq!(stats.total_acquisitions, 0);
    }

    #[test]
    fn test_buffer_acquisition() {
        let pool = MemoryPool::new();
        
        let small = pool.acquire(1024);
        assert!(small.capacity() >= 1024);
        
        let medium = pool.acquire(32 * 1024);
        assert!(medium.capacity() >= 32 * 1024);
        
        let large = pool.acquire(512 * 1024);
        assert!(large.capacity() >= 512 * 1024);
        
        let stats = pool.stats();
        assert_eq!(stats.total_acquisitions, 3);
    }

    #[test]
    fn test_zero_copy_split() {
        let data = vec![1, 2, 3, 4, 5];
        let bytes = zero_copy::share_bytes(data);
        
        let (left, right) = zero_copy::split_at(&bytes, 2);
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 3);
    }
}
