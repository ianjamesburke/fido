#!/bin/bash
# Fido Web Terminal - Unified Start Script
# Works both locally (development) and in Docker (production)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration with defaults
# `PORT` is the externally exposed port in managed environments (Cloud Run/Fly).
# Keep fido-server on an internal port to avoid colliding with nginx.
APP_PORT=${PORT:-8080}
FIDO_SERVER_PORT=${FIDO_SERVER_PORT:-3000}
TTYD_PORT=${TTYD_PORT:-7681}
NGINX_PORT=${NGINX_PORT:-$APP_PORT}
DATABASE_PATH=${DATABASE_PATH:-./fido.db}
LOG_DIR=${LOG_DIR:-./logs}

# Detect environment - check for pre-built binaries (Docker/Fly.io) or local dev
if [ -f /usr/local/bin/fido-server ] && [ -f /usr/local/bin/fido ]; then
    ENV_MODE="docker"
    FIDO_SERVER_BIN="/usr/local/bin/fido-server"
    FIDO_TUI_BIN="/usr/local/bin/fido"
else
    ENV_MODE="local"
    FIDO_SERVER_BIN="./target/release/fido-server"
    FIDO_TUI_BIN="./target/release/fido"
fi

# Docker image nginx.conf proxies to 127.0.0.1:3000.
# Prevent accidental misconfiguration from environment overrides.
if [ "$ENV_MODE" = "docker" ] && [ "$FIDO_SERVER_PORT" != "3000" ]; then
    echo -e "${YELLOW}Warning: forcing FIDO_SERVER_PORT=3000 in docker mode to match nginx upstream${NC}"
    FIDO_SERVER_PORT=3000
fi

echo -e "${BLUE}╔══════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Fido Web Terminal Launcher               ║${NC}"
echo -e "${BLUE}║     Mode: ${GREEN}${ENV_MODE}${BLUE}         ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════╝${NC}"

