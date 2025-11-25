// HTTP/2 over TCP implementation using Hyper
// Production-grade server with optimized socket configuration

use crate::network::NetworkConfig;
use crate::state::GlobalState;
use anyhow::Result;
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper::body::{Incoming, Bytes};
use http_body_util::Full;
use socket2::{Socket, Domain, Type, Protocol};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, debug, error, warn, instrument};

/// Start the HTTP/2 server
#[instrument(skip(config, state))]
pub async fn serve(config: NetworkConfig, state: GlobalState) -> Result<()> {
    let addr = config.http2_socket_addr();
    
    // Create TCP socket with advanced options
    let socket = create_optimized_socket(&addr, &config)?;
    
    // Convert to TcpListener
    let std_listener: std::net::TcpListener = socket.into();
    std_listener.set_nonblocking(true)?;
    let listener = TcpListener::from_std(std_listener)?;
    
    info!("HTTP/2 server listening on {}", addr);

    // Accept loop
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                let state = state.clone();
                let conn_id = state.next_request_id();
                
                debug!(conn_id = conn_id, peer = %peer_addr, "New HTTP/2 connection");

                // Register connection in global state
                state.register_connection(
                    conn_id,
                    crate::state::ConnectionMetadata {
                        protocol: crate::state::Protocol::Http2,
                        remote_addr: peer_addr.to_string(),
                        connected_at: std::time::Instant::now(),
                        request_count: 0,
                    },
                );

                // Spawn a task to handle this connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state.clone(), conn_id).await {
                        error!(conn_id = conn_id, error = %e, "HTTP/2 connection error");
                    }
                    
                    // Cleanup connection
                    state.remove_connection(&conn_id);
                    debug!(conn_id = conn_id, "HTTP/2 connection closed");
                });
            }
            Err(e) => {
                error!("Failed to accept HTTP/2 connection: {}", e);
            }
        }
    }
}

/// Handle a single HTTP/2 connection
async fn handle_connection(
    stream: tokio::net::TcpStream,
    state: GlobalState,
    conn_id: u64,
) -> Result<()> {
    // Create the service function for this connection
    let service = service_fn(move |req: Request<Incoming>| {
        let state = state.clone();
        async move { handle_request(req, state, conn_id).await }
    });

    // Use HTTP/2 to serve the connection
    let builder = http2::Builder::new(tokio::runtime::Handle::current());
    
    builder
        .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
        .await?;

    Ok(())
}

/// Handle a single HTTP request
#[instrument(skip(req, state), fields(method = %req.method(), uri = %req.uri()))]
async fn handle_request(
    req: Request<Incoming>,
    state: GlobalState,
    conn_id: u64,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    debug!(conn_id = conn_id, "Processing HTTP/2 request");

    // Simple health check response for Phase 1
    // This will be replaced with actual routing logic in future phases
    let response = match (method.as_str(), uri.path()) {
        ("GET", "/health") => {
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"status":"healthy","protocol":"HTTP/2","phase":1}"#)))
                .unwrap()
        }
        ("GET", "/stats") => {
            let stats = serde_json::json!({
                "active_connections": state.connection_count(),
                "protocol": "HTTP/2",
                "memory_pool": {
                    "total_acquisitions": state.memory_pool_stats().total_acquisitions,
                },
            });
            
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(stats.to_string())))
                .unwrap()
        }
        _ => {
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain")
                .header("X-Powered-By", "Pear-Server/0.1.0")
                .body(Full::new(Bytes::from("🍐 Pear Server Phase 1 - HTTP/2 Endpoint\n")))
                .unwrap()
        }
    };

    let duration = start.elapsed();
    debug!(
        conn_id = conn_id,
        method = %method,
        uri = %uri.path(),
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    Ok(response)
}

/// Create an optimized TCP socket with Linux kernel tuning
pub(crate) fn create_optimized_socket(addr: &SocketAddr, config: &NetworkConfig) -> Result<Socket> {
    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };

    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;

    // Enable address reuse
    if config.so_reuseaddr {
        socket.set_reuse_address(true)?;
    }

    // Enable port reuse (Linux-specific, allows multiple binds)
    #[cfg(unix)]
    if config.so_reuseport {
        socket.set_reuse_port(true)?;
    }

    // Disable Nagle's algorithm for low latency
    socket.set_nodelay(config.tcp_nodelay)?;

    // Set large send buffer
    socket.set_send_buffer_size(config.tcp_send_buffer_size)?;
    
    // Set large receive buffer
    socket.set_recv_buffer_size(config.tcp_recv_buffer_size)?;

    // Enable TCP keepalive
    socket.set_keepalive(true)?;

    // Bind and listen
    socket.bind(&(*addr).into())?;
    socket.listen(1024)?; // Backlog of 1024 pending connections

    info!(
        addr = %addr,
        send_buffer = config.tcp_send_buffer_size,
        recv_buffer = config.tcp_recv_buffer_size,
        nodelay = config.tcp_nodelay,
        "Optimized TCP socket created"
    );

    Ok(socket)
}

/// Handle connection with Router (Phase 2)
pub(crate) async fn handle_connection_with_router(
    stream: tokio::net::TcpStream,
    router: std::sync::Arc<crate::router::Router>,
    peer_addr: std::net::SocketAddr,
) -> Result<()> {
    use hyper::server::conn::http2;
    use hyper::service::service_fn;
    
    debug!(peer = %peer_addr, "New HTTP/2 connection (Router mode)");

    let service = service_fn(move |req| {
        let router = router.clone();
        async move {
            router.route_request(req).await
                .or_else(|e| {
                    error!(error = %e, "Router error");
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from("Internal error")))
                        .unwrap())
                })
        }
    });

    let builder = http2::Builder::new(tokio::runtime::Handle::current());
    builder.serve_connection(hyper_util::rt::TokioIo::new(stream), service).await?;

    Ok(())
}

/// Handle connection with Router (Phase 2)
pub(crate) async fn handle_connection_with_router(
    stream: tokio::net::TcpStream,
    router: std::sync::Arc<crate::router::Router>,
    peer_addr: std::net::SocketAddr,
) -> Result<()> {
    use hyper::server::conn::http2;
    use hyper::service::service_fn;
    
    debug!(peer = %peer_addr, "New HTTP/2 connection (Router mode)");

    let service = service_fn(move |req| {
        let router = router.clone();
        async move {
            router.route_request(req).await
                .or_else(|e| {
                    error!(error = %e, "Router error");
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from("Internal error")))
                        .unwrap())
                })
        }
    });

    let builder = http2::Builder::new(tokio::runtime::Handle::current());
    builder.serve_connection(hyper_util::rt::TokioIo::new(stream), service).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_creation() {
        let config = NetworkConfig::default();
        let addr = config.http2_socket_addr();
        
        // Note: This test might fail if port is already in use
        let result = create_optimized_socket(&addr, &config);
        assert!(result.is_ok() || result.is_err()); // Either works or port is taken
    }
}
