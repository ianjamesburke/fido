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

### MacOS (untested on Windows)

First, make sure you have [Rust](https://rustup.rs/) installed

```bash
brew install rust 
```


Then install Fido:

```bash
cargo install fido
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

**Session expired?** Press `Shift+L` to logout and login again.

**UI look weird?** Use a modern terminal with UTF-8 support (iTerm2, Alacritty, Ghostty).


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

