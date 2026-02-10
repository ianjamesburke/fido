use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use reqwest::blocking::Client;
use reqwest::Method;
use rusqlite::Connection;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

const COLLECTION_USERS: &str = "users";
const COLLECTION_POSTS: &str = "posts";
const COLLECTION_POST_HASHTAGS: &str = "post_hashtags";
const COLLECTION_HASHTAG_ACTIVITY: &str = "hashtag_activity";
const COLLECTION_HASHTAG_FOLLOWS: &str = "hashtag_follows";
const COLLECTION_VOTES: &str = "votes";
const COLLECTION_FRIENDS: &str = "friends";
const COLLECTION_CONFIGS: &str = "configs";
const COLLECTION_RATE_LIMITS: &str = "rate_limits";
const COLLECTION_DMS: &str = "dms";
const COLLECTION_SESSIONS: &str = "sessions";
const COLLECTION_AUDIT: &str = "audit";

#[derive(Parser, Debug)]
#[command(name = "sqlite-to-firestore")]
#[command(about = "Migrate Fido SQLite data to Firestore")]
struct Args {
    /// Path to SQLite database
    #[arg(long, default_value = "./fido.db")]
    sqlite_path: String,

    /// Firestore project id (falls back to FIREBASE_PROJECT_ID / GOOGLE_CLOUD_PROJECT)
    #[arg(long)]
    project_id: Option<String>,

    /// Firestore emulator host, example 127.0.0.1:8088
    #[arg(long)]
    emulator_host: Option<String>,

    /// Firestore bearer token for non-emulator calls (or use FIRESTORE_ACCESS_TOKEN)
    #[arg(long)]
    access_token: Option<String>,

    /// Run migration without writing data
    #[arg(long)]
    dry_run: bool,

    /// Validate migrated counts after write
    #[arg(long)]
    validate: bool,
}

#[derive(Clone)]
struct FirestoreClient {
    http: Client,
    documents_base: String,
    run_query_url: String,
    access_token: Option<String>,
    emulator: bool,
}

impl FirestoreClient {
    fn new(project_id: &str, emulator_host: Option<String>, access_token: Option<String>) -> Self {
        let emulator = emulator_host.is_some();
        let (documents_base, run_query_url) = if let Some(host) = emulator_host {
            let base = format!("http://{host}/v1/projects/{project_id}/databases/(default)");
            (
                format!("{base}/documents"),
                format!("{base}/documents:runQuery"),
            )
        } else {
            let base = format!(
                "https://firestore.googleapis.com/v1/projects/{project_id}/databases/(default)"
            );
            (
                format!("{base}/documents"),
                format!("{base}/documents:runQuery"),
            )
        };

        Self {
            http: Client::new(),
            documents_base,
            run_query_url,
            access_token,
            emulator,
        }
    }

    fn with_auth(
        &self,
        req: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if self.emulator {
            return req;
        }

        match &self.access_token {
            Some(token) => req.bearer_auth(token),
            None => req,
        }
    }

    fn patch_document(
        &self,
        collection: &str,
        doc_id: &str,
        fields: Map<String, Value>,
    ) -> Result<()> {
        let url = format!("{}/{}/{}", self.documents_base, collection, doc_id);
        self.send_json(Method::PATCH, &url, Some(json!({ "fields": fields })))?;
        Ok(())
    }

    fn query_collection_count(&self, collection: &str) -> Result<usize> {
        let query = json!({
            "structuredQuery": {
                "from": [{"collectionId": collection}]
            }
        });

        let payload = self.send_json(Method::POST, &self.run_query_url, Some(query))?;
        let count = payload
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.get("document").is_some())
            .count();

        Ok(count)
    }

    fn send_json(&self, method: Method, url: &str, body: Option<Value>) -> Result<Value> {
        let mut req = self.http.request(method, url);
        if let Some(body) = body {
            req = req.json(&body);
        }

        let resp = self
            .with_auth(req)
            .send()
            .with_context(|| format!("request failed for {url}"))?;
        let status = resp.status();
        let text = resp.text().unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Firestore request failed ({status}): {text}"));
        }

        if text.trim().is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_str(&text).context("invalid Firestore response JSON")
    }
}

