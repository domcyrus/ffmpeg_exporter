// stream/patterns.rs

use anyhow::Result;
use regex::Regex;

#[derive(Clone)]
pub struct StreamPatterns {
    pub packet_corrupt: Regex,
    pub srt_dropped: Regex,
    pub codec_error: Regex,
}

impl StreamPatterns {
    pub fn new() -> Result<Self> {
        Ok(Self {
            packet_corrupt: Regex::new(r"Packet corrupt \(stream = (\d+), dts = (\d+)\)")?,
            srt_dropped: Regex::new(r"RCV-DROPPED (\d+) packet")?,
            codec_error: Regex::new(r"\[(h264|hevc|vp8|vp9|av1).*?\] (.*?)(?:\n|$)")?,
        })
    }
}
