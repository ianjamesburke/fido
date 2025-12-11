# Fido Web Terminal Interface Startup Script (PowerShell)
# This script coordinates all services needed for the web terminal interface

param(
    [switch]$Help
)

if ($Help) {
    Write-Host "Fido Web Terminal Interface Startup Script"
    Write-Host ""
    Write-Host "Usage: .\start.ps1"
    Write-Host ""
    Write-Host "This script starts all required services:"
    Write-Host "  - Fido API Server (port 3000)"
    Write-Host "  - ttyd Terminal Service (port 7681)"
    Write-Host "  - nginx Web Server (port 8080)"
    Write-Host ""
    Write-Host "Prerequisites:"
    Write-Host "  - Rust/Cargo installed"
    Write-Host "  - nginx installed and in PATH"
    Write-Host "  - ttyd installed and in PATH"
    Write-Host ""
    Write-Host "Press Ctrl+C to stop all services"
    exit 0
}

# Configuration
$API_PORT = 3000
$NGINX_PORT = 8080
$TTYD_PORT = 7681
$FIDO_SERVER_DIR = "fido-server"

Write-Host "🐕 Starting Fido Web Terminal Interface..." -ForegroundColor Green
Write-Host "📊 Port Configuration:" -ForegroundColor Cyan
Write-Host "   - API Server: $API_PORT" -ForegroundColor White
Write-Host "   - Web Interface (nginx): $NGINX_PORT" -ForegroundColor White
Write-Host "   - Terminal (ttyd): $TTYD_PORT" -ForegroundColor White
Write-Host ""

# Function to check if a port is in use
function Test-Port {
    param([int]$Port)
    
    try {
        $connection = New-Object System.Net.Sockets.TcpClient
        $connection.Connect("localhost", $Port)
        $connection.Close()
        return $true
    }
    catch {
        return $false
    }
}

# Function to wait for service to be ready
function Wait-ForService {
    param(
        [int]$Port,
        [string]$ServiceName,
        [int]$MaxAttempts = 30
    )
    
    Write-Host "⏳ Waiting for $ServiceName to start on port $Port..." -ForegroundColor Yellow
    
    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        try {
            $response = Invoke-WebRequest -Uri "http://localhost:$Port" -TimeoutSec 1 -ErrorAction SilentlyContinue
            Write-Host "✅ $ServiceName is ready on port $Port" -ForegroundColor Green
            return $true
        }
        catch {
            if ($attempt % 5 -eq 0) {
                Write-Host "   Still waiting for $ServiceName... (attempt $attempt/$MaxAttempts)" -ForegroundColor Yellow
            }
            Start-Sleep -Seconds 1
        }
    }
    
    Write-Host "❌ $ServiceName failed to start within $MaxAttempts seconds" -ForegroundColor Red
    return $false
}

# Function to cleanup processes on exit
function Stop-Services {
    Write-Host ""
    Write-Host "🛑 Shutting down services..." -ForegroundColor Yellow
    
    if ($script:ApiProcess -and !$script:ApiProcess.HasExited) {
        Write-Host "   Stopping API server..." -ForegroundColor White
        $script:ApiProcess.Kill()
        $script:ApiProcess.WaitForExit(5000)
    }
    
    if ($script:TtydProcess -and !$script:TtydProcess.HasExited) {
        Write-Host "   Stopping ttyd..." -ForegroundColor White
        $script:TtydProcess.Kill()
        $script:TtydProcess.WaitForExit(5000)
    }
    
    if ($script:NginxProcess -and !$script:NginxProcess.HasExited) {
        Write-Host "   Stopping nginx..." -ForegroundColor White
        $script:NginxProcess.Kill()
        $script:NginxProcess.WaitForExit(5000)
    }
    
    Write-Host "✅ All services stopped" -ForegroundColor Green
}

# Set up cleanup on exit
$script:ApiProcess = $null
$script:TtydProcess = $null
$script:NginxProcess = $null

# Register cleanup function
Register-EngineEvent -SourceIdentifier PowerShell.Exiting -Action { Stop-Services }

