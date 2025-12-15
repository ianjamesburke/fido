#!/bin/bash

# Fido Web Terminal Interface Startup Script
# This script coordinates all services needed for the web terminal interface

set -e

# Configuration
API_PORT=3000
NGINX_PORT=8080
TTYD_PORT=7681
FIDO_SERVER_DIR="fido-server"

echo "🐕 Starting Fido Web Terminal Interface..."
echo "📊 Port Configuration:"
echo "   - API Server: $API_PORT"
echo "   - Web Interface (nginx): $NGINX_PORT"
echo "   - Terminal (ttyd): $TTYD_PORT"
echo ""

# Function to check if a port is in use
check_port() {
    local port=$1
    if netstat -an | grep -q ":$port "; then
        echo "⚠️  Port $port is already in use"
        return 1
    fi
    return 0
}

# Function to wait for service to be ready with health checks
wait_for_service() {
    local port=$1
    local service_name=$2
    local health_endpoint=$3
    local max_attempts=30
    local attempt=1
    
    echo "⏳ Waiting for $service_name to start on port $port..."
    
    while [ $attempt -le $max_attempts ]; do
        # Try basic connection first
        if curl -s "http://localhost:$port" > /dev/null 2>&1; then
            # If health endpoint is provided, test it specifically
            if [ -n "$health_endpoint" ]; then
                if curl -s "http://localhost:$port$health_endpoint" | grep -q "OK\|healthy"; then
                    echo "✅ $service_name is ready on port $port (health check passed)"
                    return 0
                fi
            else
                echo "✅ $service_name is ready on port $port"
                return 0
            fi
        fi
        
        if [ $((attempt % 5)) -eq 0 ]; then
            echo "   Still waiting for $service_name... (attempt $attempt/$max_attempts)"
        fi
        
        sleep 1
        attempt=$((attempt + 1))
    done
    
    echo "❌ $service_name failed to start within $max_attempts seconds"
    return 1
}

# Function to perform comprehensive health checks
health_check_all_services() {
    local all_healthy=true
    
    echo "🔍 Performing health checks..."
    
    # Check API server health
    if curl -s "http://localhost:$API_PORT/health" | grep -q "OK"; then
        echo "✅ API server health check passed"
    else
        echo "❌ API server health check failed"
        all_healthy=false
    fi
    
    # Check nginx health
    if curl -s "http://localhost:$NGINX_PORT/health" | grep -q "healthy"; then
        echo "✅ nginx health check passed"
    else
        echo "❌ nginx health check failed"
        all_healthy=false
    fi
    
    # Check ttyd health (basic connection test)
    if curl -s "http://localhost:$TTYD_PORT" > /dev/null 2>&1; then
        echo "✅ ttyd service health check passed"
    else
        echo "❌ ttyd service health check failed"
        all_healthy=false
    fi
    
    if [ "$all_healthy" = true ]; then
        echo "✅ All services are healthy"
        return 0
    else
        echo "⚠️  Some services failed health checks"
        return 1
    fi
}

