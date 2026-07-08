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
case "${1:-}" in
  "")
    ;;
  "--dry-run")
    DRY_RUN_ONLY=true
    ;;
  *)
    echo "error: unsupported argument: $1"
    echo "usage: ./scripts/deploy-cargo.sh [--dry-run]"
    exit 2
    ;;
esac

crate_version_state() {
  local crate="$1"
  local version="$2"
  local output
  output="$(mktemp)"

  if (cd "${TMPDIR:-/tmp}" && cargo info "${crate}@${version}") >"$output" 2>&1; then
    rm -f "$output"
    echo "published"
    return 0
  fi

  if grep -Eiq 'could not find|no matching package|not found' "$output"; then
    rm -f "$output"
    echo "not-published"
    return 0
  fi

  echo "error: could not determine crates.io state for ${crate}@${version}" >&2
  sed 's/^/  /' "$output" >&2
  rm -f "$output"
  return 1
}

CURRENT_VERSION="$(sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"/\1/p' Cargo.toml | head -n1)"
if [[ -z "$CURRENT_VERSION" ]]; then
  echo "error: could not parse workspace version from Cargo.toml"
  exit 1
fi

echo "Workspace version: $CURRENT_VERSION"
echo "Dry-run only:      $DRY_RUN_ONLY"

# Fail closed on dirty worktrees; release artifacts must map to an exact commit.
if [[ -n "$(git status --porcelain)" ]]; then
  if [[ "${FIDO_ALLOW_DIRTY_PUBLISH:-}" == "1" ]]; then
    echo "warning: worktree is dirty; continuing because FIDO_ALLOW_DIRTY_PUBLISH=1."
  else
    echo "error: worktree has uncommitted changes."
    echo "error: commit or stash changes before publish/dry-run so the preflight maps to one revision."
    echo "error: set FIDO_ALLOW_DIRTY_PUBLISH=1 only for intentional local diagnostics."
    exit 1
  fi
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

# Preflight: detect exact already-published versions.
TYPES_STATE="$(crate_version_state fido-types "$CURRENT_VERSION")"
FIDO_STATE="$(crate_version_state fido "$CURRENT_VERSION")"

echo "Registry state:"
echo "  fido-types@$CURRENT_VERSION: $TYPES_STATE"
echo "  fido@$CURRENT_VERSION:       $FIDO_STATE"

if [[ "$FIDO_STATE" == "published" ]]; then
  echo "error: version $CURRENT_VERSION is already published on crates.io."
  echo "error: bump version manually, commit, and push, then rerun deploy-cargo."
  exit 1
fi
if [[ "$TYPES_STATE" == "published" ]]; then
  echo "warning: fido-types $CURRENT_VERSION is already published; fido publish can continue after dry-run."
fi

echo "==> Running publish dry-runs (dependency order)"
if [[ "$TYPES_STATE" == "published" ]]; then
  echo "skipping fido-types dry-run because fido-types $CURRENT_VERSION is already published."
else
  cargo publish --dry-run -p fido-types
fi

if [[ "$DRY_RUN_ONLY" == true ]]; then
  if [[ "$TYPES_STATE" != "published" ]]; then
    echo "error: incomplete publish dry-run."
    echo "checked:   fido-types publish dry-run"
    echo "unchecked: fido publish dry-run"
    echo "reason:    fido-types $CURRENT_VERSION is not visible on crates.io yet, so fido cannot resolve its registry dependency."
    echo "next:      publish fido-types first, wait for indexing, then rerun just deploy-cargo-dry or just deploy-cargo."
    exit 1
  fi
  cargo publish --dry-run -p fido
  echo "Dry-run completed. No publish performed."
  exit 0
fi

if [[ "$TYPES_STATE" == "published" ]]; then
  echo "==> fido-types $CURRENT_VERSION already indexed; skipping dependency publish"
else
  echo "==> Publishing fido-types $CURRENT_VERSION"
  cargo publish -p fido-types

  echo "==> Waiting for crates.io index to include fido-types $CURRENT_VERSION"
  MAX_WAIT_SECONDS=300
  SLEEP_SECONDS=10
  ELAPSED=0
  while true; do
    VISIBLE_TYPES_STATE="$(crate_version_state fido-types "$CURRENT_VERSION")"
    if [[ "$VISIBLE_TYPES_STATE" == "published" ]]; then
      echo "fido-types $CURRENT_VERSION is now visible on crates.io."
      break
    fi

    if (( ELAPSED >= MAX_WAIT_SECONDS )); then
      echo "error: timed out waiting for fido-types $CURRENT_VERSION to appear on crates.io."
      echo "error: rerun just deploy-cargo after crates.io indexing catches up."
      exit 1
    fi

    sleep "$SLEEP_SECONDS"
    ELAPSED=$((ELAPSED + SLEEP_SECONDS))
  done
fi

echo "==> Running fido dry-run now that dependency is indexed"
cargo publish --dry-run -p fido

echo "==> Publishing fido $CURRENT_VERSION"
cargo publish -p fido

echo "Publish complete: $CURRENT_VERSION"
