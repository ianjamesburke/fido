//! End-to-end tests for the community rewrite (stints 0005-0010).
//!
//! Each test boots the real router on an ephemeral TCP listener and drives it
//! over real HTTP with real session tokens. GitHub-dependent flows (browse,
//! claim) run against a local fixture server with recorded response shapes.

use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

use fido_server::db::repositories::Repositories;
use fido_server::db::Database;
use fido_server::state::AppState;
use fido_types::{Channel, Community, Membership, MembershipRole, User};

/// Base64 of 32 zero bytes; GithubService requires FIDO_TOKEN_KEY at startup.
const TEST_TOKEN_KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

/// AppState construction reads process-global env vars (FIDO_TOKEN_KEY,
/// GITHUB_API_BASE). Tests run in parallel, so construction is serialized.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Per-user write rate limit on DMs/channel messages is 1/sec.
const MESSAGE_RATE_LIMIT_WAIT: Duration = Duration::from_millis(1100);

struct TestServer {
    addr: SocketAddr,
    state: AppState,
    _db_guard: TempDb,
}

struct TempDb(std::path::PathBuf);

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn spawn_server(github_api_base: Option<&str>) -> Result<TestServer> {
    let db_path = std::env::temp_dir().join(format!("fido-e2e-test-{}.sqlite", Uuid::new_v4()));
    let db = Database::new(&db_path).context("Failed to create test database")?;
    db.initialize().context("Failed to initialize schema")?;

    let repos = Repositories::new(db.pool.clone());
    let state = {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("FIDO_TOKEN_KEY", TEST_TOKEN_KEY);
        match github_api_base {
            Some(base) => std::env::set_var("GITHUB_API_BASE", base),
            None => std::env::remove_var("GITHUB_API_BASE"),
        }
        let state = AppState::new_with_repos(db, repos).context("Failed to build app state")?;
        std::env::remove_var("GITHUB_API_BASE");
        state
    };
    let router = fido_server::create_router_with_security_config(
        state.clone(),
        &fido_server::security::SecurityConfig::default(),
    );

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

/// Deterministic fixture repo id so parallel joins of the same owner/name agree.
fn fixture_repo_id(owner: &str, name: &str) -> i64 {
    let mut hash: i64 = 7;
    for byte in owner.bytes().chain([b'/']).chain(name.bytes()) {
        hash = hash.wrapping_mul(31).wrapping_add(byte as i64);
    }
    hash.abs().max(2)
}

/// Local stand-in for api.github.com using recorded response shapes.
///
/// Serves the recorded octocat/Hello-World fixtures exactly, resolves any
/// other /repos/:owner/:name with a deterministic id, and 404s owner "ghost"
/// to exercise the unresolvable-repo path.
async fn github_fixture_server() -> Result<String> {
    use axum::{
        extract::Path,
        http::StatusCode,
        response::{IntoResponse, Response},
        routing::get,
        Json, Router,
    };

    async fn starred() -> Json<Value> {
        Json(json!([
            {
                "id": 1296269,
                "name": "Hello-World",
                "full_name": "octocat/Hello-World",
                "private": false,
                "owner": { "login": "octocat" }
            }
        ]))
    }

    async fn repo() -> Json<Value> {
        Json(json!({
            "id": 1296269,
            "name": "Hello-World",
            "full_name": "octocat/Hello-World",
            "private": false,
            "owner": { "login": "octocat" },
            "permissions": {
                "admin": true,
                "maintain": true,
                "push": true,
                "triage": true,
                "pull": true
            }
        }))
    }

    async fn contributors() -> Json<Value> {
        Json(json!([
            { "login": "alice", "id": 1 },
            { "login": "octocat", "id": 583231 }
        ]))
    }

    async fn dynamic_repo(Path((owner, name)): Path<(String, String)>) -> Response {
        if owner == "ghost" {
            return StatusCode::NOT_FOUND.into_response();
        }
        Json(json!({
            "id": fixture_repo_id(&owner, &name),
            "name": name,
            "full_name": format!("{}/{}", owner, name),
            "private": false,
            "owner": { "login": owner }
        }))
        .into_response()
    }

    async fn dynamic_contributors() -> Json<Value> {
        Json(json!([]))
    }

    let app = Router::new()
        .route("/user/starred", get(starred))
        .route("/repos/octocat/Hello-World", get(repo))
        .route("/repos/octocat/Hello-World/contributors", get(contributors))
        .route("/repos/:owner/:name", get(dynamic_repo))
        .route(
            "/repos/:owner/:name/contributors",
            get(dynamic_contributors),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("fixture server");
    });
    Ok(format!("http://{}", addr))
}

