//! Shared helpers for extracting request metadata from headers.

use axum::http::HeaderMap;

/// Extract the client IP address from common proxy headers.
///
/// # Trust model
///
/// This assumes the app runs behind a **single trusted reverse proxy**
/// (nginx / Railway). `X-Forwarded-For` is a comma-separated list where each
/// hop appends the address it received the connection from. The entry appended
/// by our immediate proxy is the **right-most** one, so we take that rather
/// than the left-most, which is fully attacker-controlled and forgeable.
///
/// If additional trusted proxies are ever introduced, this single-hop
/// assumption must be revisited (e.g. a trusted-proxy CIDR allowlist).
pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(forwarded) = headers.get("X-Forwarded-For") {
        if let Ok(value) = forwarded.to_str() {
            // Take the right-most entry: the hop added by our trusted proxy.
            if let Some(last) = value.split(',').next_back().map(str::trim) {
                if !last.is_empty() {
                    return Some(last.to_string());
                }
            }
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
