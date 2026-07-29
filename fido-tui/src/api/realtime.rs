use std::time::Duration;

use fido_types::EventEnvelope;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::Error as WebSocketError;

use super::ApiClient;

const CHANNEL_CAPACITY: usize = 256;
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeConnectionStatus {
    Disabled,
    Connecting,
    Connected,
    Reconnecting,
    Unauthorized,
}

impl RealtimeConnectionStatus {
    pub fn label(self) -> &'static str {
        match self {
            RealtimeConnectionStatus::Disabled => "off",
            RealtimeConnectionStatus::Connecting => "connecting",
            RealtimeConnectionStatus::Connected => "live",
            RealtimeConnectionStatus::Reconnecting => "polling",
            RealtimeConnectionStatus::Unauthorized => "auth",
        }
    }

    pub fn uses_polling_fallback(self) -> bool {
        matches!(
            self,
            RealtimeConnectionStatus::Connecting | RealtimeConnectionStatus::Reconnecting
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimeStatusUpdate {
    pub status: RealtimeConnectionStatus,
    pub message: Option<String>,
}

impl RealtimeStatusUpdate {
    fn new(status: RealtimeConnectionStatus, message: impl Into<Option<String>>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl From<RealtimeConnectionStatus> for RealtimeStatusUpdate {
    fn from(status: RealtimeConnectionStatus) -> Self {
        Self {
            status,
            message: None,
        }
    }
}

#[derive(Debug)]
pub enum RealtimeClientEvent {
    Status(RealtimeStatusUpdate),
    Event(Box<EventEnvelope>),
    RefetchRequired,
}

enum ConnectionOutcome {
    Retry(String),
    Unauthorized(String),
    ReceiverDropped,
}

pub fn spawn_realtime_task(
    api_client: ApiClient,
) -> (JoinHandle<()>, mpsc::Receiver<RealtimeClientEvent>) {
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let handle = tokio::spawn(async move {
        run_realtime_client(api_client, tx).await;
    });
    (handle, rx)
}

async fn run_realtime_client(api_client: ApiClient, tx: mpsc::Sender<RealtimeClientEvent>) {
    let Some(token) = api_client.session_token().map(ToOwned::to_owned) else {
        let _ = tx
            .send(RealtimeClientEvent::Status(RealtimeStatusUpdate::new(
                RealtimeConnectionStatus::Disabled,
                Some("missing session token".to_string()),
            )))
            .await;
        return;
    };

    let websocket_url = api_client.websocket_url();
    let mut attempt = 0_u32;

    loop {
        let status = if attempt == 0 {
            RealtimeConnectionStatus::Connecting
        } else {
            RealtimeConnectionStatus::Reconnecting
        };
        if tx
            .send(RealtimeClientEvent::Status(RealtimeStatusUpdate::new(
                status, None,
            )))
            .await
            .is_err()
        {
            return;
        }

        match connect_and_read(&websocket_url, &token, &tx).await {
            ConnectionOutcome::ReceiverDropped => return,
            ConnectionOutcome::Unauthorized(message) => {
                let _ = tx
                    .send(RealtimeClientEvent::Status(RealtimeStatusUpdate::new(
                        RealtimeConnectionStatus::Unauthorized,
                        Some(message),
                    )))
                    .await;
                return;
            }
            ConnectionOutcome::Retry(message) => {
                let _ = tx.send(RealtimeClientEvent::RefetchRequired).await;
                if tx
                    .send(RealtimeClientEvent::Status(RealtimeStatusUpdate::new(
                        RealtimeConnectionStatus::Reconnecting,
                        Some(message),
                    )))
                    .await
                    .is_err()
                {
                    return;
                }
                tokio::time::sleep(reconnect_delay(attempt)).await;
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

/// The token may only travel over `wss://` or a loopback `ws://` origin.
fn ws_transport_is_safe(url: &str) -> bool {
    let url = url.trim();
    if url.starts_with("wss://") {
        return true;
    }
    if let Some(rest) = url.strip_prefix("ws://") {
        let host = rest.split(['/', ':']).next().unwrap_or("");
        return matches!(host, "127.0.0.1" | "localhost" | "[::1]" | "::1");
    }
    false
}

async fn connect_and_read(
    websocket_url: &str,
    token: &str,
    tx: &mpsc::Sender<RealtimeClientEvent>,
) -> ConnectionOutcome {
    if !ws_transport_is_safe(websocket_url) {
        log::warn!(
            "Refusing to send session token over insecure transport ({websocket_url}); use wss:// or a loopback address"
        );
        return ConnectionOutcome::Unauthorized(
            "insecure ws:// transport; use wss:// or a loopback server".to_string(),
        );
    }

    let mut request = match websocket_url.into_client_request() {
        Ok(request) => request,
        Err(e) => return ConnectionOutcome::Retry(format!("invalid websocket url: {e}")),
    };

    let header_value = match token.parse() {
        Ok(value) => value,
        Err(e) => return ConnectionOutcome::Retry(format!("invalid session header: {e}")),
    };
    request
        .headers_mut()
        .insert("X-Session-Token", header_value);

    let (mut socket, _) = match connect_async(request).await {
        Ok(connected) => connected,
        Err(WebSocketError::Http(response)) if response.status().as_u16() == 401 => {
            return ConnectionOutcome::Unauthorized("session rejected by /ws".to_string());
        }
        Err(e) => return ConnectionOutcome::Retry(e.to_string()),
    };

    if tx
        .send(RealtimeClientEvent::Status(RealtimeStatusUpdate::new(
            RealtimeConnectionStatus::Connected,
            None,
        )))
        .await
        .is_err()
    {
        return ConnectionOutcome::ReceiverDropped;
    }

    loop {
        let Some(frame) = socket.next().await else {
            return ConnectionOutcome::Retry("websocket closed".to_string());
        };

        match frame {
            Ok(Message::Text(text)) => match serde_json::from_str::<EventEnvelope>(&text) {
                Ok(envelope) => {
                    if tx
                        .send(RealtimeClientEvent::Event(Box::new(envelope)))
                        .await
                        .is_err()
                    {
                        return ConnectionOutcome::ReceiverDropped;
                    }
                }
                Err(e) => {
                    log::warn!("Ignoring invalid realtime event envelope: {}", e);
                }
            },
            Ok(Message::Ping(payload)) => {
                if let Err(e) = socket.send(Message::Pong(payload)).await {
                    return ConnectionOutcome::Retry(e.to_string());
                }
            }
            Ok(Message::Close(frame)) => {
                let reason = frame
                    .map(|f| f.reason.to_string())
                    .filter(|reason| !reason.is_empty())
                    .unwrap_or_else(|| "websocket closed".to_string());
                return ConnectionOutcome::Retry(reason);
            }
            Ok(Message::Pong(_)) | Ok(Message::Binary(_)) | Ok(Message::Frame(_)) => {}
            Err(e) => return ConnectionOutcome::Retry(e.to_string()),
        }
    }
}

pub(crate) fn reconnect_delay(attempt: u32) -> Duration {
    let multiplier = 2_u32.saturating_pow(attempt).min(30);
    (INITIAL_RECONNECT_DELAY * multiplier).min(MAX_RECONNECT_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_delay_caps_at_maximum() {
        assert_eq!(reconnect_delay(0), Duration::from_secs(1));
        assert_eq!(reconnect_delay(1), Duration::from_secs(2));
        assert_eq!(reconnect_delay(20), Duration::from_secs(30));
    }

    #[test]
    fn websocket_url_matches_http_origin_scheme() {
        assert_eq!(
            ApiClient::new("https://example.com").websocket_url(),
            "wss://example.com/ws"
        );
        assert_eq!(
            ApiClient::new("http://127.0.0.1:3000/").websocket_url(),
            "ws://127.0.0.1:3000/ws"
        );
    }

    #[test]
    fn ws_transport_safety_allows_wss_and_loopback_only() {
        assert!(ws_transport_is_safe("wss://example.com/ws"));
        assert!(ws_transport_is_safe("ws://127.0.0.1:3000/ws"));
        assert!(ws_transport_is_safe("ws://localhost:3000/ws"));
        // Plaintext to a remote host must not carry the token.
        assert!(!ws_transport_is_safe("ws://example.com/ws"));
        assert!(!ws_transport_is_safe("ws://10.0.0.5:3000/ws"));
    }
}
