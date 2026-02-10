//! Firestore-backed store implementations.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::stores::{
    AuditStore, ConfigStore, DirectMessageStore, FriendStore, HashtagStore, PostStore,
    RateLimitStore, SessionRecord, SessionStore, Stores, UserStore, VoteStore,
};
use fido_types::{
    ColorScheme, DirectMessage, Post, SortOrder, User, UserConfig, Vote, VoteDirection,
};

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

#[derive(Clone)]
struct FirestoreClient {
    http: Client,
    documents_base: String,
    run_query_url: String,
    emulator: bool,
    project_id: String,
    access_token: Option<String>,
    use_metadata_token: bool,
}

#[derive(Debug)]
struct FirestoreError {
    code: Option<i64>,
    status: Option<String>,
    message: String,
}

impl std::fmt::Display for FirestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Firestore error (code={:?}, status={:?}): {}",
            self.code, self.status, self.message
        )
    }
}

impl std::error::Error for FirestoreError {}

impl FirestoreClient {
    fn from_env() -> Result<Self> {
        let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
            .or_else(|_| std::env::var("FIREBASE_PROJECT_ID"))
            .context("Firestore backend selected, but GOOGLE_CLOUD_PROJECT/FIREBASE_PROJECT_ID is not set")?;

        let emulator_host = std::env::var("FIRESTORE_EMULATOR_HOST").ok();
        let emulator = emulator_host.is_some();

        let (documents_base, run_query_url) = if let Some(host) = emulator_host {
            let database_base =
                format!("http://{host}/v1/projects/{project_id}/databases/(default)");
            (
                format!("{database_base}/documents"),
                format!("{database_base}/documents:runQuery"),
            )
        } else {
            let database_base = format!(
                "https://firestore.googleapis.com/v1/projects/{project_id}/databases/(default)"
            );
            (
                format!("{database_base}/documents"),
                format!("{database_base}/documents:runQuery"),
            )
        };

        let configured_access_token = if emulator {
            None
        } else {
            std::env::var("FIRESTORE_ACCESS_TOKEN")
                .ok()
                .or_else(|| std::env::var("GOOGLE_OAUTH_ACCESS_TOKEN").ok())
        };

        // When no explicit token is configured, fetch a fresh metadata token per request.
        // Metadata tokens are short-lived, so caching them at startup causes Unauthorized
        // errors after expiration in long-lived Cloud Run instances.
        let use_metadata_token = !emulator && configured_access_token.is_none();

        Ok(Self {
            http: Client::new(),
            documents_base,
            run_query_url,
            emulator,
            project_id,
            access_token: configured_access_token,
            use_metadata_token,
        })
    }

    fn run_sync<T, F>(&self, fut: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| handle.block_on(fut))
        } else {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(fut)
        }
    }

    fn with_auth(&self, req: RequestBuilder) -> RequestBuilder {
        if self.emulator {
            return req;
        }

        if let Some(token) = &self.access_token {
            return req.bearer_auth(token);
        }

        if self.use_metadata_token {
            if let Some(token) = fetch_metadata_access_token() {
                return req.bearer_auth(token);
            }
        }

        req
    }

    fn send_json(&self, method: Method, url: String, body: Option<Value>) -> Result<Value> {
        self.run_sync(async {
            let mut req = self.http.request(method, &url);
            if let Some(body) = body {
                req = req.json(&body);
            }

            let resp = self.with_auth(req).send().await.with_context(|| {
                format!(
                    "failed to send Firestore request for project {} to {}",
                    self.project_id, url
                )
            })?;

            let status = resp.status();
            let payload = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(parse_firestore_error(status, &payload));
            }

            if payload.trim().is_empty() {
                return Ok(Value::Null);
            }

            serde_json::from_str::<Value>(&payload)
                .with_context(|| format!("invalid Firestore JSON response from {}", url))
        })
    }

    fn get_document(&self, collection: &str, doc_id: &str) -> Result<Option<Map<String, Value>>> {
        let url = format!("{}/{}/{}", self.documents_base, collection, doc_id);
        self.run_sync(async {
            let req = self.with_auth(self.http.get(&url));
            let resp = req.send().await.with_context(|| {
                format!(
                    "failed to read Firestore document {}/{} in project {}",
                    collection, doc_id, self.project_id
                )
            })?;

            if resp.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }

            let status = resp.status();
            let payload = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(parse_firestore_error(status, &payload));
            }

            let value: Value = serde_json::from_str(&payload).with_context(|| {
                format!("invalid Firestore document response for {collection}/{doc_id}")
            })?;

            let fields = value
                .get("fields")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();

            Ok(Some(fields))
        })
    }

    fn set_document(
        &self,
        collection: &str,
        doc_id: &str,
        fields: Map<String, Value>,
        update_mask: Option<&[&str]>,
    ) -> Result<()> {
        let mut url = format!("{}/{}/{}", self.documents_base, collection, doc_id);
        if let Some(mask) = update_mask {
            if !mask.is_empty() {
                let encoded = mask
                    .iter()
                    .map(|field| format!("updateMask.fieldPaths={field}"))
                    .collect::<Vec<_>>()
                    .join("&");
                url.push('?');
                url.push_str(&encoded);
            }
        }

        self.send_json(Method::PATCH, url, Some(json!({ "fields": fields })))?;
        Ok(())
    }

    fn delete_document(&self, collection: &str, doc_id: &str) -> Result<bool> {
        let url = format!("{}/{}/{}", self.documents_base, collection, doc_id);
        self.run_sync(async {
            let req = self.with_auth(self.http.delete(&url));
            let resp = req.send().await.with_context(|| {
                format!(
                    "failed to delete Firestore document {}/{} in project {}",
                    collection, doc_id, self.project_id
                )
            })?;

            if resp.status() == StatusCode::NOT_FOUND {
                return Ok(false);
            }

            let status = resp.status();
            let payload = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(parse_firestore_error(status, &payload));
            }

            Ok(true)
        })
    }

    fn query_collection(&self, collection: &str) -> Result<Vec<Map<String, Value>>> {
        let query = json!({
            "structuredQuery": {
                "from": [{"collectionId": collection}]
            }
        });

        let rows = self.send_json(Method::POST, self.run_query_url.clone(), Some(query))?;
        let items = rows
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                row.get("document")
                    .and_then(|doc| doc.get("fields"))
                    .and_then(|fields| fields.as_object().cloned())
            })
            .collect();

        Ok(items)
    }

    fn query_collection_filtered(
        &self,
        collection: &str,
        filters: &[(&str, &str)],
        order_by: Option<(&str, bool)>,
    ) -> Result<Vec<Map<String, Value>>> {
        if filters.is_empty() {
            return self.query_collection(collection);
        }

        let field_filters: Vec<Value> = filters
            .iter()
            .map(|(field, value)| {
                json!({
                    "fieldFilter": {
                        "field": { "fieldPath": field },
                        "op": "EQUAL",
                        "value": { "stringValue": value }
                    }
                })
            })
            .collect();

        let where_clause = if field_filters.len() == 1 {
            field_filters[0].clone()
        } else {
            json!({
                "compositeFilter": {
                    "op": "AND",
                    "filters": field_filters
                }
            })
        };

        let mut structured_query = json!({
            "from": [{"collectionId": collection}],
            "where": where_clause
        });

        if let Some((field, descending)) = order_by {
            let direction = if descending {
                "DESCENDING"
            } else {
                "ASCENDING"
            };
            structured_query["orderBy"] = json!([{
                "field": { "fieldPath": field },
                "direction": direction
            }]);
        }

        let query = json!({
            "structuredQuery": structured_query
        });

        let rows = self.send_json(Method::POST, self.run_query_url.clone(), Some(query))?;
        let items = rows
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                row.get("document")
                    .and_then(|doc| doc.get("fields"))
                    .and_then(|fields| fields.as_object().cloned())
            })
            .collect();

        Ok(items)
    }
}

