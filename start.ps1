# Simple Fido Web Terminal Startup Script
Write-Host "🐕 Starting Fido Web Terminal Interface..." -ForegroundColor Green

# Configuration
$API_PORT = 3000
$NGINX_PORT = 8080
$TTYD_PORT = 7681

Write-Host "📊 Port Configuration:" -ForegroundColor Cyan
Write-Host "   - API Server: $API_PORT" -ForegroundColor White
Write-Host "   - Web Interface (nginx): $NGINX_PORT" -ForegroundColor White
Write-Host "   - Terminal (ttyd): $TTYD_PORT" -ForegroundColor White
Write-Host ""

# Check prerequisites
Write-Host "🔍 Checking prerequisites..." -ForegroundColor Cyan

if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ cargo is not installed. Please install Rust." -ForegroundColor Red
    exit 1
}

# Find nginx
$nginxPath = $null
if (Test-Path ".\nginx.exe") {
    $nginxPath = ".\nginx.exe"
    Write-Host "✅ Found local nginx.exe" -ForegroundColor Green
} else {
    Write-Host "❌ nginx.exe not found in current directory" -ForegroundColor Red
    Write-Host "   Download nginx and place nginx.exe in this directory" -ForegroundColor Yellow
    exit 1
}

# Find ttyd
$ttydPath = $null
if (Test-Path ".\ttyd.exe") {
    $ttydSize = (Get-Item ".\ttyd.exe").Length
    if ($ttydSize -gt 1000) {
        $ttydPath = ".\ttyd.exe"
        Write-Host "✅ Found ttyd.exe" -ForegroundColor Green
    } else {
        Write-Host "❌ ttyd.exe is placeholder. Download real ttyd.exe" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "❌ ttyd.exe not found in current directory" -ForegroundColor Red
    Write-Host "   Download from: https://github.com/tsl0922/ttyd/releases" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ All prerequisites met" -ForegroundColor Green
Write-Host ""

try {
    # Start API server
    Write-Host "🚀 Starting API server..." -ForegroundColor Green
    $apiProcess = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "fido-server" -WorkingDirectory "fido-server" -PassThru -WindowStyle Hidden
    
    # Start ttyd
    Write-Host "🚀 Starting ttyd..." -ForegroundColor Green
    $env:FIDO_WEB_MODE = "true"
    $ttydProcess = Start-Process -FilePath $ttydPath -ArgumentList "-p", $TTYD_PORT, "-W", "cargo", "run", "--bin", "fido" -PassThru -WindowStyle Hidden
    
    # Start nginx
    Write-Host "🚀 Starting nginx..." -ForegroundColor Green
    $nginxProcess = Start-Process -FilePath $nginxPath -ArgumentList "-c", "$(Get-Location)\nginx.conf", "-p", "$(Get-Location)" -PassThru -WindowStyle Hidden
    
    Start-Sleep -Seconds 3
    
    Write-Host ""
    Write-Host "🎉 All services started!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📱 Web interface: http://localhost:$NGINX_PORT" -ForegroundColor Cyan
    Write-Host "🖥️ Direct terminal: http://localhost:$TTYD_PORT" -ForegroundColor Cyan  
    Write-Host "🔌 API server: http://localhost:$API_PORT" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Press Ctrl+C to stop all services" -ForegroundColor Yellow
    
    # Wait for user to stop
    while ($true) {
        Start-Sleep -Seconds 1
    }
}
catch {
    Write-Host "❌ Error: $($_.Exception.Message)" -ForegroundColor Red
}
finally {
    Write-Host ""
    Write-Host "🛑 Stopping services..." -ForegroundColor Yellow
    
    if ($apiProcess -and !$apiProcess.HasExited) {
        $apiProcess.Kill()
    }
    if ($ttydProcess -and !$ttydProcess.HasExited) {
        $ttydProcess.Kill()
    }
    if ($nginxProcess -and !$nginxProcess.HasExited) {
        $nginxProcess.Kill()
    }
    
    Write-Host "✅ All services stopped" -ForegroundColor Green
}