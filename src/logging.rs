use tracing_subscriber::{fmt, EnvFilter};

/// Initialize structured logging with tracing.
/// Default level: info for cron_manager, warn for everything else.
/// Override with RUST_LOG env var (e.g., RUST_LOG=cron_manager=debug).
pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("cron_manager=info,warn"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}
