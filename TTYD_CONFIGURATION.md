# ttyd Configuration for Fido Web Terminal

This document describes the ttyd configuration used for the Fido web terminal interface.

## Overview

ttyd is configured to provide a modern, developer-friendly terminal experience in the browser with:
- Dark theme optimized for long coding sessions
- Monospace fonts for proper code alignment
- ANSI color support for syntax highlighting
- Responsive WebSocket connection

## Configuration Details

### Port Configuration
- **Port**: 7681 (default)
- **Protocol**: WebSocket over HTTP
- **Access**: `http://localhost:7681`

### Theme Configuration
The terminal uses a modern dark theme based on GitHub Dark:

```json
{
  "background": "#0d1117",     // Dark background
  "foreground": "#f0f6fc",     // Light text
  "cursor": "#f0f6fc",         // Visible cursor
  "cursorAccent": "#0d1117",   // Cursor accent
  "selection": "#264f78",      // Selection highlight
  "black": "#484f58",          // ANSI black
  "red": "#ff7b72",            // ANSI red
  "green": "#7ee787",          // ANSI green
  "yellow": "#ffa657",         // ANSI yellow
  "blue": "#79c0ff",           // ANSI blue
  "magenta": "#bc8cff",        // ANSI magenta
  "cyan": "#39c5cf",           // ANSI cyan
  "white": "#b1bac4",          // ANSI white
  "brightBlack": "#6e7681",    // ANSI bright black
  "brightRed": "#ffa198",      // ANSI bright red
  "brightGreen": "#56d364",    // ANSI bright green
  "brightYellow": "#ffdf5d",   // ANSI bright yellow
  "brightBlue": "#a5b4fc",     // ANSI bright blue
  "brightMagenta": "#d2a8ff",  // ANSI bright magenta
  "brightCyan": "#56d4dd",     // ANSI bright cyan
  "brightWhite": "#f0f6fc"     // ANSI bright white
}
```

### Font Configuration
- **Font Size**: 16px (optimal for readability)
- **Font Family**: `Consolas,Monaco,'Courier New',monospace`
- **Fallback**: System monospace fonts

### Terminal Behavior
- **Cursor Style**: Block cursor with blinking
- **Scrollback**: 1000 lines of history
- **Write Access**: Enabled (`-W` flag)
- **True Color**: Supported for rich color display

### Environment Variables
- **FIDO_WEB_MODE**: Set to `true` to enable web mode in Fido TUI
- **TERM**: Automatically set by ttyd for proper terminal emulation

## Usage

### Direct ttyd Startup
Use the dedicated startup scripts for optimal configuration:

**Windows (PowerShell):**
```powershell
.\start-ttyd.ps1
```

**Linux/macOS (Bash):**
```bash
./start-ttyd.sh
```

### Custom Port
To run on a different port:

**Windows:**
```powershell
.\start-ttyd.ps1 -Port 8080
```

**Linux/macOS:**
```bash
./start-ttyd.sh -p 8080
```

### Manual ttyd Command
For manual startup with full configuration:

```bash
FIDO_WEB_MODE=true ttyd \
    -p 7681 \
    -t fontSize=16 \
    -t fontFamily="Consolas,Monaco,'Courier New',monospace" \
    -t cursorBlink=true \
    -t cursorStyle=block \
    -t scrollback=1000 \
    -t theme='{"background": "#0d1117", "foreground": "#f0f6fc", ...}' \
    -W \
    cargo run --bin fido
```

## Integration with Fido

### Web Mode Detection
When ttyd starts Fido with `FIDO_WEB_MODE=true`, the TUI:
- Uses browser storage instead of file storage
- Adapts authentication flow for web environment
- Maintains identical functionality to native mode

### Keyboard Shortcuts
All Fido keyboard shortcuts work identically in the web terminal:
- Navigation: Arrow keys, Tab, Shift+Tab
- Actions: Enter, Escape, Ctrl+C
- Application-specific shortcuts as defined in Fido TUI

### Performance Considerations
- WebSocket connection provides low-latency input
- Terminal rendering optimized for web browsers
- Scrollback limited to prevent memory issues
- True color support for rich visual experience

## Troubleshooting

### Common Issues

**ttyd not found:**
- Install ttyd from https://github.com/tsl0922/ttyd/releases
- Ensure ttyd is in PATH or use local executable

**Port already in use:**
- Check if another service is using port 7681
- Use a different port with `-p` option

**Terminal not loading:**
- Verify ttyd is running: `curl http://localhost:7681`
- Check browser console for WebSocket errors
- Ensure firewall allows connections to port 7681

**Keyboard input not working:**
- Click inside the terminal to focus
- Check browser compatibility (Chrome, Firefox, Safari supported)
- Verify WebSocket connection is established

### Browser Compatibility
- **Chrome**: Full support
- **Firefox**: Full support
- **Safari**: Full support
- **Edge**: Full support
- **Mobile browsers**: Limited support (touch input)

## Security Considerations

- ttyd runs with write access (`-W`) for full terminal functionality
- Terminal session is local to the machine running ttyd
- No authentication required for local development
- Production deployments should add authentication layers

## Performance Tuning

### For High-Latency Connections
- Reduce scrollback: `-t scrollback=500`
- Disable cursor blink: `-t cursorBlink=false`

### For Low-Memory Systems
- Reduce scrollback: `-t scrollback=100`
- Use smaller font: `-t fontSize=14`

### For High-DPI Displays
- Increase font size: `-t fontSize=18`
- Adjust theme colors for better contrast