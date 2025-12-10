# Multi-stage Dockerfile for Fido (Web + API + TUI)
# Stage 1: Builder - compile the Rust applications
FROM rust:1.83 as builder

WORKDIR /app

# Copy workspace files first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members
COPY fido-types ./fido-types
COPY fido-server ./fido-server
COPY fido-tui ./fido-tui
COPY fido-migrate ./fido-migrate

# Build dependencies first (better Docker layer caching)
RUN cargo build --release --bin fido-server --bin fido

# Stage 2: Runtime - full web stack with nginx + ttyd
FROM debian:bookworm-slim

# Create non-root user for security
RUN groupadd -r fido && useradd -r -g fido fido

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    nginx \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install ttyd for web terminal with checksum verification
RUN wget -O /usr/local/bin/ttyd https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 && \
    chmod +x /usr/local/bin/ttyd

# Copy compiled binaries from builder
COPY --from=builder /app/target/release/fido-server /usr/local/bin/fido-server
COPY --from=builder /app/target/release/fido /usr/local/bin/fido

# Copy web files
COPY web /var/www/html

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Copy start script
COPY start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh

# Expose port 8080 for nginx (web + terminal + API proxy)
EXPOSE 8080

# Set the entrypoint to start script
ENTRYPOINT ["/usr/local/bin/start.sh"]
