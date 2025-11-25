// WebSocket handler for real-time telemetry streaming
// Streams Cage status, Router stats, AI threats to dashboard clients

use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use futures::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, error};

use super::DashboardState;

/// WebSocket upgrade handler
pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<DashboardState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<DashboardState>) {
    let (mut sender, mut receiver) = socket.split();
    
    info!("New dashboard WebSocket connection");

    // Spawn telemetry streaming task
    let state_clone = state.clone();
    let mut send_task = tokio::spawn(async move {
        let mut tick_interval = interval(Duration::from_secs(1));
        
        loop {
            tick_interval.tick().await;
            
            // Collect telemetry
            let telemetry = collect_telemetry(&state_clone).await;
            
            // Send as JSON
            let json = serde_json::to_string(&telemetry)
                .unwrap_or_else(|_| "{}".to_string());
            
            if sender.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages (for interactive controls)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let axum::extract::ws::Message::Text(text) = msg {
                info!("Received dashboard command: {}", text);
                // Handle commands like pause/resume, config changes, etc.
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("Dashboard WebSocket connection closed");
}

/// Collect telemetry from all components
async fn collect_telemetry(state: &DashboardState) -> Telemetry {
    let router_stats = state.router.stats();
    let supervisor_stats = state.supervisor.stats();
    let ai_stats = state.ai_module.stats();

    Telemetry {
        timestamp: chrono::Utc::now().timestamp(),
        router: RouterTelemetry {
            total_requests: router_stats.total_requests,
            successful_requests: router_stats.successful_requests,
            failed_requests: router_stats.failed_requests,
            active_pools: router_stats.active_pools,
            success_rate: router_stats.success_rate(),
        },
        supervisor: SupervisorTelemetry {
            supervised_pools: supervisor_stats.supervised_pools,
            healing_events: supervisor_stats.healing_events,
            is_running: supervisor_stats.is_running,
        },
        ai: AiTelemetry {
            threats_detected: ai_stats.threats_detected,
            anomaly_detection_enabled: ai_stats.anomaly_detection_enabled,
        },
        // Mock Cage Pool data for demonstration
        cages: vec![
            CageTelemetry {
                id: 1,
                site: "default-site".to_string(),
                status: "running".to_string(),
                requests: 5234,
                memory_mb: 87,
                cpu_percent: 23.5,
                uptime_secs: 3627,
            },
            CageTelemetry {
                id: 2,
                site: "default-site".to_string(),
                status: "running".to_string(),
                requests: 5189,
                memory_mb: 91,
                cpu_percent: 21.8,
                uptime_secs: 3627,
            },
            CageTelemetry {
                id: 3,
                site: "default-site".to_string(),
                status: "running".to_string(),
                requests: 5301,
                memory_mb: 89,
                cpu_percent: 24.2,
                uptime_secs: 3627,
            },
        ],
    }
}

/// Complete telemetry snapshot
#[derive(Debug, serde::Serialize)]
struct Telemetry {
    timestamp: i64,
    router: RouterTelemetry,
    supervisor: SupervisorTelemetry,
    ai: AiTelemetry,
    cages: Vec<CageTelemetry>,
}

#[derive(Debug, serde::Serialize)]
struct RouterTelemetry {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    active_pools: usize,
    success_rate: f64,
}

#[derive(Debug, serde::Serialize)]
struct SupervisorTelemetry {
    supervised_pools: usize,
    healing_events: u64,
    is_running: bool,
}

#[derive(Debug, serde::Serialize)]
struct AiTelemetry {
    threats_detected: u64,
    anomaly_detection_enabled: bool,
}

#[derive(Debug, serde::Serialize)]
struct CageTelemetry {
    id: u64,
    site: String,
    status: String,
    requests: u64,
    memory_mb: u64,
    cpu_percent: f64,
    uptime_secs: u64,
}