#[derive(Default)]
struct MigrationStats {
    source_counts: HashMap<&'static str, usize>,
    written_counts: HashMap<&'static str, usize>,
}

impl MigrationStats {
    fn set_source(&mut self, key: &'static str, value: usize) {
        self.source_counts.insert(key, value);
    }

    fn inc_written(&mut self, key: &'static str) {
        let entry = self.written_counts.entry(key).or_insert(0);
        *entry += 1;
    }
}

fn fs_string(value: &str) -> Value {
    json!({"stringValue": value})
}

fn fs_int(value: i64) -> Value {
    json!({"integerValue": value.to_string()})
}

fn fs_bool(value: bool) -> Value {
    json!({"booleanValue": value})
}

fn fs_timestamp(value: &str) -> Value {
    let ts = DateTime::parse_from_rfc3339(value)
        .map(|v| v.with_timezone(&Utc).to_rfc3339())
        .unwrap_or_else(|_| value.to_string());
    json!({"timestampValue": ts})
}

fn write_doc(
    client: &FirestoreClient,
    collection: &'static str,
    doc_id: &str,
    fields: Map<String, Value>,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    if !dry_run {
        client.patch_document(collection, doc_id, fields)?;
    }
    stats.inc_written(collection);
    Ok(())
}

fn migrate_users(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, username, bio, join_date, is_test_user, is_admin, github_id, github_login FROM users",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, Option<i64>>(6)?,
            r.get::<_, Option<String>>(7)?,
        ))
    })?;

    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_USERS, rows.len());

    for (id, username, bio, join_date, is_test_user, is_admin, github_id, github_login) in rows {
        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&id));
        fields.insert("username".into(), fs_string(&username));
        if let Some(bio) = bio {
            fields.insert("bio".into(), fs_string(&bio));
        }
        fields.insert("join_date".into(), fs_timestamp(&join_date));
        fields.insert("is_test_user".into(), fs_bool(is_test_user != 0));
        fields.insert("is_admin".into(), fs_bool(is_admin != 0));
        if let Some(github_id) = github_id {
            fields.insert("github_id".into(), fs_int(github_id));
        }
        if let Some(github_login) = github_login {
            fields.insert("github_login".into(), fs_string(&github_login));
        }

        write_doc(client, COLLECTION_USERS, &id, fields, dry_run, stats)?;
    }

    Ok(())
}

