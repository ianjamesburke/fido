#!/usr/bin/env bash
# End-to-end test of the real TUI binary against a real local server.
#
# Drives the TUI in a detached tmux pane and asserts on captured frames,
# the SQLite database, and log files. GitHub is stubbed via GITHUB_API_BASE.
#
# Covers: test-user login, directory-scoped community join (repo mode),
# board title with role, posting, channel chat, community modal, Home mode
# outside a repo, opening a community from the Home list, search -> profile ->
# DM, and the repo activity feed (GitHub issues fetch + cache).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

command -v tmux >/dev/null || { echo "FAIL: tmux is required"; exit 1; }
command -v sqlite3 >/dev/null || { echo "FAIL: sqlite3 is required"; exit 1; }

PORT="${E2E_PORT:-34567}"
STUB_PORT=$((PORT + 1))
WORK="$(mktemp -d "${TMPDIR:-/tmp}/fido-e2e.XXXXXX")"
SESSION="fido-e2e-$$"
ALICE_SESSION="$SESSION-alice"
AARON_SESSION="$SESSION-aaron"
FIDO_BIN="$ROOT/target/debug/fido"
SERVER_BIN="$ROOT/target/debug/fido-server"

SERVER_PID=""
STUB_PID=""

cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    tmux kill-session -t "$ALICE_SESSION" 2>/dev/null || true
    tmux kill-session -t "$AARON_SESSION" 2>/dev/null || true
    [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
    [[ -n "$STUB_PID" ]] && kill "$STUB_PID" 2>/dev/null || true
}
trap cleanup EXIT

fail() {
    echo "FAIL: $1"
    for session_name in "$SESSION" "$ALICE_SESSION" "$AARON_SESSION"; do
        echo "--- pane: $session_name ---"
        tmux capture-pane -p -t "$session_name" 2>/dev/null || echo "(no pane)"
    done
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
pane_for() { tmux capture-pane -p -t "$1"; }

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

wait_for_in() {
    local target="$1" pattern="$2" what="$3" tries="${4:-75}"
    for ((i = 0; i < tries; i++)); do
        if pane_for "$target" 2>/dev/null | grep -qF "$pattern"; then
            return 0
        fi
        sleep 0.2
    done
    fail "timed out waiting for: $what in $target (pattern: $pattern)"
}

keys() { tmux send-keys -t "$SESSION" "$@"; sleep 0.3; }
keys_in() { local target="$1"; shift; tmux send-keys -t "$target" "$@"; sleep 0.3; }

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
    launch_tui_for "$SESSION" "$E2E_HOME" "$1"
}

