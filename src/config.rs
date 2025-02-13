use anyhow::Result;
use clap::Parser;
use std::path::Path;
use url::Url;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input stream URL/path to monitor
    #[arg(short, long)]
    pub input: String,

    /// Output file path (optional)
    #[arg(short, long, default_value = "output.ts")]
    pub output: String,

    /// Metrics port to expose Prometheus metrics
    #[arg(short, long, default_value = "9090")]
    pub metrics_port: u16,

    /// ffmpeg cli path (optional)
    #[arg(short, long, default_value = "ffmpeg")]
    pub ffmpeg_path: String,
}

#[derive(Debug, Clone)]
pub enum StreamType {
    Srt(String),
    Hls(String),
    Rdp(String),
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

        // Check if it's an RDP connection string
        if input.starts_with("rdp://") || input.contains(":3389") {
            return Ok(StreamType::Rdp(input.to_string()));
        }

        // Check if it's a file path
        let path = Path::new(input);
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

    pub fn get_ffmpeg_input_args(&self) -> Vec<String> {
        match self {
            StreamType::Srt(url) => vec!["-i".to_string(), url.clone()],
            StreamType::Hls(url) => vec![
                "-i".to_string(),
                url.clone(),
                "-live_start_index".to_string(),
                "-1".to_string(),
            ],
            StreamType::Rdp(conn) => vec![
                "-f".to_string(),
                "gdigrab".to_string(),
                "-i".to_string(),
                conn.clone(),
            ],
            StreamType::MpegTs(url) => vec![
                "-i".to_string(),
                url.clone(),
                "-analyzeduration".to_string(),
                "2000000".to_string(),
                "-probesize".to_string(),
                "1000000".to_string(),
            ],
            StreamType::Rtmp(url) => vec!["-i".to_string(), url.clone()],
            StreamType::Rtsp(url) => vec![
                "-rtsp_transport".to_string(),
                "tcp".to_string(),
                "-i".to_string(),
                url.clone(),
            ],
            StreamType::Udp(url) => vec![
                "-i".to_string(),
                url.clone(),
                "-timeout".to_string(),
                "5000000".to_string(),
            ],
            StreamType::File(path) => vec!["-i".to_string(), path.clone()],
        }
    }
}
