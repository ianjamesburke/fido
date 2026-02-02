use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::http::{extract_client_ip, extract_user_agent};
use crate::security::{AuditEvent, AuditEventType};
use crate::state::AppState;

/// Simple in-memory rate limiter
/// Tracks requests per session token with a sliding window
#[derive(Clone)]
pub struct RateLimiter {
    // Map of session_token -> (request_count, window_start)
    state: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    max_requests: u32,
    window_duration: Duration,
}

/// Information about a rate limit check result
pub struct RateLimitInfo {
    /// Whether the rate limit was exceeded
    pub exceeded: bool,
    /// Current request count in the window
    pub request_count: u32,
    /// Maximum allowed requests
    pub max_requests: u32,
    /// Seconds remaining until window resets (if exceeded)
    pub retry_after_secs: Option<u64>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }

    /// Check if a request should be allowed
    /// Returns detailed information about the rate limit status
    pub fn check_rate_limit(&self, token: &str) -> RateLimitInfo {
        let mut state = self.state.lock().unwrap();
        let now = Instant::now();

        // Clean up old entries periodically (simple cleanup)
        if state.len() > 10000 {
            state.retain(|_, (_, start)| now.duration_since(*start) < self.window_duration * 2);
        }

        match state.get_mut(token) {
            Some((count, window_start)) => {
                // Check if we're still in the same window
                if now.duration_since(*window_start) < self.window_duration {
                    if *count >= self.max_requests {
                        let remaining = self.window_duration - now.duration_since(*window_start);
                        return RateLimitInfo {
                            exceeded: true,
                            request_count: *count,
                            max_requests: self.max_requests,
                            retry_after_secs: Some(remaining.as_secs()),
                        };
                    }
                    *count += 1;
                    RateLimitInfo {
                        exceeded: false,
                        request_count: *count,
                        max_requests: self.max_requests,
                        retry_after_secs: None,
                    }
                } else {
                    // New window
                    *window_start = now;
                    *count = 1;
                    RateLimitInfo {
                        exceeded: false,
                        request_count: 1,
                        max_requests: self.max_requests,
                        retry_after_secs: None,
                    }
                }
            }
            None => {
                // First request from this token
                state.insert(token.to_string(), (1, now));
                RateLimitInfo {
                    exceeded: false,
                    request_count: 1,
                    max_requests: self.max_requests,
                    retry_after_secs: None,
                }
            }
        }
    }
}

/// Middleware to apply rate limiting to all requests
/// Logs rate limit exceeded events to the audit log
pub async fn rate_limit_middleware(
    State(app_state): State<AppState>,
    axum::Extension(limiter): axum::Extension<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract session token from headers
    let token = request
        .headers()
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok());

    // Only rate limit authenticated requests
    if let Some(token) = token {
        let rate_limit_info = limiter.check_rate_limit(token);

        if rate_limit_info.exceeded {
            // Extract client information for audit logging
            let client_ip = extract_client_ip(request.headers());
            let user_agent = extract_user_agent(request.headers());

            // Try to get user ID from the session token
            let user_id = app_state.get_authenticated_user_id_from_token(token);

            // Build details about the rate limit event
            let details = format!(
                "Rate limit exceeded: {}/{} requests in window. Retry after {} seconds. Path: {}",
                rate_limit_info.request_count,
                rate_limit_info.max_requests,
                rate_limit_info.retry_after_secs.unwrap_or(0),
                request.uri().path()
            );

            // Log the rate limit exceeded event
            let audit_event = AuditEvent::new(AuditEventType::RateLimitExceeded)
                .with_optional_user_id(user_id)
                .with_optional_ip_address(client_ip.clone())
                .with_optional_user_agent(user_agent.clone())
                .with_details(&details);

            // Log to audit system (ignore errors to not block the request)
            if let Err(e) = app_state.audit_logger.log(audit_event) {
                tracing::warn!("Failed to log rate limit event to audit log: {}", e);
            }

            // Also log via tracing for immediate visibility
            tracing::warn!(
                ip = ?client_ip,
                user_agent = ?user_agent,
                user_id = ?user_id,
                request_count = rate_limit_info.request_count,
                max_requests = rate_limit_info.max_requests,
                retry_after = ?rate_limit_info.retry_after_secs,
                path = %request.uri().path(),
                "Rate limit exceeded"
            );

            let retry_after = rate_limit_info.retry_after_secs.unwrap_or(60);
            return Ok((
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "{{\"error\": \"Rate limit exceeded. Try again in {} seconds.\"}}",
                    retry_after
                ),
            )
                .into_response());
        }
    }

    Ok(next.run(request).await)
}
