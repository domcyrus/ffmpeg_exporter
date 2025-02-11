mod config;
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

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize app state and registry
    let (app_state, registry) = AppState::new();

    // Initialize metrics
    let stdout_metrics = StdoutMetrics::new(&registry)?;
    let stderr_metrics = StderrMetrics::new(&registry)?;
    let connection_metrics = ConnectionMetrics::new(&registry)?;

    // Start the metrics server in a separate task
    let metrics_server = {
        let state = app_state.clone();
        let port = args.metrics_port;
        task::spawn(async move { server::run_server(state, port).await })
    };

    // Create the FFmpeg monitor
    let monitor = FFmpegMonitor::new(args.input, args.output)
        .context("Failed to initialize FFmpeg monitor")?;

    // Set up Ctrl+C handler
    let running = monitor.get_running_handle();
    ctrlc::set_handler(move || {
        println!("Received interrupt signal, shutting down...");
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
                eprintln!("Metrics server error: {:#}", e);
                std::process::exit(1);
            }
        }
        result = ffmpeg_task => {
            match result {
                Ok(Ok(())) => {
                    println!("FFmpeg monitor shut down gracefully");
                }
                Ok(Err(e)) => {
                    eprintln!("FFmpeg monitoring error: {:#}", e);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("FFmpeg task panicked: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
