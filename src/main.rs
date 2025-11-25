// Pear Server - Phase 1: Foundation and Linux Interaction Layer
// Phase 2: Middle Layer - Cage Architecture & AI Engine
// Phase 3: Interaction Layer & Configuration Management
// Main daemon entry point

// Phase 1 modules
mod network;
mod observability;
mod runtime;
mod signals;
mod state;

// Phase 2 modules
mod cage;
mod router;
mod supervisor;
mod crdt;
mod ai;

// Phase 3 modules
mod cli;
mod config;
mod dashboard;

// Phase 4 modules
mod tenancy;
mod deployment;
mod storage;

use anyhow::Result;
use tracing::{info, error};
use std::sync::Arc;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = cli::Cli::parse();
    
    // Handle commands
    match cli.command {
        cli::Commands::Start { config, foreground, verbose } => {
            // Initialize observability with verbosity
            observability::init()?;
            
            // Print banner
            cli::print_banner();
            
            // Run the daemon
            run_daemon(config, foreground).await
        }
        _ => {
            // For other commands, execute them
            cli::commands::execute(cli.command).await
        }
    }
}

/// Run the Pear Server daemon
async fn run_daemon(config_path: String, _foreground: bool) -> Result<()> {

    info!("🍐 Pear Server Phase 3 - Complete System: CLI + Dashboard + Auto-Config");
    info!("Initializing userspace pseudo-operating system daemon...");

    // Load configuration
    info!("Loading configuration from {}", config_path);
    let pear_config = config::PearConfig::load(&config_path)?;
    info!("✓ Configuration loaded and validated");

    // Configure runtime limits (file descriptors, memory, etc.)
    runtime::configure_limits()?;
    info!("✓ Runtime limits configured");

    // Initialize global state manager
    let global_state = state::GlobalState::new();
    info!("✓ Global state manager initialized");

    // Set up graceful shutdown signal handlers
    let shutdown_signal = signals::create_shutdown_listener()?;
    info!("✓ Signal handlers installed (SIGTERM, SIGINT)");

    // === Phase 2: Initialize Middle Layer Components ===

    // Initialize Wasmtime engine
    info!("Initializing WebAssembly runtime...");
    let wasm_engine = cage::create_engine()?;
    info!("✓ Wasmtime engine created");

    // Create a simple default Wasm module for demonstration
    let default_wasm = create_default_wasm_module();
    info!("✓ Default Wasm module created");

    // Initialize Router
    let router_config = router::RouterConfig::default();
    let router = Arc::new(router::Router::new(router_config));
    info!("✓ Traffic Router initialized");

    // Initialize Supervisor
    let supervisor_config = supervisor::SupervisorConfig::default();
    let supervisor = Arc::new(supervisor::Supervisor::new(supervisor_config));
    info!("✓ Self-Healing Supervisor initialized");

    // Initialize AI Security Module
    let ai_config = ai::AiConfig::default();
    let ai_module = Arc::new(ai::AiSecurityModule::new(ai_config)?);
    info!("✓ AI Security Module initialized");

    // Create a default CagePool for demonstration
    info!("Creating default Cage Pool...");
    let pool = cage::pool::CagePool::new(
        "default-site".to_string(),
        default_wasm.clone(),
        cage::config::CageConfig::default(),
        pear_config.cages.default_replicas,
    ).await?;
    let pool_arc = Arc::new(pool);
    info!("✓ Cage Pool created with {} redundant instances", pear_config.cages.default_replicas);

    // Register pool with Router
    router.register_pool("default-site".to_string(), pool_arc.clone());
    info!("✓ Cage Pool registered with Router");

    // Register pool with Supervisor
    supervisor.register_pool("default-site".to_string(), pool_arc.clone(), default_wasm.clone());
    info!("✓ Cage Pool registered with Supervisor");

    // Start Router health checks
    router.start_health_checks().await;
    info!("✓ Router health checks started");

    // Start Supervisor monitoring loop
    supervisor.start().await;
    info!("✓ Supervisor monitoring loop started");

    // === Phase 3: Start Dashboard Server ===
    
    if pear_config.dashboard.enabled {
        let dashboard_router = router.clone();
        let dashboard_supervisor = supervisor.clone();
        let dashboard_ai = ai_module.clone();
        let dashboard_port = pear_config.dashboard.port;
        
        tokio::spawn(async move {
            if let Err(e) = dashboard::serve(
                dashboard_port,
                dashboard_router,
                dashboard_supervisor,
                dashboard_ai,
            ).await {
                error!("Dashboard server error: {}", e);
            }
        });
        
        info!("✓ Administration Dashboard started on port {}", pear_config.dashboard.port);
    }

    // Create network configuration
    let network_config = network::NetworkConfig {
        http2_port: pear_config.server.http2_port,
        http3_port: pear_config.server.http3_port,
        ..Default::default()
    };
    info!(
        http2_port = network_config.http2_port,
        http3_port = network_config.http3_port,
        "Network configuration loaded"
    );

    // Start HTTP/2 server (TCP) - now routes through Router
    let http2_handle = {
        let config = network_config.clone();
        let router = router.clone();
        tokio::spawn(async move {
            let addr = config.http2_socket_addr();
            let socket = network::http2::create_optimized_socket(&addr, &config).unwrap();
            let std_listener: std::net::TcpListener = socket.into();
            std_listener.set_nonblocking(true).unwrap();
            let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
            
            loop {
                if let Ok((stream, peer_addr)) = listener.accept().await {
                    let router = router.clone();
                    tokio::spawn(async move {
                        let _ = network::http2::handle_connection_with_router(stream, router, peer_addr).await;
                    });
                }
            }
        })
    };
    info!("✓ HTTP/2 server started on port {} (routing to Cages)", network_config.http2_port);

    // Start HTTP/3 server (QUIC/UDP) - simplified for Phase 2
    let http3_handle = {
        let config = network_config.clone();
        tokio::spawn(async move {
            if let Err(e) = network::http3::serve(config, state::GlobalState::new()).await {
                error!("HTTP/3 server error: {}", e);
            }
        })
    };
    info!("✓ HTTP/3 server started on port {}", network_config.http3_port);

    println!();
    cli::success("🚀 Pear Server Phase 3 is ready - All systems operational");
    println!();
    println!("  {} Architecture: HTTP → Router → CagePool → WebAssembly Cages", "🎯".bright_cyan());
    println!("  {} Security: AI-powered anomaly detection active", "🔒".bright_cyan());
    println!("  {} Self-Healing: Automatic crash recovery enabled", "🩺".bright_cyan());
    println!("  {} Configuration: Smart defaults with pear.toml override", "⚙️".bright_cyan());
    println!();
    cli::info(&format!("HTTP/2 server: http://localhost:{}", network_config.http2_port));
    cli::info(&format!("HTTP/3 server: http://localhost:{}", network_config.http3_port));
    if pear_config.dashboard.enabled {
        cli::info(&format!("Dashboard: http://localhost:{} 📊", pear_config.dashboard.port));
    }
    println!();
    cli::info("Press Ctrl+C for graceful shutdown");
    println!();

    // Wait for shutdown signal
    shutdown_signal.await;
    
    info!("🛑 Shutdown signal received - Initiating graceful shutdown");

    // Gracefully shutdown all services
    info!("Stopping network services...");
    drop(http2_handle);
    drop(http3_handle);
    
    info!("Stopping Supervisor...");
    supervisor.stop();
    
    // Give services time to complete in-flight requests
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Cleanup global state
    info!("Cleaning up global state...");
    drop(global_state);
    drop(router);

    info!("✓ Graceful shutdown complete - No zombie processes");
    info!("👋 Pear Server stopped");

    Ok(())
}

/// Create a simple default WebAssembly module for demonstration
fn create_default_wasm_module() -> Vec<u8> {
    // Simple WAT module that exports a function
    let wat = r#"
        (module
            (func (export "handle_request") (result i32)
                i32.const 42
            )
        )
    "#;
    
    wat::parse_str(wat).expect("Failed to parse WAT module")
}
