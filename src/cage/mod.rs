// Cage Architecture Module
// WebAssembly-based execution environments with strict isolation and resource limits

pub mod config;
pub mod pool;

use config::CageConfig;
use anyhow::{Result, Context};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error, instrument};
use wasmtime::*;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

/// Lifecycle states of a Cage instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CageState {
    /// Cage is being initialized
    Initializing,
    
    /// Cage is healthy and ready to handle requests
    Running,
    
    /// Cage has crashed or encountered an error
    Crashed,
    
    /// Cage is being terminated
    Terminating,
    
    /// Cage has been terminated
    Terminated,
}

impl std::fmt::Display for CageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CageState::Initializing => write!(f, "INIT"),
            CageState::Running => write!(f, "RUN"),
            CageState::Crashed => write!(f, "CRASH"),
            CageState::Terminating => write!(f, "TERM"),
            CageState::Terminated => write!(f, "STOP"),
        }
    }
}

/// A Cage represents a single WebAssembly execution environment
/// Each website/application runs inside one or more Cages for redundancy
pub struct Cage {
    /// Unique identifier for this Cage instance
    id: u64,
    
    /// Human-readable name for debugging
    name: String,
    
    /// Current lifecycle state
    state: Arc<RwLock<CageState>>,
    
    /// Wasmtime engine (shared across cages for efficiency)
    engine: Engine,
    
    /// Wasmtime store containing the instance state
    store: Arc<RwLock<Store<WasiCtx>>>,
    
    /// Loaded WebAssembly module
    module: Module,
    
    /// Configuration for this Cage
    config: CageConfig,
    
    /// Request counter for metrics
    request_count: Arc<AtomicU64>,
    
    /// Active request counter for load balancing
    active_requests: Arc<AtomicU64>,
    
    /// Health status
    healthy: Arc<AtomicBool>,
    
    /// Last health check timestamp
    last_health_check: Arc<RwLock<std::time::Instant>>,
}

impl Cage {
    /// Create a new Cage instance
    #[instrument(skip(engine, wasm_bytes))]
    pub fn new(
        id: u64,
        name: String,
        engine: Engine,
        wasm_bytes: &[u8],
        config: CageConfig,
    ) -> Result<Self> {
        info!(cage_id = id, cage_name = %name, "Creating new Cage");

        // Compile the WebAssembly module
        let module = Module::new(&engine, wasm_bytes)
            .context("Failed to compile WebAssembly module")?;

        // Create WASI context with configured permissions
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();

        // Create store with resource limits
        let mut store = Store::new(&engine, wasi);
        
        // Set memory limits
        store.limiter(|_| ResourceLimiterImpl {
            memory_limit: config.memory_limit_bytes,
        });

        let cage = Self {
            id,
            name,
            state: Arc::new(RwLock::new(CageState::Initializing)),
            engine,
            store: Arc::new(RwLock::new(store)),
            module,
            config,
            request_count: Arc::new(AtomicU64::new(0)),
            active_requests: Arc::new(AtomicU64::new(0)),
            healthy: Arc::new(AtomicBool::new(true)),
            last_health_check: Arc::new(RwLock::new(std::time::Instant::now())),
        };

        Ok(cage)
    }

    /// Initialize the Cage and transition to Running state
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> Result<()> {
        debug!(cage_id = self.id, "Initializing Cage");

        // Instantiate the module
        let mut store = self.store.write().await;
        
        // Add WASI to the linker
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

        // Instantiate and get the instance
        let _instance = linker.instantiate(&mut *store, &self.module)
            .context("Failed to instantiate WebAssembly module")?;

        drop(store);

        // Transition to Running state
        let mut state = self.state.write().await;
        *state = CageState::Running;
        
        info!(cage_id = self.id, "Cage initialized successfully");
        Ok(())
    }

    /// Execute a request in this Cage
    #[instrument(skip(self, request_data))]
    pub async fn execute_request(&self, request_data: &[u8]) -> Result<Vec<u8>> {
        // Check if Cage is healthy
        if !self.is_healthy() {
            anyhow::bail!("Cage {} is not healthy", self.id);
        }

        // Increment active request counter
        self.active_requests.fetch_add(1, Ordering::Relaxed);
        
        let start = std::time::Instant::now();
        
        // Execute the request (simplified for Phase 2 - will be enhanced)
        let response = self.execute_wasm_function(request_data).await;
        
        // Decrement active request counter
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
        
        // Increment total request counter
        self.request_count.fetch_add(1, Ordering::Relaxed);

        let duration = start.elapsed();
        debug!(
            cage_id = self.id,
            duration_ms = duration.as_millis(),
            "Request executed"
        );

        response
    }

