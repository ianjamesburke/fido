#!/usr/bin/env bash
# End-to-end test of the real TUI binary against a real local server.
#
# Drives the TUI in a detached tmux pane and asserts on captured frames,
# the SQLite database, and log files. GitHub is stubbed via GITHUB_API_BASE.
#
# Covers: test-user login, directory-scoped community join (repo mode),
# board title with role, posting, community modal, Home mode outside a repo,
# opening a community from the Home list.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

command -v tmux >/dev/null || { echo "FAIL: tmux is required"; exit 1; }
command -v sqlite3 >/dev/null || { echo "FAIL: sqlite3 is required"; exit 1; }

PORT="${E2E_PORT:-34567}"
STUB_PORT=$((PORT + 1))
WORK="$(mktemp -d "${TMPDIR:-/tmp}/fido-e2e.XXXXXX")"
SESSION="fido-e2e-$$"
FIDO_BIN="$ROOT/target/debug/fido"
SERVER_BIN="$ROOT/target/debug/fido-server"

SERVER_PID=""
STUB_PID=""

cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
    [[ -n "$STUB_PID" ]] && kill "$STUB_PID" 2>/dev/null || true
}
trap cleanup EXIT

fail() {
    echo "FAIL: $1"
    echo "--- pane ---"
    tmux capture-pane -p -t "$SESSION" 2>/dev/null || echo "(no pane)"
    echo "--- server log tail ---"
    tail -n 30 "$WORK/server.log" 2>/dev/null || true
    echo "--- stub log tail ---"
    tail -n 10 "$WORK/stub.log" 2>/dev/null || true
    echo "--- artifacts kept in $WORK ---"
    trap - EXIT
    cleanup
    exit 1
}

pane() { tmux capture-pane -p -t "$SESSION"; }

# wait_for <pattern> <description> [tries]
wait_for() {
    local pattern="$1" what="$2" tries="${3:-75}"
    for ((i = 0; i < tries; i++)); do
        if pane 2>/dev/null | grep -qF "$pattern"; then
            return 0
        fi
        sleep 0.2
    done
    fail "timed out waiting for: $what (pattern: $pattern)"
}

keys() { tmux send-keys -t "$SESSION" "$@"; sleep 0.3; }

echo "==> building binaries"
cargo build --quiet --bin fido --bin fido-server

echo "==> starting github stub on :$STUB_PORT"
python3 "$ROOT/scripts/github_stub.py" "$STUB_PORT" >"$WORK/stub.log" 2>&1 &
STUB_PID=$!

echo "==> starting fido-server on :$PORT (db: $WORK/fido.db)"
PORT="$PORT" \
    DATABASE_PATH="$WORK/fido.db" \
    GITHUB_API_BASE="http://127.0.0.1:$STUB_PORT" \
    FIDO_TOKEN_KEY="$(head -c 32 /dev/urandom | base64)" \
    ENVIRONMENT="development" \
    "$SERVER_BIN" --reset-db >"$WORK/server.log" 2>&1 &
SERVER_PID=$!

for ((i = 0; i < 50; i++)); do
    curl -sf "http://127.0.0.1:$PORT/health" >/dev/null 2>&1 && break
    kill -0 "$SERVER_PID" 2>/dev/null || fail "server exited during startup"
    sleep 0.2
done
curl -sf "http://127.0.0.1:$PORT/health" >/dev/null || fail "server never became healthy"

echo "==> creating temp git repo with GitHub origin"
REPO="$WORK/testrepo"
mkdir -p "$REPO"
git -C "$REPO" init -q
git -C "$REPO" remote add origin https://github.com/testowner/testrepo.git

E2E_HOME="$WORK/home"
mkdir -p "$E2E_HOME"
NON_REPO="$WORK/non-repo"
mkdir -p "$NON_REPO"