fn parse_firestore_error(status: StatusCode, payload: &str) -> anyhow::Error {
    let parsed = serde_json::from_str::<Value>(payload).unwrap_or(Value::Null);
    let error = parsed.get("error").cloned().unwrap_or(Value::Null);

    let code = error.get("code").and_then(|v| v.as_i64());
    let message = error
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("Firestore request failed")
        })
        .to_string();
    let status_name = error
        .get("status")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    FirestoreError {
        code,
        status: status_name,
        message,
    }
    .into()
}

fn fetch_metadata_access_token() -> Option<String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect("metadata.google.internal:80").ok()?;
    let request = concat!(
        "GET /computeMetadata/v1/instance/service-accounts/default/token HTTP/1.1\r\n",
        "Host: metadata.google.internal\r\n",
        "Metadata-Flavor: Google\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(request.as_bytes()).ok()?;

    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    let (_, body) = response.split_once("\r\n\r\n")?;

    let json: Value = serde_json::from_str(body).ok()?;
    json.get("access_token")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn fs_string(value: &str) -> Value {
    json!({ "stringValue": value })
}

fn fs_int(value: i64) -> Value {
    json!({ "integerValue": value.to_string() })
}

fn fs_bool(value: bool) -> Value {
    json!({ "booleanValue": value })
}

fn fs_timestamp(value: DateTime<Utc>) -> Value {
    json!({ "timestampValue": value.to_rfc3339() })
}

fn fs_null() -> Value {
    json!({ "nullValue": Value::Null })
}

fn fs_string_array(values: &[String]) -> Value {
    json!({
        "arrayValue": {
            "values": values.iter().map(|v| fs_string(v)).collect::<Vec<_>>()
        }
    })
}

fn from_string(fields: &Map<String, Value>, key: &str) -> Option<String> {
    fields
        .get(key)
        .and_then(|v| v.get("stringValue"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn from_int(fields: &Map<String, Value>, key: &str) -> Option<i64> {
    fields
        .get(key)
        .and_then(|v| v.get("integerValue"))
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<i64>().ok())
}

fn from_bool(fields: &Map<String, Value>, key: &str) -> Option<bool> {
    fields
        .get(key)
        .and_then(|v| v.get("booleanValue"))
        .and_then(|v| v.as_bool())
}

fn from_timestamp(fields: &Map<String, Value>, key: &str) -> Option<DateTime<Utc>> {
    fields
        .get(key)
        .and_then(|v| v.get("timestampValue"))
        .and_then(|v| v.as_str())
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn from_string_array(fields: &Map<String, Value>, key: &str) -> Vec<String> {
    fields
        .get(key)
        .and_then(|v| v.get("arrayValue"))
        .and_then(|v| v.get("values"))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("stringValue")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).with_context(|| format!("invalid UUID in field '{field}': {raw}"))
}

fn user_to_fields(user: &User) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert("id".into(), fs_string(&user.id.to_string()));
    fields.insert("username".into(), fs_string(&user.username));
    match &user.bio {
        Some(bio) => {
            fields.insert("bio".into(), fs_string(bio));
        }
        None => {
            fields.insert("bio".into(), fs_null());
        }
    }
    fields.insert("join_date".into(), fs_timestamp(user.join_date));
    fields.insert("is_test_user".into(), fs_bool(user.is_test_user));
    fields.insert("is_admin".into(), fs_bool(user.is_admin));
    fields
}

fn user_from_fields(fields: &Map<String, Value>) -> Result<User> {
    let id = parse_uuid(
        &from_string(fields, "id").ok_or_else(|| anyhow!("missing user.id"))?,
        "id",
    )?;
    let username =
        from_string(fields, "username").ok_or_else(|| anyhow!("missing user.username"))?;
    let bio = from_string(fields, "bio");
    let join_date =
        from_timestamp(fields, "join_date").ok_or_else(|| anyhow!("missing user.join_date"))?;
    let is_test_user = from_bool(fields, "is_test_user").unwrap_or(false);
    let is_admin = from_bool(fields, "is_admin").unwrap_or(false);

    Ok(User {
        id,
        username,
        bio,
        join_date,
        is_test_user,
        is_admin,
    })
}

fn post_to_fields(post: &Post) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert("id".into(), fs_string(&post.id.to_string()));
    fields.insert("author_id".into(), fs_string(&post.author_id.to_string()));
    fields.insert("author_username".into(), fs_string(&post.author_username));
    fields.insert("content".into(), fs_string(&post.content));
    fields.insert("created_at".into(), fs_timestamp(post.created_at));
    fields.insert("upvotes".into(), fs_int(post.upvotes as i64));
    fields.insert("downvotes".into(), fs_int(post.downvotes as i64));
    fields.insert("hashtags".into(), fs_string_array(&post.hashtags));
    fields.insert("reply_count".into(), fs_int(post.reply_count as i64));

    if let Some(parent_id) = post.parent_post_id {
        fields.insert("parent_post_id".into(), fs_string(&parent_id.to_string()));
    }

    if let Some(reply_to_user_id) = post.reply_to_user_id {
        fields.insert(
            "reply_to_user_id".into(),
            fs_string(&reply_to_user_id.to_string()),
        );
    }

    if let Some(reply_to_username) = &post.reply_to_username {
        fields.insert("reply_to_username".into(), fs_string(reply_to_username));
    }

    fields
}

fn post_from_fields(fields: &Map<String, Value>) -> Result<Post> {
    let id = parse_uuid(
        &from_string(fields, "id").ok_or_else(|| anyhow!("missing post.id"))?,
        "id",
    )?;
    let author_id = parse_uuid(
        &from_string(fields, "author_id").ok_or_else(|| anyhow!("missing post.author_id"))?,
        "author_id",
    )?;

    Ok(Post {
        id,
        author_id,
        author_username: from_string(fields, "author_username")
            .ok_or_else(|| anyhow!("missing post.author_username"))?,
        content: from_string(fields, "content").ok_or_else(|| anyhow!("missing post.content"))?,
        created_at: from_timestamp(fields, "created_at")
            .ok_or_else(|| anyhow!("missing post.created_at"))?,
        upvotes: from_int(fields, "upvotes").unwrap_or(0) as i32,
        downvotes: from_int(fields, "downvotes").unwrap_or(0) as i32,
        hashtags: from_string_array(fields, "hashtags"),
        user_vote: None,
        parent_post_id: from_string(fields, "parent_post_id")
            .map(|s| parse_uuid(&s, "parent_post_id"))
            .transpose()?,
        reply_count: from_int(fields, "reply_count").unwrap_or(0) as i32,
        reply_to_user_id: from_string(fields, "reply_to_user_id")
            .map(|s| parse_uuid(&s, "reply_to_user_id"))
            .transpose()?,
        reply_to_username: from_string(fields, "reply_to_username"),
    })
}

fn vote_to_fields(vote: &Vote) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert("user_id".into(), fs_string(&vote.user_id.to_string()));
    fields.insert("post_id".into(), fs_string(&vote.post_id.to_string()));
    fields.insert("direction".into(), fs_string(vote.direction.as_str()));
    fields.insert("created_at".into(), fs_timestamp(vote.created_at));
    fields
}

