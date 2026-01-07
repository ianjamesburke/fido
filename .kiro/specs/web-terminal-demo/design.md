# Design Document: Web Terminal Demo

## Overview

The Web Terminal Demo provides an interactive, browser-based experience of Fido's terminal UI. It uses **ttyd** (a web terminal server) to expose the native fido-tui binary through a browser, with a mock backend providing ephemeral in-memory data. This approach requires minimal code changes—just adding a MockBackend that's enabled via environment variable.

### Key Design Decisions

1. **ttyd approach**: Run the native fido-tui binary through ttyd web terminal server—no WASM compilation needed
2. **Environment-based switching**: Use `FIDO_DEMO_MODE=true` to switch between real API and mock backend
3. **In-memory data store**: All demo data lives in Rust structs, reset on each new terminal session
4. **Docker deployment**: Package ttyd + fido-tui in a container, deploy alongside main app on Fly.io

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Browser                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    demo.html                               │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │              ttyd web terminal                       │  │  │
│  │  │         (WebSocket connection to server)             │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ WebSocket
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Docker Container (Fly.io)                     │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                        ttyd                                │  │
│  │              (web terminal server)                         │  │
│  │                          │                                 │  │
│  │                          ▼                                 │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │              fido-tui (native binary)                │  │  │
│  │  │         FIDO_DEMO_MODE=true                          │  │  │
│  │  │  ┌─────────────────────────────────────────────┐    │  │  │
│  │  │  │              MockBackend                     │    │  │  │
│  │  │  │         (in-memory, ephemeral)               │    │  │  │
│  │  │  └─────────────────────────────────────────────┘    │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│                    NO connection to production DB                │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. User opens demo page in browser
2. Browser connects to ttyd via WebSocket
3. ttyd spawns fido-tui process with `FIDO_DEMO_MODE=true`
4. fido-tui detects demo mode and uses MockBackend instead of ApiClient
5. User interacts with terminal normally
6. All data operations go to in-memory MockBackend
7. When user closes tab, ttyd terminates the process (data is lost)

## Components and Interfaces

### 1. MockBackend

A self-contained mock implementation that mirrors the ApiClient interface. Instead of making HTTP requests, it operates on in-memory data structures:

```rust
// fido-tui/src/api/mock_backend.rs
use std::sync::{Arc, Mutex};
use fido_types::*;
use uuid::Uuid;

pub struct MockBackend {
    data: Arc<Mutex<MockData>>,
    current_user: Option<User>,
    session_token: Option<String>,
}

struct MockData {
    users: Vec<User>,
    posts: Vec<Post>,
    messages: Vec<DirectMessage>,
    votes: Vec<(Uuid, Uuid, String)>, // (user_id, post_id, direction)
    followed_hashtags: Vec<(Uuid, String)>, // (user_id, hashtag)
    configs: Vec<(Uuid, UserConfig)>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(MockData::with_sample_data())),
            current_user: None,
            session_token: None,
        }
    }
    
    // Implement same methods as ApiClient but operating on in-memory data
    pub async fn get_test_users(&self) -> ApiResult<Vec<User>> {
        let data = self.data.lock().unwrap();
        Ok(data.users.clone())
    }
    
    pub async fn login(&mut self, username: String) -> ApiResult<LoginResponse> {
        let data = self.data.lock().unwrap();
        let user = data.users.iter()
            .find(|u| u.username == username)
            .cloned()
            .ok_or(ApiError::NotFound("User not found".into()))?;
        
        let token = format!("demo-token-{}", Uuid::new_v4());
        self.current_user = Some(user.clone());
        self.session_token = Some(token.clone());
        
        Ok(LoginResponse { user, session_token: token })
    }
    
    // ... similar implementations for all other methods
}
```

### 2. Sample Data Generator

Pre-populates the mock backend with realistic demo content:

