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

/// Hard cap on the number of distinct rate-limit keys held in memory.
/// Prevents spoofed session tokens or IPs from growing the map without bound
/// (a DoS vector). Once reached, expired entries are pruned; if the map is
/// still full, new keys are refused (treated as rate-limited).
const MAX_TRACKED_KEYS: usize = 100_000;

/// When the map grows past this size, prune fully-expired windows opportunistically.
const SOFT_PRUNE_THRESHOLD: usize = 10_000;

/// Simple in-memory rate limiter
/// Tracks requests per key (session token or client IP) with a sliding window
#[derive(Clone)]
pub struct RateLimiter {
    // Map of rate-limit key -> (request_count, window_start)
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
    pub fn check_rate_limit(&self, key: &str) -> RateLimitInfo {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        // Opportunistically prune fully-expired windows once the map grows large,
        // so churned keys (rotated tokens / spoofed IPs) don't accumulate.
        if state.len() > SOFT_PRUNE_THRESHOLD {
            state.retain(|_, (_, start)| now.duration_since(*start) < self.window_duration);
        }

        match state.get_mut(key) {
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
                // First request from this key. Enforce the hard memory cap before
                // inserting so unbounded unique keys can't exhaust memory.
                if state.len() >= MAX_TRACKED_KEYS {
                    // Try to reclaim space by dropping fully-expired windows.
                    state.retain(|_, (_, start)| now.duration_since(*start) < self.window_duration);

                    if state.len() >= MAX_TRACKED_KEYS {
                        // Still at capacity: refuse the new key rather than grow.
                        return RateLimitInfo {
                            exceeded: true,
                            request_count: self.max_requests,
                            max_requests: self.max_requests,
                            retry_after_secs: Some(self.window_duration.as_secs()),
                        };
                    }
                }

                state.insert(key.to_string(), (1, now));
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
    // Extract the session token, if present.
    let token = request
        .headers()
        .get("X-Session-Token")
        .and_then(|v| v.to_str().ok())
        .map(|t| t.to_string());

    // Resolve the client IP once (used both as an anonymous rate-limit key and
    // for audit logging).
    let client_ip = extract_client_ip(request.headers());

    // Rate-limit key for EVERY request:
    // - authenticated: key on the session token
    // - anonymous: fall back to the client IP
    // - neither available: a single shared bucket so headerless traffic is still
    //   bounded rather than bypassing the limiter entirely.
    let rate_key = token
        .clone()
        .or_else(|| client_ip.clone())
        .unwrap_or_else(|| "anonymous:unknown".to_string());

    let rate_limit_info = limiter.check_rate_limit(&rate_key);

    if rate_limit_info.exceeded {
        // Extract client information for audit logging
        let user_agent = extract_user_agent(request.headers());

        // Try to get user ID from the session token (if authenticated)
        let user_id = token
            .as_deref()
            .and_then(|t| app_state.get_authenticated_user_id_from_token(t));

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

    Ok(next.run(request).await)
}
