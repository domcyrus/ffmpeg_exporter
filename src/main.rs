use anyhow::{Context, Result};
use clap::Parser;

mod config;
mod logging;
mod metrics;
mod server;
mod stream;

use crate::config::{Args, StreamType};
use crate::metrics::{AppState, StreamMetrics};
use crate::stream::FFprobeMonitor;
use std::sync::atomic::Ordering;
use tokio::task;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    logging::init_logging()?;
    info!("Starting FFprobe monitor");
    debug!("Parsed arguments: {:?}", args);

    // Create app state and metrics
    let (app_state, registry) = AppState::new();
    let metrics = StreamMetrics::new(&registry)?;

    // Determine stream type
    let stream_type =
        StreamType::from_input(&args.input).context("Failed to determine stream type")?;

    // Start HTTP server in background
    let metrics_server = {
        let state = app_state.clone();
        let port = args.metrics_port;
        task::spawn(async move { server::run_server(state, port).await })
    };

    // Create monitor
    let monitor = FFprobeMonitor::new(
        args.ffprobe_path,
        args.input,
        stream_type,
        metrics,
        args.probe_size,
        args.analyze_duration,
        args.report,
    );

    // Set up Ctrl+C handler
    let running = monitor.get_running_handle();
    ctrlc::set_handler(move || {
        info!("Received interrupt signal, shutting down...");
        running.store(false, Ordering::SeqCst);
    })?;

    // Start FFprobe monitoring in a separate blocking task
    let ffprobe_task =
        task::spawn_blocking(move || monitor.run().context("Failed to run FFprobe monitor"));

    // Wait for either task to complete
    tokio::select! {
        result = metrics_server => {
            if let Err(e) = result {
                error!("Metrics server error: {:#}", e);
                std::process::exit(1);
            }
        }
        result = ffprobe_task => {
            match result {
                Ok(Ok(())) => {
                    info!("FFprobe monitor shut down gracefully");
                }
                Ok(Err(e)) => {
                    error!("FFprobe monitoring error: {:#}", e);
                    std::process::exit(1);
                }
                Err(e) => {
                    error!("FFprobe task panicked: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