```rust
// fido-tui/src/api/sample_data.rs
impl MockData {
    pub fn with_sample_data() -> Self {
        let users = create_test_users();
        let posts = create_sample_posts(&users);
        let messages = create_sample_conversations(&users);
        
        Self {
            users,
            posts,
            messages,
            votes: Vec::new(),
            followed_hashtags: Vec::new(),
            configs: Vec::new(),
        }
    }
}

fn create_test_users() -> Vec<User> {
    vec![
        User { username: "demo_user".into(), bio: Some("Welcome to Fido!".into()), .. },
        User { username: "alice".into(), bio: Some("Rust enthusiast".into()), .. },
        User { username: "bob".into(), bio: Some("Terminal lover".into()), .. },
        User { username: "charlie".into(), bio: Some("Open source contributor".into()), .. },
    ]
}

fn create_sample_posts(users: &[User]) -> Vec<Post> {
    vec![
        Post { 
            author_username: "alice".into(),
            content: "Just discovered #fido - this terminal social network is amazing! 🦀".into(),
            upvotes: 42, downvotes: 2,
            hashtags: vec!["fido".into()],
            ..
        },
        // ... more sample posts with hashtags, replies, varying votes
    ]
}
```

### 3. Demo Mode Detection

Simple environment variable check at startup:

```rust
// fido-tui/src/main.rs (modified)
fn main() {
    let demo_mode = std::env::var("FIDO_DEMO_MODE").is_ok();
    
    if demo_mode {
        // Use MockBackend
        let backend = MockBackend::new();
        run_app_with_mock(backend);
    } else {
        // Use real ApiClient (existing behavior)
        let client = ApiClient::default();
        run_app(client);
    }
}
```

### 4. Demo Dockerfile

Separate Dockerfile for the demo service:

```dockerfile
# Dockerfile.demo
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p fido-tui

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ttyd && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/fido-tui /usr/local/bin/

ENV FIDO_DEMO_MODE=true
EXPOSE 7681

CMD ["ttyd", "-p", "7681", "-W", "fido-tui"]
```

### 5. Demo Landing Page

Simple HTML page that embeds the ttyd terminal:

```html
<!-- web/demo.html -->
<!DOCTYPE html>
<html>
<head>
    <title>Fido Demo - Try it in your browser</title>
    <style>
        body { margin: 0; background: #1a1a2e; }
        #terminal { width: 100vw; height: 100vh; }
        .demo-banner {
            background: #ff6b6b;
            color: white;
            text-align: center;
            padding: 8px;
            font-family: monospace;
        }
    </style>
</head>
<body>
    <div class="demo-banner">
        🎮 DEMO MODE - All data is temporary and will be lost on refresh
    </div>
    <iframe id="terminal" src="/ttyd/" frameborder="0"></iframe>
</body>
</html>
```

## Data Models

### MockData Structure

```rust
struct MockData {
    // Core entities
    users: Vec<User>,             // Test users (demo_user, alice, bob, charlie)
    posts: Vec<Post>,             // Sample posts with hashtags and replies
    messages: Vec<DirectMessage>, // Pre-populated DM conversations
    
    // User-specific state (stored as tuples for simplicity)
    votes: Vec<(Uuid, Uuid, String)>,      // (user_id, post_id, direction)
    followed_hashtags: Vec<(Uuid, String)>, // (user_id, hashtag)
    configs: Vec<(Uuid, UserConfig)>,       // (user_id, config)
}
```

### Sample Data Specifications

