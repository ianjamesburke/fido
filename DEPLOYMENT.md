# Fido Deployment Architecture

## Overview

Fido uses a multi-service containerized architecture deployed on Fly.io, combining a web interface, embedded terminal, and REST API server. This document explains how all components work together.

## Architecture Components

### 1. **Nginx Reverse Proxy** (Port 8080)
- **Purpose**: Main entry point, serves static web content and routes requests
- **Listens on**: `0.0.0.0:8080` (Fly.io's internal_port)
- **Routes**:
  - `/` → Static web interface (`/var/www/html/`)
  - `/health` → API server health check
  - `/api/*` → Proxies to fido-server (port 3000)
  - `/terminal/*` → Proxies to ttyd web terminal (port 7681)

### 2. **Fido API Server** (Port 3000)
- **Purpose**: REST API backend for posts, users, authentication
- **Binary**: `fido-server` (Rust/Axum)
- **Listens on**: `0.0.0.0:3000` (internal only, accessed via nginx)
- **Database**: SQLite at `/data/fido.db` (persistent volume)
- **Environment Variables**:
  - `HOST=0.0.0.0` (bind to all interfaces)
  - `PORT=3000` (internal API port)
  - `DATABASE_PATH=/data/fido.db` (persistent storage)

### 3. **ttyd Web Terminal** (Port 7681)
- **Purpose**: Serves the Fido TUI via web browser
- **Binary**: `ttyd` (external tool) + `fido` (Rust/Ratatui TUI)
- **Listens on**: `127.0.0.1:7681` (internal only, accessed via nginx)
- **Special Mode**: `FIDO_WEB_MODE=true` (uses test data, no auth required)
- **Configuration**: Custom theme, font settings, minimal logging (`-v 0`)

### 4. **Static Web Interface**
- **Purpose**: Landing page with embedded terminal iframe
- **Location**: `/var/www/html/` (served by nginx)
- **Files**: `index.html`, `script.js`, `style.css`
- **Features**: 
  - Embedded terminal via iframe to `/terminal/`
  - Responsive design with terminal window styling
  - Installation instructions and feature showcase

## Deployment Flow

### Build Process (Multi-stage Docker)
1. **Builder Stage**: Compiles Rust workspace (`fido-server`, `fido` TUI)
2. **Runtime Stage**: Debian slim with nginx, ttyd, and compiled binaries
3. **Assets**: Copies web files, nginx config, startup script

### Startup Sequence (`start.sh`)
1. **API Server**: Starts `fido-server` in background on port 3000
2. **Health Check**: Waits for API server to respond on `/health`
3. **ttyd Terminal**: Starts web terminal with `fido` TUI in web mode
4. **Nginx**: Starts reverse proxy on port 8080 (foreground process)

### Port Configuration
- **External (Fly.io)**: Port 80/443 → Port 8080 (nginx)
- **Internal Services**:
  - nginx: `0.0.0.0:8080` (external facing)
  - fido-server: `0.0.0.0:3000` (API backend)
  - ttyd: `127.0.0.1:7681` (web terminal)

## Usage Scenarios

### 1. **Web Interface Usage**
- User visits `https://fido-social.fly.dev/`
- nginx serves static HTML with embedded terminal
- JavaScript loads terminal iframe from `/terminal/`
- ttyd serves interactive Fido TUI in browser
- TUI connects to API server for data (in web mode, uses test data)

### 2. **Direct CLI Installation**
- User runs `cargo install fido`
- Local `fido` binary connects to `https://fido-social.fly.dev/api/`
- Full authentication and real user data
- Direct terminal usage without web interface

### 3. **API Access**
- External clients can access REST API at `/api/*`
- GitHub OAuth authentication for user sessions
- SQLite database for posts, users, votes, direct messages

## Configuration Files

### `fly.toml`
- App configuration for Fly.io deployment
- Volume mount: `fido_data` → `/data` (persistent SQLite storage)
- Environment variables: `DATABASE_PATH`, `RUST_LOG`, `HOST`
- Service ports: 80/443 → 8080 (internal_port)

### `nginx.conf`
- Reverse proxy configuration
- Static file serving with caching headers
- API and terminal proxying with proper headers
- Security headers (XSS protection, content type, etc.)

### `start.sh`
- Multi-service startup orchestration
- Health checking with retry logic
- Environment variable setup
- Process management and cleanup

### `Dockerfile`
- Multi-stage build for optimized image size
- Rust compilation with dependency caching
- Runtime dependencies: nginx, ttyd, ca-certificates
- Security: non-root user creation

## Data Persistence

### SQLite Database
- **Location**: `/data/fido.db` (Fly.io volume mount)
- **Schema**: Users, posts, votes, direct messages, sessions
- **Initialization**: Auto-creates tables and seeds test data
- **Backup**: Handled by Fly.io volume snapshots

### Web Mode vs Production Mode
- **Web Mode** (`FIDO_WEB_MODE=true`): Uses test data, no authentication
- **Production Mode**: Full GitHub OAuth, real user data, session management

## Monitoring and Debugging

### Health Checks
- API server: `GET /health` (proxied through nginx)
- Startup validation: Waits for API server before starting other services
- Fly.io smoke tests: Validates port 8080 accessibility

### Logging
- **ttyd**: Minimal logging (`-v 0`) to reduce noise
- **API server**: Structured logging with configurable levels
- **nginx**: Access and error logs to `/var/log/nginx/`
- **Startup**: Detailed process startup and health check logs

### Common Issues
1. **Port binding**: Ensure nginx binds to `0.0.0.0:8080`, not `127.0.0.1:8080`
2. **API crashes**: Check database permissions and `/data` volume mount
3. **Terminal loading**: Verify ttyd process and `/terminal/` proxy configuration
4. **Static files**: Ensure web assets are copied to `/var/www/html/`

## Development vs Production

### Local Development
- Run components separately: `cargo run --bin fido-server`, `fido`, nginx
- Use different ports to avoid conflicts
- Direct database access for debugging

### Production Deployment
- Single container with all services
- Fly.io handles load balancing, SSL, and scaling
- Persistent volume for database storage
- Health checks and automatic restarts

## Future Enhancements

### Planned Improvements
- WebSocket support for real-time updates
- PostgreSQL migration for better scalability  
- External editor integration (`$EDITOR` support)
- Advanced monitoring and metrics collection

### Scaling Considerations
- Current: Single machine with SQLite (suitable for MVP)
- Future: Multi-region deployment with shared database
- Load balancing: Fly.io handles automatically
- Database: Migration path to PostgreSQL planned