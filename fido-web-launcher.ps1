# Fido Web Launcher Script
# This script is called by ttyd and handles session token passing from URL parameters

param(
    [string]$QueryString = ""
)

# Parse query string for session_token and mode
$sessionToken = ""
$mode = ""

if ($QueryString) {
    $params = $QueryString -split '&'
    foreach ($param in $params) {
        $keyValue = $param -split '='
        if ($keyValue.Length -eq 2) {
            $key = [System.Web.HttpUtility]::UrlDecode($keyValue[0])
            $value = [System.Web.HttpUtility]::UrlDecode($keyValue[1])
            
            if ($key -eq "session_token") {
                $sessionToken = $value
            } elseif ($key -eq "mode") {
                $mode = $value
            }
        }
    }
}

# Set environment variables for Fido
$env:FIDO_WEB_MODE = "true"

# If we have a session token, pass it to Fido
if ($sessionToken) {
    Write-Host "Starting Fido with GitHub session..." -ForegroundColor Green
    $env:FIDO_SESSION_TOKEN = $sessionToken
    cargo run --bin fido --quiet
} else {
    Write-Host "Starting Fido in demo mode..." -ForegroundColor Green
    cargo run --bin fido --quiet
}