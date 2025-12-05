#!/bin/bash

# Fido Local Development Runner
# This script starts the server and TUI client for local testing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Fido Local Development Runner${NC}"
echo "================================"
echo ""

# Check if we're in the fido directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Please run this script from the fido directory${NC}"
    exit 1
fi

# Check if server is already running
if lsof -Pi :3000 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${YELLOW}Server already running on port 3000${NC}"
    echo "Using existing server..."
    SERVER_RUNNING=true
else
    # Check if GitHub OAuth credentials are set in .env
    if [ -f "fido-server/.env" ]; then
        if ! grep -q "GITHUB_CLIENT_ID=your_github_client_id_here" fido-server/.env 2>/dev/null; then
            echo -e "${GREEN}✓ GitHub OAuth credentials configured${NC}"
        else
            echo -e "${YELLOW}⚠ GitHub OAuth not configured${NC}"
            echo "  To test GitHub login, see LOCAL_GITHUB_OAUTH_SETUP.md"
            echo "  You can still use test users (alice, bob, charlie)"
        fi
    fi
    
    echo ""
    echo "Starting local server..."
    cargo run --bin fido-server &
    SERVER_PID=$!
    SERVER_RUNNING=false
    
    # Wait for server to start
    echo "Waiting for server to initialize..."
    sleep 3
    
    # Check if server started successfully
    if ! lsof -Pi :3000 -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo -e "${RED}Error: Server failed to start${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Server started on http://localhost:3000${NC}"
fi

echo ""
echo "Starting TUI client..."
echo "Press Ctrl+C to exit"
echo ""

# Run the TUI client with local server URL
FIDO_SERVER_URL=http://localhost:3000 cargo run --bin fido

# Clean up: kill the server if we started it
if [ "$SERVER_RUNNING" = false ]; then
    echo ""
    echo "Stopping server..."
    kill $SERVER_PID 2>/dev/null || true
fi

echo -e "${GREEN}Done!${NC}"
