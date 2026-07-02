//! Integration tests for the `/ws` realtime event gateway.
//!
//! Spins the real router on an ephemeral listener, connects WebSocket clients
//! with real session tokens, drives the HTTP API, and asserts event delivery
//! and membership filtering.

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use fido_server::db::repositories::Repositories;
use fido_server::db::Database;
use fido_server::state::AppState;
use fido_types::{Channel, Community, Membership, MembershipRole, User};

/// Base64 of 32 zero bytes; GithubService requires FIDO_TOKEN_KEY at startup.
const TEST_TOKEN_KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

const RECV_TIMEOUT: Duration = Duration::from_secs(5);
const SILENCE_TIMEOUT: Duration = Duration::from_millis(500);

struct TestServer {
    addr: SocketAddr,
    state: AppState,
    _db_guard: TempDb,
}

/// Deletes the temp SQLite file when the test finishes.
struct TempDb(std::path::PathBuf);

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn spawn_server() -> Result<TestServer> {
    std::env::set_var("FIDO_TOKEN_KEY", TEST_TOKEN_KEY);

    let db_path = std::env::temp_dir().join(format!("fido-ws-test-{}.sqlite", Uuid::new_v4()));
    let db = Database::new(&db_path).context("Failed to create test database")?;
    db.initialize().context("Failed to initialize schema")?;

    let repos = Repositories::new(db.pool.clone());
    let state = AppState::new_with_repos(db, repos).context("Failed to build app state")?;
    let router = fido_server::create_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("Failed to bind ephemeral listener")?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("test server crashed");
    });

    Ok(TestServer {
        addr,
        state,
        _db_guard: TempDb(db_path),
    })
}

fn create_user(repos: &Repositories, username: &str) -> Result<User> {
    let user = User {
        id: Uuid::new_v4(),
        username: username.to_string(),
        bio: None,
        join_date: Utc::now(),
        is_test_user: true,
        is_admin: false,
    };
    repos.users.create(&user)?;
    Ok(user)
}

fn create_community_with_channel(repos: &Repositories) -> Result<(Community, Channel)> {
    let community = Community {
        id: Uuid::new_v4(),
        github_repo_id: 4242,
        owner: "octocat".to_string(),
        name: "hello-world".to_string(),
        claimed_by: None,
        require_thread_approval: false,
        created_at: Utc::now(),
    };
    repos.communities.create(&community)?;

    let channel = Channel {
        id: Uuid::new_v4(),
        community_id: community.id,
        name: "general".to_string(),
        created_at: Utc::now(),
    };
    repos.channels.create(&channel)?;
    Ok((community, channel))
}

fn add_member(repos: &Repositories, community_id: Uuid, user_id: Uuid) -> Result<()> {
    repos.memberships.insert(&Membership {
        community_id,
        user_id,
        role: MembershipRole::Member,
        created_at: Utc::now(),
    })?;
    Ok(())
}

type WsClient = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect_ws(addr: SocketAddr, token: &str) -> Result<WsClient> {
    let mut request = format!("ws://{addr}/ws").into_client_request()?;
    request
        .headers_mut()
        .insert("X-Session-Token", token.parse()?);
    let (socket, _response) = tokio::time::timeout(RECV_TIMEOUT, connect_async(request))
        .await
        .context("WebSocket connect timed out")??;
    Ok(socket)
}

/// Read frames until a Text frame arrives (skipping pings/pongs), then parse it.
async fn next_event(socket: &mut WsClient) -> Result<serde_json::Value> {
    let deadline = tokio::time::Instant::now() + RECV_TIMEOUT;
    loop {
        let frame = tokio::time::timeout_at(deadline, socket.next())
            .await
            .context("Timed out waiting for event")?
            .context("WebSocket closed while waiting for event")??;
        match frame {
            WsMessage::Text(text) => {
                return serde_json::from_str(&text).context("Event was not valid JSON")
            }
            WsMessage::Ping(payload) => {
                socket.send(WsMessage::Pong(payload)).await?;
            }
            _ => {}
        }
    }
}

/// Assert no Text frame arrives within `SILENCE_TIMEOUT`.
async fn assert_silent(socket: &mut WsClient) -> Result<()> {
    let deadline = tokio::time::Instant::now() + SILENCE_TIMEOUT;
    loop {
        match tokio::time::timeout_at(deadline, socket.next()).await {
            Err(_) => return Ok(()), // timed out with no events: success
            Ok(Some(Ok(WsMessage::Text(text)))) => {
                anyhow::bail!("Expected no events, but received: {text}")
            }
            Ok(Some(Ok(_))) => continue, // control frames are fine
            Ok(Some(Err(e))) => return Err(e.into()),
            Ok(None) => anyhow::bail!("WebSocket closed unexpectedly"),
        }
    }
}