fn vote_from_fields(fields: &Map<String, Value>) -> Result<Vote> {
    let direction_str =
        from_string(fields, "direction").ok_or_else(|| anyhow!("missing vote.direction"))?;
    let direction = VoteDirection::parse(&direction_str)
        .ok_or_else(|| anyhow!("invalid vote direction: {direction_str}"))?;

    Ok(Vote {
        user_id: parse_uuid(
            &from_string(fields, "user_id").ok_or_else(|| anyhow!("missing vote.user_id"))?,
            "user_id",
        )?,
        post_id: parse_uuid(
            &from_string(fields, "post_id").ok_or_else(|| anyhow!("missing vote.post_id"))?,
            "post_id",
        )?,
        direction,
        created_at: from_timestamp(fields, "created_at")
            .ok_or_else(|| anyhow!("missing vote.created_at"))?,
    })
}

fn dm_to_fields(dm: &DirectMessage) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert("id".into(), fs_string(&dm.id.to_string()));
    fields.insert(
        "from_user_id".into(),
        fs_string(&dm.from_user_id.to_string()),
    );
    fields.insert("to_user_id".into(), fs_string(&dm.to_user_id.to_string()));
    fields.insert("from_username".into(), fs_string(&dm.from_username));
    fields.insert("to_username".into(), fs_string(&dm.to_username));
    fields.insert("content".into(), fs_string(&dm.content));
    fields.insert("created_at".into(), fs_timestamp(dm.created_at));
    fields.insert("is_read".into(), fs_bool(dm.is_read));
    fields
}

fn dm_from_fields(fields: &Map<String, Value>) -> Result<DirectMessage> {
    Ok(DirectMessage {
        id: parse_uuid(
            &from_string(fields, "id").ok_or_else(|| anyhow!("missing dm.id"))?,
            "id",
        )?,
        from_user_id: parse_uuid(
            &from_string(fields, "from_user_id")
                .ok_or_else(|| anyhow!("missing dm.from_user_id"))?,
            "from_user_id",
        )?,
        to_user_id: parse_uuid(
            &from_string(fields, "to_user_id").ok_or_else(|| anyhow!("missing dm.to_user_id"))?,
            "to_user_id",
        )?,
        from_username: from_string(fields, "from_username").unwrap_or_default(),
        to_username: from_string(fields, "to_username").unwrap_or_default(),
        content: from_string(fields, "content").ok_or_else(|| anyhow!("missing dm.content"))?,
        created_at: from_timestamp(fields, "created_at")
            .ok_or_else(|| anyhow!("missing dm.created_at"))?,
        is_read: from_bool(fields, "is_read").unwrap_or(false),
    })
}

#[derive(Clone)]
pub struct FirestorePostStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreHashtagStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreVoteStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreUserStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreFriendStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreConfigStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreRateLimitStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreDirectMessageStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreSessionStore {
    client: Arc<FirestoreClient>,
}

#[derive(Clone)]
pub struct FirestoreAuditStore {
    client: Arc<FirestoreClient>,
}

fn sort_posts(posts: &mut [Post], sort_order: SortOrder) {
    match sort_order {
        SortOrder::Newest => posts.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        SortOrder::Popular => {
            posts.sort_by(|a, b| (b.upvotes - b.downvotes).cmp(&(a.upvotes - a.downvotes)))
        }
        SortOrder::Controversial => posts.sort_by(|a, b| {
            let a_score = std::cmp::min(a.upvotes, a.downvotes);
            let b_score = std::cmp::min(b.upvotes, b.downvotes);
            b_score.cmp(&a_score)
        }),
    }
}

impl FirestorePostStore {
    fn load_posts(&self) -> Result<Vec<Post>> {
        self.client
            .query_collection(COLLECTION_POSTS)?
            .into_iter()
            .map(|f| post_from_fields(&f))
            .collect()
    }

    fn compute_reply_counts(posts: &[Post]) -> HashMap<Uuid, i32> {
        let mut reply_counts = HashMap::new();
        for post in posts {
            if let Some(parent_id) = post.parent_post_id {
                *reply_counts.entry(parent_id).or_insert(0) += 1;
            }
        }
        reply_counts
    }

    fn apply_reply_counts(posts: &mut [Post], reply_counts: &HashMap<Uuid, i32>) {
        for post in posts {
            post.reply_count = *reply_counts.get(&post.id).unwrap_or(&0);
        }
    }
}

impl PostStore for FirestorePostStore {
    fn get_posts(&self, sort_order: SortOrder, limit: i32) -> Result<Vec<Post>> {
        let all_posts = self.load_posts()?;
        let reply_counts = Self::compute_reply_counts(&all_posts);

        let mut posts = all_posts
            .into_iter()
            .filter(|p| p.parent_post_id.is_none())
            .collect::<Vec<_>>();

        Self::apply_reply_counts(&mut posts, &reply_counts);
        sort_posts(&mut posts, sort_order);
        posts.truncate(limit.max(0) as usize);
        Ok(posts)
    }

