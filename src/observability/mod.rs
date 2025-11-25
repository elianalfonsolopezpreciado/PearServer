// Observability infrastructure using tracing crate
// Provides structured logging and telemetry without blocking the main request loop

use anyhow::Result;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialize the observability system
/// Sets up structured logging to stdout with JSON formatting for machine parsing
pub fn init() -> Result<()> {
    // Create a JSON formatter for structured logs
    let fmt_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_span_events(FmtSpan::CLOSE);

    // Configure filter from environment or use default
    // Example: RUST_LOG=pear_server=debug,quinn=info
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("pear_server=info,quinn=warn,hyper=warn"))
        .expect("Failed to create tracing filter");

    // Build and set the global subscriber
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    Ok(())
}

/// Create a span for tracing request handling
/// Use this to instrument critical paths without blocking
#[inline]
pub fn request_span(protocol: &str, path: &str) -> tracing::Span {
    tracing::info_span!(
        "request",
        protocol = protocol,
        path = path,
        request_id = %uuid::Uuid::new_v4(),
    )
}

/// Record metrics for performance monitoring
/// This is a placeholder for future integration with metrics systems
#[inline]
pub fn record_request_duration(protocol: &str, duration_ms: u64) {
    tracing::debug!(
        protocol = protocol,
        duration_ms = duration_ms,
        "request completed"
    );
}
