# fido-server/src/services

## Purpose

Business operations for communities, chat, posts, DMs, profiles, notifications, activity, and GitHub.

## Ownership

Each module owns one domain operation family. `github.rs` owns GitHub API access and encrypted token handling.

## Local Contracts

- Services coordinate repositories and external APIs. They do not construct HTTP responses.
- Keep GitHub API calls behind `GithubService` and preserve its configured API base for fixture-backed tests.
- Enforce domain authorization before writes and emit activity or notifications through the existing service paths.
- Handle token encryption and decryption only through the existing GitHub token helpers.

## Verification

```bash
cargo test -p fido-server --features sqlite-tests
cargo test --test e2e_community_rewrite -p fido-server
```

## Child DOX Index

No child instruction files.
