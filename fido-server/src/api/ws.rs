//! `GET /ws`: the authenticated WebSocket event gateway.
//!
//! Auth happens before the upgrade: the session token comes from
//! `Authorization: Bearer <token>`, `X-Session-Token`, or a `?token=` query
//! fallback; an invalid or missing token yields HTTP 401 with no upgrade.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use crate::api::ApiError;
use crate::realtime::RealtimeGateway;
use crate::state::AppState;

/// Interval between server pings.
const PING_INTERVAL: Duration = Duration::from_secs(30);
/// Number of consecutive unanswered pings before the connection is dropped.
const MAX_MISSED_PONGS: u8 = 2;
/// WebSocket close code for "going away" (RFC 6455 section 7.4.1).
const CLOSE_GOING_AWAY: u16 = 1001;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    #[serde(default)]
    token: Option<String>,
}

pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let token = extract_token(&headers, &query)
        .ok_or_else(|| ApiError::Unauthorized("Missing session token".to_string()))?;

    let user_id = state
        .get_authenticated_user_id_from_token(&token)
        .ok_or_else(|| ApiError::Unauthorized("Invalid session token".to_string()))?;

    let gateway = state.realtime.clone();
    Ok(ws.on_upgrade(move |socket| handle_connection(socket, user_id, gateway)))
}

/// Extract the session token: Authorization Bearer, X-Session-Token, or ?token=.
fn extract_token(headers: &HeaderMap, query: &WsQuery) -> Option<String> {
    if let Some(bearer) = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        return Some(bearer.trim().to_string());
    }
    if let Some(token) = headers.get("X-Session-Token").and_then(|v| v.to_str().ok()) {
        return Some(token.to_string());
    }
    query.token.clone()
}

async fn handle_connection(mut socket: WebSocket, user_id: Uuid, gateway: Arc<RealtimeGateway>) {
    let connections = gateway.registry().connect(user_id);
    tracing::info!(%user_id, connections, "WebSocket connected");

    let mut events = gateway.subscribe();
    let mut shutdown = gateway.shutdown_signal();
    let mut ping_interval =
        tokio::time::interval_at(tokio::time::Instant::now() + PING_INTERVAL, PING_INTERVAL);
    let mut missed_pongs: u8 = 0;

    loop {
        tokio::select! {
            outbound = events.recv() => match outbound {
                Ok(event) => {
                    if !event.is_for(&user_id) {
                        continue;
                    }
                    if let Err(e) = socket.send(Message::Text(event.json().to_string())).await {
                        tracing::debug!(%user_id, error = %e, "WebSocket send failed");
                        break;
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    // Never silently skip events: close so the client falls
                    // back to refetching (polling fallback contract).
                    tracing::warn!(%user_id, skipped, "WebSocket consumer lagged; closing");
                    send_close(&mut socket, "event stream lagged").await;
                    break;
                }
                Err(RecvError::Closed) => {
                    send_close(&mut socket, "server shutting down").await;
                    break;
                }
            },
            frame = socket.recv() => match frame {
                Some(Ok(Message::Pong(_))) => {
                    missed_pongs = 0;
                }
                Some(Ok(Message::Close(_))) | None => {
                    break;
                }
                Some(Ok(_)) => {
                    // Server -> client only in MVP; ignore other client frames.
                    // Pings are answered automatically by the protocol layer.
                }
                Some(Err(e)) => {
                    tracing::debug!(%user_id, error = %e, "WebSocket receive error");
                    break;
                }
            },
            _ = ping_interval.tick() => {
                if missed_pongs >= MAX_MISSED_PONGS {
                    tracing::info!(%user_id, "WebSocket missed {MAX_MISSED_PONGS} pongs; dropping");
                    break;
                }
                missed_pongs += 1;
                if let Err(e) = socket.send(Message::Ping(Vec::new())).await {
                    tracing::debug!(%user_id, error = %e, "WebSocket ping failed");
                    break;
                }
            },
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    send_close(&mut socket, "server shutting down").await;
                    break;
                }
            },
        }
    }

    let remaining = gateway.registry().disconnect(user_id);
    tracing::info!(%user_id, connections = remaining, "WebSocket disconnected");
}

async fn send_close(socket: &mut WebSocket, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: CLOSE_GOING_AWAY,
            reason: reason.into(),
        })))
        .await;
}
