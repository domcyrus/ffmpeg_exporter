FROM rust:1.75-slim as builder

# Install FFmpeg build dependencies
RUN apt-get update && apt-get install -y \
    ffmpeg \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /usr/src/ffmpeg_exporter
COPY . .

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install FFmpeg runtime dependencies
RUN apt-get update && apt-get install -y \
    ffmpeg \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the built binary
COPY --from=builder /usr/src/ffmpeg_exporter/target/release/ffmpeg_exporter /usr/local/bin/ffmpeg_exporter

# Expose Prometheus metrics port
EXPOSE 9090

ENTRYPOINT ["ffmpeg_exporter"]