    fn get_posts_by_hashtag(
        &self,
        hashtag: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> Result<Vec<Post>> {
        let target = hashtag.to_ascii_lowercase();
        let ids: HashSet<String> = self
            .client
            .query_collection(COLLECTION_POST_HASHTAGS)?
            .into_iter()
            .filter_map(|entry| {
                let tag = from_string(&entry, "hashtag")?;
                if tag.to_ascii_lowercase() != target {
                    return None;
                }
                from_string(&entry, "post_id")
            })
            .collect();

        let all_posts = self.load_posts()?;
        let reply_counts = Self::compute_reply_counts(&all_posts);

        let mut posts = all_posts
            .into_iter()
            .filter(|p| p.parent_post_id.is_none() && ids.contains(&p.id.to_string()))
            .collect::<Vec<_>>();

        Self::apply_reply_counts(&mut posts, &reply_counts);
        sort_posts(&mut posts, sort_order);
        posts.truncate(limit.max(0) as usize);
        Ok(posts)
    }

    fn get_posts_by_username(
        &self,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> Result<Vec<Post>> {
        let target = username.to_ascii_lowercase();
        let all_posts = self.load_posts()?;
        let reply_counts = Self::compute_reply_counts(&all_posts);

        let mut posts = all_posts
            .into_iter()
            .filter(|p| {
                p.parent_post_id.is_none() && p.author_username.to_ascii_lowercase() == target
            })
            .collect::<Vec<_>>();

        Self::apply_reply_counts(&mut posts, &reply_counts);
        sort_posts(&mut posts, sort_order);
        posts.truncate(limit.max(0) as usize);
        Ok(posts)
    }

    fn get_posts_by_hashtag_and_username(
        &self,
        hashtag: &str,
        username: &str,
        sort_order: SortOrder,
        limit: i32,
    ) -> Result<Vec<Post>> {
        let target_username = username.to_ascii_lowercase();
        let target_hashtag = hashtag.to_ascii_lowercase();
        let ids: HashSet<String> = self
            .client
            .query_collection(COLLECTION_POST_HASHTAGS)?
            .into_iter()
            .filter_map(|entry| {
                let tag = from_string(&entry, "hashtag")?;
                if tag.to_ascii_lowercase() != target_hashtag {
                    return None;
                }
                from_string(&entry, "post_id")
            })
            .collect();

        let all_posts = self.load_posts()?;
        let reply_counts = Self::compute_reply_counts(&all_posts);

        let mut posts = all_posts
            .into_iter()
            .filter(|p| {
                p.parent_post_id.is_none()
                    && ids.contains(&p.id.to_string())
                    && p.author_username.to_ascii_lowercase() == target_username
            })
            .collect::<Vec<_>>();

        Self::apply_reply_counts(&mut posts, &reply_counts);
        sort_posts(&mut posts, sort_order);
        posts.truncate(limit.max(0) as usize);
        Ok(posts)
    }

    fn get_by_id(&self, post_id: &Uuid) -> Result<Option<Post>> {
        let fields = self
            .client
            .get_document(COLLECTION_POSTS, &post_id.to_string())?;

        let mut post = match fields {
            Some(fields) => post_from_fields(&fields)?,
            None => return Ok(None),
        };

        let all_posts = self.load_posts()?;
        let reply_counts = Self::compute_reply_counts(&all_posts);
        post.reply_count = *reply_counts.get(post_id).unwrap_or(&0);
        Ok(Some(post))
    }

    fn get_replies(&self, post_id: &Uuid) -> Result<Vec<Post>> {
        let posts = self.load_posts()?;
        let reply_counts = Self::compute_reply_counts(&posts);
        let mut by_parent: HashMap<Uuid, Vec<Post>> = HashMap::new();

        for post in posts {
            if let Some(parent_id) = post.parent_post_id {
                by_parent.entry(parent_id).or_default().push(post);
            }
        }

        let mut out = Vec::new();
        let mut queue = vec![*post_id];

        while let Some(parent) = queue.pop() {
            if let Some(mut children) = by_parent.remove(&parent) {
                children.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                for mut child in children {
                    child.reply_count = *reply_counts.get(&child.id).unwrap_or(&0);
                    queue.push(child.id);
                    out.push(child);
                }
            }
        }

        out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(out)
    }

    fn create(&self, post: &Post) -> Result<()> {
        self.client.set_document(
            COLLECTION_POSTS,
            &post.id.to_string(),
            post_to_fields(post),
            None,
        )
    }

    fn update_content(&self, post_id: &Uuid, content: &str) -> Result<()> {
        let mut fields = Map::new();
        fields.insert("content".into(), fs_string(content));
        self.client.set_document(
            COLLECTION_POSTS,
            &post_id.to_string(),
            fields,
            Some(&["content"]),
        )
    }

    fn delete(&self, post_id: &Uuid) -> Result<()> {
        self.client
            .delete_document(COLLECTION_POSTS, &post_id.to_string())?;

        let hashtags = self.client.query_collection(COLLECTION_POST_HASHTAGS)?;
        for record in hashtags {
            let post_match = from_string(&record, "post_id")
                .map(|id| id == post_id.to_string())
                .unwrap_or(false);
            if post_match {
                if let Some(id) = from_string(&record, "id") {
                    let _ = self.client.delete_document(COLLECTION_POST_HASHTAGS, &id)?;
                }
            }
        }

        Ok(())
    }

    fn update_vote_counts(&self, post_id: &Uuid) -> Result<()> {
        let votes = self.client.query_collection(COLLECTION_VOTES)?;
        let mut up = 0_i64;
        let mut down = 0_i64;

        for vote_fields in votes {
            if from_string(&vote_fields, "post_id") != Some(post_id.to_string()) {
                continue;
            }

            match from_string(&vote_fields, "direction").as_deref() {
                Some("up") => up += 1,
                Some("down") => down += 1,
                _ => {}
            }
        }

        let mut fields = Map::new();
        fields.insert("upvotes".into(), fs_int(up));
        fields.insert("downvotes".into(), fs_int(down));
        self.client.set_document(
            COLLECTION_POSTS,
            &post_id.to_string(),
            fields,
            Some(&["upvotes", "downvotes"]),
        )
    }

    fn get_post_count(&self, user_id: &Uuid) -> Result<i32> {
        Ok(self
            .load_posts()?
            .into_iter()
            .filter(|p| p.author_id == *user_id)
            .count() as i32)
    }
}

impl HashtagStore for FirestoreHashtagStore {
    fn get_by_post(&self, post_id: &Uuid) -> Result<Vec<String>> {
        let mut tags = self
            .client
            .query_collection(COLLECTION_POST_HASHTAGS)?
            .into_iter()
            .filter(|record| from_string(record, "post_id") == Some(post_id.to_string()))
            .filter_map(|record| from_string(&record, "hashtag"))
            .collect::<Vec<_>>();

        tags.sort();
        tags.dedup();
        Ok(tags)
    }

    fn store_hashtags(&self, post_id: &Uuid, hashtags: &[String]) -> Result<()> {
        for hashtag in hashtags {
            let normalized = hashtag.trim().trim_start_matches('#').to_ascii_lowercase();
            if normalized.is_empty() {
                continue;
            }

            let id = format!("{}:{}", post_id, normalized);
            let mut fields = Map::new();
            fields.insert("id".into(), fs_string(&id));
            fields.insert("post_id".into(), fs_string(&post_id.to_string()));
            fields.insert("hashtag".into(), fs_string(&normalized));
            fields.insert("created_at".into(), fs_timestamp(Utc::now()));
            self.client
                .set_document(COLLECTION_POST_HASHTAGS, &id, fields, None)?;
        }
        Ok(())
    }

    fn delete_by_post(&self, post_id: &Uuid) -> Result<()> {
        let rows = self.client.query_collection(COLLECTION_POST_HASHTAGS)?;
        for row in rows {
            if from_string(&row, "post_id") == Some(post_id.to_string()) {
                if let Some(id) = from_string(&row, "id") {
                    let _ = self.client.delete_document(COLLECTION_POST_HASHTAGS, &id)?;
                }
            }
        }
        Ok(())
    }

    fn increment_activity(&self, user_id: &Uuid, hashtag: &str) -> Result<()> {
        let normalized = hashtag.trim().trim_start_matches('#').to_ascii_lowercase();
        if normalized.is_empty() {
            return Ok(());
        }

        let id = format!("{}:{}", user_id, normalized);
        let existing = self.client.get_document(COLLECTION_HASHTAG_ACTIVITY, &id)?;
        let current = existing
            .as_ref()
            .and_then(|f| from_int(f, "interaction_count"))
            .unwrap_or(0);

        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&id));
        fields.insert("user_id".into(), fs_string(&user_id.to_string()));
        fields.insert("hashtag".into(), fs_string(&normalized));
        fields.insert("interaction_count".into(), fs_int(current + 1));
        fields.insert("last_interaction".into(), fs_timestamp(Utc::now()));

        self.client
            .set_document(COLLECTION_HASHTAG_ACTIVITY, &id, fields, None)
    }

