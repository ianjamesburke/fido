# Fido Quick Start

Fido is a terminal community app for GitHub repositories. The directory you
launch from decides the first screen.

## Install

```bash
cargo install fido
```

By default the TUI connects to the public Fido server. Override it with
`FIDO_SERVER_URL`, `fido --server <url>`, or `~/.fido/server_url`.

## Launch From A Repo

```bash
cd path/to/github/repo
fido
```

If the nearest git repo has a GitHub `origin`, Fido joins that repo community
and opens its board. The community is created server-side on first visit.

Launch anywhere else to start in Home:

```bash
fido
```

Home lists the repo communities you have joined. Press `Enter` on a community
to open its board, and `Esc` to return to Home.

## Sign In

On first launch, choose one of:

- GitHub OAuth: select "Login with GitHub", open or copy the shown URL, approve
  the app, then return to the terminal.
- Test user: select `alice`, `bob`, or `charlie` for a quick demo without
  GitHub.

Fido stores your session at `~/.fido/session` with user-only file permissions.
Press `Shift+L` from the main app to logout.

## Core Workflows

### Repo Board

- `n` opens the post composer.
- `Enter` opens the selected thread.
- `u` and `d` vote on the selected thread.
- `p` opens the selected author's profile.
- `o` opens a synced GitHub issue or pull request in your browser.
- `i` opens community settings.
- `a` opens the thread approval queue when you are a community admin.

Posts and replies are scoped to the current repo community.

### Chat

Press `Tab` to move to Chat. Type to focus the input, then press `Enter` to
send to the community channel. Messages update live for other connected users.

### Direct Messages

Press `Tab` to move to DMs.

- `N` or `Enter` on the empty/new row starts a conversation.
- Type a username and press `Enter` to send the first message.
- Shared-community members and mutual follows auto-accept.
- Strangers create a pending request.
- Incoming requests appear in the DMs list; press `a` to accept or `x` to
  decline.

### Notifications

Press `v` from navigation mode to open notifications.

- `Enter` opens the selected notification source and marks it read.
- `a` marks all notifications read.
- `v` or `Esc` closes the panel.

Notifications cover DM requests, replies, mentions, and pending-thread updates.

### Browse Communities

Press `b` to browse starred GitHub repositories. `Enter` opens or joins the
selected repo community. `r` reloads the list.

## Controls

- `Tab` / `Shift+Tab` - Switch tabs
- `j/k` or arrows - Navigate
- `Enter` - Open selected item or submit focused input
- `Esc` - Back, clear input, close modal, or quit from Home
- `?` - Help
- `q` - Quit
- `Shift+L` - Logout
- `s` - Search users from navigation mode

Tab order is Board/Home, Chat, DMs, Profile, Settings.

## Local Development

Run the server locally:

```bash
just server
```

Run the TUI against it:

```bash
just tui-local
```

Or use Cargo directly:

```bash
cargo run --bin fido-server
cargo run --bin fido -- --server http://localhost:3000
```

Required server environment for GitHub auth:

- `GITHUB_CLIENT_ID`
- `FIDO_TOKEN_KEY`

Useful optional environment:

- `DATABASE_PATH`
- `FIDO_SERVER_URL`
- `ALLOWED_ORIGINS`
- `RUST_LOG`

## Local Testing

```bash
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
just e2e-tui
```

`just e2e-tui` builds the real binaries, starts a temporary local server with a
stubbed GitHub API, and drives the TUI in tmux. It covers repo launch, Home
mode, posting, community settings, chat, approval queue, two-user DM requests,
notifications, and request acceptance.

## Troubleshooting

### Fido Opened Home Instead Of My Repo

- Check that you launched inside a git repo.
- Check that `git remote get-url origin` points at `github.com`.
- Non-GitHub remotes intentionally fall back to Home.

### Cannot Connect To Server

```bash
curl https://fido-web-production.up.railway.app/health
```

If using a local server, verify it is running on the URL you passed to
`fido --server`.

### OAuth Browser Did Not Open

Copy the URL shown in the terminal and paste it into your browser. After
approval, return to the TUI.

### Session Looks Stale

Press `Shift+L` to logout and sign in again. You can also remove the local
session file:

```bash
rm -f ~/.fido/session
```

## Release Commands

```bash
just prerelease-check
just deploy-cargo
```

`just deploy-cargo` publishes `fido-types` first, waits for crates.io indexing,
then publishes the `fido` TUI crate.