fn migrate_posts(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut hashtags_by_post: HashMap<String, Vec<String>> = HashMap::new();
    let mut hashtag_stmt = conn.prepare(
        "SELECT ph.post_id, h.name FROM post_hashtags ph JOIN hashtags h ON h.id = ph.hashtag_id",
    )?;
    let hashtag_rows =
        hashtag_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in hashtag_rows {
        let (post_id, hashtag) = row?;
        hashtags_by_post.entry(post_id).or_default().push(hashtag);
    }

    let mut reply_count: HashMap<String, i64> = HashMap::new();
    let mut reply_stmt = conn.prepare(
        "SELECT parent_post_id, COUNT(*) FROM posts WHERE parent_post_id IS NOT NULL GROUP BY parent_post_id",
    )?;
    let reply_rows =
        reply_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    for row in reply_rows {
        let (parent_id, count) = row?;
        reply_count.insert(parent_id, count);
    }

    let mut stmt = conn.prepare(
        "SELECT p.id, p.author_id, u.username, p.content, p.created_at, p.upvotes, p.downvotes, p.parent_post_id, p.reply_to_user_id, ru.username
         FROM posts p
         JOIN users u ON u.id = p.author_id
         LEFT JOIN users ru ON ru.id = p.reply_to_user_id",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, Option<String>>(7)?,
            r.get::<_, Option<String>>(8)?,
            r.get::<_, Option<String>>(9)?,
        ))
    })?;

    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_POSTS, rows.len());

    for (
        id,
        author_id,
        author_username,
        content,
        created_at,
        upvotes,
        downvotes,
        parent_post_id,
        reply_to_user_id,
        reply_to_username,
    ) in rows
    {
        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&id));
        fields.insert("author_id".into(), fs_string(&author_id));
        fields.insert("author_username".into(), fs_string(&author_username));
        fields.insert("content".into(), fs_string(&content));
        fields.insert("created_at".into(), fs_timestamp(&created_at));
        fields.insert("upvotes".into(), fs_int(upvotes));
        fields.insert("downvotes".into(), fs_int(downvotes));
        fields.insert(
            "reply_count".into(),
            fs_int(*reply_count.get(&id).unwrap_or(&0)),
        );

        let hashtags = hashtags_by_post.get(&id).cloned().unwrap_or_default();
        let hashtag_values = hashtags.iter().map(|h| fs_string(h)).collect::<Vec<_>>();
        fields.insert(
            "hashtags".into(),
            json!({"arrayValue": {"values": hashtag_values}}),
        );

        if let Some(parent_post_id) = parent_post_id {
            fields.insert("parent_post_id".into(), fs_string(&parent_post_id));
        }
        if let Some(reply_to_user_id) = reply_to_user_id {
            fields.insert("reply_to_user_id".into(), fs_string(&reply_to_user_id));
        }
        if let Some(reply_to_username) = reply_to_username {
            fields.insert("reply_to_username".into(), fs_string(&reply_to_username));
        }

        write_doc(client, COLLECTION_POSTS, &id, fields, dry_run, stats)?;
    }

    Ok(())
}

fn migrate_post_hashtags(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT ph.post_id, h.name FROM post_hashtags ph JOIN hashtags h ON h.id = ph.hashtag_id",
    )?;

    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_POST_HASHTAGS, rows.len());

    for (post_id, hashtag) in rows {
        let normalized = hashtag.trim_start_matches('#').to_ascii_lowercase();
        let doc_id = format!("{}:{}", post_id, normalized);

        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&doc_id));
        fields.insert("post_id".into(), fs_string(&post_id));
        fields.insert("hashtag".into(), fs_string(&normalized));

        write_doc(
            client,
            COLLECTION_POST_HASHTAGS,
            &doc_id,
            fields,
            dry_run,
            stats,
        )?;
    }

    Ok(())
}

fn migrate_votes(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare("SELECT user_id, post_id, direction, created_at FROM votes")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;

    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_VOTES, rows.len());

    for (user_id, post_id, direction, created_at) in rows {
        let doc_id = format!("{}:{}", user_id, post_id);

        let mut fields = Map::new();
        fields.insert("user_id".into(), fs_string(&user_id));
        fields.insert("post_id".into(), fs_string(&post_id));
        fields.insert("direction".into(), fs_string(&direction));
        fields.insert("created_at".into(), fs_timestamp(&created_at));

        write_doc(client, COLLECTION_VOTES, &doc_id, fields, dry_run, stats)?;
    }

    Ok(())
}

fn migrate_friendships(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare("SELECT follower_id, following_id, created_at FROM follows")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_FRIENDS, rows.len());

    for (follower_id, following_id, created_at) in rows {
        let doc_id = format!("{}:{}", follower_id, following_id);
        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&doc_id));
        fields.insert("follower_id".into(), fs_string(&follower_id));
        fields.insert("following_id".into(), fs_string(&following_id));
        fields.insert(
            "created_at".into(),
            fs_timestamp(
                &DateTime::<Utc>::from_timestamp(created_at, 0)
                    .unwrap_or_else(Utc::now)
                    .to_rfc3339(),
            ),
        );

        write_doc(client, COLLECTION_FRIENDS, &doc_id, fields, dry_run, stats)?;
    }

    Ok(())
}

