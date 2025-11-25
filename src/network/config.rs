// Network configuration
// Defines ports, buffer sizes, and protocol-specific settings

use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Port for HTTP/2 over TCP (default: 8080 for dev, 80 for production)
    pub http2_port: u16,
    
    /// Port for HTTP/3 over QUIC (default: 8443 for dev, 443 for production)
    pub http3_port: u16,
    
    /// Bind address (typically 0.0.0.0 to listen on all interfaces)
    pub bind_addr: String,
    
    /// TCP socket send buffer size (2MB for high throughput)
    pub tcp_send_buffer_size: usize,
    
    /// TCP socket receive buffer size (2MB for high throughput)
    pub tcp_recv_buffer_size: usize,
    
    /// Maximum concurrent HTTP/2 streams per connection
    pub http2_max_concurrent_streams: u32,
    
    /// Maximum concurrent HTTP/3 streams per connection
    pub http3_max_concurrent_streams: u64,
    
    /// QUIC idle timeout in milliseconds
    pub quic_idle_timeout_ms: u64,
    
    /// Enable TCP_NODELAY (disable Nagle's algorithm)
    pub tcp_nodelay: bool,
    
    /// Enable SO_REUSEADDR
    pub so_reuseaddr: bool,
    
    /// Enable SO_REUSEPORT
    pub so_reuseport: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            // Use non-privileged ports for development
            // In production, set to 80/443 and ensure proper permissions
            http2_port: 8080,
            http3_port: 8443,
            bind_addr: "0.0.0.0".to_string(),
            
            // 2MB buffers for high throughput
            tcp_send_buffer_size: 2 * 1024 * 1024,
            tcp_recv_buffer_size: 2 * 1024 * 1024,
            
            // High concurrency settings
            http2_max_concurrent_streams: 1000,
            http3_max_concurrent_streams: 1000,
            
            // 30 second idle timeout for QUIC
            quic_idle_timeout_ms: 30_000,
            
            // Performance tuning
            tcp_nodelay: true,
            so_reuseaddr: true,
            so_reuseport: true,
        }
    }
}

impl NetworkConfig {
    /// Get the HTTP/2 socket address
    pub fn http2_socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.bind_addr, self.http2_port)
            .parse()
            .expect("Invalid HTTP/2 socket address")
    }

    /// Get the HTTP/3 socket address
    pub fn http3_socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.bind_addr, self.http3_port)
            .parse()
            .expect("Invalid HTTP/3 socket address")
    }

    /// Create production configuration with privileged ports
    pub fn production() -> Self {
        Self {
            http2_port: 80,
            http3_port: 443,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NetworkConfig::default();
        assert_eq!(config.http2_port, 8080);
        assert_eq!(config.http3_port, 8443);
    }

    #[test]
    fn test_production_config() {
        let config = NetworkConfig::production();
        assert_eq!(config.http2_port, 80);
        assert_eq!(config.http3_port, 443);
    }

    #[test]
    fn test_socket_addresses() {
        let config = NetworkConfig::default();
        let http2_addr = config.http2_socket_addr();
        assert_eq!(http2_addr.port(), 8080);
        
        let http3_addr = config.http3_socket_addr();
        assert_eq!(http3_addr.port(), 8443);
    }
}
