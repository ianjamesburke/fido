# fido-tui/src/ui

## Purpose

Pure Ratatui rendering for screens, tabs, modals, formatting, themes, and reusable widgets.

## Ownership

- `../ui.rs` owns the top-level render function and minimum terminal size fallback.
- `tabs.rs` owns auth and main-screen composition.
- `theme.rs` owns color selection.
- `components/` owns reusable widgets.
- `modals/` owns overlay rendering.

## Local Contracts

- Render from `App` state without network, database, or filesystem side effects.
- Keep the terminal usable at the documented minimum size of 60 by 20.
- Reuse theme colors and shared components. Avoid hard-coded colors and duplicate layout logic.
- Keep content text-only and keyboard-oriented. A visual change must not hide an available keyboard path.

## Verification

```bash
cargo test -p fido
just e2e-tui
```

## Child DOX Index

- [`components/AGENTS.md`](components/AGENTS.md): reusable UI widgets and layouts.
- [`modals/AGENTS.md`](modals/AGENTS.md): modal rendering and modal-specific composition.