fn migrate_user_configs(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT user_id, color_scheme, sort_order, max_posts_display, emoji_enabled FROM user_configs",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
        ))
    })?;
    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_CONFIGS, rows.len());

    for (user_id, color_scheme, sort_order, max_posts_display, emoji_enabled) in rows {
        let mut fields = Map::new();
        fields.insert("user_id".into(), fs_string(&user_id));
        fields.insert("color_scheme".into(), fs_string(&color_scheme));
        fields.insert("sort_order".into(), fs_string(&sort_order));
        fields.insert("max_posts_display".into(), fs_int(max_posts_display));
        fields.insert("emoji_enabled".into(), fs_bool(emoji_enabled != 0));

        write_doc(client, COLLECTION_CONFIGS, &user_id, fields, dry_run, stats)?;
    }

    Ok(())
}

fn migrate_rate_limits(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut docs: HashMap<String, Map<String, Value>> = HashMap::new();

    let mut post_stmt = conn.prepare("SELECT user_id, last_post_at FROM post_rate_limits")?;
    let post_rows =
        post_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let post_rows = post_rows.collect::<std::result::Result<Vec<_>, _>>()?;

    let mut dm_stmt = conn.prepare("SELECT user_id, last_dm_at FROM dm_rate_limits")?;
    let dm_rows =
        dm_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let dm_rows = dm_rows.collect::<std::result::Result<Vec<_>, _>>()?;

    for (user_id, last_post_at) in post_rows {
        let entry = docs.entry(user_id.clone()).or_default();
        entry.insert("user_id".into(), fs_string(&user_id));
        entry.insert("last_post_at".into(), fs_timestamp(&last_post_at));
    }

    for (user_id, last_dm_at) in dm_rows {
        let entry = docs.entry(user_id.clone()).or_default();
        entry.insert("user_id".into(), fs_string(&user_id));
        entry.insert("last_dm_at".into(), fs_timestamp(&last_dm_at));
    }

    stats.set_source(COLLECTION_RATE_LIMITS, docs.len());
    for (user_id, fields) in docs {
        write_doc(
            client,
            COLLECTION_RATE_LIMITS,
            &user_id,
            fields,
            dry_run,
            stats,
        )?;
    }

    Ok(())
}

fn migrate_dms(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT dm.id, dm.from_user_id, dm.to_user_id, fu.username, tu.username, dm.content, dm.created_at, dm.is_read
         FROM direct_messages dm
         JOIN users fu ON fu.id = dm.from_user_id
         JOIN users tu ON tu.id = dm.to_user_id",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, String>(6)?,
            r.get::<_, i64>(7)?,
        ))
    })?;
    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_DMS, rows.len());

    for (id, from_user_id, to_user_id, from_username, to_username, content, created_at, is_read) in
        rows
    {
        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&id));
        fields.insert("from_user_id".into(), fs_string(&from_user_id));
        fields.insert("to_user_id".into(), fs_string(&to_user_id));
        fields.insert("from_username".into(), fs_string(&from_username));
        fields.insert("to_username".into(), fs_string(&to_username));
        fields.insert("content".into(), fs_string(&content));
        fields.insert("created_at".into(), fs_timestamp(&created_at));
        fields.insert("is_read".into(), fs_bool(is_read != 0));

        write_doc(client, COLLECTION_DMS, &id, fields, dry_run, stats)?;
    }

    Ok(())
}

fn migrate_sessions(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt =
        conn.prepare("SELECT token, user_id, created_at, expires_at, last_activity FROM sessions")?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
        ))
    })?;
    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_SESSIONS, rows.len());

    for (token, user_id, created_at, expires_at, last_activity) in rows {
        let mut fields = Map::new();
        fields.insert("token".into(), fs_string(&token));
        fields.insert("user_id".into(), fs_string(&user_id));
        fields.insert("created_at".into(), fs_timestamp(&created_at));
        fields.insert("expires_at".into(), fs_timestamp(&expires_at));
        if let Some(last_activity) = last_activity {
            fields.insert("last_activity".into(), fs_timestamp(&last_activity));
        }

        write_doc(client, COLLECTION_SESSIONS, &token, fields, dry_run, stats)?;
    }

    Ok(())
}

