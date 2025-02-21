use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

pub fn init_logging() -> Result<()> {
    // Create a default env filter that can be overridden by RUST_LOG
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ffmpeg_monitor=debug"));

    // Initialize subscriber with stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    Ok(())
}