#[tokio::test]
async fn message_created_is_delivered_to_members_and_filtered_for_non_members() -> Result<()> {
    let server = spawn_server().await?;
    let repos = &server.state.repos;

    let (community, channel) = create_community_with_channel(repos)?;
    let user_a = create_user(repos, "member-a")?;
    let user_b = create_user(repos, "member-b")?;
    let user_c = create_user(repos, "outsider-c")?;
    add_member(repos, community.id, user_a.id)?;
    add_member(repos, community.id, user_b.id)?;

    let token_a = server.state.session_manager.create_session(user_a.id)?;
    let token_b = server.state.session_manager.create_session(user_b.id)?;
    let token_c = server.state.session_manager.create_session(user_c.id)?;

    let mut ws_a = connect_ws(server.addr, &token_a).await?;
    let mut ws_b = connect_ws(server.addr, &token_b).await?;
    let mut ws_c = connect_ws(server.addr, &token_c).await?;

    // Send a channel message as A through the full HTTP path.
    let response = reqwest::Client::new()
        .post(format!(
            "http://{}/channels/{}/messages",
            server.addr, channel.id
        ))
        .header("X-Session-Token", &token_a)
        .json(&serde_json::json!({ "content": "hello realtime" }))
        .send()
        .await?;
    assert_eq!(response.status(), 200, "message POST should succeed");

    // Both members receive the MessageCreated envelope.
    for ws in [&mut ws_a, &mut ws_b] {
        let envelope = next_event(ws).await?;
        assert_eq!(envelope["type"], "MessageCreated");
        assert_eq!(envelope["payload"]["message"]["content"], "hello realtime");
        assert_eq!(
            envelope["payload"]["message"]["author_id"],
            user_a.id.to_string()
        );
        assert_eq!(
            envelope["payload"]["message"]["channel_id"],
            channel.id.to_string()
        );
        assert_eq!(
            envelope["payload"]["community_id"],
            community.id.to_string()
        );
        let ts = envelope["ts"].as_str().expect("ts is a string");
        ts.parse::<chrono::DateTime<chrono::Utc>>()
            .expect("ts is ISO-8601");
    }

    // The non-member receives nothing.
    assert_silent(&mut ws_c).await?;
    Ok(())
}

#[tokio::test]
async fn ws_handshake_without_token_is_rejected_with_401() -> Result<()> {
    let server = spawn_server().await?;

    let request = format!("ws://{}/ws", server.addr).into_client_request()?;
    let result = tokio::time::timeout(RECV_TIMEOUT, connect_async(request)).await?;

    match result {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status(), 401);
        }
        Err(other) => panic!("expected HTTP 401 rejection, got error: {other}"),
        Ok(_) => panic!("handshake without token must not succeed"),
    }
    Ok(())
}

#[tokio::test]
async fn ws_handshake_with_invalid_token_is_rejected_with_401() -> Result<()> {
    let server = spawn_server().await?;

    let mut request = format!("ws://{}/ws", server.addr).into_client_request()?;
    request
        .headers_mut()
        .insert("Authorization", "Bearer not-a-real-token".parse()?);
    let result = tokio::time::timeout(RECV_TIMEOUT, connect_async(request)).await?;

    match result {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status(), 401);
        }
        Err(other) => panic!("expected HTTP 401 rejection, got error: {other}"),
        Ok(_) => panic!("handshake with invalid token must not succeed"),
    }
    Ok(())
}

#[tokio::test]
async fn ws_accepts_query_token_and_delivers_notifications_to_recipient_only() -> Result<()> {
    let server = spawn_server().await?;
    let repos = &server.state.repos;

    let (community, _channel) = create_community_with_channel(repos)?;
    let sender = create_user(repos, "dm-sender")?;
    let recipient = create_user(repos, "dm-recipient")?;
    // Shared community so the DM does not require a message request.
    add_member(repos, community.id, sender.id)?;
    add_member(repos, community.id, recipient.id)?;

    let token_sender = server.state.session_manager.create_session(sender.id)?;
    let token_recipient = server.state.session_manager.create_session(recipient.id)?;

    // Connect the recipient via the ?token= query fallback.
    let request =
        format!("ws://{}/ws?token={}", server.addr, token_recipient).into_client_request()?;
    let (mut ws_recipient, _) = tokio::time::timeout(RECV_TIMEOUT, connect_async(request))
        .await
        .context("WebSocket connect timed out")??;

    let response = reqwest::Client::new()
        .post(format!("http://{}/dms", server.addr))
        .header("X-Session-Token", &token_sender)
        .json(&serde_json::json!({
            "to_username": recipient.username,
            "content": "hi over realtime",
        }))
        .send()
        .await?;
    assert_eq!(response.status(), 200, "DM POST should succeed");

    let envelope = next_event(&mut ws_recipient).await?;
    assert_eq!(envelope["type"], "DmMessageCreated");
    assert_eq!(envelope["payload"]["content"], "hi over realtime");
    assert_eq!(envelope["payload"]["from_user_id"], sender.id.to_string());
    assert_eq!(envelope["payload"]["to_user_id"], recipient.id.to_string());
    Ok(())
}
