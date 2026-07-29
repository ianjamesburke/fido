//! Shared helpers for extracting request metadata from headers.

use axum::http::HeaderMap;

/// Extract the client IP address from the reverse proxy's validated hop.
///
/// # Trust model
///
/// The deploy path is `client -> Railway edge -> nginx -> fido-server` (two
/// proxy hops). nginx runs the `real_ip` module against a trusted-proxy
/// allowlist (see `nginx.conf`), resolves the true client address, and forwards
/// it as `X-Real-IP` — overwriting any value the client supplied. That single
/// header is the only trustworthy source of the client IP.
///
/// Raw `X-Forwarded-For` is deliberately **not** parsed here: under two hops
/// neither end of the list is reliable (the right-most is the Railway edge, the
/// left-most is fully attacker-forgeable), and if the app port were ever
/// directly reachable, every XFF entry would be spoofable. Trusting only the
/// nginx-written `X-Real-IP` keeps IP attribution consistent for both rate
/// limiting and audit logging.
pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(value) = real_ip.to_str() {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    if let Some(fly_ip) = headers.get("Fly-Client-IP") {
        if let Ok(value) = fly_ip.to_str() {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn resolves_from_nginx_validated_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Real-IP", "203.0.113.9".parse().unwrap());
        assert_eq!(extract_client_ip(&headers).as_deref(), Some("203.0.113.9"));
    }

    #[test]
    fn forged_x_forwarded_for_does_not_change_resolved_ip() {
        // Attacker forges XFF entries; only the nginx-written X-Real-IP is trusted.
        let mut headers = HeaderMap::new();
        headers.insert("X-Real-IP", "203.0.113.9".parse().unwrap());
        headers.insert(
            "X-Forwarded-For",
            "6.6.6.6, 7.7.7.7, 8.8.8.8".parse().unwrap(),
        );
        assert_eq!(extract_client_ip(&headers).as_deref(), Some("203.0.113.9"));
    }

    #[test]
    fn forged_x_forwarded_for_alone_is_ignored() {
        // Without the nginx hop, a bare forged XFF must not be trusted.
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "6.6.6.6, 7.7.7.7".parse().unwrap());
        assert_eq!(extract_client_ip(&headers), None);
    }

    #[test]
    fn blank_x_real_ip_falls_through() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Real-IP", "   ".parse().unwrap());
        assert_eq!(extract_client_ip(&headers), None);
    }
}