# Cleanup function - aggressive cleanup to ensure ports are freed
cleanup() {
    echo -e "\n${YELLOW}Shutting down services...${NC}"
    
    # Kill by PID first
    if [ -n "$FIDO_SERVER_PID" ]; then
        kill $FIDO_SERVER_PID 2>/dev/null || true
        kill -9 $FIDO_SERVER_PID 2>/dev/null || true
    fi
    if [ -n "$TTYD_PID" ]; then
        kill $TTYD_PID 2>/dev/null || true
        kill -9 $TTYD_PID 2>/dev/null || true
    fi
    if [ -n "$NGINX_PID" ]; then
        kill $NGINX_PID 2>/dev/null || true
        kill -9 $NGINX_PID 2>/dev/null || true
    fi
    if [ -n "${FIDO_LOG_TAIL_PID:-}" ]; then
        kill $FIDO_LOG_TAIL_PID 2>/dev/null || true
        kill -9 $FIDO_LOG_TAIL_PID 2>/dev/null || true
    fi
    
    # Also kill by port to catch any stragglers
    lsof -ti:$NGINX_PORT 2>/dev/null | xargs kill -9 2>/dev/null || true
    lsof -ti:$TTYD_PORT 2>/dev/null | xargs kill -9 2>/dev/null || true
    lsof -ti:$FIDO_SERVER_PORT 2>/dev/null | xargs kill -9 2>/dev/null || true
    
    # Kill nginx by pid file if it exists
    if [ -f "$LOG_DIR/nginx.pid" ]; then
        kill -9 $(cat "$LOG_DIR/nginx.pid") 2>/dev/null || true
        rm -f "$LOG_DIR/nginx.pid"
    fi
    
    echo -e "${GREEN}All services stopped.${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Check dependencies for local mode
check_local_deps() {
    echo -e "${YELLOW}Checking dependencies...${NC}"
    
    # Check for ttyd
    if ! command -v ttyd &> /dev/null; then
        echo -e "${RED}Error: ttyd not found${NC}"
        echo "Install with: brew install ttyd (macOS) or apt install ttyd (Linux)"
        exit 1
    fi
    
    # Check for nginx
    if ! command -v nginx &> /dev/null; then
        echo -e "${RED}Error: nginx not found${NC}"
        echo "Install with: brew install nginx (macOS) or apt install nginx (Linux)"
        exit 1
    fi
    
    # Check for compiled binaries
    if [ ! -f "$FIDO_SERVER_BIN" ]; then
        echo -e "${YELLOW}Building fido-server...${NC}"
        cargo build --release --bin fido-server
    fi
    
    if [ ! -f "$FIDO_TUI_BIN" ]; then
        echo -e "${YELLOW}Building fido-tui...${NC}"
        cargo build --release --bin fido
    fi
    
    echo -e "${GREEN}All dependencies ready!${NC}"
}

# Create log directory
mkdir -p "$LOG_DIR"

# Local mode setup
if [ "$ENV_MODE" = "local" ]; then
    check_local_deps
    
    # Create local nginx config with absolute path
    NGINX_CONF="$(pwd)/$LOG_DIR/nginx-local.conf"
    
    # Find mime.types (different location on macOS vs Linux)
    if [ -f "/opt/homebrew/etc/nginx/mime.types" ]; then
        MIME_TYPES="/opt/homebrew/etc/nginx/mime.types"
    elif [ -f "/usr/local/etc/nginx/mime.types" ]; then
        MIME_TYPES="/usr/local/etc/nginx/mime.types"
    else
        MIME_TYPES="/etc/nginx/mime.types"
    fi
    
    cat > "$NGINX_CONF" << EOF
worker_processes 1;
error_log $(pwd)/$LOG_DIR/nginx-error.log;
pid $(pwd)/$LOG_DIR/nginx.pid;

events { 
    worker_connections 1024; 
}

http {
    include $MIME_TYPES;
    default_type application/octet-stream;
    access_log $(pwd)/$LOG_DIR/nginx-access.log;
    
    upstream fido_server {
        server 127.0.0.1:$FIDO_SERVER_PORT;
    }
    
    upstream ttyd_server {
        server 127.0.0.1:$TTYD_PORT;
    }
    
    server {
        listen $NGINX_PORT;
        root $(pwd)/web;
        
        location = /health {
            proxy_pass http://fido_server/health;
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto \$scheme;
        }
        
        location = / {
            try_files /index.html =404;
        }
        
        location ~* \.(css|js|png|jpg|ico|svg)$ {
            try_files \$uri =404;
        }
        
        location ^~ /ttyd/ws {
            proxy_http_version 1.1;
            proxy_set_header Host \$host;
            proxy_set_header X-Forwarded-Proto \$scheme;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header Upgrade websocket;
            proxy_set_header Connection "Upgrade";
            proxy_read_timeout 1d;
            proxy_pass http://ttyd_server/ttyd/ws;
        }

        location /ttyd/ {
            proxy_http_version 1.1;
            proxy_set_header Host \$host;
            proxy_set_header X-Forwarded-Proto \$scheme;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_read_timeout 1d;
            proxy_pass http://ttyd_server/ttyd/;
            add_header Content-Security-Policy "frame-ancestors 'self' https://fido-prod-ijb.web.app https://fido-prod-ijb.firebaseapp.com;" always;
        }
        
        location / {
            proxy_pass http://fido_server;
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
        }
    }
}
EOF
fi

# Start fido-server
echo -e "${YELLOW}Starting fido-server on port $FIDO_SERVER_PORT...${NC}"
export HOST=0.0.0.0
# fido-server reads `PORT`; keep it internal behind nginx.
export PORT=$FIDO_SERVER_PORT
export DATABASE_PATH=$DATABASE_PATH
export RUST_LOG=${RUST_LOG:-info}

$FIDO_SERVER_BIN > "$LOG_DIR/fido-server.log" 2>&1 &
FIDO_SERVER_PID=$!
echo -e "${GREEN}  ✓ fido-server started (PID: $FIDO_SERVER_PID)${NC}"

if [ "$ENV_MODE" = "docker" ]; then
    tail -n +1 -F "$LOG_DIR/fido-server.log" &
    FIDO_LOG_TAIL_PID=$!
fi

# Wait for server to be ready
sleep 2
if ! kill -0 "$FIDO_SERVER_PID" 2>/dev/null; then
    echo -e "${RED}Error: fido-server exited during startup${NC}"
    echo -e "${YELLOW}Last 200 lines of fido-server log:${NC}"
    tail -n 200 "$LOG_DIR/fido-server.log" || true
    exit 1
fi

if ! curl -fsS "http://127.0.0.1:${FIDO_SERVER_PORT}/health" >/dev/null 2>&1; then
    echo -e "${RED}Error: fido-server health endpoint failed during startup${NC}"
    echo -e "${YELLOW}Last 200 lines of fido-server log:${NC}"
    tail -n 200 "$LOG_DIR/fido-server.log" || true
    exit 1
fi

# Start ttyd
echo -e "${YELLOW}Starting ttyd on port $TTYD_PORT...${NC}"
export FIDO_DEMO_MODE=true

# Note: -O (check-origin) removed - nginx handles origin security
# -W enables writable mode for interactive terminal
if [ "$ENV_MODE" = "local" ]; then
    ttyd -p $TTYD_PORT -W -b /ttyd -m 10 $FIDO_TUI_BIN > "$LOG_DIR/ttyd.log" 2>&1 &
else
    /usr/local/bin/ttyd -p $TTYD_PORT -W -b /ttyd -m 10 $FIDO_TUI_BIN > "$LOG_DIR/ttyd.log" 2>&1 &
fi
TTYD_PID=$!
echo -e "${GREEN}  ✓ ttyd started (PID: $TTYD_PID)${NC}"

# Start nginx
echo -e "${YELLOW}Starting nginx on port $NGINX_PORT...${NC}"
if [ "$ENV_MODE" = "local" ]; then
    nginx -c "$NGINX_CONF" &
    NGINX_PID=$!
else
    nginx -g "daemon off;" &
    NGINX_PID=$!
fi
echo -e "${GREEN}  ✓ nginx started (PID: $NGINX_PID)${NC}"

# Print success message
echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  🐕 Fido Web Terminal is running!                  ║${NC}"
echo -e "${GREEN}╠════════════════════════════════════════════════════╣${NC}"
echo -e "${GREEN}║                                                    ║${NC}"
echo -e "${GREEN}║  Web Interface: ${BLUE}http://localhost:$NGINX_PORT${GREEN}            ║${NC}"
echo -e "${GREEN}║  API Server:    ${BLUE}http://localhost:$FIDO_SERVER_PORT${GREEN}            ║${NC}"
echo -e "${GREEN}║  Terminal:      ${BLUE}http://localhost:$NGINX_PORT/ttyd/${GREEN}       ║${NC}"
echo -e "${GREEN}║                                                    ║${NC}"
echo -e "${GREEN}║  Logs: $LOG_DIR/                                   ║${NC}"
echo -e "${GREEN}║                                                    ║${NC}"
echo -e "${GREEN}║  Press Ctrl+C to stop all services                 ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════╝${NC}"
echo ""

# Keep script running and wait for processes
wait
