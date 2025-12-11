# Incremental Dockerfile - API server + Web interface
FROM rust:1.83 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY fido-types ./fido-types
COPY fido-server ./fido-server
COPY fido-tui ./fido-tui
COPY fido-migrate ./fido-migrate

# Build the server and TUI
RUN cargo build --release --bin fido-server --bin fido

# Runtime stage - add nginx for web interface
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates nginx wget && \
    rm -rf /var/lib/apt/lists/*

# Install ttyd for web terminal
RUN wget -O /usr/local/bin/ttyd https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 && \
    chmod +x /usr/local/bin/ttyd

# Copy binaries
COPY --from=builder /app/target/release/fido-server /usr/local/bin/fido-server
COPY --from=builder /app/target/release/fido /usr/local/bin/fido

# Copy web files
COPY web /var/www/html

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Copy startup script (we'll create a new one)
COPY start-web.sh /usr/local/bin/start-web.sh
RUN chmod +x /usr/local/bin/start-web.sh

# Set environment variables
ENV HOST=0.0.0.0
ENV PORT=3000
ENV DATABASE_PATH=/data/fido.db
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Create data directory
RUN mkdir -p /data && chmod 755 /data

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/start-web.sh"]