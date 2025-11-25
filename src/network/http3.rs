// HTTP/3 over QUIC implementation using Quinn
// Modern UDP-based protocol with TLS 1.3

use crate::network::NetworkConfig;
use crate::state::GlobalState;
use anyhow::Result;
use quinn::{Endpoint, ServerConfig, Connection};
use std::sync::Arc;
use tracing::{info, debug, error, warn, instrument};

/// Start the HTTP/3 server
#[instrument(skip(config, state))]
pub async fn serve(config: NetworkConfig, state: GlobalState) -> Result<()> {
    let addr = config.http3_socket_addr();

    // Create server configuration with TLS
    let server_config = create_server_config(&config)?;
    
    // Create QUIC endpoint
    let endpoint = Endpoint::server(server_config, addr)?;
    
    info!("HTTP/3 server listening on {}", addr);

    // Accept loop for incoming QUIC connections
    while let Some(connecting) = endpoint.accept().await {
        let state = state.clone();
        let conn_id = state.next_request_id();
        
        tokio::spawn(async move {
            match connecting.await {
                Ok(connection) => {
                    let peer_addr = connection.remote_address();
                    debug!(conn_id = conn_id, peer = %peer_addr, "New HTTP/3 connection");
                    
                    // Register connection
                    state.register_connection(
                        conn_id,
                        crate::state::ConnectionMetadata {
                            protocol: crate::state::Protocol::Http3,
                            remote_addr: peer_addr.to_string(),
                            connected_at: std::time::Instant::now(),
                            request_count: 0,
                        },
                    );

                    if let Err(e) = handle_connection(connection, state.clone(), conn_id).await {
                        error!(conn_id = conn_id, error = %e, "HTTP/3 connection error");
                    }
                    
                    // Cleanup
                    state.remove_connection(&conn_id);
                    debug!(conn_id = conn_id, "HTTP/3 connection closed");
                }
                Err(e) => {
                    error!("Failed to establish HTTP/3 connection: {}", e);
                }
            }
        });
    }

    info!("HTTP/3 server shutting down");
    Ok(())
}

/// Handle a single HTTP/3 QUIC connection
async fn handle_connection(
    connection: Connection,
    state: GlobalState,
    conn_id: u64,
) -> Result<()> {
    // Accept bidirectional streams (HTTP/3 requests)
    loop {
        match connection.accept_bi().await {
            Ok((mut send, mut recv)) => {
                let state = state.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(&mut send, &mut recv, state, conn_id).await {
                        error!(conn_id = conn_id, "Stream error: {}", e);
                    }
                });
            }
            Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                debug!(conn_id = conn_id, "Connection closed by application");
                break;
            }
            Err(e) => {
                error!(conn_id = conn_id, "Failed to accept stream: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle a single HTTP/3 stream (request/response)
async fn handle_stream(
    send: &mut quinn::SendStream,
    recv: &mut quinn::RecvStream,
    state: GlobalState,
    conn_id: u64,
) -> Result<()> {
    use quinn::VarInt;
    
    let start = std::time::Instant::now();
    
    // Read request data (simplified for Phase 1)
    let request_data = match recv.read_to_end(1024 * 1024).await {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to read request: {}", e);
            return Ok(());
        }
    };
    
    debug!(
        conn_id = conn_id,
        request_size = request_data.len(),
        "Received HTTP/3 request"
    );

    // Simple response for Phase 1
    // In future phases, this will parse HTTP/3 headers and route to handlers
    let response = build_http3_response(&state);
    
    // Send response
    send.write_all(&response).await?;
    send.finish().await?;

    let duration = start.elapsed();
    debug!(
        conn_id = conn_id,
        duration_ms = duration.as_millis(),
        "HTTP/3 request completed"
    );

    Ok(())
}

/// Build a simple HTTP/3 response
/// This is a placeholder - proper HTTP/3 implementation will use h3 crate
fn build_http3_response(state: &GlobalState) -> Vec<u8> {
    let stats = serde_json::json!({
        "status": "healthy",
        "protocol": "HTTP/3",
        "phase": 1,
        "active_connections": state.connection_count(),
    });

    // Simplified response (not proper HTTP/3 framing)
    // In production, use the h3 crate for proper QPACK/HTTP3 encoding
    format!(
        "🍐 Pear Server Phase 1 - HTTP/3 Endpoint\n\nStats: {}\n",
        stats
    )
    .into_bytes()
}

/// Create Quinn server configuration with TLS
fn create_server_config(config: &NetworkConfig) -> Result<ServerConfig> {
    // Generate self-signed certificate for development
    // In production, use proper certificates from Let's Encrypt or similar
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert_der = cert.serialize_der()?;
    let priv_key = cert.serialize_private_key_der();
    
    let cert_chain = vec![rustls::Certificate(cert_der)];
    let key_der = rustls::PrivateKey(priv_key);

    // Create rustls config
    let mut crypto = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)?;

    crypto.alpn_protocols = vec![b"h3".to_vec()]; // HTTP/3 ALPN
    
    // Create Quinn server config
    let mut server_config = ServerConfig::with_crypto(Arc::new(crypto));
    
    // Configure transport parameters
    let mut transport = quinn::TransportConfig::default();
    transport.max_concurrent_bidi_streams(quinn::VarInt::from_u32(config.http3_max_concurrent_streams as u32));
    transport.max_concurrent_uni_streams(quinn::VarInt::from_u32(100)); 
    transport.max_idle_timeout(Some(
        std::time::Duration::from_millis(config.quic_idle_timeout_ms)
            .try_into()
            .unwrap()
    ));

    // Large datagram size for better throughput
    transport.max_udp_payload_size(quinn::VarInt::from_u32(1500));
    
    server_config.transport = Arc::new(transport);

    info!(
        max_concurrent_streams = config.http3_max_concurrent_streams,
        idle_timeout_ms = config.quic_idle_timeout_ms,
        "HTTP/3 server configuration created"
    );

    Ok(server_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_creation() {
        let config = NetworkConfig::default();
        let server_config = create_server_config(&config);
        assert!(server_config.is_ok());
    }

    #[test]
    fn test_response_builder() {
        let state = GlobalState::new();
        let response = build_http3_response(&state);
        assert!(!response.is_empty());
        assert!(String::from_utf8_lossy(&response).contains("Pear Server"));
    }
}