| Entity | Count | Details |
|--------|-------|---------|
| Test Users | 4 | demo_user (default), alice, bob, charlie |
| Sample Posts | 15-20 | Mix of hashtags (#rust, #terminal, #fido), varying votes |
| Sample Replies | 5-10 | Threaded under some posts |
| Sample DMs | 3 conversations | Pre-populated with alice, bob |
| Hashtags | 5 | #rust, #terminal, #fido, #coding, #opensource |

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Test user authentication succeeds
*For any* test user in the mock backend's user list, logging in with that username SHALL result in a successful authentication with a valid session token.
**Validates: Requirements 1.4**

### Property 2: Post sort order consistency
*For any* sort order configuration (newest, oldest, top), the posts returned by the mock backend SHALL be ordered according to that sort criteria.
**Validates: Requirements 2.3**

### Property 3: Hashtag filter correctness
*For any* hashtag filter applied, all posts returned by the mock backend SHALL contain that hashtag in their hashtags list.
**Validates: Requirements 2.4**

### Property 4: Post creation round-trip
*For any* valid post content, creating a post and then fetching posts SHALL include the newly created post with matching content and author.
**Validates: Requirements 3.1**

### Property 5: Hashtag extraction accuracy
*For any* post content containing hashtags (words prefixed with #), the mock backend SHALL extract all hashtags and store them in the post's hashtags field.
**Validates: Requirements 3.2**

### Property 6: Vote count consistency
*For any* post and vote action (upvote/downvote), the post's vote count SHALL change by exactly 1 in the appropriate direction, and voting again SHALL toggle or change the vote.
**Validates: Requirements 3.3**

### Property 7: Reply parent linkage integrity
*For any* reply created, the reply's parent_post_id SHALL reference a valid existing post, and fetching replies for that parent SHALL include the new reply.
**Validates: Requirements 3.4**

### Property 8: Message send round-trip
*For any* message sent to a test user, fetching the conversation with that user SHALL include the sent message with correct content and timestamps.
**Validates: Requirements 4.2**

### Property 9: Message chronological ordering
*For any* conversation, messages returned by the mock backend SHALL be ordered by created_at timestamp in ascending order.
**Validates: Requirements 4.3**

### Property 10: Conversation creation
*For any* test user, starting a new conversation (sending first message) SHALL create a conversation that appears in the conversations list.
**Validates: Requirements 4.4**

### Property 11: Error message safety
*For any* error condition in the mock backend, the error message displayed SHALL not contain internal system paths, stack traces, or implementation details.
**Validates: Requirements 8.4**

## Error Handling

### Error Categories

| Category | Handling Strategy |
|----------|-------------------|
| Invalid input | Return user-friendly validation error |
| Not found | Return "not found" error with entity type |
| Auth required | Return "please log in" message |
| Internal error | Log to console, show generic "something went wrong" |

### Error Display

```rust
pub enum DemoError {
    NotFound(String),      // "Post not found"
    InvalidInput(String),  // "Post content cannot be empty"
    NotAuthenticated,      // "Please log in to continue"
    Internal,              // "Something went wrong. Please refresh."
}

impl DemoError {
    pub fn user_message(&self) -> &str {
        match self {
            Self::NotFound(entity) => &format!("{} not found", entity),
            Self::InvalidInput(msg) => msg,
            Self::NotAuthenticated => "Please log in to continue",
            Self::Internal => "Something went wrong. Please refresh the page.",
        }
    }
}
```

## Testing Strategy

### Dual Testing Approach

This feature requires both unit tests and property-based tests:

- **Unit tests**: Verify specific examples, edge cases, and integration points
- **Property-based tests**: Verify universal properties hold across all inputs

### Property-Based Testing

**Library**: `proptest` (Rust's standard PBT library)

**Configuration**: Each property test runs minimum 100 iterations.

**Test Annotation Format**: Each test is tagged with:
```rust
// **Feature: web-terminal-demo, Property {N}: {property_text}**
```

### Unit Tests

| Test Area | Coverage |
|-----------|----------|
| MockBackend initialization | Sample data is correctly populated |
| Login flow | Test user login succeeds, invalid user fails |
| Post CRUD | Create, read, vote, reply operations |
| DM operations | Send message, fetch conversation |
| Hashtag extraction | Various hashtag formats parsed correctly |
| Sort ordering | Each sort type returns correct order |

### Property Tests

| Property | Generator Strategy |
|----------|-------------------|
| Post creation round-trip | Generate random valid post content |
| Hashtag extraction | Generate strings with 0-5 hashtags |
| Vote consistency | Generate random post selection and vote sequences |
| Message ordering | Generate conversations with random timestamps |
| Filter correctness | Generate posts with random hashtags, apply random filters |

## Deployment

### Single Deployment Architecture

Both the main app and demo run in the same container, with nginx routing requests:

```
┌─────────────────────────────────────────────────────────────┐
│                    Fly.io Container                          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                     nginx (:8080)                    │    │
│  │                          │                           │    │
│  │         ┌────────────────┴────────────────┐         │    │
│  │         │                                  │         │    │
│  │         ▼                                  ▼         │    │
│  │  /demo/* → ttyd (:7681)      /* → fido-server (:3000)│   │
│  │         │                                  │         │    │
│  │         ▼                                  ▼         │    │
│  │  fido-tui (DEMO_MODE)              SQLite DB         │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Updated Dockerfile

```dockerfile
# Dockerfile (updated to include demo support)
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p fido-server -p fido-tui

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y ttyd nginx supervisor curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/fido-server /usr/local/bin/
COPY --from=builder /app/target/release/fido-tui /usr/local/bin/
COPY nginx.conf /etc/nginx/nginx.conf
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf
COPY web/ /var/www/html/

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080

CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
```

### nginx Configuration

```nginx
# nginx.conf
events { worker_connections 1024; }

http {
    include /etc/nginx/mime.types;
    
    upstream fido_server {
        server 127.0.0.1:3000;
    }
    
    server {
        listen 8080;
        root /var/www/html;
        
        # Health check endpoint
        location = /health {
            return 200 'OK';
            add_header Content-Type text/plain;
        }
        
        # Landing page with embedded demo at root (exact match)
        location = / {
            try_files /index.html =404;
        }
        
        # Static assets (CSS, JS, images)
        location ~* \.(css|js|png|jpg|ico|svg)$ {
            try_files $uri =404;
        }
        
        # ttyd terminal - handles both HTTP and WebSocket with proper path rewriting
        # Uses regex to strip /ttyd prefix and pass remainder to ttyd
        location ~ ^/ttyd(.*)$ {
            proxy_http_version 1.1;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_read_timeout 1d;  # Long timeout for terminal sessions
            proxy_pass http://127.0.0.1:7681$1;
            
            # Security headers for iframe embedding
            add_header Content-Security-Policy "frame-ancestors 'self';" always;
            add_header X-Frame-Options "SAMEORIGIN" always;
        }
        
        # API routes go to fido-server (catch-all after specific routes)
        location / {
            proxy_pass http://fido_server;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
        }
    }
}
```

**Routing summary:**
- `GET /` → `web/index.html` (landing page with embedded terminal)
- `GET /health` → Health check endpoint for Fly.io
- `GET /style.css`, `/script.js`, etc. → static files
- `/ttyd/*` → ttyd WebSocket terminal with proper path rewriting
- Everything else (`/posts`, `/auth/*`, etc.) → fido-server API

**Security notes:**
- CSP header restricts iframe embedding to same origin
- X-Frame-Options prevents clickjacking
- Long proxy timeout (1 day) prevents idle disconnects

### Supervisor Configuration

```ini
# supervisord.conf
[supervisord]
nodaemon=true
user=root

[program:fido-server]
command=/usr/local/bin/fido-server
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0

[program:ttyd]
# Flags explained:
# -p 7681: Listen on port 7681
# -o: Once mode - each connection gets a fresh process, exits on disconnect
# -O: Check origin - prevents cross-origin WebSocket hijacking
# -b /ttyd: Base path for reverse proxy routing
# -m 10: Max clients limit to prevent resource exhaustion
command=/usr/bin/ttyd -p 7681 -o -O -b /ttyd -m 10 /usr/local/bin/fido-tui
environment=FIDO_DEMO_MODE="true"
autostart=true
autorestart=true
stdout_logfile=/var/log/ttyd.log
stdout_logfile_maxbytes=10MB
stdout_logfile_backups=3
stderr_logfile=/var/log/ttyd_err.log
stderr_logfile_maxbytes=10MB
stderr_logfile_backups=3

[program:nginx]
command=/usr/sbin/nginx -g "daemon off;"
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0
```

**ttyd flags explained:**
- `-o` (once): Each client gets isolated session, process exits on disconnect (supervisor restarts)
- `-O` (check-origin): Prevents cross-origin WebSocket hijacking attacks
- `-b /ttyd`: Base path matching nginx reverse proxy configuration
- `-m 10`: Limits concurrent clients to prevent resource exhaustion

### fly.toml Update

```toml
# fly.toml (updated)
app = "fido-social"
primary_region = "iad"

[build]
  dockerfile = "Dockerfile"

[http_service]
  internal_port = 8080  # Changed from 3000 to nginx port
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 0

[checks]
  [checks.health]
    grace_period = "10s"
    interval = "30s"
    method = "GET"
    path = "/health"
    port = 8080
    timeout = "5s"
    type = "http"

[[vm]]
  cpu_kind = "shared"
  cpus = 1
  memory_mb = 1024  # Increased for running multiple services + ttyd sessions
```

**Resource considerations:**
- Memory increased to 1024MB to handle nginx + fido-server + multiple ttyd sessions
- Each ttyd session spawns a fido-tui process (~20-50MB each)
- Max 10 concurrent demo sessions limits memory usage
- Health check ensures Fly.io can verify container is serving traffic