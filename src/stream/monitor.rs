use crate::config::StreamType;
use crate::metrics::StreamMetrics;
use crate::stream::patterns::StreamPatterns;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub struct FFprobeMonitor {
    ffprobe_path: String,
    input: String,
    stream_type: StreamType,
    metrics: StreamMetrics,
    probe_size: u32,
    analyze_duration: u32,
    report: bool,
    running: Arc<AtomicBool>,
}

impl FFprobeMonitor {
    pub fn new(
        ffprobe_path: String,
        input: String,
        stream_type: StreamType,
        metrics: StreamMetrics,
        probe_size: u32,
        analyze_duration: u32,
        report: bool,
    ) -> Self {
        Self {
            ffprobe_path,
            input,
            stream_type,
            metrics,
            probe_size,
            analyze_duration,
            report,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn get_running_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    fn build_ffprobe_command(&self) -> Command {
        let mut cmd = Command::new(&self.ffprobe_path);

        #[cfg(windows)]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        // Use the stream-specific arguments from StreamType
        let args =
            self.stream_type
                .get_ffprobe_args(self.probe_size, self.analyze_duration, self.report);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

        debug!("FFprobe command: {:?}", cmd);
        cmd
    }

    #[instrument(skip(self))]
    pub fn run(&self) -> Result<()> {
        info!("Starting FFprobe monitoring for {}", self.input);
        const RETRY_DELAY: Duration = Duration::from_secs(10);

        while self.running.load(Ordering::SeqCst) {
            info!("Initiating new FFprobe process");
            let _start_time = Instant::now();
            self.metrics
                .connection_state
                .with_label_values(&[self.stream_type.get_type_str()])
                .set(1.0);

            match self.run_single_monitor() {
                Ok(()) => {
                    // Process exited normally, continue monitoring
                    info!("FFprobe process completed normally, restarting");
                    self.metrics
                        .connection_state
                        .with_label_values(&[self.stream_type.get_type_str()])
                        .set(0.0);
                    self.metrics
                        .connection_reset
                        .with_label_values(&[self.stream_type.get_type_str()])
                        .inc();

                    // Wait before restarting
                    warn!(
                        "Waiting before restarting FFprobe process for {}",
                        RETRY_DELAY.as_secs()
                    );
                    for _ in 0..100 {
                        if !self.running.load(Ordering::SeqCst) {
                            info!("Shutdown requested during restart wait");
                            return Ok(());
                        }
                        thread::sleep(RETRY_DELAY / 100);
                    }
                }
                Err(e) => {
                    error!(?e, "FFprobe process failed");
                    self.metrics
                        .connection_state
                        .with_label_values(&[self.stream_type.get_type_str()])
                        .set(0.0);
                    self.metrics
                        .connection_reset
                        .with_label_values(&[self.stream_type.get_type_str()])
                        .inc();

                    warn!(
                        "Waiting before retrying FFprobe process for {}",
                        RETRY_DELAY.as_secs()
                    );
                    for _ in 0..100 {
                        if !self.running.load(Ordering::SeqCst) {
                            info!("Shutdown requested during retry wait");
                            return Ok(());
                        }
                        thread::sleep(RETRY_DELAY / 100);
                    }
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    fn run_single_monitor(&self) -> Result<()> {
        let mut cmd = self.build_ffprobe_command();
        let mut child = cmd.spawn().context("Failed to spawn ffprobe process")?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stderr = child.stderr.take().context("Failed to capture stderr")?;

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let patterns = StreamPatterns::new()?;
        let (error_tx, error_rx) = std::sync::mpsc::channel();

        // Spawn stderr processing thread
        let stream_type = self.stream_type.clone();
        let metrics = self.metrics.clone();
        let patterns_clone = patterns.clone();
        let error_tx_clone = error_tx.clone();
        let running = self.running.clone();
        thread::spawn(move || {
            if let Err(e) = process_stderr(
                stderr_reader,
                &patterns_clone,
                &metrics,
                stream_type.get_type_str(),
            ) {
                error!(?e, "Error processing stderr");
                let _ = error_tx_clone.send(e);
                running.store(false, Ordering::SeqCst);
            }
        });

        // Process stdout in separate thread
        let metrics = self.metrics.clone();
        let stream_type = self.stream_type.clone();
        let error_tx_clone = error_tx.clone();
        let running_clone = self.running.clone();
        thread::spawn(move || {
            if let Err(e) = process_stdout(stdout_reader, &metrics, &stream_type) {
                error!(?e, "Error processing stdout");
                let _ = error_tx_clone.send(e);
                running_clone.store(false, Ordering::SeqCst);
            }
        });

        // Monitor the process and error channels
        loop {
            match error_rx.try_recv() {
                Ok(error) => {
                    let _ = child.kill();
                    return Err(error);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        let code = status.code().unwrap_or(-1);
                        return Err(anyhow::anyhow!(
                            "FFprobe process failed with exit code: {}",
                            code
                        ));
                    }
                    break;
                }
                Ok(None) => {
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Error waiting for FFprobe process: {}", e));
                }
            }

            if !self.running.load(Ordering::SeqCst) {
                let _ = child.kill();
                break;
            }
        }

        Ok(())
    }
}

fn process_stderr(
    reader: impl BufRead,
    patterns: &StreamPatterns,
    metrics: &StreamMetrics,
    stream_type: &str,
) -> Result<()> {
    for line in reader.lines() {
        let line = line.context("Failed to read stderr line")?;
        debug!("FFprobe stderr: {}", line);

        // Check for SRT dropped packets
        if let Some(caps) = patterns.srt_dropped.captures(&line) {
            if let Some(count) = caps.get(1).and_then(|m| m.as_str().parse::<f64>().ok()) {
                metrics
                    .dropped_packets
                    .with_label_values(&[stream_type])
                    .inc_by(count);
            }
        }

        // Check for corrupt packets
        if let Some(caps) = patterns.packet_corrupt.captures(&line) {
            if let Some(stream_id) = caps.get(1) {
                let stream_id = stream_id.as_str();
                metrics
                    .packet_corrupt
                    .with_label_values(&[stream_id, "unknown"])
                    .inc();
            }
        }

        // Check for codec-specific errors
        if let Some(caps) = patterns.codec_error.captures(&line) {
            let error_type = match caps.get(2).map(|m| m.as_str()) {
                Some(msg) if msg.contains("SEI") => "sei_error",
                Some(msg) if msg.contains("PPS") => "pps_error",
                Some(msg) if msg.contains("decode_slice_header") => "slice_header_error",
                Some(msg) if msg.contains("no frame") => "missing_frame",
                _ => "other",
            };
            metrics
                .codec_errors
                .with_label_values(&[error_type, "0"])
                .inc();
        }
    }
    Ok(())
}

fn process_stdout(
    reader: impl BufRead,
    metrics: &StreamMetrics,
    stream_type: &StreamType,
) -> Result<()> {
    let mut frame_times: Vec<(String, f64)> = Vec::new();
    let mut last_fps_update = Instant::now();

    for line in reader.lines() {
        let line = line.context("Failed to read stdout line")?;
        debug!("FFprobe stdout: {:?}", line);
        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() < 3 {
            continue;
        }

        match parts[0] {
            "packet" => process_packet_line(&parts, metrics)?,
            "frame" => process_frame_line(
                &parts,
                metrics,
                stream_type,
                &mut frame_times,
                &mut last_fps_update,
            )?,
            _ => continue,
        }
    }

    Ok(())
}

fn process_packet_line(parts: &[&str], metrics: &StreamMetrics) -> Result<()> {
    if parts.len() >= 12 {
        let media_type = parts[1];
        let stream_id = parts[2];

        if let Ok(size) = parts[9].parse::<f64>() {
            metrics
                .bitrate
                .with_label_values(&[stream_id, media_type])
                .set(size * 8.0 / 1000.0);
        }

        // Check flags for corruption
        if parts.len() >= 11 && parts[11].contains('C') {
            metrics
                .packet_corrupt
                .with_label_values(&[stream_id, media_type])
                .inc();
        }
    }
    Ok(())
}

fn process_frame_line(
    parts: &[&str],
    metrics: &StreamMetrics,
    stream_type: &StreamType,
    frame_times: &mut Vec<(String, f64)>,
    last_fps_update: &mut Instant,
) -> Result<()> {
    if parts.len() >= 6 {
        let media_type = parts[1];
        let stream_id = parts[2];

        metrics
            .frame_counter
            .with_label_values(&["processed", stream_id, media_type])
            .inc();

        if let Ok(pts_time) = parts[5].parse::<f64>() {
            frame_times.push((format!("{}_{}", stream_id, media_type), pts_time));

            // Keep only last 100 frames per stream
            while frame_times.len() > 100 {
                frame_times.remove(0);
            }

            // Update FPS every second
            if last_fps_update.elapsed().as_secs() >= 1 {
                // Group frames by stream_id and media_type
                let mut stream_frames: HashMap<String, Vec<f64>> = HashMap::new();

                for (key, time) in frame_times.iter() {
                    stream_frames.entry(key.clone()).or_default().push(*time);
                }

                // Calculate FPS for each stream
                for (key, times) in stream_frames {
                    if times.len() >= 2 {
                        let time_diff = times.last().unwrap() - times.first().unwrap();
                        let fps = times.len() as f64 / time_diff;

                        let (stream_id, media_type) =
                            key.split_once('_').unwrap_or(("0", "unknown"));

                        metrics
                            .fps
                            .with_label_values(&[stream_type.get_type_str(), stream_id, media_type])
                            .set(fps);
                    }
                }
                *last_fps_update = Instant::now();
            }
        }
    }
    Ok(())
}
