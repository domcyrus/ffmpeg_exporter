# Test Scripts

This directory contains scripts for testing the FFmpeg Exporter. The scripts help create test streams and simulate network conditions for development and testing purposes.

## Scripts Overview

- `loop.sh`: Creates test streams (SRT/HLS) from a video file
- `network_sim.sh`: Simulates network conditions (macOS only)

## Test Stream Script (loop.sh)

Creates an infinite loop stream from a video file. Supports both SRT and HLS streaming protocols.

### Prerequisites

- FFmpeg installed on your system
- A video file for testing

### Usage

```bash
./loop.sh --input FILE [--hls] [--port PORT]
```

Required:

- `--input FILE`: Input video file to stream

Optional:

- `--hls`: Start an HLS stream instead of SRT
- `--port PORT`: Set the SRT listener port (default: 9999)

### Examples

```bash
# Start an SRT stream using a local video file
./loop.sh --input video.mp4

# Start an SRT stream on port 8888
./loop.sh --input video.mp4 --port 8888

# Start an HLS stream
./loop.sh --input video.mp4 --hls

# Show help
./loop.sh --help
```

### Output Locations

- SRT stream: Available at `srt://localhost:PORT` (default PORT is 9999)
- HLS stream: Available at `/tmp/streaming/stream.m3u8`
- Temporary video clip: Created at `/tmp/10sec.mp4`

## Network Simulation Script (network_sim.sh)

⚠️ **macOS Only**: This script uses macOS-specific tools (`pfctl` and `dummynet`) for network simulation.

Simulates various network conditions like packet loss and latency for testing stream behavior under different network conditions.

### Prerequisites

- macOS operating system
- Administrative privileges (sudo access)

### Usage

```bash
./network_sim.sh {enable|disable|status}
```

Commands:

- `enable`: Start network simulation
- `disable`: Stop network simulation
- `status`: Show current network rules

### Configuration

Edit these variables at the top of the script:

```bash
INTERFACE="lo0"      # Network interface (lo0 for localhost, en0 for Wi-Fi)
PACKET_LOSS="0.1"    # 10% packet loss
LATENCY="100ms"      # 100ms delay
```

### Examples

```bash
# Enable network simulation
sudo ./network_sim.sh enable

# Check current rules
sudo ./network_sim.sh status

# Disable simulation
sudo ./network_sim.sh disable
```

## Testing Scenarios

Here are some common testing scenarios combining both scripts:

### Basic Stream Testing

1. Start a test stream:

```bash
./loop.sh --input video.mp4
```

2. Monitor the stream:

```bash
cargo run -- --input srt://127.0.0.1:9999
# or if installed:
ffmpeg_exporter --input srt://127.0.0.1:9999
```

### Testing with Network Issues

1. Start a test stream:

```bash
./loop.sh --input video.mp4
```

2. Enable network simulation:

```bash
sudo ./network_sim.sh enable
```

3. Monitor the stream:

```bash
cargo run -- --input srt://127.0.0.1:9999
```

4. Check metrics to observe behavior under degraded network conditions

### Testing Stream Recovery

1. Start a test stream:

```bash
./loop.sh --input video.mp4
```

2. Start monitoring:

```bash
cargo run -- --input srt://127.0.0.1:9999
```

3. Temporarily stop the stream:

```bash
# Press Ctrl+C in the stream window
```

4. Restart the stream:

```bash
./loop.sh --input video.mp4
```

5. Observe reconnection metrics

## Troubleshooting

### Test Stream Issues

1. If FFmpeg fails to start:
   - Check if FFmpeg is installed: `ffmpeg -version`
   - Verify input file exists and is readable
   - Ensure the output port is available

2. If the stream is not accessible:
   - Check if the port is already in use
   - Verify network interface is working
   - For HLS: Ensure `/tmp/streaming` is writable

### Network Simulation Issues

1. Permission errors:
   - Run with sudo
   - Check SIP (System Integrity Protection) status

2. If simulation doesn't work:
   - Verify correct network interface
   - Check dummynet status: `sysctl net.inet.ip.dummynet.enabled`
   - Verify pfctl status: `sudo pfctl -si`

3. Reset network settings:

```bash
sudo ./network_sim.sh disable
sudo pfctl -F all -f /etc/pf.conf
```

## Linux Users

For Linux systems, you can simulate network conditions using `tc` instead of the provided network_sim.sh script:

```bash
# Add latency and packet loss
sudo tc qdisc add dev lo root netem delay 100ms loss 10%

# Remove rules
sudo tc qdisc del dev lo root
```

The stream testing script (`loop.sh`) works the same way on Linux systems.
