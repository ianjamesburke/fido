# fido-tui/src/ui/components

## Purpose

Reusable Ratatui widgets for layout, panels, lists, navigation, modal frames, banners, and empty states.

## Ownership

Each module owns one visual primitive. Callers supply state and layout bounds; components render without side effects.

## Local Contracts

- Use the active theme and the supplied `Rect`.
- Keep components small and composable. Screen-level state selection belongs in the caller.
- Preserve focus and selection styling so keyboard navigation remains legible.
- Avoid terminal-size assumptions that conflict with the 60 by 20 fallback.

## Verification

```bash
just e2e-tui
```

## Child DOX Index

No child instruction files.
