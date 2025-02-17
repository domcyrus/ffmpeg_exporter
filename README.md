# FFmpeg Exporter

[![CI](https://github.com/domcyrus/ffmpeg_exporter/actions/workflows/ci.yaml/badge.svg)](https://github.com/domcyrus/ffmpeg_exporter/actions/workflows/ci.yaml)

A Prometheus exporter that uses FFprobe (part of the FFmpeg toolkit) to expose detailed metrics about media streams. It supports various stream types including SRT, HLS, RTMP, RTSP, and more.

## Features

- Automatic stream type detection from URL/path
- Prometheus metrics exposure
- Automatic reconnection on stream failures
- Support for multiple stream protocols
- Detailed stream health metrics including:
  - Per-stream FPS monitoring
  - Bitrate tracking
  - Packet corruption detection
  - Codec error reporting
  - Connection state monitoring
- Structured logging with configurable levels

## Installation

### Pre-built Binaries

You can download pre-built binaries for Linux and Windows from the [releases page](https://github.com/domcyrus/ffmpeg_exporter/releases).

For Windows users:

1. Download the `ffmpeg_exporter-x86_64-pc-windows-gnu.tar.gz` file
2. Extract the `ffmpeg_exporter.exe`
3. Ensure FFprobe is installed and available in your PATH or specify its location using `--ffprobe-path`

For Linux users:

1. Download the `ffmpeg_exporter-x86_64-unknown-linux-gnu.tar.gz` file
2. Extract the `ffmpeg_exporter` binary
3. Make it executable: `chmod +x ffmpeg_exporter`

### Using Docker

The easiest way to run FFmpeg Exporter is using Docker:

```bash
# Pull the image
docker pull ghcr.io/domcyrus/ffmpeg_exporter:latest

# Run the container
docker run -p 9090:9090 ghcr.io/domcyrus/ffmpeg_exporter --input <INPUT_URL>
```

### Manual Installation

If you prefer to build from source, you'll need:

#### Prerequisites

- Rust 1.70 or higher (install via [rustup](https://rustup.rs/))
- FFprobe 4.4 or higher (part of FFmpeg)

##### Ubuntu/Debian

```bash
# Install FFmpeg (includes ffprobe) and build dependencies
apt-get update && apt-get install -y ffmpeg pkg-config
```

##### Windows

1. Install FFmpeg (includes ffprobe):
   - Download from [FFmpeg official website](https://ffmpeg.org/download.html)
   - Add FFmpeg to your system PATH or use `--ffprobe-path`
2. Install Visual Studio build tools or MinGW-w64

##### macOS

```bash
# Using Homebrew
brew install ffmpeg
```

#### Building from Source

```bash
# Clone the repository
git clone https://github.com/domcyrus/ffmpeg_exporter.git

# Build
cd ffmpeg_exporter
cargo build --release

# Install (optional)
cargo install --path .
```

The built binary will be in `target/release/ffmpeg_exporter`

## Usage

Basic usage:

```bash
ffmpeg_exporter --input <INPUT_URL> [OPTIONS]
```

### Command Line Options

```
OPTIONS:
    -i, --input <URL>                 Input stream URL/path to monitor
    -m, --metrics-port <PORT>         Metrics port to expose Prometheus metrics [default: 9090]
    -f, --ffprobe-path <PATH>        FFprobe executable path [default: ffprobe or ffprobe.exe on Windows]
        --probe-size <BYTES>         Additional probe size in bytes [default: 2500]
        --analyze-duration <MICROS>   Analysis duration in microseconds [default: 5000000]
    -r, --report                      Enable reporting log [default: false]
    -h, --help                        Print help information
    -V, --version                     Print version information
```

### Examples

```bash
# Monitor an SRT stream
ffmpeg_exporter --input srt://server:9999

# Monitor an HLS stream with custom probe size
ffmpeg_exporter --input https://example.com/stream.m3u8 --probe-size 5000

# Monitor with custom FFprobe path and metrics port
ffmpeg_exporter --input rtmp://server/live/stream --ffprobe-path /usr/local/bin/ffprobe --metrics-port 8080

# Enable detailed FFprobe reporting
ffmpeg_exporter --input rtsp://camera:554/stream --report

# Run with debug logging
RUST_LOG=debug ffmpeg_exporter --input srt://server:9999
```

### Supported Stream Types

The tool automatically detects the stream type from the input URL:

- SRT (srt://)
- HLS (.m3u8)
- RTMP (rtmp://)
- RTSP (rtsp://)
- MPEGTS (.ts)
- UDP (udp://)
- File (local media files)

## Metrics

The exporter exposes Prometheus metrics on `http://localhost:9090/metrics` by default. Available metrics include:

### Stream Processing Metrics

- `ffmpeg_fps`: Current frames per second (gauge)
  - Labels: `stream_type`, `stream_id`, `media_type`
- `ffmpeg_frames`: Number of processed frames (gauge)
  - Labels: `type`, `stream_id`, `media_type`
- `ffmpeg_bitrate_kbits`: Current bitrate in kbits/s (gauge)
  - Labels: `stream_id`, `media_type`

### Error Metrics

- `ffmpeg_packet_corrupt_total`: Total number of corrupt packets (counter)
  - Labels: `stream_id`, `media_type`
- `ffmpeg_codec_errors_total`: Total number of codec-specific errors (counter)
  - Labels: `error_type`, `stream_id`
- `ffmpeg_dropped_packets_total`: Total number of dropped packets (counter)
  - Labels: `stream_type`

### Connection Metrics

- `ffmpeg_stream_connection_state`: Current connection state (gauge)
  - `1` = connected
  - `0` = disconnected
  - Labels: `stream_type`
- `ffmpeg_stream_connection_reset_total`: Total number of connection resets (counter)
  - Labels: `stream_type`

### Example Metrics Output

```
# HELP ffmpeg_bitrate_kbits Current bitrate in kbits/s
# TYPE ffmpeg_bitrate_kbits gauge
ffmpeg_bitrate_kbits{media_type="audio",stream_id="1"} 2.952
ffmpeg_bitrate_kbits{media_type="video",stream_id="0"} 16.52

# HELP ffmpeg_fps Current frames per second
# TYPE ffmpeg_fps gauge
ffmpeg_fps{media_type="audio",stream_id="1",stream_type="srt"} 3.668
ffmpeg_fps{media_type="video",stream_id="0",stream_type="srt"} 17.372

# HELP ffmpeg_stream_connection_state Current connection state
# TYPE ffmpeg_stream_connection_state gauge
ffmpeg_stream_connection_state{stream_type="srt"} 1
```

## Logging

The exporter uses structured logging via the `tracing` crate. All logs are written to stdout/stderr.

### Log Levels

- ERROR: Critical issues requiring immediate attention
- WARN: Concerning but non-fatal issues (corrupt packets, temporary failures)
- INFO: Important state changes and operational events
- DEBUG: Detailed information useful for troubleshooting
- TRACE: Very detailed protocol-level information

### Configuring Log Level

```bash
# Set global log level
RUST_LOG=debug ffmpeg_exporter --input srt://server:9999

# Set different levels for different modules
RUST_LOG=info,ffmpeg_monitor=debug ffmpeg_exporter --input srt://server:9999
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[MIT](LICENSE)
