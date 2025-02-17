// config.rs

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use url::Url;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input stream URL/path to monitor
    #[arg(short, long)]
    pub input: String,

    /// Metrics port to expose Prometheus metrics
    #[arg(short, long, default_value = "9090")]
    pub metrics_port: u16,

    /// ffprobe cli path (optional)
    #[arg(short, long, default_value = if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" })]
    pub ffprobe_path: String,

    /// Additional probe size in bytes
    #[arg(long, default_value = "2500")]
    pub probe_size: u32,

    /// Analysis duration in microseconds
    #[arg(long, default_value = "5000000")]
    pub analyze_duration: u32,

    /// Enable reporting log
    #[arg(short, long, default_value = "false")]
    pub report: bool,
}

#[derive(Debug, Clone)]
pub enum StreamType {
    Srt(String),
    Hls(String),
    MpegTs(String),
    Rtmp(String),
    Rtsp(String),
    Udp(String),
    File(String),
}

impl StreamType {
    pub fn from_input(input: &str) -> Result<Self> {
        // Try to parse as URL first
        if let Ok(url) = Url::parse(input) {
            return match url.scheme() {
                "srt" => Ok(StreamType::Srt(input.to_string())),
                "rtmp" => Ok(StreamType::Rtmp(input.to_string())),
                "rtsp" => Ok(StreamType::Rtsp(input.to_string())),
                "udp" => Ok(StreamType::Udp(input.to_string())),
                "http" | "https" => {
                    if input.ends_with(".m3u8") || input.ends_with(".m3u") {
                        Ok(StreamType::Hls(input.to_string()))
                    } else if input.ends_with(".ts") {
                        Ok(StreamType::MpegTs(input.to_string()))
                    } else {
                        Ok(StreamType::Hls(input.to_string()))
                    }
                }
                scheme => anyhow::bail!("Unsupported URL scheme: {}", scheme),
            };
        }

        // Check if it's a file path
        let path = PathBuf::from(input);
        if path.exists() {
            return match path.extension().and_then(|ext| ext.to_str()) {
                Some("ts") => Ok(StreamType::MpegTs(input.to_string())),
                Some("m3u8") | Some("m3u") => Ok(StreamType::Hls(input.to_string())),
                Some(_) => Ok(StreamType::File(input.to_string())),
                None => anyhow::bail!("Unable to determine file type"),
            };
        }

        anyhow::bail!("Unable to determine stream type for input: {}", input)
    }

    pub fn get_type_str(&self) -> &'static str {
        match self {
            StreamType::Srt(_) => "srt",
            StreamType::Hls(_) => "hls",
            StreamType::MpegTs(_) => "mpegts",
            StreamType::Rtmp(_) => "rtmp",
            StreamType::Rtsp(_) => "rtsp",
            StreamType::Udp(_) => "udp",
            StreamType::File(_) => "file",
        }
    }

    pub fn get_ffprobe_args(
        &self,
        probe_size: u32,
        analyze_duration: u32,
        report: bool,
    ) -> Vec<String> {
        let mut args = vec![
            "-show_packets".to_string(),
            "-show_frames".to_string(),
            "-of".to_string(),
            "csv".to_string(),
        ];

        // Add report argument if enabled
        if report {
            // add at the beginning of the args
            args.extend_from_slice(&["-report".to_string()]);
        }

        // Add stream-specific arguments
        match self {
            StreamType::Rtsp(_) => {
                args.extend_from_slice(&["-rtsp_transport".to_string(), "tcp".to_string()]);
            }
            StreamType::Hls(_) => {
                args.extend_from_slice(&["-live_start_index".to_string(), "-1".to_string()]);
            }
            _ => {}
        }

        // Add common probe arguments
        args.extend_from_slice(&[
            "-probesize".to_string(),
            probe_size.to_string(),
            "-analyzeduration".to_string(),
            analyze_duration.to_string(),
        ]);

        // Add input argument last
        args.extend_from_slice(&[
            "-i".to_string(),
            match self {
                StreamType::Srt(url) => url.clone(),
                StreamType::Hls(url) => url.clone(),
                StreamType::MpegTs(url) => url.clone(),
                StreamType::Rtmp(url) => url.clone(),
                StreamType::Rtsp(url) => url.clone(),
                StreamType::Udp(url) => url.clone(),
                StreamType::File(url) => url.clone(),
            },
        ]);

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_type_from_input() {
        assert!(matches!(
            StreamType::from_input("srt://localhost:1234").unwrap(),
            StreamType::Srt(_)
        ));
        assert!(matches!(
            StreamType::from_input("http://example.com/stream.m3u8").unwrap(),
            StreamType::Hls(_)
        ));
        assert!(matches!(
            StreamType::from_input("rtmp://server/live/stream").unwrap(),
            StreamType::Rtmp(_)
        ));
    }

    #[test]
    fn test_ffprobe_args() {
        let stream_type = StreamType::Srt("srt://localhost:1234".to_string());
        let args = stream_type.get_ffprobe_args(5000000, 5000000, true);
        assert!(args.contains(&"-report".to_string()));
        assert!(args.contains(&"-show_packets".to_string()));
        assert!(args.contains(&"-show_frames".to_string()));
        assert!(args.contains(&"srt://localhost:1234".to_string()));
    }
}