    fn get_active_by_user(&self, user_id: &Uuid, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut rows = self
            .client
            .query_collection(COLLECTION_HASHTAG_ACTIVITY)?
            .into_iter()
            .filter(|row| from_string(row, "user_id") == Some(user_id.to_string()))
            .filter_map(|row| {
                let tag = from_string(&row, "hashtag")?;
                let count = from_int(&row, "interaction_count").unwrap_or(0);
                Some((tag, count))
            })
            .collect::<Vec<_>>();

        rows.sort_by(|a, b| b.1.cmp(&a.1));
        rows.truncate(limit);
        Ok(rows)
    }

    fn get_followed_by_user(&self, user_id: &Uuid) -> Result<Vec<String>> {
        let mut rows = self
            .client
            .query_collection(COLLECTION_HASHTAG_FOLLOWS)?
            .into_iter()
            .filter(|row| from_string(row, "user_id") == Some(user_id.to_string()))
            .filter_map(|row| from_string(&row, "hashtag"))
            .collect::<Vec<_>>();

        rows.sort();
        rows.dedup();
        Ok(rows)
    }

    fn get_post_count(&self, name: &str) -> Result<i32> {
        let target = name.trim().trim_start_matches('#').to_ascii_lowercase();
        Ok(self
            .client
            .query_collection(COLLECTION_POST_HASHTAGS)?
            .into_iter()
            .filter(|row| from_string(row, "hashtag") == Some(target.clone()))
            .count() as i32)
    }

    fn follow_hashtag(&self, user_id: &Uuid, name: &str) -> Result<()> {
        let hashtag = name.trim().trim_start_matches('#').to_ascii_lowercase();
        if hashtag.is_empty() {
            return Ok(());
        }

        let id = format!("{}:{}", user_id, hashtag);
        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&id));
        fields.insert("user_id".into(), fs_string(&user_id.to_string()));
        fields.insert("hashtag".into(), fs_string(&hashtag));
        fields.insert("followed_at".into(), fs_timestamp(Utc::now()));

        self.client
            .set_document(COLLECTION_HASHTAG_FOLLOWS, &id, fields, None)
    }

    fn unfollow_hashtag(&self, user_id: &Uuid, name: &str) -> Result<()> {
        let hashtag = name.trim().trim_start_matches('#').to_ascii_lowercase();
        if hashtag.is_empty() {
            return Ok(());
        }
        let id = format!("{}:{}", user_id, hashtag);
        let _ = self
            .client
            .delete_document(COLLECTION_HASHTAG_FOLLOWS, &id)?;
        Ok(())
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let q = query.trim().trim_start_matches('#').to_ascii_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let mut set = HashSet::new();
        for row in self.client.query_collection(COLLECTION_POST_HASHTAGS)? {
            if let Some(tag) = from_string(&row, "hashtag") {
                if tag.contains(&q) {
                    set.insert(tag);
                }
            }
        }

        let mut tags = set.into_iter().collect::<Vec<_>>();
        tags.sort();
        tags.truncate(limit);
        Ok(tags)
    }
}

impl VoteStore for FirestoreVoteStore {
    fn upsert_vote(&self, user_id: &Uuid, post_id: &Uuid, direction: VoteDirection) -> Result<()> {
        let vote = Vote {
            user_id: *user_id,
            post_id: *post_id,
            direction,
            created_at: Utc::now(),
        };
        let doc_id = format!("{}:{}", user_id, post_id);
        self.client
            .set_document(COLLECTION_VOTES, &doc_id, vote_to_fields(&vote), None)
    }

    fn get_vote(&self, user_id: &Uuid, post_id: &Uuid) -> Result<Option<Vote>> {
        let doc_id = format!("{}:{}", user_id, post_id);
        let doc = self.client.get_document(COLLECTION_VOTES, &doc_id)?;
        match doc {
            Some(fields) => Ok(Some(vote_from_fields(&fields)?)),
            None => Ok(None),
        }
    }

    fn calculate_karma(&self, user_id: &Uuid) -> Result<i32> {
        let posts: HashSet<String> = self
            .client
            .query_collection(COLLECTION_POSTS)?
            .into_iter()
            .filter(|row| from_string(row, "author_id") == Some(user_id.to_string()))
            .filter_map(|row| from_string(&row, "id"))
            .collect();

        let karma = self
            .client
            .query_collection(COLLECTION_VOTES)?
            .into_iter()
            .filter_map(|row| {
                let post_id = from_string(&row, "post_id")?;
                if !posts.contains(&post_id) {
                    return None;
                }
                match from_string(&row, "direction").as_deref() {
                    Some("up") => Some(1_i32),
                    Some("down") => Some(-1_i32),
                    _ => Some(0_i32),
                }
            })
            .sum();

        Ok(karma)
    }
}

impl UserStore for FirestoreUserStore {
    fn get_by_id(&self, user_id: &Uuid) -> Result<Option<User>> {
        match self
            .client
            .get_document(COLLECTION_USERS, &user_id.to_string())?
        {
            Some(fields) => Ok(Some(user_from_fields(&fields)?)),
            None => Ok(None),
        }
    }

    fn get_by_username(&self, username: &str) -> Result<Option<User>> {
        // Try an exact indexed lookup first.
        for row in self.client.query_collection_filtered(
            COLLECTION_USERS,
            &[("username", username)],
            None,
        )? {
            return Ok(Some(user_from_fields(&row)?));
        }

        // Fallback for older mixed-case data.
        let target = username.to_ascii_lowercase();
        for row in self.client.query_collection(COLLECTION_USERS)? {
            if from_string(&row, "username")
                .map(|u| u.to_ascii_lowercase() == target)
                .unwrap_or(false)
            {
                return Ok(Some(user_from_fields(&row)?));
            }
        }

        Ok(None)
    }

    fn get_test_users(&self) -> Result<Vec<User>> {
        let mut users = self
            .client
            .query_collection(COLLECTION_USERS)?
            .into_iter()
            .filter(|row| from_bool(row, "is_test_user").unwrap_or(false))
            .map(|row| user_from_fields(&row))
            .collect::<Result<Vec<_>>>()?;

        users.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(users)
    }

    fn list_all(&self) -> Result<Vec<User>> {
        let mut users = self
            .client
            .query_collection(COLLECTION_USERS)?
            .into_iter()
            .map(|row| user_from_fields(&row))
            .collect::<Result<Vec<_>>>()?;

        users.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(users)
    }

    fn update_bio(&self, user_id: &Uuid, bio: &str) -> Result<()> {
        let mut fields = Map::new();
        fields.insert("bio".into(), fs_string(bio));
        self.client.set_document(
            COLLECTION_USERS,
            &user_id.to_string(),
            fields,
            Some(&["bio"]),
        )
    }

