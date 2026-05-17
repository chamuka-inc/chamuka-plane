#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Checking prerequisites..."

if ! command -v docker &>/dev/null; then
    echo "ERROR: docker not found. Install Docker to run the walking skeleton."
    exit 1
fi

if ! docker info &>/dev/null; then
    echo "ERROR: Docker daemon not running."
    exit 1
fi

echo "==> Building runtime image..."
"$REPO_ROOT/runtime-images/build.sh" node24 latest

echo "==> Starting infrastructure..."
cd "$SCRIPT_DIR"
docker compose up -d --wait

echo "==> Waiting for controller to be healthy..."
for i in $(seq 1 30); do
    if curl -sf http://localhost:9090/health >/dev/null 2>&1; then
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "ERROR: Controller did not become healthy in 30s"
        docker compose logs
        docker compose down
        exit 1
    fi
    sleep 1
done

echo "==> Running walking skeleton test..."
npx --yes tsx "$SCRIPT_DIR/test.ts"
EXIT_CODE=$?

echo "==> Tearing down infrastructure..."
docker compose down

exit $EXIT_CODE
