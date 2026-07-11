# fido-tui

## Purpose

This crate builds the `fido` terminal client published to crates.io.

## Ownership

- `src/` owns startup, terminal lifecycle, state, input handling, API calls, and Ratatui rendering.
- `tests/` owns client integration coverage.
- `README.md` documents client installation and usage.

## Local Contracts

- The TUI starts with a first frame before network startup work completes.
- Keep input handling keyboard-first. Every user action must have a keyboard path.
- Run in a GitHub repository to open that repository's community; run elsewhere to open Home mode.
- Session files stay under `~/.fido/`. Release binaries do not auto-load a repository `.env` unless `FIDO_LOAD_DOTENV` is explicitly enabled.
- Keep rendering responsive. Avoid blocking network or disk work in the event loop or render path.

## Verification

```bash
cargo test -p fido
just e2e-tui-startup
just e2e-tui
```

Use `just e2e-tui` for any changed interaction, screen, navigation, community behavior, or displayed data.

## Child DOX Index

- [`src/AGENTS.md`](src/AGENTS.md): client module boundaries.
