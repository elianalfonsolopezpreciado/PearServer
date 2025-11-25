// Configuration Management Module
// Handles pear.toml loading, defaults, and validation

pub mod acme;

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Result, Context};
use tracing::{info, warn};

/// Main Pear Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PearConfig {
    #[serde(default)]
    pub server: ServerConfig,
    
    #[serde(default)]
    pub ssl: SslConfig,
    
    #[serde(default)]
    pub cages: CagesConfig,
    
    #[serde(default)]
    pub ai: AiConfig,
    
    #[serde(default)]
    pub dashboard: DashboardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_http2_port")]
    pub http2_port: u16,
    
    #[serde(default = "default_http3_port")]
    pub http3_port: u16,
    
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    #[serde(default)]
    pub auto_cert: bool,
    
    #[serde(default)]
    pub email: Option<String>,
    
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CagesConfig {
    #[serde(default = "default_replicas")]
    pub default_replicas: usize,
    
    #[serde(default = "default_memory_limit")]
    pub memory_limit_mb: usize,
    
    #[serde(default = "default_cpu_timeout")]
    pub cpu_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_true")]
    pub enable_anomaly_detection: bool,
    
    #[serde(default = "default_threshold")]
    pub anomaly_threshold: f64,
    
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    #[serde(default = "default_dashboard_port")]
    pub port: u16,
    
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// Default value functions
fn default_http2_port() -> u16 { 8080 }
fn default_http3_port() -> u16 { 8443 }
fn default_dashboard_port() -> u16 { 9000 }
fn default_bind_addr() -> String { "0.0.0.0".to_string() }
fn default_replicas() -> usize { 3 }
fn default_memory_limit() -> usize { 128 }
fn default_cpu_timeout() -> u64 { 1000 }
fn default_threshold() -> f64 { 0.8 }
fn default_sample_rate() -> f64 { 0.1 }
fn default_true() -> bool { true }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http2_port: default_http2_port(),
            http3_port: default_http3_port(),
            bind_addr: default_bind_addr(),
        }
    }
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            auto_cert: false,
            email: None,
            domains: Vec::new(),
        }
    }
}

impl Default for CagesConfig {
    fn default() -> Self {
        Self {
            default_replicas: default_replicas(),
            memory_limit_mb: default_memory_limit(),
            cpu_timeout_ms: default_cpu_timeout(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enable_anomaly_detection: default_true(),
            anomaly_threshold: default_threshold(),
            sample_rate: default_sample_rate(),
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            port: default_dashboard_port(),
            enabled: default_true(),
        }
    }
}

impl Default for PearConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            ssl: SslConfig::default(),
            cages: CagesConfig::default(),
            ai: AiConfig::default(),
            dashboard: DashboardConfig::default(),
        }
    }
}

impl PearConfig {
    /// Load configuration from file or use defaults
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if path.exists() {
            info!("Loading configuration from {}", path.display());
            let contents = std::fs::read_to_string(path)
                .context("Failed to read configuration file")?;
            
            let config: PearConfig = toml::from_str(&contents)
                .context("Failed to parse configuration file")?;
            
            config.validate()?;
            Ok(config)
        } else {
            warn!("Configuration file not found, using defaults");
            info!("Create pear.toml to customize configuration");
            Ok(Self::default())
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate ports
        if self.server.http2_port == 0 {
            anyhow::bail!("HTTP/2 port cannot be 0");
        }
        
        if self.server.http3_port == 0 {
            anyhow::bail!("HTTP/3 port cannot be 0");
        }
        
        if self.dashboard.port == 0 {
            anyhow::bail!("Dashboard port cannot be 0");
        }
        
        // Validate Cage config
        if self.cages.default_replicas == 0 {
            anyhow::bail!("Default replicas must be at least 1");
        }
        
        if self.cages.memory_limit_mb < 16 {
            anyhow::bail!("Memory limit must be at least 16MB");
        }
        
        // Validate AI config
        if self.ai.anomaly_threshold < 0.0 || self.ai.anomaly_threshold > 1.0 {
            anyhow::bail!("Anomaly threshold must be between 0.0 and 1.0");
        }
        
        if self.ai.sample_rate < 0.0 || self.ai.sample_rate > 1.0 {
            anyhow::bail!("Sample rate must be between 0.0 and 1.0");
        }
        
        // Validate SSL config
        if self.ssl.auto_cert {
            if self.ssl.email.is_none() {
                anyhow::bail!("Email is required when auto_cert is enabled");
            }
            
            if self.ssl.domains.is_empty() {
                anyhow::bail!("At least one domain is required when auto_cert is enabled");
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PearConfig::default();
        assert_eq!(config.server.http2_port, 8080);
        assert_eq!(config.cages.default_replicas, 3);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_threshold() {
        let mut config = PearConfig::default();
        config.ai.anomaly_threshold = 1.5;
        assert!(config.validate().is_err());
    }
}
