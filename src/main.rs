mod config;
mod logging;
mod metrics;
mod server;
mod stream;

use anyhow::{Context, Result};
use clap::Parser;
use config::Args;
use metrics::{AppState, ConnectionMetrics, StderrMetrics, StdoutMetrics};
use std::sync::atomic::Ordering;
use stream::FFmpegMonitor;
use tokio::task;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    logging::init_logging()?;
    info!("Starting FFmpeg monitor");
    debug!("Parsed arguments: {:?}", args);

    let (app_state, registry) = AppState::new();

    let stdout_metrics = StdoutMetrics::new(&registry)?;
    let stderr_metrics = StderrMetrics::new(&registry)?;
    let connection_metrics = ConnectionMetrics::new(&registry)?;

    let metrics_server = {
        let state = app_state.clone();
        let port = args.metrics_port;
        task::spawn(async move { server::run_server(state, port).await })
    };

    let monitor = FFmpegMonitor::new(args.input, args.output, args.ffmpeg_path)
        .context("Failed to initialize FFmpeg monitor")?;

    // Set up Ctrl+C handler
    let running = monitor.get_running_handle();
    ctrlc::set_handler(move || {
        info!("Received interrupt signal, shutting down...");
        running.store(false, Ordering::SeqCst);
    })?;

    // Start FFmpeg monitoring in a separate blocking task
    let ffmpeg_task = task::spawn_blocking(move || {
        monitor
            .run(stdout_metrics, stderr_metrics, connection_metrics)
            .context("Failed to run FFmpeg monitor")
    });

    // Wait for either task to complete
    tokio::select! {
        result = metrics_server => {
            if let Err(e) = result {
                error!("Metrics server error: {:#}", e);
                std::process::exit(1);
            }
        }
        result = ffmpeg_task => {
            match result {
                Ok(Ok(())) => {
                    info!("FFmpeg monitor shut down gracefully");
                }
                Ok(Err(e)) => {
                    error!("FFmpeg monitoring error: {:#}", e);
                    std::process::exit(1);
                }
                Err(e) => {
                    error!("FFmpeg task panicked: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
