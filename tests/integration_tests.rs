// Integration Tests for Pear Server Phase 4
// Tests multi-tenancy, canary deployments, and security features

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_creation_and_isolation() {
        // Test tenant creation
        let tenant_manager = crate::tenancy::TenantManager::new();
        
        let tenant_id = tenant_manager.create_tenant(
            "Test Corp".to_string(),
            "test@example.com".to_string(),
            crate::tenancy::ResourceQuota::default(),
        ).expect("Failed to create tenant");
        
        assert!(tenant_manager.get_tenant(tenant_id).is_some());
        
        // Test site creation
        let site_id = tenant_manager.add_site(
            tenant_id,
            "Test Site".to_string(),
            Some("test.example.com".to_string()),
        ).expect("Failed to create site");
        
        assert!(!site_id.is_empty());
    }

    #[tokio::test]
    async fn test_tenant_quota_enforcement() {
        let tenant_manager = crate::tenancy::TenantManager::new();
        
        let quota = crate::tenancy::ResourceQuota {
            max_sites: 2,
            max_storage_gb: 1,
            max_memory_per_cage_mb: 128,
            max_cages_per_site: 3,
            max_requests_per_second: Some(100),
        };
        
        let tenant_id = tenant_manager.create_tenant(
            "Limited Corp".to_string(),
            "limited@example.com".to_string(),
            quota,
        ).expect("Failed to create tenant");
        
        // Should allow first two sites
        tenant_manager.add_site(tenant_id, "Site 1".to_string(), None).unwrap();
        tenant_manager.add_site(tenant_id, "Site 2".to_string(), None).unwrap();
        
        // Should fail on third site
        let result = tenant_manager.add_site(tenant_id, "Site 3".to_string(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_ddos_detector() {
        use crate::ai::ddos::DDoSDetector;
        use std::net::Ipv4Addr;
        
        let detector = DDoSDetector::new(10, 20, 3600);
        let ip = std::net::IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        
        // First few requests should be allowed
        for _ in 0..5 {
            let decision = detector.check_request(ip);
            assert!(decision.is_allowed());
        }
    }

    #[test]
    fn test_path_monitor() {
        use crate::ai::path_monitor::PathMonitor;
        use std::net::Ipv4Addr;
        
        let monitor = PathMonitor::new(3);
        let ip = std::net::IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        
        // Scanning for sensitive paths should be detected
        use crate::ai::path_monitor::PathDecision;
        
        let decision1 = monitor.check_path(ip, "/.env", 404);
        assert_eq!(decision1, PathDecision::Suspicious);
        
        let decision2 = monitor.check_path(ip, "/wp-admin", 404);
        assert_eq!(decision2, PathDecision::Suspicious);
        
        // Third attempt should trigger ban
        let decision3 = monitor.check_path(ip, "/.git/config", 404);
        assert_eq!(decision3, PathDecision::Banned);
    }

    #[test]
    fn test_performance_baseline() {
        use crate::ai::performance_baseline::PerformanceMonitor;
        use std::time::Duration;
        
        let mut monitor = PerformanceMonitor::new(100, 2.0);
        
        // Establish baseline with normal latencies
        for _ in 0..50 {
            monitor.record_latency(Duration::from_millis(100));
        }
        
        // Anomalous latency should be detected
        let alert = monitor.record_latency(Duration::from_millis(500));
        assert!(alert.is_anomaly());
    }

    #[test]
    fn test_canary_deployment() {
        use crate::deployment::CanaryManager;
        
        let manager = CanaryManager::new();
        
        // Create canary
        let info = manager.create_canary(
            "test-site".to_string(),
            vec![0, 1, 2, 3],
        ).expect("Failed to create canary");
        
        assert!(!info.beta_secret.is_empty());
        
        // Should route with correct secret
        assert!(manager.should_route_to_canary(
            "test-site",
            Some(&info.beta_secret),
            None
        ));
        
        // Should not route with wrong secret
        assert!(!manager.should_route_to_canary(
            "test-site",
            Some("wrong"),
            None
        ));
    }

    #[test]
    fn test_polyglot_detection() {
        use crate::runtime::polyglot::{PolyglotAdapter, DetectedLanguage};
        use tempfile::TempDir;
        
        let temp = TempDir::new().unwrap();
        let adapter = PolyglotAdapter::new("/tmp/runtimes");
        
        // Test PHP detection
        std::fs::write(temp.path().join("index.php"), "<?php echo 'test'; ?>").unwrap();
        let lang = adapter.detect_language(temp.path()).unwrap();
        assert_eq!(lang, DetectedLanguage::PHP);
    }

    #[test]
    fn test_storage_quota() {
        use crate::tenancy::quota::QuotaEnforcer;
        use crate::tenancy::ResourceQuota;
        
        let quota = ResourceQuota {
            max_sites: 5,
            max_storage_gb: 1,
            max_memory_per_cage_mb: 128,
            max_cages_per_site: 3,
            max_requests_per_second: None,
        };
        
        let enforcer = QuotaEnforcer::new(quota);
        
        // Should allow allocation within limit
        assert!(enforcer.can_allocate_storage(512).is_ok());
        
        // Should block allocation over limit
        assert!(enforcer.can_allocate_storage(2048).is_err());
    }

    #[tokio::test]
    async fn test_authentication() {
        use crate::tenancy::auth::{AuthManager, Role};
        
        let auth = AuthManager::new();
        
        // Test root admin token
        let claims = auth.validate_token("root_admin_secret_token").unwrap();
        assert_eq!(claims.role, Role::RootAdmin);
        
        // Test tenant token
        let tenant_id = uuid::Uuid::new_v4();
        let token = auth.generate_tenant_token(tenant_id);
        let claims = auth.validate_token(&token).unwrap();
        assert_eq!(claims.role, Role::TenantAdmin);
        assert_eq!(claims.tenant_id, Some(tenant_id));
    }
}
