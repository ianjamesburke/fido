set dotenv-load

# Start Firebase emulators (Firestore + Emulator UI)
emulator project_id="demo-fido":
    firebase emulators:start --project {{project_id}}

# Alias for emulator
emu project_id="demo-fido":
    just emulator {{project_id}}

# Start the Fido server against the local Firestore emulator
# Note: auto-seeding runs on startup when the emulator is empty.
server project_id="demo-fido" emulator_host="127.0.0.1:8088":
    DB_BACKEND=firestore \
    FIREBASE_PROJECT_ID={{project_id}} \
    GOOGLE_CLOUD_PROJECT={{project_id}} \
    FIRESTORE_EMULATOR_HOST={{emulator_host}} \
    FIRESTORE_SEED_TEST_DATA=true \
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
# Uses Firestore emulator by default and enables test-data seeding.
web project_id="demo-fido" emulator_host="127.0.0.1:8088":
    DB_BACKEND=firestore \
    FIREBASE_PROJECT_ID={{project_id}} \
    GOOGLE_CLOUD_PROJECT={{project_id}} \
    FIRESTORE_EMULATOR_HOST={{emulator_host}} \
    FIRESTORE_SEED_TEST_DATA=true \
    ./start.sh

# Start local web stack in explicit demo auth mode (test users only)
web-demo project_id="demo-fido" emulator_host="127.0.0.1:8088":
    DB_BACKEND=firestore \
    FIREBASE_PROJECT_ID={{project_id}} \
    GOOGLE_CLOUD_PROJECT={{project_id}} \
    FIRESTORE_EMULATOR_HOST={{emulator_host}} \
    FIRESTORE_SEED_TEST_DATA=true \
    FIDO_DEMO_MODE=true \
    ./start.sh

# Run full test suite
test:
    cargo test --workspace

# Deploy web stack to Firebase Hosting + Cloud Run
firebase-deploy project_id:
    FIREBASE_PROJECT_ID={{project_id}} ./scripts/deploy-firebase.sh

# Deploy web stack using env/.env and active gcloud project defaults
deploy:
    ./scripts/deploy-firebase.sh

# Publish crates to crates.io in dependency order (no auto-bump).
# Usage:
#   just deploy-cargo
#   just deploy-cargo dry-run=true
deploy-cargo dry-run="false":
    chmod +x ./scripts/deploy-cargo.sh
    if [ "{{dry-run}}" = "true" ]; then ./scripts/deploy-cargo.sh --dry-run; else ./scripts/deploy-cargo.sh; fi

# Run Firestore emulator smoke test
firestore-emulator-check project_id="demo-fido":
    FIREBASE_PROJECT_ID={{project_id}} ./scripts/run-firestore-emulator-check.sh

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
