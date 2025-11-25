// Suspicious Path Monitor
// Detects scanning for sensitive endpoints and bans malicious IPs

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;
use dashmap::DashMap;
use tracing::{warn, info};

/// Suspicious path monitor
pub struct PathMonitor {
    /// Sensitive paths to monitor
    sensitive_paths: Vec<String>,
    
    /// Scan attempts per IP
    scan_attempts: Arc<DashMap<IpAddr, ScanTracker>>,
    
    /// Threshold before banning
    ban_threshold: usize,
    
    /// Banned IPs
    banned_ips: Arc<DashMap<IpAddr, Instant>>,
}

/// Scan tracking per IP
struct ScanTracker {
    attempts: Vec<ScanAttempt>,
    first_attempt: Instant,
}

#[derive(Clone)]
struct ScanAttempt {
    path: String,
    timestamp: Instant,
}

impl PathMonitor {
    /// Create a new path monitor
    pub fn new(ban_threshold: usize) -> Self {
        info!("Initializing Suspicious Path Monitor");
        
        Self {
            sensitive_paths: Self::default_sensitive_paths(),
            scan_attempts: Arc::new(DashMap::new()),
            ban_threshold,
            banned_ips: Arc::new(DashMap::new()),
        }
    }

    /// Default list of sensitive paths
    fn default_sensitive_paths() -> Vec<String> {
        vec![
            ".env".to_string(),
            ".git".to_string(),
            ".git/config".to_string(),
            "wp-admin".to_string(),
            "wp-login.php".to_string(),
            "admin".to_string(),
            "phpmyadmin".to_string(),
            "config.php".to_string(),
            "web.config".to_string(),
            "backup.sql".to_string(),
            "database.sql".to_string(),
            ".htaccess".to_string(),
            "composer.json".to_string(),
            "package.json".to_string(),
            "Dockerfile".to_string(),
            ".dockerignore".to_string(),
            ".ssh/id_rsa".to_string(),
            "id_rsa".to_string(),
            "authorized_keys".to_string(),
        ]
    }

    /// Check if path is suspicious
    pub fn check_path(&self, ip: IpAddr, path: &str, status_code: u16) -> PathDecision {
        // Check if IP is banned
        if self.banned_ips.contains_key(&ip) {
            return PathDecision::Banned;
        }

        // Check if path is sensitive
        let is_sensitive = self.is_sensitive_path(path);
        
        // Only track 404s on sensitive paths (scanning behavior)
        if is_sensitive && status_code == 404 {
            self.record_scan_attempt(ip, path.to_string());
            
            // Check if threshold exceeded
            if let Some(tracker) = self.scan_attempts.get(&ip) {
                if tracker.attempts.len() >= self.ban_threshold {
                    warn!(
                        ip = %ip,
                        attempts = tracker.attempts.len(),
                        "Suspicious scanning detected - banning IP"
                    );
                    
                    self.ban_ip(ip);
                    return PathDecision::Banned;
                }
            }
            
            PathDecision::Suspicious
        } else {
            PathDecision::Safe
        }
    }

    /// Check if path is sensitive
    fn is_sensitive_path(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();
        
        self.sensitive_paths.iter().any(|sensitive| {
            path_lower.contains(sensitive)
        })
    }

    /// Record a scan attempt
    fn record_scan_attempt(&self, ip: IpAddr, path: String) {
        let mut entry = self.scan_attempts.entry(ip).or_insert_with(|| ScanTracker {
            attempts: Vec::new(),
            first_attempt: Instant::now(),
        });

        let tracker = entry.value_mut();
        tracker.attempts.push(ScanAttempt {
            path,
            timestamp: Instant::now(),
        });

        // Keep only recent attempts (last hour)
        let one_hour_ago = Instant::now() - std::time::Duration::from_secs(3600);
        tracker.attempts.retain(|attempt| attempt.timestamp > one_hour_ago);
    }

    /// Ban an IP
    fn ban_ip(&self, ip: IpAddr) {
        self.banned_ips.insert(ip, Instant::now());
    }

    /// Manually ban an IP
    pub fn manual_ban(&self, ip: IpAddr) {
        warn!(ip = %ip, "Manually banning IP");
        self.ban_ip(ip);
    }

    /// Unban an IP
    pub fn unban(&self, ip: IpAddr) -> bool {
        self.banned_ips.remove(&ip).is_some()
    }

    /// Add custom sensitive path
    pub fn add_sensitive_path(&mut self, path: String) {
        self.sensitive_paths.push(path);
    }

    /// Get statistics
    pub fn stats(&self) -> PathMonitorStats {
        let total_scan_attempts: usize = self.scan_attempts.iter()
            .map(|e| e.attempts.len())
            .sum();

        PathMonitorStats {
            tracked_ips: self.scan_attempts.len(),
            banned_ips: self.banned_ips.len(),
            total_scan_attempts,
            sensitive_paths_count: self.sensitive_paths.len(),
        }
    }

    /// Get scanned paths for an IP
    pub fn get_scan_history(&self, ip: IpAddr) -> Vec<String> {
        self.scan_attempts.get(&ip)
            .map(|tracker| {
                tracker.attempts.iter()
                    .map(|attempt| attempt.path.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Path check decision
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathDecision {
    Safe,
    Suspicious,
    Banned,
}

/// Path monitor statistics
#[derive(Debug, Clone)]
pub struct PathMonitorStats {
    pub tracked_ips: usize,
    pub banned_ips: usize,
    pub total_scan_attempts: usize,
    pub sensitive_paths_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_path_monitor_creation() {
        let monitor = PathMonitor::new(5);
        assert_eq!(monitor.ban_threshold, 5);
    }

    #[test]
    fn test_sensitive_path_detection() {
        let monitor = PathMonitor::new(5);
        
        assert!(monitor.is_sensitive_path("/.env"));
        assert!(monitor.is_sensitive_path("/wp-admin/"));
        assert!(monitor.is_sensitive_path("/.git/config"));
        assert!(!monitor.is_sensitive_path("/index.html"));
    }

    #[test]
    fn test_scanning_detection() {
        let monitor = PathMonitor::new(3);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

        // First two attempts should be suspicious
        assert_eq!(monitor.check_path(ip, "/.env", 404), PathDecision::Suspicious);
        assert_eq!(monitor.check_path(ip, "/wp-admin", 404), PathDecision::Suspicious);
        
        // Third attempt should trigger ban
        assert_eq!(monitor.check_path(ip, "/.git/config", 404), PathDecision::Banned);
        
        // Subsequent requests should remain banned
        assert_eq!(monitor.check_path(ip, "/index.html", 200), PathDecision::Banned);
    }

    #[test]
    fn test_safe_path() {
        let monitor = PathMonitor::new(5);
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        assert_eq!(monitor.check_path(ip, "/index.html", 200), PathDecision::Safe);
        assert_eq!(monitor.check_path(ip, "/about", 200), PathDecision::Safe);
    }
}
