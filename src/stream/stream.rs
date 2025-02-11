use crate::config::StreamType;
use crate::metrics::{ConnectionMetrics, StderrMetrics, StdoutMetrics};
use anyhow::{Context, Result};
use regex::Regex;
use std::io::{self, BufRead};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct StreamPatterns {
    pub fps: Regex,
    pub frame: Regex,
    pub speed: Regex,
    pub bitrate: Regex,
}

impl StreamPatterns {
    pub fn new() -> Self {
        Self {
            fps: Regex::new(r"fps=\s*(\d+\.?\d*)").unwrap(),
            frame: Regex::new(r"frame=\s*(\d+)").unwrap(),
            speed: Regex::new(r"speed=\s*(\d+\.?\d*)x").unwrap(),
            bitrate: Regex::new(r"bitrate=\s*(\d+\.?\d*)kbits/s").unwrap(),
        }
    }
}

pub struct FFmpegMonitor {
    output: String,
    stream_type: StreamType,
    running: Arc<AtomicBool>,
}

impl FFmpegMonitor {
    pub fn new(input: String, output: String) -> Result<Self> {
        let stream_type = StreamType::from_input(&input)
            .with_context(|| format!("Failed to determine stream type for input: {}", input))?;
        // remove the output file if it exists
        if std::path::Path::new(&output).exists() {
            std::fs::remove_file(&output).context("Failed to remove existing output file")?;
        }

        Ok(Self {
            output,
            stream_type,
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn get_running_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    fn build_ffmpeg_command(&self) -> Command {
        let mut ffmpeg = Command::new("ffmpeg");

        let input_args = self.stream_type.get_ffmpeg_input_args();
        for arg in input_args {
            ffmpeg.arg(arg);
        }

        ffmpeg
            .arg("-stats")
            .arg("-stats_period")
            .arg("1")
            .arg("-progress")
            .arg("pipe:1")
            .arg(&self.output)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        ffmpeg
    }

    pub fn run(
        &self,
        stdout_metrics: StdoutMetrics,
        stderr_metrics: StderrMetrics,
        connection_metrics: ConnectionMetrics,
    ) -> Result<()> {
        const RETRY_DELAY: Duration = Duration::from_secs(10);

        while self.running.load(Ordering::SeqCst) {
            let start_time = Instant::now();
            connection_metrics.connection_state.set(1.0); // Connected

            match self.start_single_process(
                stdout_metrics.clone(),
                stderr_metrics.clone(),
                connection_metrics.clone(),
                start_time,
            ) {
                Ok(()) => {
                    // Clean exit (e.g., from Ctrl+C)
                    connection_metrics.connection_state.set(0.0);
                    break;
                }
                Err(e) => {
                    eprintln!("FFmpeg process failed: {:#}", e);
                    connection_metrics.connection_state.set(0.0);
                    connection_metrics.reconnect_attempts.inc();
                    connection_metrics.record_error("connection_failed");

                    // Wait before retrying, but check running flag periodically
                    for _ in 0..100 {
                        if !self.running.load(Ordering::SeqCst) {
                            return Ok(());
                        }
                        thread::sleep(RETRY_DELAY / 100);
                    }
                }
            }
        }

        Ok(())
    }

    fn start_single_process(
        &self,
        stdout_metrics: StdoutMetrics,
        stderr_metrics: StderrMetrics,
        connection_metrics: ConnectionMetrics,
        start_time: Instant,
    ) -> Result<()> {
        let mut ffmpeg = self
            .build_ffmpeg_command()
            .spawn()
            .context("Failed to spawn ffmpeg process")?;

        let stdout = ffmpeg.stdout.take().context("Failed to capture stdout")?;
        let stderr = ffmpeg.stderr.take().context("Failed to capture stderr")?;

        let stdout_reader = io::BufReader::new(stdout);
        let stderr_reader = io::BufReader::new(stderr);

        let patterns = StreamPatterns::new();

        // Create channels for error propagation
        let (error_tx, error_rx) = std::sync::mpsc::channel();

        // Handle stdout in separate thread
        let patterns_clone = patterns.clone();
        let stdout_metrics_clone = stdout_metrics.clone();
        let error_tx_clone = error_tx.clone();
        let running = self.running.clone();
        thread::spawn(move || {
            if let Err(e) =
                Self::process_stdout(stdout_reader, patterns_clone, stdout_metrics_clone)
            {
                eprintln!("Error processing stdout: {:#}", e);
                let _ = error_tx_clone.send(e);
                running.store(false, Ordering::SeqCst);
            }
        });

        // Handle stderr in separate thread
        let error_tx_clone = error_tx.clone();
        let running_clone = self.running.clone();
        thread::spawn(move || {
            if let Err(e) = Self::process_stderr(stderr_reader, stderr_metrics) {
                eprintln!("Error processing stderr: {:#}", e);
                let _ = error_tx_clone.send(e);
                running_clone.store(false, Ordering::SeqCst);
            }
        });

        // Start uptime tracking thread
        let running_clone = self.running.clone();
        let current_uptime = connection_metrics.current_uptime.clone();
        thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                let uptime = start_time.elapsed().as_secs() as f64;
                current_uptime.set(uptime);
                thread::sleep(Duration::from_secs(1));
            }
        });

        // Monitor the process and error channels
        loop {
            // Check for errors from stdout/stderr processing
            match error_rx.try_recv() {
                Ok(error) => {
                    let _ = ffmpeg.kill();
                    return Err(error);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // No errors, continue checking
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // All senders dropped, check process status
                    break;
                }
            }

            // Check if the process is still running
            match ffmpeg.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        let code = status.code().unwrap_or(-1);
                        return Err(anyhow::anyhow!(
                            "FFmpeg process failed with exit code: {}",
                            code
                        ));
                    }
                    break;
                }
                Ok(None) => {
                    // Process still running, wait a bit before checking again
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Error waiting for FFmpeg process: {}", e));
                }
            }

            // Check if we should stop
            if !self.running.load(Ordering::SeqCst) {
                let _ = ffmpeg.kill();
                break;
            }
        }

        Ok(())
    }

    fn process_stdout(
        reader: impl BufRead,
        patterns: StreamPatterns,
        metrics: StdoutMetrics,
    ) -> Result<()> {
        for line in reader.lines() {
            let line = line.context("Failed to read stdout line")?;

            if let Some(captures) = patterns.fps.captures(&line) {
                let fps = captures[1]
                    .parse::<f64>()
                    .context("Failed to parse FPS value")?;
                metrics.fps.set(fps);
            }
            if let Some(captures) = patterns.frame.captures(&line) {
                if let Ok(frames) = captures[1].parse::<f64>() {
                    metrics
                        .frame_counter
                        .with_label_values(&["processed"])
                        .set(frames);
                }
            }
            if let Some(captures) = patterns.speed.captures(&line) {
                if let Ok(speed) = captures[1].parse::<f64>() {
                    metrics.speed.set(speed);
                }
            }
            if let Some(captures) = patterns.bitrate.captures(&line) {
                if let Ok(bitrate) = captures[1].parse::<f64>() {
                    metrics.bitrate.set(bitrate);
                }
            }
        }
        Ok(())
    }

    fn process_stderr(reader: impl BufRead, metrics: StderrMetrics) -> Result<()> {
        let frame_error_regex = Regex::new(r"concealing.*in (I|P|B) frame")
            .context("Failed to compile frame error regex")?;

        for line in reader.lines() {
            let line = line.context("Failed to read stderr line")?;

            // Log all stderr output for debugging
            eprintln!("FFmpeg stderr: {}", line);

            if let Some(stream_id) = line.find("corrupt packet") {
                metrics
                    .packet_corrupt
                    .with_label_values(&[&stream_id.to_string()])
                    .inc();
            }

            if line.contains("error while decoding") {
                metrics
                    .decoding_errors
                    .with_label_values(&["general"])
                    .inc();
            }

            if let Some(captures) = frame_error_regex.captures(&line) {
                let frame_type = captures.get(1).map_or("unknown", |m| m.as_str());
                metrics
                    .decoding_errors
                    .with_label_values(&[frame_type])
                    .inc();
            }
        }
        Ok(())
    }
}
