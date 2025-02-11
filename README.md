# FFmpeg Exporter

A Prometheus exporter for FFmpeg streams that exposes detailed metrics about media streams. It supports various stream types including SRT, HLS, RTMP, RTSP, and more.

## Features

- Automatic stream type detection from URL/path
- Prometheus metrics exposure
- Automatic reconnection on stream failures
- Support for multiple stream protocols
- Detailed stream health metrics
- Connection state monitoring

## Installation

### Using Docker

The easiest way to run FFmpeg Exporter is using Docker:

```bash
# Build the image
docker build -t ffmpeg_exporter .

# Run the container
docker run -p 9090:9090 ffmpeg_exporter --input <INPUT_URL>
```

### Manual Installation

If you prefer to run without Docker, you'll need:

#### Prerequisites

- Rust 1.70 or higher (install via [rustup](https://rustup.rs/))
- FFmpeg 4.4 or higher

##### Ubuntu/Debian

```bash
# Install FFmpeg and build dependencies
apt-get update && apt-get install -y ffmpeg pkg-config
```

##### macOS

```bash
# Using Homebrew
brew install ffmpeg
```

##### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/ffmpeg_exporter.git

# Build
cd ffmpeg_exporter
cargo build --release

# Install (optional)
cargo install --path .
```

The built binary will be in `target/release/ffmpeg_exporter`

## Test Scripts

The scripts/ directory contains helper scripts for testing:

- Test stream generation from video files (SRT/HLS)
- Network condition simulation for macOS (packet loss, latency)

See scripts/README.md for detailed documentation of these testing tools.

## Usage

Basic usage:

```bash
ffmpeg_exporter --input <INPUT_URL> [--output <OUTPUT_FILE>] [--metrics-port <PORT>]
```

Examples:

```bash
# Monitor an SRT stream
ffmpeg_exporter --input srt://server:9999

# Monitor an HLS stream
ffmpeg_exporter --input https://example.com/stream.m3u8

# Monitor with custom output and metrics port
ffmpeg_exporter --input rtmp://server/live/stream --output output.ts --metrics-port 8080
```

### Supported Stream Types

The tool automatically detects the stream type from the input URL:

- SRT (srt://)
- HLS (.m3u8)
- RTMP (rtmp://)
- RTSP (rtsp://)
- MPEGTS (.ts)
- UDP (udp://)
- RDP (rdp:// or :3389)

## Metrics

The tool exposes Prometheus metrics on `http://localhost:9090/metrics` by default. Here are the available metrics:

### Stream Processing Metrics

- `ffmpeg_fps`: Current frames per second (gauge)
- `ffmpeg_frames`: Number of processed frames (gauge)
- `ffmpeg_speed`: Current processing speed relative to realtime (gauge)
- `ffmpeg_bitrate_kbits`: Current bitrate in kbits/s (gauge)

### Error Metrics

- `ffmpeg_decoding_errors_total`: Total number of decoding errors by frame type (counter)
  - Labels: `frame_type` ("I", "P", "B", "general")
- `ffmpeg_packet_corrupt_total`: Total number of corrupt packets (counter)
  - Labels: `stream` (stream identifier)

### Connection Metrics

- `ffmpeg_stream_connection_state`: Current connection state (gauge)
  - `1` = connected
  - `0` = disconnected
- `ffmpeg_stream_current_uptime_seconds`: Current uptime of the stream connection (gauge)
- `ffmpeg_stream_reconnect_attempts_total`: Total number of reconnection attempts (counter)
- `ffmpeg_stream_last_error`: Timestamp of the last error by type (gauge)
  - Labels: `error_type`

### Example Metrics Output

```
# HELP ffmpeg_bitrate_kbits Current bitrate in kbits/s
# TYPE ffmpeg_bitrate_kbits gauge
ffmpeg_bitrate_kbits 1026

# HELP ffmpeg_decoding_errors_total Total number of decoding errors
# TYPE ffmpeg_decoding_errors_total counter
ffmpeg_decoding_errors_total{frame_type="B"} 95
ffmpeg_decoding_errors_total{frame_type="I"} 21
ffmpeg_decoding_errors_total{frame_type="P"} 109
ffmpeg_decoding_errors_total{frame_type="general"} 167

# HELP ffmpeg_fps Current frames per second
# TYPE ffmpeg_fps gauge
ffmpeg_fps 27.91

# HELP ffmpeg_frames Number of frames processed by type
# TYPE ffmpeg_frames gauge
ffmpeg_frames{type="processed"} 420

# HELP ffmpeg_speed Current processing speed (relative to realtime)
# TYPE ffmpeg_speed gauge
ffmpeg_speed 1.36

# HELP ffmpeg_stream_connection_state Current connection state (1 = connected, 0 = disconnected)
# TYPE ffmpeg_stream_connection_state gauge
ffmpeg_stream_connection_state 1

# HELP ffmpeg_stream_current_uptime_seconds Current uptime of the stream connection in seconds
# TYPE ffmpeg_stream_current_uptime_seconds gauge
ffmpeg_stream_current_uptime_seconds 76

# HELP ffmpeg_stream_last_error Timestamp of the last error by type
# TYPE ffmpeg_stream_last_error gauge
ffmpeg_stream_last_error{error_type="connection_failed"} 1739278307

# HELP ffmpeg_stream_reconnect_attempts_total Total number of stream reconnection attempts
# TYPE ffmpeg_stream_reconnect_attempts_total counter
ffmpeg_stream_reconnect_attempts_total 4
```

## Recommended Prometheus Queries

Here are some useful PromQL queries for monitoring:

```promql
# Stream availability over time (as percentage)
avg_over_time(ffmpeg_stream_connection_state[1h]) * 100

# Connection stability (number of reconnects per hour)
rate(ffmpeg_stream_reconnect_attempts_total[1h]) * 3600

# Average bitrate over the last 5 minutes
avg_over_time(ffmpeg_bitrate_kbits[5m])

# Frame processing health
rate(ffmpeg_frames{type="processed"}[1m])

# Error rate by frame type
rate(ffmpeg_decoding_errors_total[5m])
```

## Building from Source

Requirements:

- Rust 1.70 or higher
- FFmpeg 4.4 or higher
- Standard build tools (gcc, make, etc.)

```bash
# Clone the repository
git clone https://github.com/yourusername/stream_mon.git

# Build
cd stream_mon
cargo build --release

# Run tests
cargo test

# Install
cargo install --path .
```

## License

[MIT](LICENSE)
