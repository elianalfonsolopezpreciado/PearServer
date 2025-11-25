// Zero-Copy Bind Mount Storage Module
// Shared read-only access across Cages using Wasmtime preopened directories

pub mod bind_mount;

use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use uuid::Uuid;

/// Storage manager for tenant and site files
pub struct StorageManager {
    /// Base storage directory
    base_path: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Ensure base directory exists
        std::fs::create_dir_all(&base_path)
            .context("Failed to create storage base directory")?;
        
        info!(path = %base_path.display(), "Storage manager initialized");
        
        Ok(Self { base_path })
    }

    /// Get tenant directory path
    pub fn tenant_dir(&self, tenant_id: Uuid) -> PathBuf {
        self.base_path.join("tenants").join(tenant_id.to_string())
    }

    /// Get site directory path  
    pub fn site_dir(&self, tenant_id: Uuid, site_id: &str) -> PathBuf {
        self.tenant_dir(tenant_id).join("sites").join(site_id)
    }

    /// Create tenant directory
    pub fn create_tenant_storage(&self, tenant_id: Uuid) -> Result<PathBuf> {
        let tenant_dir = self.tenant_dir(tenant_id);
        std::fs::create_dir_all(&tenant_dir)
            .with_context(|| format!("Failed to create tenant directory: {}", tenant_dir.display()))?;
        
        info!(tenant_id = %tenant_id, path = %tenant_dir.display(), "Tenant storage created");
        
        Ok(tenant_dir)
    }

    /// Create site directory
    pub fn create_site_storage(&self, tenant_id: Uuid, site_id: &str) -> Result<PathBuf> {
        let site_dir = self.site_dir(tenant_id, site_id);
        std::fs::create_dir_all(&site_dir)
            .with_context(|| format!("Failed to create site directory: {}", site_dir.display()))?;
        
        info!(
            tenant_id = %tenant_id,
            site_id = %site_id,
            path = %site_dir.display(),
            "Site storage created"
        );
        
        Ok(site_dir)
    }

    /// Calculate storage usage for a directory
    pub fn calculate_usage<P: AsRef<Path>>(&self, path: P) -> Result<usize> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Ok(0);
        }

        let mut total_bytes = 0usize;
        
        for entry in walkdir::WalkDir::new(path) {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        if let Ok(metadata) = entry.metadata() {
                            total_bytes += metadata.len() as usize;
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Error walking directory for usage calculation");
                }
            }
        }

        Ok(total_bytes)
    }

    /// Get storage usage in MB
    pub fn usage_mb<P: AsRef<Path>>(&self, path: P) -> Result<usize> {
        let bytes = self.calculate_usage(path)?;
        Ok(bytes / (1024 * 1024))
    }

    /// Check if storage quota is exceeded
    pub fn check_quota(&self, used_mb: usize, limit_mb: usize) -> Result<()> {
        if used_mb > limit_mb {
            anyhow::bail!(
                "Storage quota exceeded: {} MB used, {} MB limit",
                used_mb,
                limit_mb
            );
        }
        Ok(())
    }

    /// Delete tenant storage
    pub fn delete_tenant_storage(&self, tenant_id: Uuid) -> Result<()> {
        let tenant_dir = self.tenant_dir(tenant_id);
        
        if tenant_dir.exists() {
            std::fs::remove_dir_all(&tenant_dir)
                .with_context(|| format!("Failed to delete tenant storage: {}", tenant_dir.display()))?;
            
            info!(tenant_id = %tenant_id, "Tenant storage deleted");
        }
        
        Ok(())
    }

    /// Delete site storage
    pub fn delete_site_storage(&self, tenant_id: Uuid, site_id: &str) -> Result<()> {
        let site_dir = self.site_dir(tenant_id, site_id);
        
        if site_dir.exists() {
            std::fs::remove_dir_all(&site_dir)
                .with_context(|| format!("Failed to delete site storage: {}", site_dir.display()))?;
            
            info!(tenant_id = %tenant_id, site_id = %site_id, "Site storage deleted");
        }
        
        Ok(())
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new("/srv/pear-storage").expect("Failed to initialize default storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_storage_manager_creation() {
        let temp = TempDir::new().unwrap();
        let manager = StorageManager::new(temp.path()).unwrap();
        
        assert_eq!(manager.base_path, temp.path());
    }

    #[test]
    fn test_tenant_storage_creation() {
        let temp = TempDir::new().unwrap();
        let manager = StorageManager::new(temp.path()).unwrap();
        
        let tenant_id = Uuid::new_v4();
        let tenant_dir = manager.create_tenant_storage(tenant_id).unwrap();
        
        assert!(tenant_dir.exists());
        assert!(tenant_dir.is_dir());
    }

    #[test]
    fn test_usage_calculation() {
        let temp = TempDir::new().unwrap();
        let manager = StorageManager::new(temp.path()).unwrap();
        
        let tenant_id = Uuid::new_v4();
        let site_dir = manager.create_site_storage(tenant_id, "test-site").unwrap();
        
        // Write a file
        std::fs::write(site_dir.join("test.txt"), "Hello, World!").unwrap();
        
        let usage = manager.calculate_usage(&site_dir).unwrap();
        assert!(usage > 0);
    }
}
