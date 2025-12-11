#!/bin/bash
set -e

echo "=== MINIMAL FIDO SERVER STARTUP ==="
echo "Environment variables:"
env | grep -E "(HOST|PORT|DATABASE|RUST)" || echo "No relevant env vars found"

echo "Binary check:"
ls -la /usr/local/bin/fido-server

echo "Starting fido-server directly..."
exec /usr/local/bin/fido-server