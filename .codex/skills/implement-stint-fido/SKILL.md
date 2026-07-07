---
name: implement-stint-fido
description: Fido-specific .stint implementation, PR, merge, release, crates publish, reinstall, and Railway verification workflow. Use in /Users/ianburke/Documents/GitHub/fido when the user asks to implement a stint task, do P1 prerelease work, merge a Fido stint PR, publish or release Fido, reinstall fido locally, deploy Railway, or close Fido stint tasks.
---

# Fido Stint Implementation

Use this skill instead of the generic stint workflow for Fido. It codifies the expected path from task selection through implementation, local install, PR merge, crates publish, Railway deploy, and task closeout. Trigger it for `stint:fido` requests.

## Operating Rules

- Lead with execution. If the user asks to implement, merge, release, publish, reinstall, or deploy, carry the work through that boundary instead of stopping at a proposal.
- Read `AGENTS.md`, the relevant `.stint/tasks/*` files, current branch state, and staged diffs before changing code.
- Keep unrelated dirt out of the active branch. Never revert user changes unless the user explicitly asks.
- Use live evidence: current `.stint` state, current PR state, installed `fido --version`, crates.io metadata from outside the workspace, Railway deployment state, and production `/health`.
- Do not bump, publish, or deploy for docs-only or skill-only changes unless the user explicitly asks.
- Do not mark a stint task done until the implementation is merged into canonical `main`.

## Canonical Boundaries

- Canonical checkout: `/Users/ianburke/Documents/GitHub/fido`.
- Canonical branch: `main`.
- Feature worktrees: `worktrees/feature/stint-<id>-<slug>` unless an existing matching worktree or branch should be resumed.
- Local task state lives under `.stint/tasks`. Run `stint check` before and after task mutations.
- If a single PR intentionally handles multiple tasks, claim all included tasks. If local blocker edges make the same-PR batch invalid, repair the local task graph deliberately and re-run `stint check`.

## Preflight

From the canonical checkout:

```bash
git status --short --branch
git fetch origin
git worktree list
stint check
stint list --all
```

Then inspect each target task file and any existing PR, branch, or worktree for that task. Resume existing work when it is clearly the same task.

## Implementation

Create or enter the feature worktree, then keep edits scoped to the task. Prefer existing repo patterns over new abstractions.

Important Fido seams:

- Router changes usually need both `fido-server/src/lib.rs` and `fido-server/src/main.rs`.
- TUI startup, auth, and first-frame behavior usually touches `fido-tui/src/main.rs`, `fido-tui/src/event_loop.rs`, `fido-tui/src/app/*`, and `fido-tui/src/api/*`.
- Directory-scoped community behavior is rooted in `fido-tui/src/repo_context.rs` and the server community routes.
- The TUI crate publishes as package `fido`, not `fido-tui`.
- `fido-server` is not published to crates.io.
- Local server logs default to `logs/fido-server.log`; `just server-log` tails logs but does not start the server.

## Self-Test Ladder

Run the narrowest meaningful tests first, then broaden for release-impacting changes.

Always run for Rust changes:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Add focused gates by risk:

```bash
cargo test -p fido-server --features sqlite-tests
just e2e-tui-startup
just e2e-tui
```

Use `just e2e-tui-startup` for startup, auth handoff, first-frame, and installed-binary regressions. Use `just e2e-tui` for TUI navigation, posting, repo-community, or community settings changes.

If `cargo fmt` touches unrelated files, restore unrelated formatting and format only the intended files. Locale warnings from the e2e scripts about `C.UTF-8` are informational unless the command exits nonzero.

For installed binary checks:

```bash
cargo install --path fido-tui --force
fido --version
```

Then run a first-frame or e2e smoke against the installed binary when startup behavior changed.

## PR Flow

Commit from the feature worktree with the task id in the message when practical. Push and open a PR with:

- the included stint task ids,
- the behavior change,
- the exact validation commands and outcomes,
- release or deployment notes if applicable.

Before merge, verify the PR state with `gh pr view` and checks with `gh pr checks` when CI exists.

## Merge And Closeout

If the user asks to merge, release, install, publish, or promote:

1. Install the PR locally if the change affects runtime behavior.
2. Run the relevant self-tests from the ladder.
3. Merge the PR from the canonical checkout with `gh pr merge`.
4. Sync canonical `main` with `git pull --ff-only origin main`.
5. Run `stint done <task-id>` for each merged task.
6. Run `stint check`.
7. Clean up the feature worktree and branch only after they are clean.

If branch deletion is blocked by an attached worktree, remove the clean worktree first, then delete the branch.

## Release, Publish, Reinstall, Deploy

Only release when the Rust behavior changed or the user explicitly asks.

Patch bump:

```bash
just bump patch
cargo test --workspace
git status --short
git commit -am "chore: bump version to <version>"
git push origin main
```

Publish:

```bash
just deploy-cargo
```

The publish script publishes `fido-types` first, waits for crates.io propagation, then publishes package `fido`. Do not look for `fido-server` on crates.io.

Verify crates from outside the workspace, for example `/tmp`:

```bash
cargo info fido@<version>
cargo info fido-types@<version>
```

Reinstall from crates:

```bash
cargo install fido --version <version> --force
fido --version
```

Deploy Railway when server or web-terminal behavior changed, or when the user says promote/deploy:

```bash
just deploy
railway deployment list
curl -fsS https://fido-web-production.up.railway.app/health
```

Wait for the final Railway deployment state to be `SUCCESS`; a completed upload or build log is not enough. If the service returns a transient 502 during switchover, wait and retry. Expected health response is `OK`.

Use build logs only when needed:

```bash
railway logs --build <deployment-id>
```

## Final Report

Report only what matters:

- PR number or commit hash,
- stint ids closed,
- installed version,
- publish status,
- Railway deployment id and `/health` result,
- any tests that could not be run.
