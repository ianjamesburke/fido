# Rate Limiting in Fido

## Overview

Fido implements multi-layer rate limiting to prevent API abuse and spam:

1. **Per-user post rate limiting** (database-backed)
2. **Global API rate limiting** (in-memory)

## Post Rate Limiting

**Limit:** 1 post or reply per 10 minutes per user

**Applies to:**
- `POST /posts` - Creating new posts
- `POST /posts/:id/reply` - Creating replies

**Implementation:**
- Tracked in `post_rate_limits` table
- Stores `last_post_at` timestamp per user
- Returns `429 Too Many Requests` with time remaining

**Error Response:**
```json
{
  "error": "Rate limit exceeded. Please wait 8m 32s before posting again."
}
```

## Global API Rate Limiting

**Limit:** 100 requests per minute per authenticated user

**Applies to:** All API endpoints

**Implementation:**
- In-memory sliding window
- Tracks requests per session token
- Automatically cleans up old entries

**Error Response:**
```json
{
  "error": "Rate limit exceeded. Try again in 45 seconds."
}
```

## Cleanup Tool

Use `./cleanup_spam.sh` to manage spam:

```bash
./cleanup_spam.sh fido.db
```

**Features:**
- View top posters in last 24 hours
- Show posts from specific user
- Delete all posts from a user
- Delete user completely (with confirmation)
- View database statistics

## Adjusting Rate Limits

### Post Rate Limit

Edit `fido-server/src/api/posts.rs`:

```rust
let rate_limit_duration = Duration::minutes(10); // Change this value
```

### Global API Rate Limit

Edit `fido-server/src/main.rs`:

```rust
let rate_limiter = RateLimiter::new(100, 60); // (requests, seconds)
```

## Testing Rate Limits

```bash
# Test post rate limit
curl -X POST http://localhost:8080/posts \
  -H "X-Session-Token: your-token" \
  -H "Content-Type: application/json" \
  -d '{"content": "Test post"}'

# Immediately try again (should fail)
curl -X POST http://localhost:8080/posts \
  -H "X-Session-Token: your-token" \
  -H "Content-Type: application/json" \
  -d '{"content": "Another test"}'
```

## Future Enhancements

- IP-based rate limiting for unauthenticated endpoints
- Configurable rate limits via settings file
- Redis-backed rate limiting for distributed deployments
- Different rate limits for different user tiers