    fn create_or_update_from_github(
        &self,
        github_id: i64,
        github_login: &str,
        name: Option<&str>,
    ) -> Result<User> {
        for row in self.client.query_collection(COLLECTION_USERS)? {
            if from_int(&row, "github_id") == Some(github_id) {
                let mut user = user_from_fields(&row)?;
                let desired_name = name
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| github_login.to_string());
                if user.username != desired_name {
                    user.username = desired_name;
                    let mut fields = user_to_fields(&user);
                    fields.insert("github_id".into(), fs_int(github_id));
                    fields.insert("github_login".into(), fs_string(github_login));
                    self.client.set_document(
                        COLLECTION_USERS,
                        &user.id.to_string(),
                        fields,
                        None,
                    )?;
                }
                return Ok(user);
            }
        }

        let user = User {
            id: Uuid::new_v4(),
            username: name
                .map(|n| n.to_string())
                .unwrap_or_else(|| github_login.to_string()),
            bio: Some("New to Fido!".to_string()),
            join_date: Utc::now(),
            is_test_user: false,
            is_admin: false,
        };

        let mut fields = user_to_fields(&user);
        fields.insert("github_id".into(), fs_int(github_id));
        fields.insert("github_login".into(), fs_string(github_login));

        self.client
            .set_document(COLLECTION_USERS, &user.id.to_string(), fields, None)?;

        Ok(user)
    }
}

impl FirestoreFriendStore {
    fn link_id(a: &Uuid, b: &Uuid) -> String {
        format!("{}:{}", a, b)
    }
}

impl FriendStore for FirestoreFriendStore {
    fn is_following(&self, follower_id: &Uuid, following_id: &Uuid) -> Result<bool> {
        let id = Self::link_id(follower_id, following_id);
        Ok(self.client.get_document(COLLECTION_FRIENDS, &id)?.is_some())
    }

    fn are_mutual_friends(&self, user_a: &Uuid, user_b: &Uuid) -> Result<bool> {
        Ok(self.is_following(user_a, user_b)? && self.is_following(user_b, user_a)?)
    }

    fn follow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> Result<()> {
        let id = Self::link_id(follower_id, following_id);
        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&id));
        fields.insert("follower_id".into(), fs_string(&follower_id.to_string()));
        fields.insert("following_id".into(), fs_string(&following_id.to_string()));
        fields.insert("created_at".into(), fs_timestamp(Utc::now()));
        self.client
            .set_document(COLLECTION_FRIENDS, &id, fields, None)
    }

    fn unfollow_user(&self, follower_id: &Uuid, following_id: &Uuid) -> Result<usize> {
        let id = Self::link_id(follower_id, following_id);
        Ok(usize::from(
            self.client.delete_document(COLLECTION_FRIENDS, &id)?,
        ))
    }

    fn get_following(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let user_id_str = user_id.to_string();
        self.client
            .query_collection_filtered(
                COLLECTION_FRIENDS,
                &[("follower_id", user_id_str.as_str())],
                None,
            )?
            .into_iter()
            .filter_map(|row| from_string(&row, "following_id"))
            .map(|raw| parse_uuid(&raw, "following_id"))
            .collect()
    }

    fn get_followers(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let user_id_str = user_id.to_string();
        self.client
            .query_collection_filtered(
                COLLECTION_FRIENDS,
                &[("following_id", user_id_str.as_str())],
                None,
            )?
            .into_iter()
            .filter_map(|row| from_string(&row, "follower_id"))
            .map(|raw| parse_uuid(&raw, "follower_id"))
            .collect()
    }

    fn get_mutual_friends(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let following = self.get_following(user_id)?;
        let followers: HashSet<Uuid> = self.get_followers(user_id)?.into_iter().collect();

        Ok(following
            .into_iter()
            .filter(|id| followers.contains(id))
            .collect())
    }

    fn get_follower_count(&self, user_id: &Uuid) -> Result<usize> {
        Ok(self.get_followers(user_id)?.len())
    }

    fn get_following_count(&self, user_id: &Uuid) -> Result<usize> {
        Ok(self.get_following(user_id)?.len())
    }
}

impl ConfigStore for FirestoreConfigStore {
    fn get(&self, user_id: &Uuid) -> Result<UserConfig> {
        let maybe_fields = self
            .client
            .get_document(COLLECTION_CONFIGS, &user_id.to_string())?;

        if let Some(fields) = maybe_fields {
            let color = from_string(&fields, "color_scheme")
                .and_then(|s| ColorScheme::parse(&s))
                .unwrap_or_default();
            let sort = from_string(&fields, "sort_order")
                .and_then(|s| SortOrder::parse(&s))
                .unwrap_or_default();
            let max_posts_display = from_int(&fields, "max_posts_display").unwrap_or(25) as i32;
            let emoji_enabled = from_bool(&fields, "emoji_enabled").unwrap_or(true);

            Ok(UserConfig {
                user_id: *user_id,
                color_scheme: color,
                sort_order: sort,
                max_posts_display,
                emoji_enabled,
            })
        } else {
            Ok(UserConfig {
                user_id: *user_id,
                ..UserConfig::default()
            })
        }
    }

    fn update(&self, config: &UserConfig) -> Result<()> {
        let mut fields = Map::new();
        fields.insert("user_id".into(), fs_string(&config.user_id.to_string()));
        fields.insert(
            "color_scheme".into(),
            fs_string(config.color_scheme.as_str()),
        );
        fields.insert("sort_order".into(), fs_string(config.sort_order.as_str()));
        fields.insert(
            "max_posts_display".into(),
            fs_int(config.max_posts_display as i64),
        );
        fields.insert("emoji_enabled".into(), fs_bool(config.emoji_enabled));

        self.client.set_document(
            COLLECTION_CONFIGS,
            &config.user_id.to_string(),
            fields,
            None,
        )
    }
}

impl RateLimitStore for FirestoreRateLimitStore {
    fn get_last_post_at(&self, user_id: &Uuid) -> Result<Option<DateTime<Utc>>> {
        let maybe_doc = self
            .client
            .get_document(COLLECTION_RATE_LIMITS, &user_id.to_string())?;

        Ok(maybe_doc
            .as_ref()
            .and_then(|f| from_timestamp(f, "last_post_at")))
    }

    fn update_last_post_at(&self, user_id: &Uuid, at: DateTime<Utc>) -> Result<()> {
        let mut fields = self
            .client
            .get_document(COLLECTION_RATE_LIMITS, &user_id.to_string())?
            .unwrap_or_default();
        fields.insert("user_id".into(), fs_string(&user_id.to_string()));
        fields.insert("last_post_at".into(), fs_timestamp(at));

        self.client
            .set_document(COLLECTION_RATE_LIMITS, &user_id.to_string(), fields, None)
    }

    fn get_last_dm_at(&self, user_id: &Uuid) -> Result<Option<DateTime<Utc>>> {
        let maybe_doc = self
            .client
            .get_document(COLLECTION_RATE_LIMITS, &user_id.to_string())?;

        Ok(maybe_doc
            .as_ref()
            .and_then(|f| from_timestamp(f, "last_dm_at")))
    }

    fn update_last_dm_at(&self, user_id: &Uuid, at: DateTime<Utc>) -> Result<()> {
        let mut fields = self
            .client
            .get_document(COLLECTION_RATE_LIMITS, &user_id.to_string())?
            .unwrap_or_default();
        fields.insert("user_id".into(), fs_string(&user_id.to_string()));
        fields.insert("last_dm_at".into(), fs_timestamp(at));

        self.client
            .set_document(COLLECTION_RATE_LIMITS, &user_id.to_string(), fields, None)
    }
}

