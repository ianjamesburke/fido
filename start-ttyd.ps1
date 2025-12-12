# Fido ttyd Terminal Service Startup Script
# Starts ttyd with optimized configuration for Fido web terminal

param(
    [int]$Port = 7681,
    [switch]$Help
)

if ($Help) {
    Write-Host "Fido ttyd Terminal Service Startup Script"
    Write-Host ""
    Write-Host "Usage: .\start-ttyd.ps1 [-Port <port>]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Port <port>    Port to run ttyd on (default: 7681)"
    Write-Host "  -Help           Show this help message"
    Write-Host ""
    Write-Host "This script starts ttyd with:"
    Write-Host "  - Modern dark theme optimized for developers"
    Write-Host "  - Monospace font configuration"
    Write-Host "  - Fido TUI in web mode (FIDO_WEB_MODE=true)"
    Write-Host "  - Proper WebSocket configuration"
    exit 0
}

# Check for ttyd (local copy or system installation)
$ttydPath = $null
if (Test-Path ".\ttyd.exe") {
    # Check if it's a real ttyd executable (not the placeholder)
    $ttydSize = (Get-Item ".\ttyd.exe").Length
    if ($ttydSize -gt 1000) {  # Real ttyd should be much larger than 9 bytes
        $ttydPath = ".\ttyd.exe"
    } else {
        Write-Host "❌ ttyd placeholder found. Please download real ttyd.exe." -ForegroundColor Red
        Write-Host "   Download from: https://github.com/tsl0922/ttyd/releases" -ForegroundColor Yellow
        Write-Host "   Place the real ttyd.exe in this directory" -ForegroundColor Yellow
        exit 1
    }
} elseif (Get-Command ttyd -ErrorAction SilentlyContinue) {
    $ttydPath = "ttyd"
} else {
    Write-Host "❌ ttyd is not installed. Please install ttyd." -ForegroundColor Red
    Write-Host "   Download from: https://github.com/tsl0922/ttyd/releases" -ForegroundColor Yellow
    exit 1
}

Write-Host "Starting ttyd terminal service on port $Port..." -ForegroundColor Green
Write-Host "Theme: Modern dark theme with GitHub Dark colors" -ForegroundColor Cyan
Write-Host "Font: Monospace (Consolas, Monaco, Courier New)" -ForegroundColor Cyan
Write-Host "Mode: Web terminal (FIDO_WEB_MODE=true)" -ForegroundColor Cyan
Write-Host ""

# Set environment variable for web mode
$env:FIDO_WEB_MODE = "true"

# ttyd arguments with modern dark theme and optimal settings
$ttydArgs = @(
    "-p", $Port,
    "-t", "fontSize=16",
    "-t", "fontFamily=Consolas,Monaco,'Courier New',monospace",
    "-t", "cursorBlink=true",
    "-t", "cursorStyle=block",
    "-t", "scrollback=1000",
    "-t", 'theme={"background": "#0d1117", "foreground": "#f0f6fc", "cursor": "#f0f6fc", "cursorAccent": "#0d1117", "selection": "#264f78", "black": "#484f58", "red": "#ff7b72", "green": "#7ee787", "yellow": "#ffa657", "blue": "#79c0ff", "magenta": "#bc8cff", "cyan": "#39c5cf", "white": "#b1bac4", "brightBlack": "#6e7681", "brightRed": "#ffa198", "brightGreen": "#56d364", "brightYellow": "#ffdf5d", "brightBlue": "#a5b4fc", "brightMagenta": "#d2a8ff", "brightCyan": "#56d4dd", "brightWhite": "#f0f6fc"}',
    "-W",  # Allow write access
    "cargo", "run", "--bin", "fido"
)

Write-Host "Starting with arguments:" -ForegroundColor Yellow
Write-Host "   Port: $Port" -ForegroundColor White
Write-Host "   Font Size: 16px" -ForegroundColor White
Write-Host "   Font Family: Consolas, Monaco, Courier New, monospace" -ForegroundColor White
Write-Host "   Cursor: Block with blink" -ForegroundColor White
Write-Host "   Scrollback: 1000 lines" -ForegroundColor White
Write-Host "   Theme: GitHub Dark" -ForegroundColor White
Write-Host "   Write Access: Enabled" -ForegroundColor White
Write-Host ""

try {
    # Start ttyd
    & $ttydPath @ttydArgs
}
catch {
    Write-Host "Failed to start ttyd: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}