#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CRATES_DIR="$REPO_ROOT/crates"

IMAGE_NAME="${1:-node24}"
TAG="${2:-latest}"
FULL_TAG="sandbox-runtime:${IMAGE_NAME}-${TAG}"

echo "==> Building agentd + sandbox-init for linux/amd64..."
cd "$CRATES_DIR"

# Build for linux — use cross if available, otherwise require native linux or docker build
if command -v cross &>/dev/null; then
    cross build --release -p agentd --bins --target x86_64-unknown-linux-gnu
    BIN_DIR="$CRATES_DIR/target/x86_64-unknown-linux-gnu/release"
elif [[ "$(uname -s)" == "Linux" ]]; then
    cargo build --release -p agentd --bins
    BIN_DIR="$CRATES_DIR/target/release"
else
    echo "==> Not on Linux and 'cross' not installed."
    echo "    Install cross: cargo install cross"
    echo "    Or build inside Docker (using multi-stage build)."
    echo ""
    echo "==> Falling back to multi-stage Docker build..."
    cd "$SCRIPT_DIR/$IMAGE_NAME"
    docker build \
        -f Dockerfile.multistage \
        -t "$FULL_TAG" \
        "$REPO_ROOT"
    echo "==> Built $FULL_TAG (multi-stage)"
    exit 0
fi

echo "==> Copying binaries to image context..."
cp "$BIN_DIR/agentd" "$SCRIPT_DIR/$IMAGE_NAME/agentd"
cp "$BIN_DIR/sandbox-init" "$SCRIPT_DIR/$IMAGE_NAME/sandbox-init"

echo "==> Building Docker image: $FULL_TAG"
cd "$SCRIPT_DIR/$IMAGE_NAME"
docker build -t "$FULL_TAG" .

rm -f "$SCRIPT_DIR/$IMAGE_NAME/agentd" "$SCRIPT_DIR/$IMAGE_NAME/sandbox-init"

echo "==> Done: $FULL_TAG"
