#!/usr/bin/env bash
set -euo pipefail

# Publish crates to crates.io in dependency-safe order (no version bumping).
# Order:
#   1) fido-types
#   2) fido
#
# Usage:
#   ./scripts/deploy-cargo.sh
#   ./scripts/deploy-cargo.sh --dry-run

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

DRY_RUN_ONLY=false
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN_ONLY=true
fi

CURRENT_VERSION="$(sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"/\1/p' Cargo.toml | head -n1)"
if [[ -z "$CURRENT_VERSION" ]]; then
  echo "error: could not parse workspace version from Cargo.toml"
  exit 1
fi

echo "Workspace version: $CURRENT_VERSION"
echo "Dry-run only:      $DRY_RUN_ONLY"

# Warn if worktree is dirty.
if [[ -n "$(git status --porcelain)" ]]; then
  echo "warning: you have uncommitted changes."
  echo "warning: publish is safer after version bump + commit (+ push if needed)."
fi

# Ensure dependency pin matches workspace version.
PINNED_TYPES_VERSION="$(sed -nE 's/^fido-types = \{ path = "fido-types", version = "([0-9]+\.[0-9]+\.[0-9]+)" \}/\1/p' Cargo.toml | head -n1)"
if [[ -z "$PINNED_TYPES_VERSION" ]]; then
  echo "error: could not parse fido-types dependency pin from Cargo.toml"
  exit 1
fi
if [[ "$PINNED_TYPES_VERSION" != "$CURRENT_VERSION" ]]; then
  echo "error: version mismatch."
  echo "error: workspace version is $CURRENT_VERSION, fido-types pin is $PINNED_TYPES_VERSION."
  echo "warning: bump/fix versions manually, commit, then rerun."
  exit 1
fi

# Preflight: detect already-published versions.
TYPES_PUBLISHED="$(cargo search fido-types 2>/dev/null | sed -nE 's/^fido-types = "([0-9]+\.[0-9]+\.[0-9]+)".*/\1/p' | head -n1)"
FIDO_PUBLISHED="$(cargo search fido --limit 20 2>/dev/null | sed -nE 's/^fido = "([0-9]+\.[0-9]+\.[0-9]+)".*/\1/p' | head -n1)"

if [[ "$TYPES_PUBLISHED" == "$CURRENT_VERSION" || "$FIDO_PUBLISHED" == "$CURRENT_VERSION" ]]; then
  echo "warning: version $CURRENT_VERSION is already published on crates.io:"
  echo "  fido-types: ${TYPES_PUBLISHED:-not found}"
  echo "  fido:       ${FIDO_PUBLISHED:-not found}"
  echo "warning: bump version manually, commit, and push, then rerun deploy-cargo."
  exit 1
fi

echo "==> Running publish dry-runs (dependency order)"
cargo publish --dry-run -p fido-types
cargo publish --dry-run -p fido

if [[ "$DRY_RUN_ONLY" == true ]]; then
  echo "Dry-run completed. No publish performed."
  exit 0
fi

echo "==> Publishing fido-types $CURRENT_VERSION"
cargo publish -p fido-types

echo "==> Waiting for crates.io index to catch up"
sleep 30

echo "==> Publishing fido $CURRENT_VERSION"
cargo publish -p fido

echo "Publish complete: $CURRENT_VERSION"