launch_tui() { # $1 = working directory
    tmux new-session -d -s "$SESSION" -x 140 -y 40 -c "$1" \
        "env HOME='$E2E_HOME' '$FIDO_BIN' --server http://127.0.0.1:$PORT --verbose"
}

# --- Scenario 1: repo mode — launch inside a repo, land on its board -------
echo "==> scenario 1: repo mode join + post"
launch_tui "$REPO"
wait_for "Authentication" "auth screen"
keys 'l'
wait_for "alice" "test users list"
keys Enter
wait_for "testowner/testrepo" "repo community board title"
pane | grep -qF "member" || fail "board title should show the caller's role"
pane | grep -qF "unclaimed" || fail "board title should show unclaimed state"

keys 'n'
wait_for "New Post" "composer modal" 25
tmux send-keys -t "$SESSION" -l "hello from the e2e harness"
sleep 0.3
keys Enter
wait_for "hello from the e2e harness" "posted thread on board"

# Database-level assertion: the post landed in the repo's community.
POST_COUNT=$(sqlite3 "$WORK/fido.db" \
    "SELECT count(*) FROM posts p JOIN communities c ON p.community_id = c.id
     WHERE c.owner = 'testowner' AND c.name = 'testrepo'
       AND p.content = 'hello from the e2e harness';")
[[ "$POST_COUNT" == "1" ]] || fail "post not found in testowner/testrepo community (count=$POST_COUNT)"

# --- Scenario 2: community modal ---------------------------------------------
echo "==> scenario 2: community settings modal"
keys 'i'
wait_for "Your role" "community modal"
pane | grep -qF "Claim admin" || fail "modal should offer claim on an unclaimed community"
keys Escape
sleep 0.3
pane | grep -qF "Your role" && fail "community modal should close on Esc"

tmux kill-session -t "$SESSION"

# --- Scenario 3: Home mode — launch outside a repo ---------------------------
echo "==> scenario 3: home mode lists joined communities"
launch_tui "$NON_REPO"
wait_for "Your Communities" "home list (session restored)"
pane | grep -qF "testowner/testrepo" || fail "home list should show the joined community"
pane | grep -qF "Home" || fail "tab bar should read Home outside a repo"

keys Enter
wait_for "hello from the e2e harness" "board opened from home list"
keys Escape
wait_for "Your Communities" "Esc returns to home list"

tmux kill-session -t "$SESSION"

# --- Scenario 4: search -> profile -> message connection path ----------------
# Server seeds three test users (alice, bob, charlie) plus a pre-accepted
# alice<->bob DM thread; we're logged in as alice (session restored from
# scenario 1) and message bob via search -> profile -> DM.
echo "==> scenario 4: search user, view profile, send DM"
launch_tui "$REPO"
wait_for "testowner/testrepo" "repo community board (session restored, scenario 4)"

keys 's'
wait_for "Search Users" "user search modal"
tmux send-keys -t "$SESSION" -l "bob"
sleep 0.3
wait_for "bob" "search result row for bob"
keys Enter
wait_for "User Profile" "profile modal opens"
pane | grep -qF "@bob" || fail "profile modal should show @bob"

keys 'm'
wait_for "Message Input (Enter to send)" "DMs tab conversation with bob"
tmux send-keys -t "$SESSION" -l "hello from e2e"
sleep 0.3
keys Enter
wait_for "hello from e2e" "sent DM to appear in transcript"

MSG_COUNT=$(sqlite3 "$WORK/fido.db" \
    "SELECT count(*) FROM direct_messages dm
     JOIN users s ON dm.from_user_id = s.id
     JOIN users r ON dm.to_user_id = r.id
     WHERE s.username = 'alice' AND r.username = 'bob'
       AND dm.content = 'hello from e2e';")
[[ "$MSG_COUNT" == "1" ]] || fail "DM from alice to bob not found (count=$MSG_COUNT)"

tmux kill-session -t "$SESSION"

echo "PASS: all TUI e2e scenarios"
rm -rf "$WORK"
