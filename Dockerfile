# Fido Web Terminal - Multi-service Docker Image
# Includes: fido-server (API), fido-tui (TUI), ttyd (web terminal), nginx (reverse proxy)

# Stage 1: Build Rust binaries
# Keep builder/runtime on the same Debian generation to avoid glibc ABI mismatch.
#
# Base images are pinned by multi-arch manifest digest so builds are reproducible
# and a registry-side tag change cannot silently alter the deployed image (same
# care as the ttyd SHA-256 pin below). To refresh a digest, query the registry:
#   TOKEN=$(curl -s "https://auth.docker.io/token?service=registry.docker.io&scope=repository:library/rust:pull" | sed -E 's/.*"token":"([^"]+)".*/\1/')
#   curl -sI -H "Authorization: Bearer $TOKEN" \
#     -H "Accept: application/vnd.oci.image.index.v1+json" \
#     https://registry-1.docker.io/v2/library/rust/manifests/1.91-bookworm \
#     | tr -d '\r' | grep -i '^docker-content-digest:'
FROM rust:1.91-bookworm@sha256:c1e5f19e773b7878c3f7a805dd00a495e747acbdc76fb2337a4ebf0418896b33 as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY fido-types ./fido-types
COPY fido-server ./fido-server
COPY fido-tui ./fido-tui

# Build release binaries
RUN cargo build --release --bin fido-server --bin fido

# Stage 2: Runtime image
# Pinned by digest (see the refresh command above; use repository library/debian
# and tag bookworm-slim).
FROM debian:bookworm-slim@sha256:60eac759739651111db372c07be67863818726f754804b8707c90979bda511df

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        nginx \
        gettext-base \
        curl \
        gosu \
    && rm -rf /var/lib/apt/lists/*

# Install ttyd from GitHub releases (not available in Debian repos)
# Pin to a known-good SHA-256 so a compromised/altered release asset fails the build.
# Hash is for the ttyd 1.7.7 `ttyd.x86_64` release asset. Confirm if the pin ever fails.
RUN curl -fL https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 -o /usr/local/bin/ttyd && \
    echo "8a217c968aba172e0dbf3f34447218dc015bc4d5e59bf51db2f2cd12b7be4f55  /usr/local/bin/ttyd" | sha256sum -c - && \
    chmod +x /usr/local/bin/ttyd

# Copy compiled binaries
COPY --from=builder /app/target/release/fido-server /usr/local/bin/fido-server
COPY --from=builder /app/target/release/fido /usr/local/bin/fido

# Copy configuration files
COPY nginx.conf /etc/nginx/nginx.conf.template
COPY nginx-api.conf /etc/nginx/nginx-api.conf.template
COPY start.sh /usr/local/bin/start.sh

# Copy web assets
COPY web /var/www/html

# Make start script executable
RUN chmod +x /usr/local/bin/start.sh

# Create a non-root user to run the services.
RUN useradd --system --create-home --home-dir /home/fido --shell /usr/sbin/nologin fido

# Create writable directories and hand ownership to the fido user.
# - /data                : persistent volume mount (SQLite when not ephemeral)
# - /var/log/fido        : app logs + rendered TUI wrapper + nginx pid (LOG_DIR)
# - /var/lib/nginx       : nginx client_body/proxy/fastcgi temp paths
# - /var/log/nginx       : nginx default access/error logs
# - /etc/nginx           : start.sh renders nginx.conf here at runtime via envsubst
RUN mkdir -p /data /var/log/fido /var/lib/nginx /var/log/nginx && \
    chmod 755 /data /var/log/fido && \
    chown -R fido:fido /data /var/log/fido /var/lib/nginx /var/log/nginx /etc/nginx

# Environment variables
ENV HOST=0.0.0.0
# External listener (nginx). Railway/Cloud Run inject PORT at runtime.
ENV PORT=8080
# Internal API listener (fido-server) behind nginx.
ENV FIDO_SERVER_PORT=3000
ENV TTYD_PORT=7681
# Deployment mode: `demo` (default) runs the public web terminal; the persistent
# API service overrides this to `api` (and mounts a volume at /data). One image,
# two Railway services — see docs/DEPLOY.md.
ENV FIDO_DEPLOY_MODE=demo
ENV DATABASE_PATH=/tmp/fido-web-demo.db
ENV LOG_DIR=/var/log/fido
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Health check uses the PORT env var (Railway overrides it at runtime)
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health || exit 1

# The entrypoint starts as root only long enough to fix ownership of the
# runtime-mounted volume (Railway mounts /data as root, shadowing the image-time
# chown), then drops to the non-root `fido` user via gosu for all services.
ENTRYPOINT ["/bin/sh", "-c", "chown -R fido:fido /data /var/log/fido 2>/dev/null || true; exec gosu fido /usr/local/bin/start.sh"]
