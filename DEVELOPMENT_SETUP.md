# Fido Web Terminal Interface - Development Setup

## Port Configuration
- **API Server**: 3000 (Fido backend)
- **Web Interface**: 8080 (nginx serving static files + proxying)
- **Terminal Service**: 7681 (ttyd web terminal)

## Prerequisites Status

### ✅ Completed
- **Rust/Cargo**: Installed and working
- **nginx**: Installed and configured (`nginx.conf` ready)
- **Project Build**: `cargo build` succeeds
- **API Server**: Starts successfully on port 3000
- **Web Files**: Static files ready in `web/` directory
- **Startup Scripts**: Both `start.sh` (Linux/Mac) and `start.ps1` (Windows) ready

### ❌ Pending
- **ttyd**: Needs real executable (currently placeholder)
  - Download from: https://github.com/tsl0922/ttyd/releases
  - Place as `ttyd.exe` in project root

## Quick Start (once ttyd is installed)

### Windows
```powershell
.\start.ps1
```

### Linux/Mac
```bash
./start.sh
```

## Manual Testing

### API Server
```bash
cd fido-server
cargo run --bin fido-server
# Test: curl http://localhost:3000/posts
```

### Native TUI
```bash
cargo run --bin fido
```

### nginx Configuration Test
```bash
nginx -t -c nginx.conf -p .
```

## Current Status
Task 1 is essentially complete except for ttyd installation. All other components are verified working.