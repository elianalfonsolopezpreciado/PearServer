// Resource Quota Enforcement
// Tracks and enforces tenant resource limits

use super::{ResourceQuota, TenantUsage};
use anyhow::{Result, bail};

/// Quota enforcer
pub struct QuotaEnforcer {
    quota: ResourceQuota,
    current_usage: TenantUsage,
}

impl QuotaEnforcer {
    /// Create a new quota enforcer
    pub fn new(quota: ResourceQuota) -> Self {
        Self {
            quota,
            current_usage: TenantUsage {
                sites_used: 0,
                sites_limit: quota.max_sites,
                storage_used_mb: 0,
                storage_limit_mb: quota.max_storage_gb * 1024,
                cages_running: 0,
            },
        }
    }

    /// Check if site creation is allowed
    pub fn can_create_site(&self) -> Result<()> {
        if self.current_usage.sites_used >= self.quota.max_sites {
            bail!(
                "Site quota exceeded: {}/{} sites used",
                self.current_usage.sites_used,
                self.quota.max_sites
            );
        }
        Ok(())
    }

    /// Check if storage allocation is allowed
    pub fn can_allocate_storage(&self, size_mb: usize) -> Result<()> {
        let new_usage = self.current_usage.storage_used_mb + size_mb;
        if new_usage > self.current_usage.storage_limit_mb {
            bail!(
                "Storage quota exceeded: would use {}/{} MB",
                new_usage,
                self.current_usage.storage_limit_mb
            );
        }
        Ok(())
    }

    /// Check if Cage creation is allowed
    pub fn can_create_cage(&self, site_cage_count: usize) -> Result<()> {
        if site_cage_count >= self.quota.max_cages_per_site {
            bail!(
                "Cage quota exceeded for site: {}/{} cages",
                site_cage_count,
                self.quota.max_cages_per_site
            );
        }
        Ok(())
    }

    /// Check memory quota for Cage
    pub fn validate_cage_memory(&self, requested_mb: usize) -> Result<()> {
        if requested_mb > self.quota.max_memory_per_cage_mb {
            bail!(
                "Memory quota exceeded: requested {} MB, limit is {} MB",
                requested_mb,
                self.quota.max_memory_per_cage_mb
            );
        }
        Ok(())
    }

    /// Update usage
    pub fn update_usage(&mut self, usage: TenantUsage) {
        self.current_usage = usage;
    }

    /// Get current usage percentage
    pub fn usage_percentage(&self) -> QuotaUsagePercentage {
        QuotaUsagePercentage {
            sites: (self.current_usage.sites_used as f64 / self.quota.max_sites as f64 * 100.0) as u8,
            storage: (self.current_usage.storage_used_mb as f64 / self.current_usage.storage_limit_mb as f64 * 100.0) as u8,
        }
    }
}

/// Quota usage percentage
#[derive(Debug, Clone)]
pub struct QuotaUsagePercentage {
    pub sites: u8,
    pub storage: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_quota_enforcement() {
        let quota = ResourceQuota {
            max_sites: 2,
            ..Default::default()
        };
        
        let mut enforcer = QuotaEnforcer::new(quota);
        
        // Should allow first two sites
        assert!(enforcer.can_create_site().is_ok());
        
        enforcer.current_usage.sites_used = 2;
        
        // Should block third site
        assert!(enforcer.can_create_site().is_err());
    }

    #[test]
    fn test_storage_quota_enforcement() {
        let quota = ResourceQuota {
            max_storage_gb: 1, // 1 GB = 1024 MB
            ..Default::default()
        };
        
        let enforcer = QuotaEnforcer::new(quota);
        
        // Should allow small allocation
        assert!(enforcer.can_allocate_storage(512).is_ok());
        
        // Should block large allocation
        assert!(enforcer.can_allocate_storage(2048).is_err());
    }

    #[test]
    fn test_memory_quota_enforcement() {
        let quota = ResourceQuota {
            max_memory_per_cage_mb: 128,
            ..Default::default()
        };
        
        let enforcer = QuotaEnforcer::new(quota);
        
        // Should allow within limit
        assert!(enforcer.validate_cage_memory(64).is_ok());
        assert!(enforcer.validate_cage_memory(128).is_ok());
        
        // Should block over limit
        assert!(enforcer.validate_cage_memory(256).is_err());
    }
}
