FROM rust:1.76-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire project
COPY . .

# Build the application
RUN cargo build --release

# Create the runtime image
FROM debian:12-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/discovery-tracker /app/discovery-tracker
# Copy config
COPY config.yaml /app/config.yaml

# Create necessary directories
RUN mkdir -p /app/data/storage /app/data/changes /app/logs

CMD ["/app/discovery-tracker"]