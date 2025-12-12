# Fido Quick Start Guide

## Get Started in 2 Steps

### Step 1: Install Fido

```bash
cargo install fido
```

This installs the Fido TUI client. The client automatically connects to the production server at `https://fido-social.fly.dev` - no server setup required!

---

### Step 2: Launch and Authenticate

```bash
fido
```

**First-time authentication:**

You'll see an authentication screen with two options:

#### Option A: GitHub OAuth (Recommended)

1. Select "Login with GitHub"
2. Your browser will open to GitHub's OAuth authorization page
3. Click "Authorize" to grant Fido access to your GitHub identity
4. Return to your terminal - you're logged in!

**What happens during OAuth:**
- Fido requests your GitHub username and ID (no repo access)
- The server creates a Fido account linked to your GitHub identity
- A session token is generated and saved to `~/.fido/session`
- Your session persists for 30 days or until you logout

**If your browser doesn't open:**
- Copy the URL shown in the terminal
- Paste it into your browser manually
- Complete the authorization
- The TUI will detect the successful login automatically

#### Option B: Test Users (For Testing)

For quick testing without GitHub:
1. Select one of the test users (alice, bob, or charlie)
2. Press `Enter` to login
3. Explore the app with pre-populated test data

**Note:** Test users are shared across all Fido instances and are for demonstration purposes only.

---

## Session Management

### Session Persistence

Your authentication session is stored locally at `~/.fido/session` with secure file permissions (0600). This means:

- You stay logged in across TUI restarts
- No need to re-authenticate every time
- Session expires after 30 days of inactivity
- Only you can read the session file

### Logout

Press `Shift+L` at any time to logout. This will:
- Invalidate your session on the server
- Delete your local session file
- Return you to the authentication screen

### Multiple Devices

You can login to Fido from multiple devices simultaneously. Each device gets its own session token.

---

## Server Configuration

### Default (Production)

By default, Fido connects to: `https://fido-social.fly.dev`

### Local Development

To connect to a local server for development:

**Using environment variable:**
```bash
FIDO_SERVER_URL=http://localhost:3000 fido
```

**Using CLI flag:**
```bash
fido --server http://localhost:3000
```

**Check current server:**
- Switch to the Settings tab in Fido TUI
- The current server URL and type will be displayed in the settings

**Configuration priority (highest to lowest):**
1. CLI argument: `--server <URL>`
2. Environment variable: `FIDO_SERVER_URL=<URL>`
3. Saved configuration file: `~/.fido/server_config.json`
4. Default: `https://fido-social.fly.dev` (production) or `http://127.0.0.1:3000` (web mode)
- The current server URL is displayed at the top

---

## Quick Controls Reference

### Global Navigation
- `Tab` - Next tab (Posts → DMs → Profile → Settings)
- `Shift+Tab` - Previous tab
- `?` - Help (shows all shortcuts)
- `q` or `Esc` - Quit
- `Shift+L` - Logout

### Posts Tab
- `j` / `↓` - Next post
- `k` / `↑` - Previous post
- `u` - Upvote selected post
- `d` - Downvote selected post
- `n` - Create new post
- `r` - Refresh posts

### Creating a Post
1. Press `n` to open new post modal
2. Type your message (max 280 characters)
3. Use hashtags: `#rust #terminal #fido`
4. Use emoji shortcodes: `:smile: :rocket: :fire:`
5. Press `Ctrl+Enter` to submit
6. Press `Esc` to cancel

### DMs Tab
- `j` / `↓` - Next conversation
- `k` / `↑` - Previous conversation
- Type message in input field
- `Ctrl+Enter` - Send message

### Profile Tab
- `j` / `↓` - Navigate your posts
- `k` / `↑` - Navigate your posts
- `e` - Edit bio

### Settings Tab
- `j` / `↓` - Next setting
- `k` / `↑` - Previous setting
- `←` / `→` / `Enter` - Change value
- Type numbers for "Max Posts Display"
- `s` - Save settings

---

## Try These Features

### 1. Create Your First Post
```
Press 'n' → Type:
"Hello Fido! Testing this terminal app. #rust #terminal"
→ Press Ctrl+Enter
```

### 2. Vote on Posts
```
Navigate to any post → Press 'u' to upvote
Watch the vote count increase!
```

### 3. Send a Direct Message
```
Switch to DMs tab → Select a conversation → Type a message → Ctrl+Enter
```