# Function to cleanup processes on exit
cleanup() {
    echo ""
    echo "🛑 Shutting down services..."
    
    # Kill background processes
    if [ ! -z "$API_PID" ]; then
        echo "   Stopping API server (PID: $API_PID)..."
        kill $API_PID 2>/dev/null || true
    fi
    
    if [ ! -z "$TTYD_PID" ]; then
        echo "   Stopping ttyd (PID: $TTYD_PID)..."
        kill $TTYD_PID 2>/dev/null || true
    fi
    
    if [ ! -z "$NGINX_PID" ]; then
        echo "   Stopping nginx (PID: $NGINX_PID)..."
        kill $NGINX_PID 2>/dev/null || true
    fi
    
    echo "✅ All services stopped"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Check prerequisites
echo "🔍 Checking prerequisites..."

if ! command -v cargo &> /dev/null; then
    echo "❌ cargo is not installed. Please install Rust."
    exit 1
fi

if ! command -v nginx &> /dev/null; then
    echo "❌ nginx is not installed. Please install nginx."
    echo "   On Ubuntu/Debian: sudo apt-get install nginx"
    echo "   On macOS: brew install nginx"
    echo "   On Windows: choco install nginx (run as administrator)"
    exit 1
fi

if ! command -v ttyd &> /dev/null; then
    echo "❌ ttyd is not installed. Please install ttyd."
    echo "   On Ubuntu/Debian: sudo apt-get install ttyd"
    echo "   On macOS: brew install ttyd"
    echo "   On Windows: Download from https://github.com/tsl0922/ttyd/releases"
    exit 1
fi

# Check if ports are available
echo "🔍 Checking port availability..."
check_port $API_PORT || exit 1
check_port $NGINX_PORT || exit 1
check_port $TTYD_PORT || exit 1

echo "✅ All prerequisites met and ports available"
echo ""

# Start API server
echo "🚀 Starting Fido API server on port $API_PORT..."
cd "$FIDO_SERVER_DIR"
cargo run --bin fido-server &
API_PID=$!
cd ..

# Wait for API server to be ready
wait_for_service $API_PORT "API server" "/health" || exit 1

# Start ttyd with Fido TUI in web mode
echo "🚀 Starting ttyd terminal service on port $TTYD_PORT..."
FIDO_WEB_MODE=true ttyd \
    -p $TTYD_PORT \
    -t fontSize=16 \
    -t fontFamily="Consolas,Monaco,'Courier New',monospace" \
    -t cursorBlink=true \
    -t cursorStyle=block \
    -t scrollback=1000 \
    -t theme='{"background": "#0d1117", "foreground": "#f0f6fc", "cursor": "#f0f6fc", "cursorAccent": "#0d1117", "selection": "#264f78", "black": "#484f58", "red": "#ff7b72", "green": "#7ee787", "yellow": "#ffa657", "blue": "#79c0ff", "magenta": "#bc8cff", "cyan": "#39c5cf", "white": "#b1bac4", "brightBlack": "#6e7681", "brightRed": "#ffa198", "brightGreen": "#56d364", "brightYellow": "#ffdf5d", "brightBlue": "#a5b4fc", "brightMagenta": "#d2a8ff", "brightCyan": "#56d4dd", "brightWhite": "#f0f6fc"}' \
    -W \
    cargo run --bin fido &
TTYD_PID=$!

# Wait for ttyd to be ready
wait_for_service $TTYD_PORT "ttyd terminal service" "" || exit 1

# Start nginx
echo "🚀 Starting nginx web server on port $NGINX_PORT..."
nginx -c "$(pwd)/nginx.conf" -p "$(pwd)" &
NGINX_PID=$!

# Wait for nginx to be ready
wait_for_service $NGINX_PORT "nginx web server" "/health" || exit 1

echo ""
echo "🎉 All services started successfully!"

# Perform comprehensive health checks
health_check_all_services

echo ""
echo "📱 Access the web interface at: http://localhost:$NGINX_PORT"
echo "🖥️  Direct terminal access at: http://localhost:$TTYD_PORT"
echo "🔌 API server running at: http://localhost:$API_PORT"
echo ""
echo "Press Ctrl+C to stop all services"

# Keep the script running and monitor services
health_check_interval=30
last_health_check=0

while true; do
    sleep 5
    
    # Check if any service has died
    if ! kill -0 $API_PID 2>/dev/null; then
        echo "❌ API server has stopped unexpectedly"
        cleanup
    fi
    
    if ! kill -0 $TTYD_PID 2>/dev/null; then
        echo "❌ ttyd service has stopped unexpectedly"
        cleanup
    fi
    
    if ! kill -0 $NGINX_PID 2>/dev/null; then
        echo "❌ nginx service has stopped unexpectedly"
        cleanup
    fi
    
    # Periodic health checks
    current_time=$(date +%s)
    if [ $((current_time - last_health_check)) -ge $health_check_interval ]; then
        if ! health_check_all_services > /dev/null 2>&1; then
            echo "⚠️  Health check failed - some services may be unhealthy"
        fi
        last_health_check=$current_time
    fi
done