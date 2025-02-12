#!/bin/bash
set -e  # Exit on error

# Configuration
TEMP_FILE="/tmp/10sec.mp4"
SRT_PORT=9999
RTP_PORT=5004
MPEGTS_PORT=5005
UDP_PORT=5006
HLS_DIR="/tmp/streaming"
HLS_SEGMENT_TIME=4
DEFAULT_ADDRESS="127.0.0.1"

# Function to show usage
show_usage() {
    echo "Usage: $0 --input FILE [--protocol PROTOCOL] [--port PORT] [--address ADDRESS]"
    echo
    echo "Required:"
    echo "  --input FILE       Input video file to stream"
    echo
    echo "Optional:"
    echo "  --protocol PROTO   Streaming protocol (srt/hls/rtp/mpegts/udp) (default: srt)"
    echo "  --port PORT        Set streaming port (default varies by protocol)"
    echo "  --address ADDRESS  Set destination address (default: 127.0.0.1)"
    exit 1
}

# Function to check if ffmpeg is installed
check_ffmpeg() {
    if ! command -v ffmpeg &> /dev/null; then
        echo "Error: ffmpeg is not installed"
        exit 1
    fi
}

# Function to create test file
create_test_file() {
    echo "Creating 10 second test file..."
    rm -f "${TEMP_FILE}"
    ffmpeg -i "${INPUT_FILE}" -t 10 -c copy "${TEMP_FILE}"
}

# Function to start SRT stream
start_srt_stream() {
    echo "Starting SRT stream on port ${SRT_PORT}..."
    ffmpeg -stream_loop -1 -re -i "${TEMP_FILE}" \
        -c copy \
        -f mpegts "srt://0.0.0.0:${SRT_PORT}?mode=listener"
}

# Function to start RTP stream
start_rtp_stream() {
    echo "Starting RTP stream to ${DEST_ADDRESS}:${RTP_PORT}..."
    ffmpeg -stream_loop -1 -re -i "${TEMP_FILE}" \
        -c copy \
        -f rtp "rtp://${DEST_ADDRESS}:${RTP_PORT}"
}

# Function to start MPEG-TS stream
start_mpegts_stream() {
    echo "Starting MPEG-TS stream to ${DEST_ADDRESS}:${MPEGTS_PORT}..."
    ffmpeg -stream_loop -1 -re -i "${TEMP_FILE}" \
        -c copy \
        -f mpegts "tcp://${DEST_ADDRESS}:${MPEGTS_PORT}?listen=1"
}

# Function to start UDP stream
start_udp_stream() {
    echo "Starting UDP stream to ${DEST_ADDRESS}:${UDP_PORT}..."
    ffmpeg -stream_loop -1 -re -i "${TEMP_FILE}" \
        -c copy \
        -f mpegts "udp://${DEST_ADDRESS}:${UDP_PORT}"
}

# Function to start HLS stream
start_hls_stream() {
    echo "Starting HLS stream in ${HLS_DIR}..."
    mkdir -p "${HLS_DIR}"
    rm -f "${HLS_DIR}"/*.ts "${HLS_DIR}"/*.m3u8

    ffmpeg -re -stream_loop -1 -i "${TEMP_FILE}" \
        -c:v copy -c:a copy \
        -f hls \
        -hls_time "${HLS_SEGMENT_TIME}" \
        -hls_playlist_type event \
        -hls_flags delete_segments \
        "${HLS_DIR}/stream.m3u8"
}

# Main script
check_ffmpeg

# Process command line arguments
STREAM_TYPE="srt"
INPUT_FILE=""
DEST_ADDRESS="${DEFAULT_ADDRESS}"

while [[ $# -gt 0 ]]; do
    case $1 in
        --protocol)
            STREAM_TYPE="$2"
            shift 2
            ;;
        --port)
            case "${STREAM_TYPE}" in
                "srt") SRT_PORT="$2" ;;
                "rtp") RTP_PORT="$2" ;;
                "mpegts") MPEGTS_PORT="$2" ;;
                "udp") UDP_PORT="$2" ;;
            esac
            shift 2
            ;;
        --address)
            DEST_ADDRESS="$2"
            shift 2
            ;;
        --input)
            INPUT_FILE="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            ;;
        *)
            echo "Unknown parameter: $1"
            show_usage
            ;;
    esac
done

# Validate input file
if [ -z "${INPUT_FILE}" ]; then
    echo "Error: Input file is required"
    show_usage
fi

if [ ! -f "${INPUT_FILE}" ]; then
    echo "Error: Input file '${INPUT_FILE}' does not exist"
    exit 1
fi

create_test_file

case "${STREAM_TYPE}" in
    "srt")
        start_srt_stream
        ;;
    "rtp")
        start_rtp_stream
        ;;
    "mpegts")
        start_mpegts_stream
        ;;
    "udp")
        start_udp_stream
        ;;
    "hls")
        start_hls_stream
        ;;
    *)
        echo "Error: Unknown streaming protocol '${STREAM_TYPE}'"
        show_usage
        ;;
esac
