use regex::Regex;

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
