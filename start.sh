#!/bin/bash
set -e

# Start the API server in the background
echo "Starting Fido API server..."
fido-server &

# Wait for API server to be ready
sleep 2

# Start ttyd with the TUI in web mode (uses test users only)
echo "Starting ttyd web terminal..."
FIDO_WEB_MODE=true ttyd -W -p 7681 -t 'theme={"background": "#0a0a0a"}' fido &

# Start nginx
echo "Starting nginx..."
nginx -g 'daemon off;'
