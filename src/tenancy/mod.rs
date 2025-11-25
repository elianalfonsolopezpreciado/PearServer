// Multi-Tenancy Module
// Complete tenant isolation and resource management

pub mod auth;
pub mod quota;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use dashmap::DashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{Result, Context};
use tracing::{info, warn};

/// Tenant manager
pub struct TenantManager {
    /// All tenants
    tenants: Arc<DashMap<Uuid, Tenant>>,
    
    /// Default tenant (for backward compatibility)
    default_tenant_id: Uuid,
}

/// Tenant data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub quota: ResourceQuota,
    pub sites: Vec<Site>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: TenantStatus,
}

/// Site within a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub domain: Option<String>,
    pub cage_count: usize,
    pub storage_used_mb: usize,
    pub created_at: DateTime<Utc>,
}

/// Resource quota per tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub max_sites: usize,
    pub max_storage_gb: usize,
    pub max_memory_per_cage_mb: usize,
    pub max_cages_per_site: usize,
    pub max_requests_per_second: Option<usize>,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_sites: 5,
            max_storage_gb: 10,
            max_memory_per_cage_mb: 128,
            max_cages_per_site: 3,
            max_requests_per_second: None,
        }
    }
}

/// Tenant status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TenantStatus {
    Active,
    Suspended,
    Deleted,
}

impl TenantManager {
    /// Create a new tenant manager
    pub fn new() -> Self {
        info!("Initializing Tenant Manager");
        
        let default_tenant_id = Uuid::new_v4();
        let tenants = Arc::new(DashMap::new());
        
        // Create default tenant
        let default_tenant = Tenant {
            id: default_tenant_id,
            name: "Default Tenant".to_string(),
            email: "admin@localhost".to_string(),
            quota: ResourceQuota::default(),
            sites: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: TenantStatus::Active,
        };
        
        tenants.insert(default_tenant_id, default_tenant);
        
        Self {
            tenants,
            default_tenant_id,
        }
    }

    /// Create a new tenant
    pub fn create_tenant(&self, name: String, email: String, quota: ResourceQuota) -> Result<Uuid> {
        let tenant_id = Uuid::new_v4();
        
        let tenant = Tenant {
            id: tenant_id,
            name: name.clone(),
            email,
            quota,
            sites: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: TenantStatus::Active,
        };
        
        self.tenants.insert(tenant_id, tenant);
        
        info!(tenant_id = %tenant_id, name = %name, "Tenant created");
        
        // Create tenant directory
        self.create_tenant_directory(tenant_id)?;
        
        Ok(tenant_id)
    }

    /// Get tenant by ID
    pub fn get_tenant(&self, tenant_id: Uuid) -> Option<Tenant> {
        self.tenants.get(&tenant_id).map(|t| t.clone())
    }

    /// Get default tenant ID
    pub fn default_tenant_id(&self) -> Uuid {
        self.default_tenant_id
    }

    /// Add site to tenant
    pub fn add_site(&self, tenant_id: Uuid, site_name: String, domain: Option<String>) -> Result<String> {
        let mut tenant_entry = self.tenants.get_mut(&tenant_id)
            .context("Tenant not found")?;
        
        let tenant = tenant_entry.value_mut();
        
        // Check quota
        if tenant.sites.len() >= tenant.quota.max_sites {
            anyhow::bail!("Site quota exceeded (max: {})", tenant.quota.max_sites);
        }
        
        let site_id = format!("site-{}", Uuid::new_v4());
        let site = Site {
            id: site_id.clone(),
            name: site_name.clone(),
            domain,
            cage_count: 0,
            storage_used_mb: 0,
            created_at: Utc::now(),
        };
        
        tenant.sites.push(site);
        tenant.updated_at = Utc::now();
        
        info!(tenant_id = %tenant_id, site_id = %site_id, "Site added to tenant");
        
        // Create site directory
        self.create_site_directory(tenant_id, &site_id)?;
        
        Ok(site_id)
    }

    /// Remove site from tenant
    pub fn remove_site(&self, tenant_id: Uuid, site_id: &str) -> Result<()> {
        let mut tenant_entry = self.tenants.get_mut(&tenant_id)
            .context("Tenant not found")?;
        
        let tenant = tenant_entry.value_mut();
        tenant.sites.retain(|s| s.id != site_id);
        tenant.updated_at = Utc::now();
        
        info!(tenant_id = %tenant_id, site_id = %site_id, "Site removed from tenant");
        
        Ok(())
    }

