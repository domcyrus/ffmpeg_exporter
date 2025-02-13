use prometheus::{Counter, CounterVec, Gauge, GaugeVec, Opts, Registry};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct StdoutMetrics {
    pub fps: Gauge,
    pub frame_counter: GaugeVec,
    pub speed: Gauge,
    pub bitrate: Gauge,
}

impl StdoutMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        debug!("Initializing stdout metrics");

        let fps_gauge = Gauge::new("ffmpeg_fps", "Current frames per second")?;
        let frame_counter = GaugeVec::new(
            Opts::new("ffmpeg_frames", "Number of frames processed by type"),
            &["type"],
        )?;
        let speed_gauge = Gauge::new(
            "ffmpeg_speed",
            "Current processing speed (relative to realtime)",
        )?;
        let bitrate_gauge = Gauge::new("ffmpeg_bitrate_kbits", "Current bitrate in kbits/s")?;

        // Register metrics
        registry.register(Box::new(fps_gauge.clone()))?;
        registry.register(Box::new(frame_counter.clone()))?;
        registry.register(Box::new(speed_gauge.clone()))?;
        registry.register(Box::new(bitrate_gauge.clone()))?;

        Ok(Self {
            fps: fps_gauge,
            frame_counter,
            speed: speed_gauge,
            bitrate: bitrate_gauge,
        })
    }
}

#[derive(Clone)]
pub struct StderrMetrics {
    pub packet_corrupt: CounterVec,
    pub decoding_errors: CounterVec,
}

impl StderrMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        debug!("Initializing stderr metrics");
        let packet_corrupt = CounterVec::new(
            Opts::new(
                "ffmpeg_packet_corrupt_total",
                "Total number of corrupt packets",
            ),
            &["stream"],
        )?;

        let decoding_errors = CounterVec::new(
            Opts::new(
                "ffmpeg_decoding_errors_total",
                "Total number of decoding errors",
            ),
            &["frame_type"],
        )?;

        // Register metrics
        registry.register(Box::new(packet_corrupt.clone()))?;
        registry.register(Box::new(decoding_errors.clone()))?;

        Ok(Self {
            packet_corrupt,
            decoding_errors,
        })
    }
}

#[derive(Clone)]
pub struct ConnectionMetrics {
    pub reconnect_attempts: Counter,
    pub connection_state: Gauge,
    pub current_uptime: Gauge,
    pub last_error: GaugeVec,
}

impl ConnectionMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        debug!("Initializing connection metrics");

        let reconnect_attempts = Counter::new(
            "ffmpeg_stream_reconnect_attempts_total",
            "Total number of stream reconnection attempts",
        )?;

        let connection_state = Gauge::new(
            "ffmpeg_stream_connection_state",
            "Current connection state (1 = connected, 0 = disconnected)",
        )?;

        let current_uptime = Gauge::new(
            "ffmpeg_stream_current_uptime_seconds",
            "Current uptime of the stream connection in seconds",
        )?;

        let last_error = GaugeVec::new(
            Opts::new(
                "ffmpeg_stream_last_error",
                "Timestamp of the last error by type",
            ),
            &["error_type"],
        )?;

        // Register metrics
        registry.register(Box::new(reconnect_attempts.clone()))?;
        registry.register(Box::new(connection_state.clone()))?;
        registry.register(Box::new(current_uptime.clone()))?;
        registry.register(Box::new(last_error.clone()))?;

        info!("Connection metrics initialized successfully");

        Ok(Self {
            reconnect_attempts,
            connection_state,
            current_uptime,
            last_error,
        })
    }

    pub fn record_error(&self, error_type: &str) {
        error!(error_type, "Stream error occurred");
        self.last_error.with_label_values(&[error_type]).set(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as f64,
        );
    }
}
