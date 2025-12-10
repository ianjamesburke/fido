#!/bin/bash
set -e

# Function to check if API server is ready
check_api_ready() {
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if curl -f http://localhost:${PORT:-8080}/health >/dev/null 2>&1; then
            echo "API server is ready!"
            return 0
        fi
        echo "Waiting for API server... (attempt $attempt/$max_attempts)"
        sleep 1
        ((attempt++))
    done
    
    echo "API server failed to start within timeout"
    return 1
}

# Start the API server in the background
echo "Starting Fido API server on port ${PORT:-8080}..."
fido-server &
API_PID=$!

# Wait for API server to be ready with proper health check
if ! check_api_ready; then
    echo "Failed to start API server"
    kill $API_PID 2>/dev/null || true
    exit 1
fi

# Start ttyd with the TUI in web mode (uses test users only)
echo "Starting ttyd web terminal..."
FIDO_WEB_MODE=true ttyd -W -p 7681 -t 'theme={"background": "#0a0a0a"}' --client-option 'fontSize=14' --client-option 'fontFamily="Monaco, Menlo, Ubuntu Mono, Consolas, monospace"' fido &
TTYD_PID=$!

# Function to cleanup on exit
cleanup() {
    echo "Shutting down services..."
    kill $API_PID $TTYD_PID 2>/dev/null || true
    nginx -s quit 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Start nginx
echo "Starting nginx..."
nginx -g 'daemon off;'
