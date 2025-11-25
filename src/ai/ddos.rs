// DDoS Detection Module
// Implements leaky bucket rate limiting and pattern recognition

use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use tracing::{warn, info, debug};

/// DDoS detector using leaky bucket algorithm
pub struct DDoSDetector {
    /// Leaky buckets per IP address
    buckets: Arc<DashMap<IpAddr, LeakyBucket>>,
    
    /// Requests per second threshold
    threshold: usize,
    
    /// Bucket capacity
    capacity: usize,
    
    /// Leak rate (requests per second)
    leak_rate: f64,
    
    /// Banned IPs
    banned_ips: Arc<DashMap<IpAddr, BanInfo>>,
    
    /// Ban duration
    ban_duration: Duration,
}

/// Leaky bucket for rate limiting
struct LeakyBucket {
    /// Current token count
    tokens: f64,
    
    /// Last update time
    last_update: Instant,
    
    /// Request count
    request_count: u64,
}

/// Ban information
struct BanInfo {
    banned_at: Instant,
    reason: String,
    request_count: u64,
}

impl DDoSDetector {
    /// Create a new DDoS detector
    pub fn new(threshold: usize, capacity: usize, ban_duration_secs: u64) -> Self {
        info!(
            threshold = threshold,
            capacity = capacity,
            "Initializing DDoS detector"
        );
        
        Self {
            buckets: Arc::new(DashMap::new()),
            threshold,
            capacity,
            leak_rate: threshold as f64,
            banned_ips: Arc::new(DashMap::new()),
            ban_duration: Duration::from_secs(ban_duration_secs),
        }
    }

    /// Check if request should be allowed
    pub fn check_request(&self, ip: IpAddr) -> RequestDecision {
        // Check if IP is banned
        if let Some(ban_info) = self.banned_ips.get(&ip) {
            if ban_info.banned_at.elapsed() < self.ban_duration {
                debug!(ip = %ip, "Request blocked - IP banned");
                return RequestDecision::Banned {
                    reason: ban_info.reason.clone(),
                    until: ban_info.banned_at + self.ban_duration,
                };
            } else {
                // Ban expired, remove
                self.banned_ips.remove(&ip);
            }
        }

        // Get or create bucket
        let mut entry = self.buckets.entry(ip).or_insert_with(|| LeakyBucket {
            tokens: self.capacity as f64,
            last_update: Instant::now(),
            request_count: 0,
        });

        let bucket = entry.value_mut();
        
        // Leak tokens based on time elapsed
        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last_update).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.leak_rate).min(self.capacity as f64);
        bucket.last_update = now;

        // Try to consume a token
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            bucket.request_count += 1;
            RequestDecision::Allow
        } else {
            // Bucket empty, potential DDoS
            bucket.request_count += 1;
            
            warn!(
                ip = %ip,
                request_count = bucket.request_count,
                "Rate limit exceeded - potential DDoS"
            );

            // Ban if threshold exceeded
            if bucket.request_count > (self.threshold * 10) as u64 {
                self.ban_ip(ip, "DDoS pattern detected".to_string(), bucket.request_count);
                RequestDecision::Banned {
                    reason: "DDoS pattern detected".to_string(),
                    until: Instant::now() + self.ban_duration,
                }
            } else {
                RequestDecision::RateLimited {
                    retry_after: Duration::from_secs(1),
                }
            }
        }
    }

    /// Ban an IP address
    fn ban_ip(&self, ip: IpAddr, reason: String, request_count: u64) {
        warn!(ip = %ip, reason = %reason, "Banning IP address");
        
        self.banned_ips.insert(ip, BanInfo {
            banned_at: Instant::now(),
            reason,
            request_count,
        });
    }

    /// Manually ban an IP
    pub fn manual_ban(&self, ip: IpAddr, reason: String) {
        self.ban_ip(ip, reason, 0);
    }

    /// Unban an IP
    pub fn unban(&self, ip: IpAddr) -> bool {
        self.banned_ips.remove(&ip).is_some()
    }

    /// Get statistics
    pub fn stats(&self) -> DDoSStats {
        DDoSStats {
            active_buckets: self.buckets.len(),
            banned_ips: self.banned_ips.len(),
            total_bans: self.banned_ips.iter().map(|e| e.request_count).sum(),
        }
    }

    /// Clean up old buckets
    pub async fn cleanup(&self) {
        let old_threshold = Instant::now() - Duration::from_secs(300); // 5 minutes
        
        self.buckets.retain(|_, bucket| {
            bucket.last_update > old_threshold
        });
        
        // Remove expired bans
        self.banned_ips.retain(|_, ban| {
            ban.banned_at.elapsed() < self.ban_duration
        });
    }
}

/// Request decision
#[derive(Debug, Clone)]
pub enum RequestDecision {
    Allow,
    RateLimited {
        retry_after: Duration,
    },
    Banned {
        reason: String,
        until: Instant,
    },
}

impl RequestDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RequestDecision::Allow)
    }
}

/// DDoS statistics
#[derive(Debug, Clone)]
pub struct DDoSStats {
    pub active_buckets: usize,
    pub banned_ips: usize,
    pub total_bans: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_ddos_detector_creation() {
        let detector = DDoSDetector::new(100, 200, 3600);
        assert_eq!(detector.threshold, 100);
    }

    #[test]
    fn test_rate_limiting() {
        let detector = DDoSDetector::new(10, 20, 3600);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // First few requests should be allowed
        for _ in 0..10 {
            let decision = detector.check_request(ip);
            assert!(decision.is_allowed());
        }
    }

    #[test]
    fn test_manual_ban() {
        let detector = DDoSDetector::new(100, 200, 3600);
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        detector.manual_ban(ip, "Test ban".to_string());
        
        let decision = detector.check_request(ip);
        assert!(!decision.is_allowed());
    }
}
