#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${FIREBASE_PROJECT_ID:-demo-fido}"
EMULATOR_HOST="${FIRESTORE_EMULATOR_HOST:-127.0.0.1:8088}"
BASE_URL="http://${EMULATOR_HOST}/v1/projects/${PROJECT_ID}/databases/(default)/documents"
DOC_ID="$(date +%s)"

if ! command -v firebase >/dev/null 2>&1; then
  echo "firebase CLI is required"
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required"
  exit 1
fi

cat >/tmp/fido-firestore-payload.json <<JSON
{
  "fields": {
    "message": {"stringValue": "hello from emulator"},
    "created_at": {"timestampValue": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"}
  }
}
JSON

echo "==> Writing doc to emulator: smoke/${DOC_ID}"
curl -sS -X POST \
  "${BASE_URL}/smoke?documentId=${DOC_ID}" \
  -H "Content-Type: application/json" \
  --data-binary @/tmp/fido-firestore-payload.json \
  >/tmp/fido-firestore-write.json

if ! grep -q '"name"' /tmp/fido-firestore-write.json; then
  echo "failed to create firestore document"
  cat /tmp/fido-firestore-write.json
  exit 1
fi

echo "==> Reading doc back"
curl -sS "${BASE_URL}/smoke/${DOC_ID}" >/tmp/fido-firestore-read.json
if ! grep -q '"hello from emulator"' /tmp/fido-firestore-read.json; then
  echo "failed to read firestore document"
  cat /tmp/fido-firestore-read.json
  exit 1
fi

echo "Firestore emulator smoke test passed."
