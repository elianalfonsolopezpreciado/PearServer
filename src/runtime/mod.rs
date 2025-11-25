    limits::set_file_descriptor_limit()?;
    limits::log_system_info();
    Ok(())
}

/// Get optimal worker thread count for the Tokio runtime
/// Returns the number of CPU cores available
pub fn worker_thread_count() -> usize {
    num_cpus::get()
}

/// Configuration for Tokio runtime
/// Note: When using #[tokio::main], these settings are applied via macro attributes
/// This struct documents our runtime requirements
pub struct RuntimeConfig {
    /// Number of worker threads (defaults to CPU count)
    pub worker_threads: usize,
    
    /// Thread stack size (8MB for deep async call stacks)
    pub thread_stack_size: usize,
    
    /// Enable I/O driver
    pub enable_io: bool,
    
    /// Enable time driver
    pub enable_time: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: worker_thread_count(),
            thread_stack_size: 8 * 1024 * 1024, // 8MB
            enable_io: true,
            enable_time: true,
        }
    }
}

impl RuntimeConfig {
    pub fn log_info(&self) {
        info!(
            worker_threads = self.worker_threads,
            thread_stack_size = self.thread_stack_size,
            "Tokio runtime configuration"
        );
    }
}

// Re-export num_cpus for convenience
use num_cpus;
