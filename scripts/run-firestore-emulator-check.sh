#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${FIREBASE_PROJECT_ID:-demo-fido}"

if ! command -v firebase >/dev/null 2>&1; then
  echo "firebase CLI is required"
  exit 1
fi

echo "==> Starting Firestore emulator and running smoke test"
firebase emulators:exec --only firestore \
  --project "${PROJECT_ID}" \
  "FIREBASE_PROJECT_ID=${PROJECT_ID} ./scripts/firestore-emulator-smoke.sh"
