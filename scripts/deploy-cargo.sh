#!/usr/bin/env bash
set -euo pipefail

# Deploy crates to crates.io in dependency-safe order.
# Default behavior bumps patch version, then publishes:
#   1) fido-types
#   2) fido
#
# Usage:
#   ./scripts/deploy-cargo.sh                 # patch bump + publish
#   ./scripts/deploy-cargo.sh minor           # minor bump + publish
#   ./scripts/deploy-cargo.sh major --dry-run # major bump + validate only

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BUMP_KIND="${1:-patch}"
DRY_RUN_ONLY=false
if [[ "${2:-}" == "--dry-run" || "${1:-}" == "--dry-run" ]]; then
  DRY_RUN_ONLY=true
fi

if [[ "$BUMP_KIND" == "--dry-run" ]]; then
  BUMP_KIND="patch"
fi

if [[ "$BUMP_KIND" != "patch" && "$BUMP_KIND" != "minor" && "$BUMP_KIND" != "major" ]]; then
  echo "error: bump kind must be one of: patch|minor|major"
  exit 1
fi

echo "==> Reading current workspace version"
CURRENT_VERSION="$(sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"/\1/p' Cargo.toml | head -n1)"
if [[ -z "$CURRENT_VERSION" ]]; then
  echo "error: could not parse workspace version from Cargo.toml"
  exit 1
fi

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
case "$BUMP_KIND" in
  patch) PATCH=$((PATCH + 1)) ;;
  minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
  major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
esac
NEXT_VERSION="${MAJOR}.${MINOR}.${PATCH}"

echo "Current version: $CURRENT_VERSION"
echo "Next version:    $NEXT_VERSION"
echo "Bump:            $BUMP_KIND"
echo "Dry-run only:    $DRY_RUN_ONLY"

echo "==> Updating workspace and dependency versions"
# workspace.package version
sed -i.bak -E "s/^version = \"${CURRENT_VERSION}\"$/version = \"${NEXT_VERSION}\"/" Cargo.toml
# workspace dependency pin for fido-types
sed -i.bak -E "s/^fido-types = \{ path = \"fido-types\", version = \"${CURRENT_VERSION}\" \}$/fido-types = { path = \"fido-types\", version = \"${NEXT_VERSION}\" }/" Cargo.toml
rm -f Cargo.toml.bak

echo "==> Formatting metadata changes"
cargo fmt

echo "==> Running publish dry-runs (dependency order)"
cargo publish --dry-run -p fido-types
cargo publish --dry-run -p fido

if [[ "$DRY_RUN_ONLY" == true ]]; then
  echo "Dry-run completed. Version files updated to $NEXT_VERSION (not published)."
  exit 0
fi

echo "==> Publishing fido-types $NEXT_VERSION"
cargo publish -p fido-types

echo "==> Waiting for crates.io index to catch up"
sleep 30

echo "==> Publishing fido $NEXT_VERSION"
cargo publish -p fido

echo "Publish complete: $NEXT_VERSION"