launch_tui_for() { # $1 = tmux session, $2 = HOME, $3 = working directory
    tmux new-session -d -s "$1" -x 140 -y 40 -c "$3" \
        "env HOME='$2' '$FIDO_BIN' --server http://127.0.0.1:$PORT --verbose"
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

# --- Scenario 1b: repo-scoped channel chat -----------------------------------
echo "==> scenario 1b: repo community chat"
keys Tab
wait_for "#general" "repo community chat channel"
tmux send-keys -t "$SESSION" -l "hello from channel e2e"
sleep 0.3
keys Enter
wait_for "hello from channel e2e" "sent channel message"

CHAT_COUNT=$(sqlite3 "$WORK/fido.db" \
    "SELECT count(*) FROM messages m
     JOIN channels ch ON m.channel_id = ch.id
     JOIN communities c ON ch.community_id = c.id
     JOIN users u ON m.author_id = u.id
     WHERE c.owner = 'testowner' AND c.name = 'testrepo'
       AND ch.name = 'general'
       AND u.username = 'alice'
       AND m.content = 'hello from channel e2e';")
[[ "$CHAT_COUNT" == "1" ]] || fail "channel message not found in testowner/testrepo #general (count=$CHAT_COUNT)"
keys BTab
wait_for "hello from the e2e harness" "returned from chat to board"

# --- Scenario 2: community modal ---------------------------------------------
echo "==> scenario 2: community settings modal"
keys 'i'
wait_for "Your role" "community modal"
pane | grep -qF "Claim admin" || fail "modal should offer claim on an unclaimed community"
keys Escape
sleep 0.3
pane | grep -qF "Your role" && fail "community modal should close on Esc"

tmux kill-session -t "$SESSION"

# --- Scenario 2b: admin approval queue ---------------------------------------
echo "==> scenario 2b: pending thread approval queue"
sqlite3 "$WORK/fido.db" \
    "UPDATE communities SET require_thread_approval = 1
     WHERE owner = 'testowner' AND name = 'testrepo';
     UPDATE memberships
        SET role = 'admin'
      WHERE community_id = (SELECT id FROM communities WHERE owner = 'testowner' AND name = 'testrepo')
        AND user_id = (SELECT id FROM users WHERE username = 'alice');
     DELETE FROM post_rate_limits
      WHERE user_id = (SELECT id FROM users WHERE username = 'alice');"

launch_tui "$REPO"
wait_for "testowner/testrepo" "repo community board with approval required"
pane | grep -qF "admin" || fail "board title should show admin role after promotion"

keys 'n'
wait_for "New Post" "composer modal for pending thread" 25
tmux send-keys -t "$SESSION" -l "pending approval from e2e"
sleep 0.3
keys Enter
wait_for "Thread submitted for admin approval." "pending approval author feedback"

PENDING_COUNT=$(sqlite3 "$WORK/fido.db" \
    "SELECT count(*) FROM posts p
     JOIN communities c ON p.community_id = c.id
     WHERE c.owner = 'testowner' AND c.name = 'testrepo'
       AND p.content = 'pending approval from e2e'
       AND p.approved = 0;")
[[ "$PENDING_COUNT" == "1" ]] || fail "pending thread not found before approval (count=$PENDING_COUNT)"

keys 'a'
wait_for "Pending Threads" "approval queue opens"
wait_for "pending approval from e2e" "pending thread appears in approval queue"
keys 'a'
wait_for "No pending threads" "pending queue empty after approval"

APPROVED_COUNT=$(sqlite3 "$WORK/fido.db" \
    "SELECT count(*) FROM posts p
     JOIN communities c ON p.community_id = c.id
     WHERE c.owner = 'testowner' AND c.name = 'testrepo'
       AND p.content = 'pending approval from e2e'
       AND p.approved = 1;")
[[ "$APPROVED_COUNT" == "1" ]] || fail "thread not approved from queue (count=$APPROVED_COUNT)"

keys Escape
wait_for "pending approval from e2e" "approved thread appears on board"

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

# --- Scenario 4b: pending DM request inbox from two live TUI sessions --------
# Alice stays open on the DMs tab while a separate non-community test user sends
# the first message from another HOME. The request should arrive over realtime,
# render as a request row, and accept into an accepted conversation.
echo "==> scenario 4b: two-session pending DM request inbox"
sqlite3 "$WORK/fido.db" \
    "INSERT OR IGNORE INTO users (id, username, bio, join_date, is_test_user, is_admin)
         VALUES ('550e8400-e29b-41d4-a716-44665544a001', 'aaron', 'Outside any community', '2023-12-31T00:00:00Z', 1, 0);
     INSERT OR IGNORE INTO user_configs (user_id, color_scheme, sort_order, max_posts_display, emoji_enabled)
         VALUES ('550e8400-e29b-41d4-a716-44665544a001', 'Default', 'Newest', 25, 1);"

AARON_HOME="$WORK/home-aaron"
mkdir -p "$AARON_HOME"

launch_tui_for "$ALICE_SESSION" "$E2E_HOME" "$REPO"
wait_for_in "$ALICE_SESSION" "testowner/testrepo" "alice repo community board"
keys_in "$ALICE_SESSION" Tab
wait_for_in "$ALICE_SESSION" "#general" "alice chat tab"
keys_in "$ALICE_SESSION" Tab
wait_for_in "$ALICE_SESSION" "Conversations" "alice DMs tab"

launch_tui_for "$AARON_SESSION" "$AARON_HOME" "$NON_REPO"
wait_for_in "$AARON_SESSION" "Authentication" "aaron auth screen"
keys_in "$AARON_SESSION" 'l'
wait_for_in "$AARON_SESSION" "aaron" "aaron test user row"
keys_in "$AARON_SESSION" Enter
wait_for_in "$AARON_SESSION" "Your Communities" "aaron home mode"
keys_in "$AARON_SESSION" Tab
keys_in "$AARON_SESSION" Tab
wait_for_in "$AARON_SESSION" "Conversations" "aaron DMs tab"

keys_in "$AARON_SESSION" 'n'
wait_for_in "$AARON_SESSION" "New Conversation" "aaron new conversation modal"
tmux send-keys -t "$AARON_SESSION" -l "alice"
sleep 0.6
wait_for_in "$AARON_SESSION" "alice" "alice search result in DM modal"
keys_in "$AARON_SESSION" Enter
wait_for_in "$AARON_SESSION" "New conversation with @alice" "aaron pending draft for alice"
tmux send-keys -t "$AARON_SESSION" -l "request from aaron e2e"
sleep 0.3
keys_in "$AARON_SESSION" Enter
wait_for_in "$AARON_SESSION" "Request sent" "aaron sees pending request state"

wait_for_in "$ALICE_SESSION" "@aaron wants to chat" "alice receives pending DM request" 100
keys_in "$ALICE_SESSION" Up
keys_in "$ALICE_SESSION" 'a'
sleep 0.6

DM_STATE=$(sqlite3 "$WORK/fido.db" \
    "SELECT dc.state
       FROM dm_conversations dc
       JOIN users ua ON dc.user_a = ua.id
       JOIN users ub ON dc.user_b = ub.id
      WHERE (ua.username = 'aaron' AND ub.username = 'alice')
         OR (ua.username = 'alice' AND ub.username = 'aaron');")
[[ "$DM_STATE" == "accepted" ]] || fail "pending DM request was not accepted from Alice UI (state=$DM_STATE)"

DM_REQUEST_MSG_COUNT=$(sqlite3 "$WORK/fido.db" \
    "SELECT count(*) FROM direct_messages dm
     JOIN users s ON dm.from_user_id = s.id
     JOIN users r ON dm.to_user_id = r.id
     WHERE s.username = 'aaron' AND r.username = 'alice'
       AND dm.content = 'request from aaron e2e';")
[[ "$DM_REQUEST_MSG_COUNT" == "1" ]] || fail "pending DM request message not found (count=$DM_REQUEST_MSG_COUNT)"

tmux kill-session -t "$ALICE_SESSION"
tmux kill-session -t "$AARON_SESSION"

# --- Scenario 5: repo activity feed --------------------------------------
# Opening the board triggers a background fetch of GitHub issues for the
# repo; the stub returns one open issue that should interleave into the
# posts feed and get cached in community_activity.
echo "==> scenario 5: repo activity feed loads and caches"
launch_tui "$REPO"
wait_for "testowner/testrepo" "repo community board (session restored, scenario 5)"
wait_for "Stub issue for e2e" "stub issue interleaved into posts feed"

count=$(sqlite3 "$WORK/fido.db" "SELECT COUNT(*) FROM community_activity;")
[ "$count" -ge 1 ] || fail "expected community_activity cache row"

tmux kill-session -t "$SESSION"

echo "PASS: all TUI e2e scenarios"
rm -rf "$WORK"
