#!/bin/bash
set -e

echo "=== FIDO WEB + API STARTUP ==="
echo "Environment variables:"
env | grep -E "(HOST|PORT|DATABASE|RUST)" || echo "No relevant env vars found"

# Start the API server first (we know this works)
echo "Starting fido-server on port 3000..."
/usr/local/bin/fido-server &
API_PID=$!

# Give the API server a moment to start
sleep 2

# Check if API server is running
if ! kill -0 $API_PID 2>/dev/null; then
    echo "ERROR: API server failed to start!"
    exit 1
fi

echo "API server started successfully (PID: $API_PID)"

# Start ttyd with the TUI in web mode
echo "Starting ttyd web terminal on port 7681..."
FIDO_WEB_MODE=true ttyd -W -p 7681 -v 0 -t 'theme={"background": "#0a0a0a"}' --client-option 'fontSize=14' --client-option 'fontFamily="Monaco, Menlo, Ubuntu Mono, Consolas, monospace"' fido &
TTYD_PID=$!

echo "ttyd started (PID: $TTYD_PID)"

# Function to cleanup on exit
cleanup() {
    echo "Shutting down services..."
    kill $API_PID $TTYD_PID 2>/dev/null || true
    nginx -s quit 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Start nginx (this will run in foreground)
echo "Starting nginx on port 8080..."
exec nginx -g 'daemon off;'