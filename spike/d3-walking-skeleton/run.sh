#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEV_FLEET="$REPO_ROOT/dev-fleet"
IMAGE_NAME="sandbox-spike"
IMAGE_TAG="latest"
REGISTRY="localhost:5000"

echo "=== D.3 Walking Skeleton Spike ==="
echo "Repo root: $REPO_ROOT"
echo "Dev fleet: $DEV_FLEET"
echo

# --- Step 1: Build agentd-spike for linux/amd64 ---
echo ">>> Step 1: Building agentd-spike (linux/amd64)..."
cd "$SCRIPT_DIR/agentd"

# Build in podman container for linux/amd64
podman run --rm -v "$SCRIPT_DIR/agentd:/build:Z" -w /build \
    rust:1.80-slim \
    sh -c "cargo build --release && strip target/release/agentd-spike"
BINARY="target/release/agentd-spike"

echo "    Binary: $BINARY ($(du -h "$BINARY" | cut -f1))"

# --- Step 2: Build Docker image ---
echo
echo ">>> Step 2: Building Docker image..."
cd "$SCRIPT_DIR"

# Build context needs the binary
mkdir -p image/bin
cp "agentd/$BINARY" image/bin/agentd

# Simpler Dockerfile that copies the pre-built binary
cat > image/Dockerfile.prebuilt <<'DOCKERFILE'
FROM node:24-slim

COPY bin/agentd /usr/local/bin/agentd
RUN chmod +x /usr/local/bin/agentd

ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/agentd"]
DOCKERFILE

podman build -t "$REGISTRY/$IMAGE_NAME:$IMAGE_TAG" -f image/Dockerfile.prebuilt image/
echo "    Image built: $REGISTRY/$IMAGE_NAME:$IMAGE_TAG"

# --- Step 3: Ensure dev-fleet host services are running ---
echo
echo ">>> Step 3: Starting dev-fleet host services..."
cd "$DEV_FLEET"

# Start the host-side services (postgres, controller, proxy, registry)
podman compose -f host/docker-compose.yml up -d 2>&1 | grep -v "Executing external" || true

echo "    Waiting for controller..."
for i in $(seq 1 30); do
    if curl -sf http://127.0.0.1:8080/pub/health >/dev/null 2>&1; then
        echo "    Controller healthy after ${i}s"
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "FAIL: Controller not healthy after 30s"
        podman compose -f host/docker-compose.yml logs controller
        exit 1
    fi
    sleep 1
done

# --- Step 4: Push image to local registry ---
echo
echo ">>> Step 4: Pushing image to local registry..."
podman push "$REGISTRY/$IMAGE_NAME:$IMAGE_TAG" --tls-verify=false 2>&1 || {
    echo "    Push failed. Trying with localhost..."
    podman tag "$REGISTRY/$IMAGE_NAME:$IMAGE_TAG" "localhost:5000/$IMAGE_NAME:$IMAGE_TAG"
    podman push "localhost:5000/$IMAGE_NAME:$IMAGE_TAG" --tls-verify=false
}
echo "    Pushed: $REGISTRY/$IMAGE_NAME:$IMAGE_TAG"

# --- Step 5: Bring up at least one drone VM ---
echo
echo ">>> Step 5: Ensuring at least one drone VM is running..."
cd "$DEV_FLEET"

DRONE_STATUS=$(vagrant status drone-1 2>/dev/null | grep drone-1 || echo "not created")
if echo "$DRONE_STATUS" | grep -q "running"; then
    echo "    drone-1 already running"
else
    echo "    Starting drone-1 (this may take a few minutes on first run)..."
    DRONE_COUNT=1 vagrant up drone-1
fi

# --- Step 6: Run the test ---
echo
echo ">>> Step 6: Running walking-skeleton test..."
cd "$SCRIPT_DIR/test"
npm install --silent 2>/dev/null || true

CONTROLLER_URL="http://192.168.56.1:8080" \
PROXY_URL="http://192.168.56.1:9090" \
IMAGE="$REGISTRY/$IMAGE_NAME:$IMAGE_TAG" \
npx tsx test.ts

echo
echo "=== D.3 Walking Skeleton Spike Complete ==="
