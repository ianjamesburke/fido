# Fido Web Terminal - Multi-service Docker Image
# Includes: fido-server (API), fido-tui (TUI), ttyd (web terminal), nginx (reverse proxy)

# Stage 1: Build Rust binaries
# Keep builder/runtime on the same Debian generation to avoid glibc ABI mismatch.
FROM rust:1.91-bookworm as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY fido-types ./fido-types
COPY fido-server ./fido-server
COPY fido-tui ./fido-tui
COPY fido-migrate ./fido-migrate

# Build release binaries
RUN cargo build --release --bin fido-server --bin fido

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        nginx \
        gettext-base \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Install ttyd from GitHub releases (not available in Debian repos)
RUN curl -L https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 -o /usr/local/bin/ttyd && \
    chmod +x /usr/local/bin/ttyd

# Copy compiled binaries
COPY --from=builder /app/target/release/fido-server /usr/local/bin/fido-server
COPY --from=builder /app/target/release/fido /usr/local/bin/fido

# Copy configuration files
COPY nginx.conf /etc/nginx/nginx.conf.template
COPY start.sh /usr/local/bin/start.sh

# Copy web assets
COPY web /var/www/html

# Make start script executable
RUN chmod +x /usr/local/bin/start.sh

# Create necessary directories
RUN mkdir -p /data /var/log/fido && chmod 755 /data /var/log/fido

# Environment variables
ENV HOST=0.0.0.0
# External listener (nginx). Railway/Cloud Run inject PORT at runtime.
ENV PORT=8080
# Internal API listener (fido-server) behind nginx.
ENV FIDO_SERVER_PORT=3000
ENV TTYD_PORT=7681
ENV DATABASE_PATH=/data/fido.db
ENV LOG_DIR=/var/log/fido
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Health check uses the PORT env var (Railway overrides it at runtime)
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health || exit 1

# Use start script as entrypoint
ENTRYPOINT ["/usr/local/bin/start.sh"]