    /// Execute a WebAssembly function (internal implementation)
    async fn execute_wasm_function(&self, _request_data: &[u8]) -> Result<Vec<u8>> {
        // For Phase 2, return a simple response
        // In Phase 3, this will actually invoke Wasm functions
        
        // Simulate some processing
        tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;

        let response = format!(
            "{{\"cage_id\":{},\"status\":\"ok\",\"message\":\"Processed by Cage {}\"}}",
            self.id, self.name
        );
        
        Ok(response.into_bytes())
    }

    /// Perform a health check
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> bool {
        let state = self.state.read().await;
        
        match *state {
            CageState::Running => {
                // Update last health check time
                let mut last_check = self.last_health_check.write().await;
                *last_check = std::time::Instant::now();
                
                self.healthy.store(true, Ordering::Relaxed);
                true
            }
            CageState::Crashed | CageState::Terminating | CageState::Terminated => {
                self.healthy.store(false, Ordering::Relaxed);
                false
            }
            CageState::Initializing => {
                // Still initializing, not ready but not unhealthy
                false
            }
        }
    }

    /// Check if Cage is healthy
    pub fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    /// Get current state
    pub async fn state(&self) -> CageState {
        *self.state.read().await
    }

    /// Get number of active requests
    pub fn active_request_count(&self) -> u64 {
        self.active_requests.load(Ordering::Relaxed)
    }

    /// Get total request count
    pub fn total_request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Get Cage ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get Cage name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Mark Cage as crashed
    pub async fn mark_crashed(&self) {
        warn!(cage_id = self.id, "Marking Cage as crashed");
        let mut state = self.state.write().await;
        *state = CageState::Crashed;
        self.healthy.store(false, Ordering::Relaxed);
    }

    /// Gracefully terminate the Cage
    #[instrument(skip(self))]
    pub async fn terminate(&self) -> Result<()> {
        info!(cage_id = self.id, "Terminating Cage");
        
        let mut state = self.state.write().await;
        *state = CageState::Terminating;
        
        // Wait for active requests to complete (with timeout)
        let timeout = tokio::time::Duration::from_secs(5);
        let start = std::time::Instant::now();
        
        while self.active_requests.load(Ordering::Relaxed) > 0 {
            if start.elapsed() > timeout {
                warn!(cage_id = self.id, "Timeout waiting for requests to complete");
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        *state = CageState::Terminated;
        self.healthy.store(false, Ordering::Relaxed);
        
        info!(cage_id = self.id, "Cage terminated");
        Ok(())
    }
}

/// Resource limiter implementation for Wasmtime
struct ResourceLimiterImpl {
    memory_limit: usize,
}

impl ResourceLimiter for ResourceLimiterImpl {
    fn memory_growing(&mut self, current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool, Error> {
        if desired > self.memory_limit {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn table_growing(&mut self, _current: u32, _desired: u32, _maximum: Option<u32>) -> Result<bool, Error> {
        Ok(true)
    }
}

/// Create a shared Wasmtime engine with optimizations
pub fn create_engine() -> Result<Engine> {
    let mut config = Config::new();
    
    // Enable optimizations
    config.cranelift_opt_level(OptLevel::Speed);
    
    // Enable parallel compilation for faster startup
    config.parallel_compilation(true);
    
    // Memory configuration
    config.static_memory_maximum_size(128 * 1024 * 1024); // 128MB max
    
    // Disable features we don't need for security
    config.wasm_threads(false);
    config.wasm_simd(true);
    config.wasm_bulk_memory(true);

    Engine::new(&config).context("Failed to create Wasmtime engine")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cage_creation() {
        // Simple WAT (WebAssembly Text) module for testing
        let wat = r#"
            (module
                (func (export "add") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add)
            )
        "#;
        
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let engine = create_engine().unwrap();
        let config = CageConfig::default();
        
        let cage = Cage::new(1, "test-cage".to_string(), engine, &wasm_bytes, config);
        assert!(cage.is_ok());
    }

    #[tokio::test]
    async fn test_cage_health_check() {
        let wat = r#"(module)"#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let engine = create_engine().unwrap();
        let config = CageConfig::default();
        
        let cage = Cage::new(1, "test-cage".to_string(), engine, &wasm_bytes, config).unwrap();
        cage.initialize().await.unwrap();
        
        let is_healthy = cage.health_check().await;
        assert!(is_healthy);
    }
}
