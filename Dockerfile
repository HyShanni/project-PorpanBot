FROM rust:latest

WORKDIR /app

# Install system dependencies needed for songbird, ffmpeg, and yt-dlp
RUN apt-get update && apt-get install -y \
    ffmpeg \
    cmake \
    libopus-dev \
    pkg-config \
    python3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Download latest yt-dlp binary directly to avoid debian outdated package or pip breaking
RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

COPY . .

# Build the Rust application
RUN cargo build --release

# Run the binary
CMD ["./target/release/bot4fun"]