    /// Update tenant quota
    pub fn update_quota(&self, tenant_id: Uuid, quota: ResourceQuota) -> Result<()> {
        let mut tenant_entry = self.tenants.get_mut(&tenant_id)
            .context("Tenant not found")?;
        
        let tenant = tenant_entry.value_mut();
        tenant.quota = quota;
        tenant.updated_at = Utc::now();
        
        info!(tenant_id = %tenant_id, "Quota updated");
        
        Ok(())
    }

    /// Suspend tenant
    pub fn suspend_tenant(&self, tenant_id: Uuid) -> Result<()> {
        let mut tenant_entry = self.tenants.get_mut(&tenant_id)
            .context("Tenant not found")?;
        
        let tenant = tenant_entry.value_mut();
        tenant.status = TenantStatus::Suspended;
        tenant.updated_at = Utc::now();
        
        warn!(tenant_id = %tenant_id, "Tenant suspended");
        
        Ok(())
    }

    /// Activate tenant
    pub fn activate_tenant(&self, tenant_id: Uuid) -> Result<()> {
        let mut tenant_entry = self.tenants.get_mut(&tenant_id)
            .context("Tenant not found")?;
        
        let tenant = tenant_entry.value_mut();
        tenant.status = TenantStatus::Active;
        tenant.updated_at = Utc::now();
        
        info!(tenant_id = %tenant_id, "Tenant activated");
        
        Ok(())
    }

    /// List all tenants (Root Admin only)
    pub fn list_tenants(&self) -> Vec<Tenant> {
        self.tenants.iter().map(|e| e.value().clone()).collect()
    }

    /// Get tenant directory path
    pub fn tenant_directory(&self, tenant_id: Uuid) -> String {
        format!("/srv/tenants/{}", tenant_id)
    }

    /// Get site directory path
    pub fn site_directory(&self, tenant_id: Uuid, site_id: &str) -> String {
        format!("/srv/tenants/{}/sites/{}", tenant_id, site_id)
    }

    /// Create tenant directory
    fn create_tenant_directory(&self, tenant_id: Uuid) -> Result<()> {
        let path = self.tenant_directory(tenant_id);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create tenant directory: {}", path))?;
        Ok(())
    }

    /// Create site directory
    fn create_site_directory(&self, tenant_id: Uuid, site_id: &str) -> Result<()> {
        let path = self.site_directory(tenant_id, site_id);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create site directory: {}", path))?;
        Ok(())
    }

    /// Check if tenant can create more sites
    pub fn can_create_site(&self, tenant_id: Uuid) -> bool {
        if let Some(tenant) = self.tenants.get(&tenant_id) {
            tenant.sites.len() < tenant.quota.max_sites
        } else {
            false
        }
    }

    /// Get usage statistics for tenant
    pub fn get_usage(&self, tenant_id: Uuid) -> Option<TenantUsage> {
        self.tenants.get(&tenant_id).map(|tenant| {
            let total_storage_mb: usize = tenant.sites.iter()
                .map(|s| s.storage_used_mb)
                .sum();
            
            let total_cages: usize = tenant.sites.iter()
                .map(|s| s.cage_count)
                .sum();

            TenantUsage {
                sites_used: tenant.sites.len(),
                sites_limit: tenant.quota.max_sites,
                storage_used_mb: total_storage_mb,
                storage_limit_mb: tenant.quota.max_storage_gb * 1024,
                cages_running: total_cages,
            }
        })
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tenant usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsage {
    pub sites_used: usize,
    pub sites_limit: usize,
    pub storage_used_mb: usize,
    pub storage_limit_mb: usize,
    pub cages_running: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_manager_creation() {
        let manager = TenantManager::new();
        assert_eq!(manager.tenants.len(), 1); // Default tenant
    }

    #[test]
    fn test_create_tenant() {
        let manager = TenantManager::new();
        
        let tenant_id = manager.create_tenant(
            "Test Corp".to_string(),
            "test@example.com".to_string(),
            ResourceQuota::default(),
        ).unwrap();
        
        let tenant = manager.get_tenant(tenant_id).unwrap();
        assert_eq!(tenant.name, "Test Corp");
        assert_eq!(tenant.status, TenantStatus::Active);
    }

    #[test]
    fn test_site_quota() {
        let manager = TenantManager::new();
        
        let quota = ResourceQuota {
            max_sites: 2,
            ..Default::default()
        };
        
        let tenant_id = manager.create_tenant(
            "Limited Corp".to_string(),
            "limited@example.com".to_string(),
            quota,
        ).unwrap();
        
        // Should succeed
        manager.add_site(tenant_id, "Site 1".to_string(), None).unwrap();
        manager.add_site(tenant_id, "Site 2".to_string(), None).unwrap();
        
        // Should fail (quota exceeded)
        let result = manager.add_site(tenant_id, "Site 3".to_string(), None);
        assert!(result.is_err());
    }
}
