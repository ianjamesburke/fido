# Fido

A Rust based terminal social platform for developers reminicent of the BBS days

Built for the Kiroween 2025.

![Fido](assets/Screenshot%202025-12-07%20at%209.43.19 PM.png)

![Fido](assets/Screenshot%202025-12-07%20at%209.53.56 PM.png)

## What is it?

Fido is a social network that lives in your terminal. Think Twitter, but keyboard-driven and without the noise. Post updates, chat with other developers, upvote good content, downvote lame content.

## Live Demo Here
https://fido-social.fly.dev/


## Installation

### Option 1: Install from crates.io

First, make sure you have [Rust](https://rustup.rs/) installed

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Fido
cargo install fido
```

### Option 2: Build from source

```bash
git clone https://github.com/ianburke/fido.git
cd fido
cargo build --release
```

## Quick Start

Launch it:
```bash
fido
```

You'll see an auth screen. Login with GitHub (your browser will open) or pick a test user to try it out.

That's it. Press `?` for help, `Tab` to switch tabs, `n` to post, `q` to quit.

Your session saves to `~/.fido/session`. Press `Shift+L` to logout.

See [QUICKSTART.md](QUICKSTART.md) for more details.

## Features

- **Keyboard-driven** - `j/k` to navigate, `u/d` to vote, `n` to post
- **Direct messages** - Private conversations with other users
- **GitHub auth** - Login with your GitHub account
- **Customizable** - Themes, sorting, display preferences
- **Fast** - Terminal-native, no web bloat

## Key Controls

- `Tab` - Switch tabs
- `j/k` or arrows - Navigate
- `u/d` - Upvote/Downvote
- `n` - New post
- `?` - Help
- `q` - Quit

## Development Scripts

### Web Terminal Demo

Launch the full web-based terminal demo (includes nginx, ttyd, and server):

```bash
./start.sh
```

This script:
- Builds the Rust binaries if needed
- Starts the Fido API server on port 3000
- Runs ttyd (terminal-over-web) on port 7681  
- Configures nginx as a reverse proxy on port 8080
- Provides a web interface at http://localhost:8080

The script works in both local development and Docker environments.

### Publishing to Crates.io

Publish new versions to crates.io with version checking:

```bash
./publish.sh
```

This script:
- Validates version consistency across workspace crates
- Checks if versions are already published
- Runs dry-run tests before publishing
- Publishes `fido-types` first, then `fido` (respects dependency order)
- Handles the 20-second wait for crates.io indexing

## Tech Stack

Built with Rust using a workspace architecture:

### Core Crates
- **fido-types** - Shared models and data structures
- **fido-tui** - Terminal UI client (main binary, uses Ratatui)
- **fido-server** - REST API server (Axum + SQLite)
- **fido-migrate** - Database migration utilities

### Technologies
- **Ratatui** - Terminal UI framework
- **Axum** - Fast, ergonomic web framework
- **SQLite** - Lightweight database with r2d2 connection pooling
- **tokio** - Async runtime
- **oauth2** - GitHub authentication
- Deployed on Fly.io

## Documentation

- [QUICKSTART.md](QUICKSTART.md) - Detailed getting started guide
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design and technical details
- [CLAUDE.md](CLAUDE.md) - Development guidance for AI assistants

## Troubleshooting

**Session expired?** Press `Shift+L` to logout and login again.

**UI look weird?** Use a modern terminal with UTF-8 support (iTerm2, Alacritty, Ghostty).


## Development

### Local Development Setup

To run Fido locally for development:

```bash
# Clone the repository
git clone https://github.com/ianburke/fido.git
cd fido

# Build the workspace
cargo build

# Start the server
cargo run --bin fido-server

# In another terminal, connect to it
cargo run --bin fido -- --server http://localhost:3000
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p fido-server
cargo test -p fido-tui
cargo test -p fido-types

# Format code
cargo fmt

# Check for linting issues
cargo clippy
```

### Web Demo

For the full web terminal experience (see Development Scripts above):

```bash
./start.sh
```

## License

MIT

