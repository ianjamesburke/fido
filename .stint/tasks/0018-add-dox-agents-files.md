---
id: "0018"
title: "Add DOX AGENTS.md files: adopt root template and initialize child docs tree"
status: todo
priority: p2
estimate: "1h"
blocked_by: [17]
gh_issue: []
area:
  - "docs/tooling"
tags:
  - "agents"
  - "docs"
---

Adopt the DOX `AGENTS.md` framework in this repo so AI agents have a maintained instruction tree instead of a single root file.

## Scope

- Review the current root `AGENTS.md` and merge in the relevant DOX conventions without losing Fido-specific guidance.
- Add child `AGENTS.md` files for the main work areas (`fido-server/`, `fido-tui/`, `fido-types/`, and any deeper directories that need local edit rules or indexes).
- Build the initial docs tree/index structure DOX expects so an agent can traverse from repo root to the touched area before editing.
- Keep instructions concrete and repo-local: architecture seams, ownership boundaries, test commands, and area-specific pitfalls.
- Update README only if it needs a short note pointing contributors/agents at the new docs tree.

## Non-Scope

- Changing product behavior or shipping feature work.
- Rewriting unrelated docs outside the agent-instruction tree.

## Why

The repo already has a root `AGENTS.md`, but DOX is designed around a maintained hierarchy. If we want agent edits to stay precise as the codebase grows, the child files and local indexes need to exist.

## References

- `AGENTS.md` — current root instructions to preserve and extend
- `https://github.com/agent0ai/dox` — source template and hierarchy model