try {
    # Check prerequisites
    Write-Host "🔍 Checking prerequisites..." -ForegroundColor Cyan
    
    if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "❌ cargo is not installed. Please install Rust." -ForegroundColor Red
        exit 1
    }
    
    # Check for nginx (local copy or system installation)
    $nginxPath = $null
    if (Test-Path ".\nginx.exe") {
        $nginxPath = ".\nginx.exe"
    } elseif (Get-Command nginx -ErrorAction SilentlyContinue) {
        $nginxPath = "nginx"
    } else {
        Write-Host "❌ nginx is not installed. Please install nginx." -ForegroundColor Red
        Write-Host "   On Windows: choco install nginx (run as administrator)" -ForegroundColor Yellow
        Write-Host "   Or download from: http://nginx.org/en/download.html" -ForegroundColor Yellow
        exit 1
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
    
    # Check if ports are available
    Write-Host "🔍 Checking port availability..." -ForegroundColor Cyan
    
    if (Test-Port $API_PORT) {
        Write-Host "⚠️  Port $API_PORT is already in use" -ForegroundColor Red
        exit 1
    }
    
    if (Test-Port $NGINX_PORT) {
        Write-Host "⚠️  Port $NGINX_PORT is already in use" -ForegroundColor Red
        exit 1
    }
    
    if (Test-Port $TTYD_PORT) {
        Write-Host "⚠️  Port $TTYD_PORT is already in use" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "✅ All prerequisites met and ports available" -ForegroundColor Green
    Write-Host ""
    
    # Start API server
    Write-Host "🚀 Starting Fido API server on port $API_PORT..." -ForegroundColor Green
    $script:ApiProcess = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "fido-server" -WorkingDirectory $FIDO_SERVER_DIR -PassThru -WindowStyle Hidden
    
    # Wait for API server to be ready
    if (!(Wait-ForService $API_PORT "API server")) {
        exit 1
    }
    
    # Start ttyd with Fido TUI in web mode
    Write-Host "🚀 Starting ttyd terminal service on port $TTYD_PORT..." -ForegroundColor Green
    $env:FIDO_WEB_MODE = "true"
    $ttydArgs = @(
        "-p", $TTYD_PORT,
        "-t", "fontSize=14",
        "-t", 'theme={"background": "#0f0f0f", "foreground": "#f5f5f5"}',
        "cargo", "run", "--bin", "fido"
    )
    $script:TtydProcess = Start-Process -FilePath $ttydPath -ArgumentList $ttydArgs -PassThru -WindowStyle Hidden
    
    # Wait for ttyd to be ready
    if (!(Wait-ForService $TTYD_PORT "ttyd terminal service")) {
        exit 1
    }
    
    # Start nginx
    Write-Host "🚀 Starting nginx web server on port $NGINX_PORT..." -ForegroundColor Green
    $nginxArgs = @("-c", "$(Get-Location)\nginx.conf", "-p", "$(Get-Location)")
    $script:NginxProcess = Start-Process -FilePath $nginxPath -ArgumentList $nginxArgs -PassThru -WindowStyle Hidden
    
    # Wait for nginx to be ready
    if (!(Wait-ForService $NGINX_PORT "nginx web server")) {
        exit 1
    }
    
    Write-Host ""
    Write-Host "🎉 All services started successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📱 Access the web interface at: http://localhost:$NGINX_PORT" -ForegroundColor Cyan
    Write-Host "🖥️  Direct terminal access at: http://localhost:$TTYD_PORT" -ForegroundColor Cyan
    Write-Host "🔌 API server running at: http://localhost:$API_PORT" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Press Ctrl+C to stop all services" -ForegroundColor Yellow
    
    # Keep the script running and monitor services
    while ($true) {
        Start-Sleep -Seconds 5
        
        # Check if any service has died
        if ($script:ApiProcess.HasExited) {
            Write-Host "❌ API server has stopped unexpectedly" -ForegroundColor Red
            break
        }
        
        if ($script:TtydProcess.HasExited) {
            Write-Host "❌ ttyd service has stopped unexpectedly" -ForegroundColor Red
            break
        }
        
        if ($script:NginxProcess.HasExited) {
            Write-Host "❌ nginx service has stopped unexpectedly" -ForegroundColor Red
            break
        }
    }
}
catch {
    Write-Host "❌ An error occurred: $($_.Exception.Message)" -ForegroundColor Red
}
finally {
    Stop-Services
}