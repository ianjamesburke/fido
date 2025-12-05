# Fido 🐕

A terminal-based social platform for developers. No algorithms, no ads, just posts and conversations.

Built for the [Kiroween 2024 Hackathon](https://kiro.ai) using spec-driven development.

## What is it?

Fido is a social network that lives in your terminal. Think Twitter, but keyboard-driven and without the noise. Post updates, chat with other developers, upvote good content. Everything happens in your terminal with zero distractions.

## Installation

You'll need Rust installed. Get it from [rustup.rs](https://rustup.rs/) if you don't have it.

Then install Fido:

```bash
cargo install fido
```

The client connects to a live server at `https://fido-social.fly.dev` - no setup required.

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
- **Markdown posts** - Format your thoughts with hashtags and emoji shortcodes
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

## Tech Stack

Built with Rust:
- **Ratatui** - TUI framework
- **Axum** - API server
- **SQLite** - Database
- Deployed on Fly.io

## Documentation

- [QUICKSTART.md](QUICKSTART.md) - Detailed getting started guide
- [CONTRIBUTING.md](CONTRIBUTING.md) - How to contribute
- [DEPLOYMENT.md](DEPLOYMENT.md) - Server deployment guide
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design

## Troubleshooting

**Can't connect?** Check your internet and verify the server is up: `curl https://fido-social.fly.dev/health`

**Browser won't open for OAuth?** Copy the URL from the terminal and paste it manually.

**Session expired?** Press `Shift+L` to logout and login again.

**Emojis look weird?** Use a modern terminal with UTF-8 support (Windows Terminal, iTerm2, Alacritty).

More help in [QUICKSTART.md](QUICKSTART.md).

## Contributing

Want to help? Check out [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions.

To run locally:
```bash
# Start the server
cargo run --bin fido-server

# In another terminal, connect to it
fido --server http://localhost:3000
```

## License

MIT

---

Built for Kiroween 2024 with [Ratatui](https://github.com/ratatui-org/ratatui), [Axum](https://github.com/tokio-rs/axum), and SQLite.
