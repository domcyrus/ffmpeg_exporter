#!/bin/bash
set -e  # Exit on error

# Configuration
TEMP_FILE="/tmp/10sec.mp4"
SRT_PORT=9999
HLS_DIR="/tmp/streaming"
HLS_SEGMENT_TIME=4

# Function to show usage
show_usage() {
    echo "Usage: $0 --input FILE [--hls] [--port PORT]"
    echo
    echo "Required:"
    echo "  --input FILE    Input video file to stream"
    echo
    echo "Optional:"
    echo "  --hls          Start HLS stream instead of SRT"
    echo "  --port PORT    Set SRT port (default: 9999)"
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

while [[ $# -gt 0 ]]; do
    case $1 in
        --hls)
            STREAM_TYPE="hls"
            shift
            ;;
        --port)
            SRT_PORT="$2"
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
    "hls")
        start_hls_stream
        ;;
esac
