// Administration Dashboard Module
// Real-time monitoring and management interface

pub mod websocket;
pub mod telemetry;

use axum::{
    Router,
    routing::get,
    response::Html,
};
use tower_http::services::ServeDir;
use std::sync::Arc;
use tracing::{info, error};

/// Dashboard server state
pub struct DashboardState {
    /// Reference to router for stats
    pub router: Arc<crate::router::Router>,
    
    /// Reference to supervisor for healing stats
    pub supervisor: Arc<crate::supervisor::Supervisor>,
    
    /// Reference to AI module for threat stats
    pub ai_module: Arc<crate::ai::AiSecurityModule>,
}

/// Start the dashboard server
pub async fn serve(
    port: u16,
    router: Arc<crate::router::Router>,
    supervisor: Arc<crate::supervisor::Supervisor>,
    ai_module: Arc<crate::ai::AiSecurityModule>,
) -> anyhow::Result<()> {
    info!(port = port, "Starting administration dashboard");

    let state = Arc::new(DashboardState {
        router,
        supervisor,
        ai_module,
    });

    // Build our application with routes
    let app = Router::new()
        .route("/", get(dashboard_index))
        .route("/ws", get(websocket::handler))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    info!("Dashboard server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Dashboard HTML page
async fn dashboard_index() -> Html<&'static str> {
    Html(include_str!("../../static/dashboard.html"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_state_creation() {
        // Test that dashboard state can be created
        // Full test would require mock router, supervisor, AI module
    }
}