impl FirestoreDirectMessageStore {
    fn query_messages_between(
        &self,
        from_user_id: &Uuid,
        to_user_id: &Uuid,
    ) -> Result<Vec<DirectMessage>> {
        let from_user_id_str = from_user_id.to_string();
        let to_user_id_str = to_user_id.to_string();

        self.client
            .query_collection_filtered(
                COLLECTION_DMS,
                &[
                    ("from_user_id", from_user_id_str.as_str()),
                    ("to_user_id", to_user_id_str.as_str()),
                ],
                Some(("created_at", false)),
            )?
            .into_iter()
            .map(|fields| dm_from_fields(&fields))
            .collect::<Result<Vec<_>>>()
    }
}

impl DirectMessageStore for FirestoreDirectMessageStore {
    fn create(&self, dm: &DirectMessage) -> Result<()> {
        self.client
            .set_document(COLLECTION_DMS, &dm.id.to_string(), dm_to_fields(dm), None)
    }

    fn get_conversation(&self, user1_id: &Uuid, user2_id: &Uuid) -> Result<Vec<DirectMessage>> {
        let mut dms = self.query_messages_between(user1_id, user2_id)?;
        dms.extend(self.query_messages_between(user2_id, user1_id)?);

        dms.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(dms)
    }

    fn get_conversations_list(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let user_id_str = user_id.to_string();
        let mut latest_by_user = HashMap::new();

        let sent = self
            .client
            .query_collection_filtered(
                COLLECTION_DMS,
                &[("from_user_id", user_id_str.as_str())],
                Some(("created_at", true)),
            )?
            .into_iter()
            .map(|fields| dm_from_fields(&fields))
            .collect::<Result<Vec<_>>>()?;

        let received = self
            .client
            .query_collection_filtered(
                COLLECTION_DMS,
                &[("to_user_id", user_id_str.as_str())],
                Some(("created_at", true)),
            )?
            .into_iter()
            .map(|fields| dm_from_fields(&fields))
            .collect::<Result<Vec<_>>>()?;

        for dm in sent.into_iter().chain(received.into_iter()) {
            let other_user_id = if dm.from_user_id == *user_id {
                dm.to_user_id
            } else {
                dm.from_user_id
            };

            latest_by_user
                .entry(other_user_id)
                .and_modify(|latest: &mut DateTime<Utc>| {
                    if dm.created_at > *latest {
                        *latest = dm.created_at;
                    }
                })
                .or_insert(dm.created_at);
        }

        let mut out = latest_by_user.into_iter().collect::<Vec<_>>();
        out.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(out.into_iter().map(|(user_id, _)| user_id).collect())
    }

    fn mark_as_read(&self, user_id: &Uuid, other_user_id: &Uuid) -> Result<()> {
        for dm in self.query_messages_between(other_user_id, user_id)? {
            if dm.is_read {
                continue;
            }

            let mut fields = Map::new();
            fields.insert("is_read".into(), fs_bool(true));
            self.client.set_document(
                COLLECTION_DMS,
                &dm.id.to_string(),
                fields,
                Some(&["is_read"]),
            )?;
        }

        Ok(())
    }

    fn delete_conversation(&self, user_id: &Uuid, other_user_id: &Uuid) -> Result<()> {
        let mut message_ids = HashSet::new();
        for dm in self.query_messages_between(user_id, other_user_id)? {
            message_ids.insert(dm.id);
        }
        for dm in self.query_messages_between(other_user_id, user_id)? {
            message_ids.insert(dm.id);
        }

        for message_id in message_ids {
            let _ = self
                .client
                .delete_document(COLLECTION_DMS, &message_id.to_string())?;
        }
        Ok(())
    }
}

impl SessionStore for FirestoreSessionStore {
    fn create_session(
        &self,
        token: &str,
        user_id: Uuid,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        last_activity: DateTime<Utc>,
    ) -> Result<()> {
        let mut fields = Map::new();
        fields.insert("token".into(), fs_string(token));
        fields.insert("user_id".into(), fs_string(&user_id.to_string()));
        fields.insert("created_at".into(), fs_timestamp(created_at));
        fields.insert("expires_at".into(), fs_timestamp(expires_at));
        fields.insert("last_activity".into(), fs_timestamp(last_activity));

        self.client
            .set_document(COLLECTION_SESSIONS, token, fields, None)
    }

    fn get_session(&self, token: &str) -> Result<Option<SessionRecord>> {
        let doc = self.client.get_document(COLLECTION_SESSIONS, token)?;

        match doc {
            Some(fields) => {
                let user_id = parse_uuid(
                    &from_string(&fields, "user_id")
                        .ok_or_else(|| anyhow!("missing session.user_id"))?,
                    "user_id",
                )?;
                let expires_at = from_timestamp(&fields, "expires_at")
                    .ok_or_else(|| anyhow!("missing session.expires_at"))?;
                let last_activity = from_timestamp(&fields, "last_activity");
                Ok(Some(SessionRecord {
                    user_id,
                    expires_at,
                    last_activity,
                }))
            }
            None => Ok(None),
        }
    }

    fn update_activity(&self, token: &str, at: DateTime<Utc>) -> Result<()> {
        let mut fields = Map::new();
        fields.insert("last_activity".into(), fs_timestamp(at));
        self.client
            .set_document(COLLECTION_SESSIONS, token, fields, Some(&["last_activity"]))
    }

    fn delete_session(&self, token: &str) -> Result<usize> {
        Ok(usize::from(
            self.client.delete_document(COLLECTION_SESSIONS, token)?,
        ))
    }

    fn cleanup_expired_sessions(&self, now: DateTime<Utc>) -> Result<usize> {
        let mut deleted = 0_usize;
        for row in self.client.query_collection(COLLECTION_SESSIONS)? {
            let expires_at = from_timestamp(&row, "expires_at");
            if expires_at.map(|ts| ts < now).unwrap_or(false) {
                if let Some(token) = from_string(&row, "token") {
                    if self.client.delete_document(COLLECTION_SESSIONS, &token)? {
                        deleted += 1;
                    }
                }
            }
        }
        Ok(deleted)
    }

    fn invalidate_user_sessions(&self, user_id: Uuid) -> Result<usize> {
        let mut deleted = 0_usize;
        for row in self.client.query_collection(COLLECTION_SESSIONS)? {
            if from_string(&row, "user_id") != Some(user_id.to_string()) {
                continue;
            }
            if let Some(token) = from_string(&row, "token") {
                if self.client.delete_document(COLLECTION_SESSIONS, &token)? {
                    deleted += 1;
                }
            }
        }
        Ok(deleted)
    }
}

impl AuditStore for FirestoreAuditStore {
    fn log_event(
        &self,
        event_type: &str,
        user_id: Option<Uuid>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        details: Option<&str>,
        timestamp: DateTime<Utc>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();

        let mut fields = Map::new();
        fields.insert("id".into(), fs_string(&id.to_string()));
        fields.insert("event_type".into(), fs_string(event_type));
        fields.insert("timestamp".into(), fs_timestamp(timestamp));

        if let Some(user_id) = user_id {
            fields.insert("user_id".into(), fs_string(&user_id.to_string()));
        }
        if let Some(ip) = ip_address {
            fields.insert("ip_address".into(), fs_string(ip));
        }
        if let Some(agent) = user_agent {
            fields.insert("user_agent".into(), fs_string(agent));
        }
        if let Some(details) = details {
            fields.insert("details".into(), fs_string(details));
        }

        self.client
            .set_document(COLLECTION_AUDIT, &id.to_string(), fields, None)?;

        Ok(id)
    }
}

