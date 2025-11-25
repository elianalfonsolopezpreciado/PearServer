// Network module - Dual-protocol networking stack
// Supports both HTTP/2 over TCP and HTTP/3 over QUIC

pub mod config;
pub mod http2;
pub mod http3;
pub mod router_integration;

pub use config::NetworkConfig;

// Re-export router-integrated serve functions
pub use router_integration::{serve_with_router as http2_serve_with_router};
