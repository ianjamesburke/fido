#!/bin/bash
set -e

# Function to check if API server is ready
check_api_ready() {
    local max_attempts=${HEALTH_CHECK_ATTEMPTS:-10}
    local attempt=1
    local port=${PORT:-3000}
    
    while [ $attempt -le $max_attempts ]; do
        # Check if the process is still running
        if ! kill -0 $API_PID 2>/dev/null; then
            echo "API server process died!"
            return 1
        fi
        
        # Use wget instead of curl, and try different addresses
        if wget -q --spider "http://127.0.0.1:${port}/health" 2>/dev/null; then
            echo "API server is ready on port ${port}!"
            return 0
        elif wget -q --spider "http://localhost:${port}/health" 2>/dev/null; then
            echo "API server is ready on port ${port}!"
            return 0
        fi
        
        echo "Waiting for API server... (attempt $attempt/$max_attempts)"
        sleep 2
        ((attempt++))
    done
    
    echo "API server failed to start within timeout"
    echo "Debug: Checking if port is listening..."
    ss -tlnp 2>/dev/null | grep ":${port}" || echo "Port ${port} not listening"
    return 1
}

# Start the API server in the background
PORT=${PORT:-3000}
echo "Starting Fido API server on port ${PORT}..."
export PORT
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
FIDO_WEB_MODE=true ttyd -W -p 7681 -v 0 -t 'theme={"background": "#0a0a0a"}' --client-option 'fontSize=14' --client-option 'fontFamily="Monaco, Menlo, Ubuntu Mono, Consolas, monospace"' fido &
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