/// Validate Firestore backend prerequisites are present.
pub fn validate_firestore_env() -> Result<()> {
    let _ = FirestoreClient::from_env()?;
    Ok(())
}

/// Build Firestore-backed stores from environment configuration.
pub fn stores_from_env() -> Result<Stores> {
    let client = Arc::new(FirestoreClient::from_env()?);
    seed_emulator_test_data_if_needed(&client)?;

    Ok(Stores {
        posts: Arc::new(FirestorePostStore {
            client: Arc::clone(&client),
        }),
        hashtags: Arc::new(FirestoreHashtagStore {
            client: Arc::clone(&client),
        }),
        votes: Arc::new(FirestoreVoteStore {
            client: Arc::clone(&client),
        }),
        users: Arc::new(FirestoreUserStore {
            client: Arc::clone(&client),
        }),
        friends: Arc::new(FirestoreFriendStore {
            client: Arc::clone(&client),
        }),
        config: Arc::new(FirestoreConfigStore {
            client: Arc::clone(&client),
        }),
        rate_limits: Arc::new(FirestoreRateLimitStore {
            client: Arc::clone(&client),
        }),
        dms: Arc::new(FirestoreDirectMessageStore {
            client: Arc::clone(&client),
        }),
        sessions: Arc::new(FirestoreSessionStore {
            client: Arc::clone(&client),
        }),
        audit: Arc::new(FirestoreAuditStore { client }),
    })
}

fn seed_emulator_test_data_if_needed(client: &Arc<FirestoreClient>) -> Result<()> {
    if !client.emulator {
        return Ok(());
    }

    let seed_enabled = std::env::var("FIRESTORE_SEED_TEST_DATA")
        .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(true);
    if !seed_enabled {
        return Ok(());
    }

    let existing_test_users = client
        .query_collection(COLLECTION_USERS)?
        .into_iter()
        .filter(|f| from_bool(f, "is_test_user").unwrap_or(false))
        .count();
    if existing_test_users > 0 {
        return Ok(());
    }

    let users = vec![
        User {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001")?,
            username: "alice".to_string(),
            bio: Some("Rust enthusiast and terminal lover".to_string()),
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: true,
        },
        User {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002")?,
            username: "bob".to_string(),
            bio: Some("Terminal UI designer and developer".to_string()),
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        },
        User {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003")?,
            username: "charlie".to_string(),
            bio: Some("Database expert and systems thinker".to_string()),
            join_date: Utc::now(),
            is_test_user: true,
            is_admin: false,
        },
    ];

    for user in &users {
        client.set_document(
            COLLECTION_USERS,
            &user.id.to_string(),
            user_to_fields(user),
            None,
        )?;

        let config = UserConfig {
            user_id: user.id,
            color_scheme: ColorScheme::Dark,
            sort_order: SortOrder::Newest,
            max_posts_display: 25,
            emoji_enabled: true,
        };
        let mut config_fields = Map::new();
        config_fields.insert("user_id".into(), fs_string(&config.user_id.to_string()));
        config_fields.insert(
            "color_scheme".into(),
            fs_string(config.color_scheme.as_str()),
        );
        config_fields.insert("sort_order".into(), fs_string(config.sort_order.as_str()));
        config_fields.insert(
            "max_posts_display".into(),
            fs_int(config.max_posts_display as i64),
        );
        config_fields.insert("emoji_enabled".into(), fs_bool(config.emoji_enabled));
        client.set_document(
            COLLECTION_CONFIGS,
            &config.user_id.to_string(),
            config_fields,
            None,
        )?;
    }

    let sample_posts = vec![
        Post {
            id: Uuid::new_v4(),
            author_id: users[0].id,
            author_username: users[0].username.clone(),
            content: "Just shipped Firestore emulator support. #rust #firebase".to_string(),
            created_at: Utc::now(),
            upvotes: 4,
            downvotes: 0,
            hashtags: vec!["rust".to_string(), "firebase".to_string()],
            user_vote: None,
            parent_post_id: None,
            reply_count: 0,
            reply_to_user_id: None,
            reply_to_username: None,
        },
        Post {
            id: Uuid::new_v4(),
            author_id: users[1].id,
            author_username: users[1].username.clone(),
            content: "Testing full local stack now. #terminal #fido".to_string(),
            created_at: Utc::now(),
            upvotes: 2,
            downvotes: 0,
            hashtags: vec!["terminal".to_string(), "fido".to_string()],
            user_vote: None,
            parent_post_id: None,
            reply_count: 0,
            reply_to_user_id: None,
            reply_to_username: None,
        },
    ];

    for post in sample_posts {
        client.set_document(
            COLLECTION_POSTS,
            &post.id.to_string(),
            post_to_fields(&post),
            None,
        )?;
        for hashtag in &post.hashtags {
            let tag = hashtag.to_ascii_lowercase();
            let tag_doc_id = format!("{}:{}", post.id, tag);
            let mut tag_fields = Map::new();
            tag_fields.insert("id".into(), fs_string(&tag_doc_id));
            tag_fields.insert("post_id".into(), fs_string(&post.id.to_string()));
            tag_fields.insert("hashtag".into(), fs_string(&tag));
            tag_fields.insert("created_at".into(), fs_timestamp(Utc::now()));
            client.set_document(COLLECTION_POST_HASHTAGS, &tag_doc_id, tag_fields, None)?;
        }
    }

    tracing::info!("Seeded Firestore emulator with default test users and posts");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_document_roundtrip() {
        let post = Post {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            author_username: "alice".to_string(),
            content: "hello #rust".to_string(),
            created_at: Utc::now(),
            upvotes: 3,
            downvotes: 1,
            hashtags: vec!["rust".to_string()],
            user_vote: None,
            parent_post_id: None,
            reply_count: 2,
            reply_to_user_id: None,
            reply_to_username: None,
        };

        let fields = post_to_fields(&post);
        let parsed = post_from_fields(&fields).expect("post parse failed");

        assert_eq!(parsed.id, post.id);
        assert_eq!(parsed.author_id, post.author_id);
        assert_eq!(parsed.author_username, post.author_username);
        assert_eq!(parsed.content, post.content);
        assert_eq!(parsed.upvotes, post.upvotes);
        assert_eq!(parsed.downvotes, post.downvotes);
        assert_eq!(parsed.reply_count, post.reply_count);
    }

    #[test]
    fn test_user_document_roundtrip() {
        let user = User {
            id: Uuid::new_v4(),
            username: "dev".to_string(),
            bio: Some("bio".to_string()),
            join_date: Utc::now(),
            is_test_user: false,
            is_admin: true,
        };

        let fields = user_to_fields(&user);
        let parsed = user_from_fields(&fields).expect("user parse failed");

        assert_eq!(parsed.id, user.id);
        assert_eq!(parsed.username, user.username);
        assert_eq!(parsed.bio, user.bio);
        assert_eq!(parsed.is_admin, user.is_admin);
    }
}
