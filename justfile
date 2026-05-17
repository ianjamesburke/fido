set dotenv-load

# Start the Fido server locally with SQLite
server:
    cargo run --bin fido-server

# Start the Fido server with fresh database (deletes and recreates)
server-reset:
    cargo run --bin fido-server -- --reset-db

# Start the Fido TUI client
tui:
    cargo run --bin fido

# Start TUI connected to local server
tui-local:
    cargo run --bin fido -- --server http://localhost:3000

# Start full local web stack (fido-server + ttyd + nginx)
web:
    ./start.sh

# Start local web stack in explicit demo auth mode (test users only)
web-demo:
    FIDO_DEMO_MODE=true ./start.sh

# Run full test suite
test:
    cargo test --workspace

# Deploy to Railway (triggers rebuild from current branch)
deploy:
    railway up

# Publish crates to crates.io in dependency order (no auto-bump).
# Usage:
#   just deploy-cargo
#   just deploy-cargo dry-run=true
deploy-cargo dry-run="false":
    chmod +x ./scripts/deploy-cargo.sh
    if [ "{{dry-run}}" = "true" ]; then ./scripts/deploy-cargo.sh --dry-run; else ./scripts/deploy-cargo.sh; fi

# Bump version in workspace Cargo.toml (patch, minor, or major)
# Usage:
#   just bump          # bumps patch version (0.1.14 -> 0.1.15)
#   just bump patch    # same as above
#   just bump minor    # bumps minor version (0.1.14 -> 0.2.0)
#   just bump major    # bumps major version (0.1.14 -> 1.0.0)
bump level="patch":
    #!/usr/bin/env bash
    set -euo pipefail

    # Read current version from Cargo.toml
    current=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    echo "Current version: $current"

    # Parse version components
    IFS='.' read -r major minor patch <<< "$current"

    # Calculate new version based on level
    case "{{level}}" in
        patch)
            new_version="$major.$minor.$((patch + 1))"
            ;;
        minor)
            new_version="$major.$((minor + 1)).0"
            ;;
        major)
            new_version="$((major + 1)).0.0"
            ;;
        *)
            echo "Error: Invalid bump level '{{level}}'. Use 'patch', 'minor', or 'major'."
            exit 1
            ;;
    esac

    echo "New version: $new_version"

    # Update workspace.package version
    sed -i.bak "s/^version = \"$current\"/version = \"$new_version\"/" Cargo.toml

    # Update fido-types dependency version
    sed -i.bak "s/fido-types = { path = \"fido-types\", version = \"$current\" }/fido-types = { path = \"fido-types\", version = \"$new_version\" }/" Cargo.toml

    # Remove backup files
    rm -f Cargo.toml.bak

    echo "✓ Version bumped from $current to $new_version"
    echo "  Updated: [workspace.package] version"
    echo "  Updated: fido-types dependency version"
