//! Shared helpers for extracting request metadata from headers.

use axum::http::HeaderMap;

/// Extract the client IP address from common proxy headers.
pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(forwarded) = headers.get("X-Forwarded-For") {
        if let Ok(value) = forwarded.to_str() {
            return Some(value.split(',').next().unwrap_or(value).trim().to_string());
        }
    }

    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(value) = real_ip.to_str() {
            return Some(value.to_string());
        }
    }

    if let Some(fly_ip) = headers.get("Fly-Client-IP") {
        if let Ok(value) = fly_ip.to_str() {
            return Some(value.to_string());
        }
    }

    None
}

/// Extract the User-Agent string, if present.
pub fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}
