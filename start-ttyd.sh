#!/bin/bash

# Fido ttyd Terminal Service Startup Script
# Starts ttyd with optimized configuration for Fido web terminal

set -e

# Default configuration
PORT=7681

# Help function
show_help() {
    echo "Fido ttyd Terminal Service Startup Script"
    echo ""
    echo "Usage: ./start-ttyd.sh [-p <port>] [-h]"
    echo ""
    echo "Options:"
    echo "  -p <port>    Port to run ttyd on (default: 7681)"
    echo "  -h           Show this help message"
    echo ""
    echo "This script starts ttyd with:"
    echo "  - Modern dark theme optimized for developers"
    echo "  - Monospace font configuration"
    echo "  - Fido TUI in web mode (FIDO_WEB_MODE=true)"
    echo "  - Proper WebSocket configuration"
}

# Parse command line arguments
while getopts "p:h" opt; do
    case $opt in
        p)
            PORT=$OPTARG
            ;;
        h)
            show_help
            exit 0
            ;;
        \?)
            echo "Invalid option: -$OPTARG" >&2
            show_help
            exit 1
            ;;
    esac
done

# Check for ttyd
if ! command -v ttyd &> /dev/null; then
    echo "❌ ttyd is not installed. Please install ttyd."
    echo "   On Ubuntu/Debian: sudo apt-get install ttyd"
    echo "   On macOS: brew install ttyd"
    echo "   On Windows: Download from https://github.com/tsl0922/ttyd/releases"
    exit 1
fi

echo "🚀 Starting ttyd terminal service on port $PORT..."
echo "🎨 Theme: Modern dark theme with GitHub Dark colors"
echo "🔤 Font: Monospace (Consolas, Monaco, Courier New)"
echo "🌐 Mode: Web terminal (FIDO_WEB_MODE=true)"
echo ""

echo "🔧 Starting with configuration:"
echo "   Port: $PORT"
echo "   Font Size: 16px"
echo "   Font Family: Consolas, Monaco, Courier New, monospace"
echo "   Cursor: Block with blink"
echo "   Scrollback: 1000 lines"
echo "   Theme: GitHub Dark"
echo "   Write Access: Enabled"
echo ""

# Set environment variable for web mode
export FIDO_WEB_MODE=true

# Start ttyd with modern dark theme and optimal settings
exec ttyd \
    -p $PORT \
    -t fontSize=16 \
    -t fontFamily="Consolas,Monaco,'Courier New',monospace" \
    -t cursorBlink=true \
    -t cursorStyle=block \
    -t scrollback=1000 \
    -t theme='{"background": "#0d1117", "foreground": "#f0f6fc", "cursor": "#f0f6fc", "cursorAccent": "#0d1117", "selection": "#264f78", "black": "#484f58", "red": "#ff7b72", "green": "#7ee787", "yellow": "#ffa657", "blue": "#79c0ff", "magenta": "#bc8cff", "cyan": "#39c5cf", "white": "#b1bac4", "brightBlack": "#6e7681", "brightRed": "#ffa198", "brightGreen": "#56d364", "brightYellow": "#ffdf5d", "brightBlue": "#a5b4fc", "brightMagenta": "#d2a8ff", "brightCyan": "#56d4dd", "brightWhite": "#f0f6fc"}' \
    -W \
    cargo run --bin fido