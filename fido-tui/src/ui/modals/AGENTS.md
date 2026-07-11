# fido-tui/src/ui/modals

## Purpose

Rendering and composition for community, composer, help, notifications, post, and social overlays.

## Ownership

Each module renders its modal family. `mod.rs` exports modal renderers used by the screen composition layer.

## Local Contracts

- Modal state and key handling live in `app/`; this directory renders the state it receives.
- Keep modal layout within the current terminal bounds and preserve Escape-based dismissal behavior.
- Put a new modal's user-visible shortcuts in the help surface and its key handling in `app/handlers/`.

## Verification

```bash
just e2e-tui
```

## Child DOX Index

No child instruction files.
