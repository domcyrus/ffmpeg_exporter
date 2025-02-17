use anyhow::Result;
use prometheus::{CounterVec, GaugeVec, Opts, Registry};

#[derive(Clone)]
pub struct StreamMetrics {
    pub fps: GaugeVec,
    pub frame_counter: GaugeVec,
    pub bitrate: GaugeVec,
    pub packet_corrupt: CounterVec,
    pub connection_state: GaugeVec,
    pub connection_reset: CounterVec,
    pub dropped_packets: CounterVec,
    pub codec_errors: CounterVec,
}

impl StreamMetrics {
    pub fn new(registry: &Registry) -> Result<Self> {
        let fps = GaugeVec::new(
            Opts::new("ffmpeg_fps", "Current frames per second"),
            &["stream_type", "stream_id", "media_type"],
        )?;

        let frame_counter = GaugeVec::new(
            Opts::new("ffmpeg_frames", "Number of frames processed"),
            &["type", "stream_id", "media_type"],
        )?;

        let bitrate = GaugeVec::new(
            Opts::new("ffmpeg_bitrate_kbits", "Current bitrate in kbits/s"),
            &["stream_id", "media_type"],
        )?;

        let packet_corrupt = CounterVec::new(
            Opts::new(
                "ffmpeg_packet_corrupt_total",
                "Total number of corrupt packets",
            ),
            &["stream_id", "media_type"],
        )?;

        let connection_state = GaugeVec::new(
            Opts::new(
                "ffmpeg_stream_connection_state",
                "Current connection state (1 = connected, 0 = disconnected)",
            ),
            &["stream_type"],
        )?;

        let connection_reset = CounterVec::new(
            Opts::new(
                "ffmpeg_stream_connection_reset_total",
                "Total number of connection resets",
            ),
            &["stream_type"],
        )?;

        let dropped_packets = CounterVec::new(
            Opts::new(
                "ffmpeg_dropped_packets_total",
                "Total number of dropped packets",
            ),
            &["stream_type"],
        )?;

        let codec_errors = CounterVec::new(
            Opts::new(
                "ffmpeg_codec_errors_total",
                "Total number of codec-specific errors",
            ),
            &["error_type", "stream_id"],
        )?;

        // Register all metrics
        registry.register(Box::new(fps.clone()))?;
        registry.register(Box::new(frame_counter.clone()))?;
        registry.register(Box::new(bitrate.clone()))?;
        registry.register(Box::new(packet_corrupt.clone()))?;
        registry.register(Box::new(connection_state.clone()))?;
        registry.register(Box::new(connection_reset.clone()))?;
        registry.register(Box::new(dropped_packets.clone()))?;
        registry.register(Box::new(codec_errors.clone()))?;

        Ok(Self {
            fps,
            frame_counter,
            bitrate,
            packet_corrupt,
            connection_state,
            connection_reset,
            dropped_packets,
            codec_errors,
        })
    }
}