fn migrate_audit_logs(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, user_id, ip_address, user_agent, details, timestamp FROM audit_logs",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, String>(6)?,
        ))
    })?;
    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_AUDIT, rows.len());

    for (id, event_type, user_id, ip_address, user_agent, details, timestamp) in rows {
        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&id));
        fields.insert("event_type".into(), fs_string(&event_type));
        fields.insert("timestamp".into(), fs_timestamp(&timestamp));
        if let Some(user_id) = user_id {
            fields.insert("user_id".into(), fs_string(&user_id));
        }
        if let Some(ip_address) = ip_address {
            fields.insert("ip_address".into(), fs_string(&ip_address));
        }
        if let Some(user_agent) = user_agent {
            fields.insert("user_agent".into(), fs_string(&user_agent));
        }
        if let Some(details) = details {
            fields.insert("details".into(), fs_string(&details));
        }

        write_doc(client, COLLECTION_AUDIT, &id, fields, dry_run, stats)?;
    }

    Ok(())
}

fn migrate_hashtag_activity(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT uha.user_id, h.name, uha.interaction_count, uha.last_interaction
         FROM user_hashtag_activity uha
         JOIN hashtags h ON h.id = uha.hashtag_id",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, Option<i64>>(3)?,
        ))
    })?;

    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_HASHTAG_ACTIVITY, rows.len());

    for (user_id, hashtag, interaction_count, last_interaction) in rows {
        let normalized = hashtag.trim_start_matches('#').to_ascii_lowercase();
        let doc_id = format!("{}:{}", user_id, normalized);

        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&doc_id));
        fields.insert("user_id".into(), fs_string(&user_id));
        fields.insert("hashtag".into(), fs_string(&normalized));
        fields.insert("interaction_count".into(), fs_int(interaction_count));
        if let Some(last_interaction) = last_interaction {
            fields.insert(
                "last_interaction".into(),
                fs_timestamp(
                    &DateTime::<Utc>::from_timestamp(last_interaction, 0)
                        .unwrap_or_else(Utc::now)
                        .to_rfc3339(),
                ),
            );
        }

        write_doc(
            client,
            COLLECTION_HASHTAG_ACTIVITY,
            &doc_id,
            fields,
            dry_run,
            stats,
        )?;
    }

    Ok(())
}

fn migrate_hashtag_follows(
    conn: &Connection,
    client: &FirestoreClient,
    dry_run: bool,
    stats: &mut MigrationStats,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT uhf.user_id, h.name, uhf.followed_at
         FROM user_hashtag_follows uhf
         JOIN hashtags h ON h.id = uhf.hashtag_id",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;

    let rows = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    stats.set_source(COLLECTION_HASHTAG_FOLLOWS, rows.len());

    for (user_id, hashtag, followed_at) in rows {
        let normalized = hashtag.trim_start_matches('#').to_ascii_lowercase();
        let doc_id = format!("{}:{}", user_id, normalized);

        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&doc_id));
        fields.insert("user_id".into(), fs_string(&user_id));
        fields.insert("hashtag".into(), fs_string(&normalized));
        fields.insert(
            "followed_at".into(),
            fs_timestamp(
                &DateTime::<Utc>::from_timestamp(followed_at, 0)
                    .unwrap_or_else(Utc::now)
                    .to_rfc3339(),
            ),
        );

        write_doc(
            client,
            COLLECTION_HASHTAG_FOLLOWS,
            &doc_id,
            fields,
            dry_run,
            stats,
        )?;
    }

    Ok(())
}

