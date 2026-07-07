#!/usr/bin/env bash
# Focused startup smoke for first-frame regressions.
#
# Launches the real TUI with an isolated HOME and unreachable server. The
# first frame should render before any update check or session validation can
# block on network IO.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIDO_BIN="$ROOT/target/debug/fido"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/fido-startup-smoke.XXXXXX")"
SESSION="fido-startup-smoke-$$"

cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

fail() {
    echo "FAIL: $1"
    echo "--- pane ---"
    tmux capture-pane -p -t "$SESSION" 2>/dev/null || echo "(no pane)"
    exit 1
}

command -v tmux >/dev/null || { echo "FAIL: tmux is required"; exit 1; }

cd "$ROOT"
echo "==> building fido TUI"
cargo build --quiet --bin fido

mkdir -p "$WORK/home" "$WORK/cwd"

echo "==> launching startup smoke"
tmux new-session -d -s "$SESSION" -x 100 -y 30 -c "$WORK/cwd" \
    "env HOME='$WORK/home' FIDO_SERVER_URL='http://127.0.0.1:9' '$FIDO_BIN'"

for ((i = 0; i < 20; i++)); do
    pane="$(tmux capture-pane -p -t "$SESSION" 2>/dev/null || true)"
    if printf '%s\n' "$pane" | grep -Eq "Fido - Terminal|Authentication|Loading|Choose authentication"; then
        echo "PASS: TUI rendered first frame before startup network work"
        exit 0
    fi
    sleep 0.1
done

fail "timed out waiting for first meaningful frame"
