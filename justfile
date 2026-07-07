set dotenv-load

# Start the Fido server locally with SQLite
server:
    #!/usr/bin/env bash
    set -euo pipefail
    log="${FIDO_SERVER_LOG:-logs/fido-server.log}"
    max_bytes="${FIDO_SERVER_LOG_MAX_BYTES:-10485760}"
    mkdir -p "$(dirname "$log")"
    : > "$log"
    cargo run --bin fido-server 2>&1 | while IFS= read -r line; do
        printf '%s\n' "$line"
        printf '%s\n' "$line" >> "$log"
        if [ "$(wc -c < "$log")" -ge "$max_bytes" ]; then
            mv "$log" "$log.1"
            : > "$log"
        fi
    done

# Start the Fido server with fresh database (deletes and recreates)
server-reset:
    #!/usr/bin/env bash
    set -euo pipefail
    log="${FIDO_SERVER_LOG:-logs/fido-server.log}"
    max_bytes="${FIDO_SERVER_LOG_MAX_BYTES:-10485760}"
    mkdir -p "$(dirname "$log")"
    : > "$log"
    cargo run --bin fido-server -- --reset-db 2>&1 | while IFS= read -r line; do
        printf '%s\n' "$line"
        printf '%s\n' "$line" >> "$log"
        if [ "$(wc -c < "$log")" -ge "$max_bytes" ]; then
            mv "$log" "$log.1"
            : > "$log"
        fi
    done

# Tail the local Fido server log written by `just server` / `just server-reset`
server-log:
    mkdir -p logs
    touch logs/fido-server.log
    tail -f logs/fido-server.log

# Start the Fido TUI client
tui:
    cargo run --bin fido

# Start TUI connected to local server
tui-local:
    cargo run --bin fido -- --server http://localhost:3000

# Start full local web stack (fido-server + ttyd + nginx)
web:
    ./start.sh

# Run full test suite
test:
    cargo test --workspace

# End-to-end test: drive the real TUI in tmux against a real local server
e2e-tui:
    chmod +x ./scripts/e2e_tui.sh
    ./scripts/e2e_tui.sh

# Focused startup smoke: first frame must render before network startup work
e2e-tui-startup:
    chmod +x ./scripts/e2e_tui_startup_smoke.sh
    ./scripts/e2e_tui_startup_smoke.sh

# Deploy to Railway (triggers rebuild from current branch)
deploy:
    railway up

# Publish crates to crates.io in dependency order (no auto-bump).
# Usage:
#   just deploy-cargo          # real publish
#   just deploy-cargo-dry      # dry-run only, never publishes
deploy-cargo:
    chmod +x ./scripts/deploy-cargo.sh
    ./scripts/deploy-cargo.sh

deploy-cargo-dry:
    chmod +x ./scripts/deploy-cargo.sh
    ./scripts/deploy-cargo.sh --dry-run

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