fn print_summary(stats: &MigrationStats, validate_counts: Option<HashMap<&'static str, usize>>) {
    println!("\nMigration summary");
    println!("=================");

    let mut keys = stats.source_counts.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    for key in keys {
        let source = stats.source_counts.get(key).copied().unwrap_or(0);
        let written = stats.written_counts.get(key).copied().unwrap_or(0);
        if let Some(ref validation) = validate_counts {
            let target = validation.get(key).copied().unwrap_or(0);
            println!("{key:20} source={source:6} written={written:6} firestore={target:6}");
        } else {
            println!("{key:20} source={source:6} written={written:6}");
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let project_id = args
        .project_id
        .or_else(|| std::env::var("FIREBASE_PROJECT_ID").ok())
        .or_else(|| std::env::var("GOOGLE_CLOUD_PROJECT").ok())
        .context(
            "missing project id: pass --project-id or set FIREBASE_PROJECT_ID/GOOGLE_CLOUD_PROJECT",
        )?;

    let emulator_host = args
        .emulator_host
        .or_else(|| std::env::var("FIRESTORE_EMULATOR_HOST").ok());
    let access_token = args
        .access_token
        .or_else(|| std::env::var("FIRESTORE_ACCESS_TOKEN").ok());

    if emulator_host.is_none() && access_token.is_none() {
        return Err(anyhow!(
            "non-emulator migration requires --access-token or FIRESTORE_ACCESS_TOKEN"
        ));
    }

    let conn = Connection::open(&args.sqlite_path)
        .with_context(|| format!("failed to open sqlite db at {}", args.sqlite_path))?;

    let client = FirestoreClient::new(&project_id, emulator_host.clone(), access_token);

    println!("Starting SQLite -> Firestore migration");
    println!("  sqlite path: {}", args.sqlite_path);
    println!("  project id : {}", project_id);
    println!(
        "  emulator   : {}",
        emulator_host.as_deref().unwrap_or("no")
    );
    println!("  dry-run    : {}", args.dry_run);

    let mut stats = MigrationStats::default();

    migrate_users(&conn, &client, args.dry_run, &mut stats)?;
    migrate_posts(&conn, &client, args.dry_run, &mut stats)?;
    migrate_post_hashtags(&conn, &client, args.dry_run, &mut stats)?;
    migrate_votes(&conn, &client, args.dry_run, &mut stats)?;
    migrate_friendships(&conn, &client, args.dry_run, &mut stats)?;
    migrate_user_configs(&conn, &client, args.dry_run, &mut stats)?;
    migrate_rate_limits(&conn, &client, args.dry_run, &mut stats)?;
    migrate_dms(&conn, &client, args.dry_run, &mut stats)?;
    migrate_sessions(&conn, &client, args.dry_run, &mut stats)?;
    migrate_audit_logs(&conn, &client, args.dry_run, &mut stats)?;
    migrate_hashtag_activity(&conn, &client, args.dry_run, &mut stats)?;
    migrate_hashtag_follows(&conn, &client, args.dry_run, &mut stats)?;

    let validate_counts = if args.validate && !args.dry_run {
        let mut counts = HashMap::new();
        for collection in [
            COLLECTION_USERS,
            COLLECTION_POSTS,
            COLLECTION_POST_HASHTAGS,
            COLLECTION_VOTES,
            COLLECTION_FRIENDS,
            COLLECTION_DMS,
            COLLECTION_CONFIGS,
            COLLECTION_RATE_LIMITS,
            COLLECTION_SESSIONS,
            COLLECTION_AUDIT,
            COLLECTION_HASHTAG_ACTIVITY,
            COLLECTION_HASHTAG_FOLLOWS,
        ] {
            counts.insert(collection, client.query_collection_count(collection)?);
        }
        Some(counts)
    } else {
        None
    };

    print_summary(&stats, validate_counts);

    if args.dry_run {
        println!("\nDry-run complete. No documents were written.");
    } else {
        println!("\nMigration complete.");
    }

    Ok(())
}
