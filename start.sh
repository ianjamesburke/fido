#!/bin/bash
set -e

# Function to check if API server is ready
check_api_ready() {
    local max_attempts=${HEALTH_CHECK_ATTEMPTS:-15}
    local attempt=1
    local port=${API_PORT:-8080}
    
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
        elif wget -q --spider "http://0.0.0.0:${port}/health" 2>/dev/null; then
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
    echo "Server logs:"
    cat /tmp/fido-server.log 2>/dev/null || echo "No server logs found"
    return 1
}

# Start the API server in the background
API_PORT=3000
echo "Starting Fido API server on port ${API_PORT}..."
export PORT=${API_PORT}
export HOST=${HOST:-0.0.0.0}
export DATABASE_PATH=${DATABASE_PATH:-/data/fido.db}

# Ensure database directory exists and has proper permissions
mkdir -p "$(dirname "$DATABASE_PATH")"
chmod 755 "$(dirname "$DATABASE_PATH")"

# Start server with error logging
echo "Environment: HOST=${HOST}, PORT=${API_PORT}, DATABASE_PATH=${DATABASE_PATH}"



# Verify binary exists and is executable
echo "Verifying fido-server binary..."
if [ ! -f "/usr/local/bin/fido-server" ]; then
    echo "ERROR: fido-server binary not found"
    exit 1
fi

if [ ! -x "/usr/local/bin/fido-server" ]; then
    echo "ERROR: fido-server binary is not executable"
    exit 1
fi

echo "Binary verified successfully"

# Start the server directly with proper error handling
echo "Starting fido-server with environment:"
echo "  HOST=${HOST}"
echo "  PORT=${API_PORT}"
echo "  DATABASE_PATH=${DATABASE_PATH}"
echo "  RUST_LOG=${RUST_LOG:-info}"

# First try running in foreground for 2 seconds to capture immediate errors
echo "Testing server startup in foreground..."
timeout 2 /usr/local/bin/fido-server 2>&1 | tee /tmp/fido-server-test.log || {
    echo "Foreground test completed with exit code: $?"
    echo "Captured output:"
    cat /tmp/fido-server-test.log
}

# Now start server in background with full error capture
echo "Starting server in background..."
/usr/local/bin/fido-server > /tmp/fido-server.log 2>&1 &
API_PID=$!

# Give it a moment to start
sleep 3

# Check if process is still running and capture detailed error info
if ! kill -0 $API_PID 2>/dev/null; then
    echo "ERROR: fido-server process died immediately!"
    echo "Process exit status: $?"
    echo "=== FULL SERVER LOGS ==="
    cat /tmp/fido-server.log 2>/dev/null || echo "No logs captured"
    echo "=== END LOGS ==="
    
    # Additional debugging
    echo "=== DEBUGGING INFO ==="
    echo "Binary info:"
    ls -la /usr/local/bin/fido-server
    echo "Library dependencies:"
    ldd /usr/local/bin/fido-server 2>/dev/null || echo "ldd failed"
    echo "Environment:"
    env | grep -E "(HOST|PORT|DATABASE|RUST)" || echo "No relevant env vars"
    echo "=== END DEBUGGING ==="
    exit 1
fi

echo "fido-server started successfully with PID: $API_PID"

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
