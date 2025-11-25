// HTTP/2 server with Router integration
// Extended version that routes requests through the Router

use crate::router::Router;
use super::{NetworkConfig, http2};
use anyhow::Result;
use std::sync::Arc;
use tracing::{info};

/// Start HTTP/2 server with Router integration
pub async fn serve_with_router(
    config: NetworkConfig,
    router: Arc<Router>,
) -> Result<()> {
    info!("Starting HTTP/2 server with Router integration");
    
    let addr = config.http2_socket_addr();
    
    // Create optimized socket
    let socket = http2::create_optimized_socket(&addr, &config)?;
    
    // Convert to TcpListener
    let std_listener: std::net::TcpListener = socket.into();
    std_listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(std_listener)?;
    
    info!("HTTP/2 server (Router mode) listening on {}", addr);

    // Accept loop
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                let router = router.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = http2::handle_connection_with_router(stream, router,peer_addr).await {
                        tracing::error!("HTTP/2 (Router) connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                tracing::error!("Failed to accept HTTP/2 connection: {}", e);
            }
        }
    }
}