### 4. Edit Your Profile
```
Switch to Profile tab → Press 'e' → Update your bio → Ctrl+Enter
```

### 5. Change Settings
```
Switch to Settings tab → Navigate with j/k → Change values → Press 's' to save
```

---

## Understanding Test Users

If you login with a test user, you'll see:

### Test Users
- **alice** - Rust enthusiast (Default theme, 25 posts max)
- **bob** - UI designer (Dark theme, 50 posts max)
- **charlie** - Database expert (Solarized theme, 30 posts max)

### Sample Content
- Pre-loaded posts with hashtags and emojis
- Direct messages across conversations
- Various vote counts demonstrating the voting system
- Different user configurations

**Note:** Test users are shared across all Fido instances. For a personal experience, use GitHub OAuth.

---

## Troubleshooting

### Connection Issues

**Problem:** "Cannot connect to server"

**Solutions:**
1. Check your internet connection
2. Verify the server is up: `curl https://fido-social.fly.dev/health`
3. Wait a moment and try again (server may be starting up)
4. Check if you're behind a firewall or proxy

### Authentication Issues

**Problem:** Browser won't open for GitHub OAuth

**Solutions:**
1. Copy the URL from the terminal and paste it into your browser manually
2. Ensure you have a default browser configured
3. Check that your system allows the app to open URLs

**Problem:** "Session expired" or "Invalid session"

**Solutions:**
1. Your session may have expired (30 days)
2. Press `Shift+L` to logout and login again
3. Delete `~/.fido/session` and restart Fido
4. Check file permissions: `ls -la ~/.fido/session` (should be `-rw-------`)

**Problem:** OAuth authorization fails

**Solutions:**
1. Ensure you clicked "Authorize" on the GitHub page
2. Check that you're logged into GitHub
3. Try the OAuth flow again
4. Check server logs if the issue persists

### Session File Issues

**Problem:** "Cannot create session file"

**Solutions:**
1. Check that `~/.fido/` directory exists and is writable
2. Verify file permissions: `ls -la ~/.fido/`
3. Manually create directory: `mkdir -p ~/.fido && chmod 700 ~/.fido`
4. Check disk space: `df -h ~`

### Display Issues

**Problem:** Emojis or special characters not displaying correctly

**Solutions:**
- Use a modern terminal (Windows Terminal, iTerm2, Alacritty, etc.)
- Ensure terminal supports UTF-8 encoding
- Try a different font (Cascadia Code, Fira Code, JetBrains Mono)
- Update your terminal emulator to the latest version

### Local Development Issues

**Problem:** Can't connect to local server

**Solutions:**
1. Ensure local server is running: `cargo run --bin fido-server`
2. Verify server URL: `fido --server http://localhost:3000`
3. Check server logs for errors
4. Ensure port 3000 is not blocked by firewall

---

## What to Test

### Core Workflows
- [ ] Login with different users
- [ ] Create posts with hashtags
- [ ] Vote on posts (upvote/downvote)
- [ ] Send direct messages
- [ ] Edit your bio
- [ ] Change settings and verify they persist
- [ ] Logout and login again

### Edge Cases
- [ ] Create post with exactly 280 characters
- [ ] Try to create empty post (should not submit)
- [ ] Use emoji shortcodes (`:smile:` → 😊)
- [ ] Test keyboard navigation in all tabs
- [ ] Switch between users in multiple TUI instances
- [ ] Use CLI to send DMs

### Performance
- [ ] Scroll through all posts quickly
- [ ] Switch tabs rapidly
- [ ] Open/close modals multiple times
- [ ] Navigate with j/k held down

---

## Next Steps

After getting started:

1. **Explore the full README**: Check out all features and documentation links
2. **Read the architecture docs**: `ARCHITECTURE.md` - Understand how Fido works
3. **Check deployment guide**: `DEPLOYMENT.md` - Learn about the production setup
4. **Contribute**: `CONTRIBUTING.md` - Help improve Fido!

---

## You're Ready

Fido is a fully functional terminal social platform.

**Remember:**
- Your session persists across restarts (stored in `~/.fido/session`)
- Press `?` anytime for help and keyboard shortcuts
- Press `Shift+L` to logout
- Use `fido --server <URL>` to connect to a different server
- Multiple TUI instances can run simultaneously
