# Fido Web Terminal Interface - Local Development Setup

This document provides instructions for setting up the Fido web terminal interface for local development.

## Overview

The web terminal interface consists of three main services:

1. **Fido API Server** (port 3000) - Backend REST API
2. **ttyd Terminal Service** (port 7681) - Web-based terminal emulator
3. **nginx Web Server** (port 8080) - Static file server and reverse proxy

## Prerequisites

### Required Software

1. **Rust and Cargo** - For building and running Fido
   - Install from: https://rustup.rs/
   - Verify: `cargo --version`

2. **nginx** - Web server and reverse proxy
   - **Windows**: `choco install nginx` (requires admin privileges)
   - **macOS**: `brew install nginx`
   - **Ubuntu/Debian**: `sudo apt-get install nginx`
   - **Manual**: Download from http://nginx.org/en/download.html

3. **ttyd** - Terminal-to-web interface
   - **Windows**: Download from https://github.com/tsl0922/ttyd/releases
   - **macOS**: `brew install ttyd`
   - **Ubuntu/Debian**: `sudo apt-get install ttyd`

### Port Requirements

Ensure the following ports are available:
- **3000** - Fido API Server
- **7681** - ttyd Terminal Service  
- **8080** - nginx Web Interface

## Quick Start

### Option 1: Automated Setup (Recommended)

**Windows (PowerShell):**
```powershell
.\start.ps1
```

**Linux/macOS (Bash):**
```bash
chmod +x start.sh
./start.sh
```

### Option 2: Manual Setup

1. **Start the API Server:**
   ```bash
   cd fido-server
   cargo run --bin fido-server
   ```

2. **Start ttyd Terminal Service:**
   ```bash
   FIDO_WEB_MODE=true ttyd -p 7681 -t fontSize=14 -t 'theme={"background": "#0f0f0f", "foreground": "#f5f5f5"}' cargo run --bin fido
   ```

3. **Start nginx Web Server:**
   ```bash
   nginx -c "$(pwd)/nginx.conf" -p "$(pwd)"
   ```

## Verification Steps

After starting all services, verify they're working:

1. **API Server Test:**
   ```bash
   curl http://localhost:3000/posts
   ```
   Should return JSON array of posts.

2. **Web Interface Test:**
   Open browser to: http://localhost:8080
   Should display the Fido web interface.

3. **Terminal Test:**
   Open browser to: http://localhost:7681
   Should display the Fido TUI in a web terminal.

4. **Integrated Test:**
   - Navigate to http://localhost:8080
   - Verify the terminal iframe loads and displays Fido TUI
   - Test keyboard input in the terminal

## Configuration Files

### nginx.conf
- Serves static files from `web/` directory
- Proxies API requests to port 3000 (no `/api` prefix)
- Proxies terminal requests (`/terminal/`) to port 7681
- Includes CORS headers for web terminal compatibility

### start.sh / start.ps1
- Automated startup scripts for all services
- Includes health checks and port availability verification
- Handles graceful shutdown on Ctrl+C
- Monitors services and restarts if needed

## Troubleshooting

### Common Issues

1. **Port Already in Use:**
   ```
   Error: Port 3000 is already in use
   ```
   - Check what's using the port: `netstat -an | grep :3000`
   - Kill the process or use different ports

2. **nginx Permission Denied:**
   ```
   Error: nginx: [alert] could not open error log file
   ```
   - Ensure `logs/` directory exists: `mkdir logs`
   - Check file permissions

3. **ttyd Not Found:**
   ```
   Error: ttyd: command not found
   ```
   - Install ttyd using package manager or download binary
   - Ensure ttyd is in your PATH

4. **Cargo Build Fails:**
   ```
   Error: could not compile fido
   ```
   - Run `cargo build` first to check for compilation errors
   - Ensure all dependencies are available

### Service-Specific Debugging

**API Server:**
- Check logs in terminal where `cargo run --bin fido-server` is running
- Verify database initialization completed successfully
- Test endpoints directly: `curl http://localhost:3000/health`

**ttyd Terminal:**
- Access directly at http://localhost:7681 to isolate issues
- Check if `FIDO_WEB_MODE` environment variable is set
- Verify Fido TUI works in native mode: `cargo run --bin fido`

**nginx:**
- Check error logs: `tail -f logs/error.log`
- Verify configuration syntax: `nginx -t -c nginx.conf`
- Test static file serving: `curl http://localhost:8080/index.html`

## Development Workflow

1. **Code Changes:**
   - API changes: Restart API server (`Ctrl+C` and restart)
   - TUI changes: Restart ttyd service
   - Web files: No restart needed (nginx serves directly)

2. **Testing:**
   - Unit tests: `cargo test`
   - Integration tests: Use the verification steps above
   - Manual testing: Use the web interface at http://localhost:8080

3. **Debugging:**
   - Enable debug logging in Fido TUI
   - Use browser developer tools for web interface issues
   - Check service logs for backend issues

## Environment Variables

- `FIDO_WEB_MODE=true` - Enables web mode for Fido TUI
- `FIDO_SERVER_URL` - API server URL (default: http://localhost:3000)
- `GITHUB_CLIENT_ID` - GitHub OAuth client ID (for authentication)
- `GITHUB_CLIENT_SECRET` - GitHub OAuth client secret (for authentication)

## Next Steps

After successful setup:

1. Configure GitHub OAuth for authentication (Task 2)
2. Implement web mode detection in Fido TUI (Task 3)
3. Add storage adapter system (Task 4)
4. Implement test user isolation (Task 5)

## Support

If you encounter issues:

1. Check this troubleshooting section
2. Verify all prerequisites are installed
3. Test each service individually
4. Check the GitHub issues for known problems
5. Create a new issue with detailed error messages and system information