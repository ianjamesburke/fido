# fido-tui/src/app/handlers

## Purpose

Keyboard event dispatch for global navigation, modals, posts, profiles, DMs, and settings.

## Ownership

`mod.rs` owns dispatch order. Feature files own their named key handling.

## Local Contracts

- Handle only `KeyEventKind::Press`.
- Preserve priority: quit, help, overlays, confirmations, modals, detail views, global keys, then screen-specific handlers.
- Do not let a new key path bypass an open modal or unsaved-settings confirmation.
- Keep shortcut behavior discoverable in the help UI when adding a user-facing command.

## Verification

```bash
just e2e-tui
```

## Child DOX Index

No child instruction files.