struct Client {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl Client {
    fn new(addr: SocketAddr, token: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base: format!("http://{}", addr),
            token: token.to_string(),
        }
    }

    async fn get(&self, path: &str) -> Result<reqwest::Response> {
        Ok(self
            .http
            .get(format!("{}{}", self.base, path))
            .header("X-Session-Token", &self.token)
            .send()
            .await?)
    }

    async fn post(&self, path: &str, body: &Value) -> Result<reqwest::Response> {
        Ok(self
            .http
            .post(format!("{}{}", self.base, path))
            .header("X-Session-Token", &self.token)
            .json(body)
            .send()
            .await?)
    }

    async fn delete(&self, path: &str) -> Result<reqwest::Response> {
        Ok(self
            .http
            .delete(format!("{}{}", self.base, path))
            .header("X-Session-Token", &self.token)
            .send()
            .await?)
    }
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

fn login(server: &TestServer, user: &User) -> Result<Client> {
    let token = server.state.session_manager.create_session(user.id)?;
    Ok(Client::new(server.addr, &token))
}

fn seed_community(
    repos: &Repositories,
    require_thread_approval: bool,
) -> Result<(Community, Channel)> {
    let community = Community {
        id: Uuid::new_v4(),
        github_repo_id: rand_repo_id(),
        owner: "octocat".to_string(),
        name: format!("repo-{}", Uuid::new_v4().simple()),
        claimed_by: None,
        require_thread_approval,
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

fn rand_repo_id() -> i64 {
    // Derive a positive pseudo-random id from a UUID so parallel tests never collide.
    (Uuid::new_v4().as_u128() % i64::MAX as u128) as i64
}

fn add_member(
    repos: &Repositories,
    community_id: Uuid,
    user_id: Uuid,
    role: MembershipRole,
) -> Result<()> {
    repos.memberships.insert(&Membership {
        community_id,
        user_id,
        role,
        created_at: Utc::now(),
    })?;
    Ok(())
}

async fn json_body(response: reqwest::Response) -> Result<Value> {
    Ok(response.json().await?)
}

// ---------------------------------------------------------------------------
// Stint 0005: communities API
// ---------------------------------------------------------------------------

#[tokio::test]
async fn communities_join_list_detail_leave_e2e() -> Result<()> {
    let fixture_base = github_fixture_server().await?;
    let server = spawn_server(Some(&fixture_base)).await?;
    let repos = &server.state.repos;

    let alice = create_user(repos, "e2e-alice")?;
    let bob = create_user(repos, "e2e-bob")?;
    let alice_client = login(&server, &alice)?;
    let bob_client = login(&server, &bob)?;

    // Unauthenticated requests are rejected.
    let anon = reqwest::Client::new()
        .get(format!("http://{}/communities", server.addr))
        .send()
        .await?;
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    // Join resolves the repo via GitHub and lazily creates the community
    // with a default general channel.
    let join_body = json!({
        "owner": "octocat",
        "name": "lazy-created-repo",
    });
    let response = alice_client.post("/communities/join", &join_body).await?;
    assert_eq!(response.status(), StatusCode::OK, "join should succeed");
    let view = json_body(response).await?;
    let community_id = view["community"]["id"].as_str().unwrap().to_string();
    assert_eq!(
        view["community"]["github_repo_id"],
        fixture_repo_id("octocat", "lazy-created-repo")
    );
    assert_eq!(view["membership"]["role"], "member");
    assert_eq!(view["member_count"], 1);
    assert_eq!(view["channels"][0]["name"], "general");

    // Second user joining the same repo reuses the community.
    let response = bob_client.post("/communities/join", &join_body).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let view = json_body(response).await?;
    assert_eq!(view["community"]["id"].as_str().unwrap(), community_id);
    assert_eq!(view["member_count"], 2);

    // Rejoining is idempotent.
    let response = alice_client.post("/communities/join", &join_body).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let view = json_body(response).await?;
    assert_eq!(view["member_count"], 2);

    // Invalid repo names are rejected.
    let response = alice_client
        .post("/communities/join", &json!({ "owner": "a/b", "name": "x" }))
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // A repo GitHub can't resolve is a 404, not a lazily created community.
    let response = alice_client
        .post(
            "/communities/join",
            &json!({ "owner": "ghost", "name": "no-such-repo" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // List shows the joined community.
    let listed = json_body(alice_client.get("/communities").await?).await?;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["community"]["id"].as_str().unwrap(), community_id);

    // Detail view works by id.
    let detail = json_body(
        alice_client
            .get(&format!("/communities/{}", community_id))
            .await?,
    )
    .await?;
    assert_eq!(detail["member_count"], 2);

    // Leave removes the membership.
    let response = alice_client
        .delete(&format!("/communities/{}/membership", community_id))
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let listed = json_body(alice_client.get("/communities").await?).await?;
    assert!(listed.as_array().unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn communities_browse_and_claim_e2e() -> Result<()> {
    let fixture_base = github_fixture_server().await?;
    let server = spawn_server(Some(&fixture_base)).await?;
    let repos = &server.state.repos;

    let alice = create_user(repos, "alice")?;
    let client = login(&server, &alice)?;
    server
        .state
        .github_service
        .store_token(alice.id, "gho_recorded_fixture")?;

    // Browse lists starred repos with no community state yet.
    let browse = json_body(client.get("/communities/browse").await?).await?;
    assert_eq!(browse.as_array().unwrap().len(), 1);
    assert_eq!(browse[0]["full_name"], "octocat/Hello-World");
    assert!(browse[0]["community"].is_null());
    assert!(browse[0]["membership"].is_null());

    // Join assigns the contributor role from the contributors fixture.
    let response = client
        .post(
            "/communities/join",
            &json!({ "owner": "octocat", "name": "Hello-World" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let view = json_body(response).await?;
    assert_eq!(view["membership"]["role"], "contributor");
    let community_id = view["community"]["id"].as_str().unwrap().to_string();

    // Browse now reflects the joined community.
    let browse = json_body(client.get("/communities/browse").await?).await?;
    assert!(browse[0]["community"].is_object());
    assert_eq!(browse[0]["membership"]["role"], "contributor");

    // Claim verifies admin permission via GitHub and grants admin.
    let response = client
        .post(&format!("/communities/{}/claim", community_id), &json!({}))
        .await?;
    assert_eq!(response.status(), StatusCode::OK, "claim should succeed");
    let view = json_body(response).await?;
    assert_eq!(view["community"]["claimed_by"], alice.id.to_string());
    assert_eq!(view["membership"]["role"], "admin");

    Ok(())
}

// ---------------------------------------------------------------------------
// Stint 0006: channel chat API
// ---------------------------------------------------------------------------

#[tokio::test]
async fn chat_channel_messages_e2e() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    let (community, channel) = seed_community(repos, false)?;
    let member = create_user(repos, "chat-member")?;
    let other_member = create_user(repos, "chat-other")?;
    let outsider = create_user(repos, "chat-outsider")?;
    add_member(repos, community.id, member.id, MembershipRole::Member)?;
    add_member(repos, community.id, other_member.id, MembershipRole::Member)?;

    let member_client = login(&server, &member)?;
    let other_client = login(&server, &other_member)?;
    let outsider_client = login(&server, &outsider)?;

    // Members list channels; outsiders are rejected.
    let channels = json_body(
        member_client
            .get(&format!("/communities/{}/channels", community.id))
            .await?,
    )
    .await?;
    assert_eq!(channels.as_array().unwrap().len(), 1);
    assert_eq!(channels[0]["name"], "general");

    let response = outsider_client
        .get(&format!("/communities/{}/channels", community.id))
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Members send; outsiders cannot.
    let messages_path = format!("/channels/{}/messages", channel.id);
    let response = member_client
        .post(&messages_path, &json!({ "content": "first message" }))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let first = json_body(response).await?;
    assert_eq!(first["content"], "first message");
    assert_eq!(first["author_id"], member.id.to_string());

    let response = outsider_client
        .post(&messages_path, &json!({ "content": "sneaking in" }))
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Sending twice within a second trips the per-user message rate limit.
    let response = member_client
        .post(&messages_path, &json!({ "content": "too fast" }))
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // A different user is not affected by the first user's limit.
    let response = other_client
        .post(&messages_path, &json!({ "content": "second message" }))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let second = json_body(response).await?;

    // History returns both messages; outsiders get 403.
    let history = json_body(member_client.get(&messages_path).await?).await?;
    assert_eq!(history.as_array().unwrap().len(), 2);

    let response = outsider_client.get(&messages_path).await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Cursor pagination: messages strictly before the second message.
    let before = json_body(
        member_client
            .get(&format!(
                "{}?before={}",
                messages_path,
                second["id"].as_str().unwrap()
            ))
            .await?,
    )
    .await?;
    assert_eq!(before.as_array().unwrap().len(), 1);
    assert_eq!(before[0]["content"], "first message");

    // before and after together are rejected.
    let response = member_client
        .get(&format!(
            "{}?before={}&after={}",
            messages_path,
            Uuid::new_v4(),
            Uuid::new_v4()
        ))
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Empty content is rejected by validation.
    tokio::time::sleep(MESSAGE_RATE_LIMIT_WAIT).await;
    let response = member_client
        .post(&messages_path, &json!({ "content": "" }))
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

// ---------------------------------------------------------------------------
// Stint 0007: community threads board
// ---------------------------------------------------------------------------

#[tokio::test]
async fn threads_board_scoped_and_approval_e2e() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    let (community_a, _) = seed_community(repos, false)?;
    let (community_b, _) = seed_community(repos, false)?;
    let author = create_user(repos, "thread-author")?;
    let replier = create_user(repos, "thread-replier")?;
    let outsider = create_user(repos, "thread-outsider")?;
    add_member(repos, community_a.id, author.id, MembershipRole::Member)?;
    add_member(repos, community_a.id, replier.id, MembershipRole::Member)?;
    add_member(repos, community_b.id, outsider.id, MembershipRole::Member)?;

    let author_client = login(&server, &author)?;
    let replier_client = login(&server, &replier)?;
    let outsider_client = login(&server, &outsider)?;

    // Member creates a thread in community A.
    let response = author_client
        .post(
            "/posts",
            &json!({ "community_id": community_a.id, "content": "hello threads" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let post = json_body(response).await?;
    let post_id = post["id"].as_str().unwrap().to_string();
    assert_eq!(post["approved"], true);

    // Non-member cannot post into community A.
    let response = outsider_client
        .post(
            "/posts",
            &json!({ "community_id": community_a.id, "content": "outsider post" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Feed is scoped: members see it, non-members are rejected.
    let feed = json_body(
        author_client
            .get(&format!("/posts?community_id={}", community_a.id))
            .await?,
    )
    .await?;
    assert_eq!(feed.as_array().unwrap().len(), 1);

    let response = outsider_client
        .get(&format!("/posts?community_id={}", community_a.id))
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Community B's feed does not leak community A's post.
    let feed_b = json_body(
        outsider_client
            .get(&format!("/posts?community_id={}", community_b.id))
            .await?,
    )
    .await?;
    assert!(feed_b.as_array().unwrap().is_empty());

    // Reply and vote flow.
    let response = replier_client
        .post(
            &format!("/posts/{}/reply", post_id),
            &json!({ "content": "nice thread" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = replier_client
        .post(
            &format!("/posts/{}/vote", post_id),
            &json!({ "direction": "up" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let thread = json_body(
        author_client
            .get(&format!("/posts/{}/thread", post_id))
            .await?,
    )
    .await?;
    assert_eq!(thread["post"]["upvotes"], 1);
    assert_eq!(thread["replies"].as_array().unwrap().len(), 1);

    // Outsider cannot read the thread.
    let response = outsider_client
        .get(&format!("/posts/{}/thread", post_id))
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    Ok(())
}

#[tokio::test]
async fn thread_approval_queue_e2e() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    let (community, _) = seed_community(repos, true)?;
    let author = create_user(repos, "approval-author")?;
    let admin = create_user(repos, "approval-admin")?;
    let member = create_user(repos, "approval-member")?;
    add_member(repos, community.id, author.id, MembershipRole::Member)?;
    add_member(repos, community.id, admin.id, MembershipRole::Admin)?;
    add_member(repos, community.id, member.id, MembershipRole::Member)?;

    let author_client = login(&server, &author)?;
    let admin_client = login(&server, &admin)?;
    let member_client = login(&server, &member)?;

    // Posting into an approval-gated community creates an unapproved thread.
    let response = author_client
        .post(
            "/posts",
            &json!({ "community_id": community.id, "content": "awaiting approval" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let post = json_body(response).await?;
    let post_id = post["id"].as_str().unwrap().to_string();
    assert_eq!(post["approved"], false);

    // The pending thread is hidden from the public feed.
    let feed = json_body(
        member_client
            .get(&format!("/posts?community_id={}", community.id))
            .await?,
    )
    .await?;
    assert!(feed.as_array().unwrap().is_empty());

    // Only admins can see the pending queue.
    let pending_path = format!("/communities/{}/posts/pending", community.id);
    let response = member_client.get(&pending_path).await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let pending = json_body(admin_client.get(&pending_path).await?).await?;
    assert_eq!(pending.as_array().unwrap().len(), 1);
    assert_eq!(pending[0]["id"].as_str().unwrap(), post_id);

    // Only admins can approve.
    let approve_path = format!("/posts/{}/approve", post_id);
    let response = member_client.post(&approve_path, &json!({})).await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = admin_client.post(&approve_path, &json!({})).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let approved = json_body(response).await?;
    assert_eq!(approved["approved"], true);

    // The approved thread now appears in the feed and the queue is empty.
    let feed = json_body(
        member_client
            .get(&format!("/posts?community_id={}", community.id))
            .await?,
    )
    .await?;
    assert_eq!(feed.as_array().unwrap().len(), 1);
    let pending = json_body(admin_client.get(&pending_path).await?).await?;
    assert!(pending.as_array().unwrap().is_empty());

    // The author is notified their thread was approved.
    let notifications = json_body(author_client.get("/notifications").await?).await?;
    assert!(notifications
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["type"] == "thread_approved" || n["type"] == "ThreadApproved"));

    Ok(())
}

// ---------------------------------------------------------------------------
// Stint 0008: DM message requests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dm_message_requests_e2e() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    let stranger = create_user(repos, "dm-stranger")?;
    let target = create_user(repos, "dm-target")?;
    let declined_sender = create_user(repos, "dm-declined")?;

    let stranger_client = login(&server, &stranger)?;
    let target_client = login(&server, &target)?;
    let declined_client = login(&server, &declined_sender)?;

    // A DM between strangers creates a pending request.
    let response = stranger_client
        .post(
            "/dms",
            &json!({ "to_username": target.username, "content": "hello stranger" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let requests = json_body(target_client.get("/dms/requests").await?).await?;
    assert_eq!(requests.as_array().unwrap().len(), 1);

    // A second message while the request is pending is rejected.
    tokio::time::sleep(MESSAGE_RATE_LIMIT_WAIT).await;
    let response = stranger_client
        .post(
            "/dms",
            &json!({ "to_username": target.username, "content": "hello again" }),
        )
        .await?;
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "second pending message must be rejected"
    );

    // Accepting the request opens the conversation both ways.
    let response = target_client
        .post(&format!("/dms/requests/{}/accept", stranger.id), &json!({}))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let requests = json_body(target_client.get("/dms/requests").await?).await?;
    assert!(requests.as_array().unwrap().is_empty());

    let response = stranger_client
        .post(
            "/dms",
            &json!({ "to_username": target.username, "content": "accepted now" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let conversation = json_body(
        target_client
            .get(&format!("/dms/conversations/{}", stranger.id))
            .await?,
    )
    .await?;
    assert_eq!(conversation.as_array().unwrap().len(), 2);

    // Declining a request blocks further messages from that sender.
    let response = declined_client
        .post(
            "/dms",
            &json!({ "to_username": target.username, "content": "please respond" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let response = target_client
        .post(
            &format!("/dms/requests/{}/decline", declined_sender.id),
            &json!({}),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    tokio::time::sleep(MESSAGE_RATE_LIMIT_WAIT).await;
    let response = declined_client
        .post(
            "/dms",
            &json!({ "to_username": target.username, "content": "ignoring your decline" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    Ok(())
}

#[tokio::test]
async fn dm_shared_community_auto_accepts_e2e() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    let (community, _) = seed_community(repos, false)?;
    let sender = create_user(repos, "dm-neighbor-a")?;
    let recipient = create_user(repos, "dm-neighbor-b")?;
    add_member(repos, community.id, sender.id, MembershipRole::Member)?;
    add_member(repos, community.id, recipient.id, MembershipRole::Member)?;

    let sender_client = login(&server, &sender)?;
    let recipient_client = login(&server, &recipient)?;

    // Shared community members skip the request state entirely.
    let response = sender_client
        .post(
            "/dms",
            &json!({ "to_username": recipient.username, "content": "hi neighbor" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let requests = json_body(recipient_client.get("/dms/requests").await?).await?;
    assert!(
        requests.as_array().unwrap().is_empty(),
        "shared-community DM must not create a request"
    );

    // The recipient can reply immediately.
    let response = recipient_client
        .post(
            "/dms",
            &json!({ "to_username": sender.username, "content": "hi back" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}

// ---------------------------------------------------------------------------
// Stint 0009: notifications
// ---------------------------------------------------------------------------

#[tokio::test]
async fn notifications_fanout_and_read_e2e() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    let (community, _) = seed_community(repos, false)?;
    let author = create_user(repos, "notif-author")?;
    let replier = create_user(repos, "notif-replier")?;
    let mentioned = create_user(repos, "notif-mentioned")?;
    let stranger = create_user(repos, "notif-stranger")?;
    add_member(repos, community.id, author.id, MembershipRole::Member)?;
    add_member(repos, community.id, replier.id, MembershipRole::Member)?;
    add_member(repos, community.id, mentioned.id, MembershipRole::Member)?;

    let author_client = login(&server, &author)?;
    let replier_client = login(&server, &replier)?;
    let mentioned_client = login(&server, &mentioned)?;
    let stranger_client = login(&server, &stranger)?;

    // A post mentioning a member notifies them.
    let response = author_client
        .post(
            "/posts",
            &json!({
                "community_id": community.id,
                "content": "ping @notif-mentioned check this out"
            }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let post = json_body(response).await?;
    let post_id = post["id"].as_str().unwrap().to_string();

    let notifications = json_body(mentioned_client.get("/notifications").await?).await?;
    let mention = notifications
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["type"] == "mention" || n["type"] == "Mention")
        .expect("mention notification exists")
        .clone();
    assert_eq!(mention["actor_id"], author.id.to_string());
    assert_eq!(mention["read"], false);

    // A reply notifies the thread author.
    let response = replier_client
        .post(
            &format!("/posts/{}/reply", post_id),
            &json!({ "content": "replying to notify you" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let notifications = json_body(author_client.get("/notifications").await?).await?;
    assert!(notifications
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["type"] == "reply" || n["type"] == "Reply"));

    // A stranger DM produces a dm_request notification.
    let response = stranger_client
        .post(
            "/dms",
            &json!({ "to_username": author.username, "content": "dm request incoming" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let notifications = json_body(author_client.get("/notifications").await?).await?;
    assert!(notifications
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["type"] == "dm_request" || n["type"] == "DmRequest"));

    // Unread counts reflect pending notifications, then mark-read clears them.
    let counts = json_body(mentioned_client.get("/notifications/unread-count").await?).await?;
    assert!(!counts.as_array().unwrap().is_empty());

    let response = mentioned_client
        .post(
            "/notifications/mark-read",
            &json!({ "notification_id": mention["id"] }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let counts = json_body(mentioned_client.get("/notifications/unread-count").await?).await?;
    assert!(counts.as_array().unwrap().is_empty());

    // mark-read with all=true clears everything for the author.
    let response = author_client
        .post("/notifications/mark-read", &json!({ "all": true }))
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let counts = json_body(author_client.get("/notifications/unread-count").await?).await?;
    assert!(counts.as_array().unwrap().is_empty());

    // Validation: bad limit and conflicting mark-read payloads are rejected.
    let response = author_client.get("/notifications?limit=0").await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = author_client
        .post(
            "/notifications/mark-read",
            &json!({ "notification_id": Uuid::new_v4(), "all": true }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

// ---------------------------------------------------------------------------
// Community badge SVG endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn community_badge_is_public_and_reflects_member_count() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    // Unknown repo 404s, no auth header sent.
    let unknown = reqwest::Client::new()
        .get(format!(
            "http://{}/badge/octocat/does-not-exist.svg",
            server.addr
        ))
        .send()
        .await?;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    let content_type = unknown
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(content_type.starts_with("text/plain"));

    // Seed a community with two members.
    let (community, _channel) = seed_community(repos, false)?;
    let alice = create_user(repos, "e2e-badge-alice")?;
    let bob = create_user(repos, "e2e-badge-bob")?;
    add_member(repos, community.id, alice.id, MembershipRole::Admin)?;
    add_member(repos, community.id, bob.id, MembershipRole::Member)?;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{}/badge/{}/{}.svg",
            server.addr, community.owner, community.name
        ))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("image/svg+xml")
    );
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("public, max-age=300")
    );
    let body = response.text().await?;
    assert!(body.contains("2 members"));

    Ok(())
}

// ---------------------------------------------------------------------------
// Stint 0025: authenticate user search + profile-read endpoints
// ---------------------------------------------------------------------------

#[tokio::test]
async fn user_search_and_profile_read_require_auth() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;
    let alice = create_user(repos, "e2e-auth-alice")?;

    let anon = reqwest::Client::new();
    let base = format!("http://{}", server.addr);

    // Anonymous user search is rejected.
    let search = anon
        .get(format!("{}/users/search?q=e2e", base))
        .send()
        .await?;
    assert_eq!(search.status(), StatusCode::UNAUTHORIZED);

    // Anonymous profile read is rejected.
    let profile = anon
        .get(format!("{}/users/{}/profile", base, alice.id))
        .send()
        .await?;
    assert_eq!(profile.status(), StatusCode::UNAUTHORIZED);

    // With a valid session token both succeed.
    let client = login(&server, &alice)?;
    assert_eq!(
        client.get("/users/search?q=e2e").await?.status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .get(&format!("/users/{}/profile", alice.id))
            .await?
            .status(),
        StatusCode::OK
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Stint 0032: enforce membership on community roster/metadata reads
// ---------------------------------------------------------------------------

#[tokio::test]
async fn community_detail_and_roster_are_member_only() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    let (community, _channel) = seed_community(repos, false)?;
    let member = create_user(repos, "roster-member")?;
    let outsider = create_user(repos, "roster-outsider")?;
    add_member(repos, community.id, member.id, MembershipRole::Member)?;

    let member_client = login(&server, &member)?;
    let outsider_client = login(&server, &outsider)?;

    let detail = format!("/communities/{}", community.id);
    let roster = format!("/communities/{}/members", community.id);

    // Members get 200 on both.
    assert_eq!(member_client.get(&detail).await?.status(), StatusCode::OK);
    assert_eq!(member_client.get(&roster).await?.status(), StatusCode::OK);

    // Non-members are forbidden on both.
    assert_eq!(
        outsider_client.get(&detail).await?.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        outsider_client.get(&roster).await?.status(),
        StatusCode::FORBIDDEN
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Stint 0033: reject self-votes + scope mention notifications to members
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_vote_is_rejected_and_mentions_are_member_scoped() -> Result<()> {
    let server = spawn_server(None).await?;
    let repos = &server.state.repos;

    let (community, _channel) = seed_community(repos, false)?;
    let author = create_user(repos, "vote-author")?;
    let member = create_user(repos, "mention-member")?;
    let outsider = create_user(repos, "mention-outsider")?;
    add_member(repos, community.id, author.id, MembershipRole::Member)?;
    add_member(repos, community.id, member.id, MembershipRole::Member)?;
    // `outsider` is a real user but NOT a member of this community.

    let author_client = login(&server, &author)?;

    // Author creates a thread mentioning both a member and a non-member.
    let response = author_client
        .post(
            "/posts",
            &json!({
                "community_id": community.id,
                "content": "hey @mention-member and @mention-outsider"
            }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let post = json_body(response).await?;
    let post_id = post["id"].as_str().unwrap().to_string();

    // Self-vote is rejected.
    let response = author_client
        .post(
            &format!("/posts/{}/vote", post_id),
            &json!({ "direction": "up" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // The member mention produces a notification; the non-member's does not.
    let member_notifs = json_body(login(&server, &member)?.get("/notifications").await?).await?;
    assert_eq!(
        member_notifs.as_array().map(|a| a.len()).unwrap_or(0),
        1,
        "member should receive the mention notification"
    );

    let outsider_notifs =
        json_body(login(&server, &outsider)?.get("/notifications").await?).await?;
    assert!(
        outsider_notifs.as_array().map(|a| a.is_empty()).unwrap_or(true),
        "non-member must not receive a mention notification"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Stint 0037: case-insensitive GitHub contributor matching
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contributor_role_matches_github_login_case_insensitively() -> Result<()> {
    let fixture_base = github_fixture_server().await?;
    let server = spawn_server(Some(&fixture_base)).await?;
    let repos = &server.state.repos;

    // The contributors fixture lists "alice"; this user's login differs only in
    // case, so a case-insensitive match must grant the Contributor role.
    let alice = create_user(repos, "ALICE")?;
    let client = login(&server, &alice)?;

    let response = client
        .post(
            "/communities/join",
            &json!({ "owner": "octocat", "name": "Hello-World" }),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK, "join should succeed");
    let view = json_body(response).await?;
    assert_eq!(view["membership"]["role"], "contributor");
    Ok(())
}
