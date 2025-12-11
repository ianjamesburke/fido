# Dockerfile - API server only
FROM rust:1.83 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY fido-types ./fido-types
COPY fido-server ./fido-server
COPY fido-tui ./fido-tui
COPY fido-migrate ./fido-migrate

# Build the server
RUN cargo build --release --bin fido-server

# Runtime stage
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy binary and web assets
COPY --from=builder /app/target/release/fido-server /usr/local/bin/fido-server
COPY web /web

# Set environment variables
ENV HOST=0.0.0.0
ENV PORT=8080
ENV DATABASE_PATH=/data/fido.db
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Create data directory
RUN mkdir -p /data && chmod 755 /data

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/fido-server"]