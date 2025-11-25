// Cage configuration
// Defines resource limits and WASI permissions for Cage instances

use serde::{Deserialize, Serialize};

/// Configuration for a Cage instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CageConfig {
    /// Maximum memory in bytes (default: 128MB)
    pub memory_limit_bytes: usize,
    
    /// Maximum CPU time per request in milliseconds (default: 1000ms)
    pub cpu_timeout_ms: u64,
    
    /// Maximum number of concurrent requests per Cage
    pub max_concurrent_requests: usize,
    
    /// Allow filesystem access
    pub allow_filesystem: bool,
    
    /// Allow network access
    pub allow_network: bool,
    
    /// Preopened directories (if filesystem is allowed)
    pub preopen_dirs: Vec<String>,
}

impl Default for CageConfig {
    fn default() -> Self {
        Self {
            memory_limit_bytes: 128 * 1024 * 1024, // 128MB
            cpu_timeout_ms: 1000,                    // 1 second
            max_concurrent_requests: 100,
            allow_filesystem: false,                 // Disabled by default for security
            allow_network: false,                    // Disabled by default for security
            preopen_dirs: vec![],
        }
    }
}

impl CageConfig {
    /// Create a development configuration with relaxed limits
    pub fn development() -> Self {
        Self {
            memory_limit_bytes: 256 * 1024 * 1024, // 256MB
            cpu_timeout_ms: 5000,                    // 5 seconds
            max_concurrent_requests: 50,
            allow_filesystem: true,
            allow_network: true,
            preopen_dirs: vec![],
        }
    }

    /// Create a production configuration with strict limits
    pub fn production() -> Self {
        Self {
            memory_limit_bytes: 64 * 1024 * 1024,  // 64MB
            cpu_timeout_ms: 500,                     // 500ms
            max_concurrent_requests: 200,
            allow_filesystem: false,
            allow_network: false,
            preopen_dirs: vec![],
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.memory_limit_bytes < 1024 * 1024 {
            return Err("Memory limit must be at least 1MB".to_string());
        }
        
        if self.cpu_timeout_ms == 0 {
            return Err("CPU timeout must be greater than 0".to_string());
        }
        
        if self.max_concurrent_requests == 0 {
            return Err("Max concurrent requests must be greater than 0".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CageConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.memory_limit_bytes, 128 * 1024 * 1024);
    }

    #[test]
    fn test_production_config() {
        let config = CageConfig::production();
        assert!(config.validate().is_ok());
        assert!(!config.allow_filesystem);
        assert!(!config.allow_network);
    }

    #[test]
    fn test_validation() {
        let mut config = CageConfig::default();
        config.memory_limit_bytes = 1024; // Too small
        assert!(config.validate().is_err());
    }
}
