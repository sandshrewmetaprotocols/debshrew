# Use the official Rust image as the base
FROM rust:1.82 as builder

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the entire debshrew workspace
COPY . .

# Build the debshrew binary in release mode
RUN cargo build --release --bin debshrew

# Create a smaller runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/debshrew /usr/local/bin/debshrew

# Create a directory for transforms
RUN mkdir -p /app/transforms

# Set the working directory
WORKDIR /app

# Make debshrew executable
RUN chmod +x /usr/local/bin/debshrew

# Expose any necessary ports (if debshrew has a web interface or API)
# EXPOSE 8080

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/debshrew"]