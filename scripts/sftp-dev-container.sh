#!/usr/bin/env bash
# Run the Latchix dev SFTP server in a rootless Podman container.
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)
IMAGE=${LATCHIX_TEST_IMAGE:-localhost/latchix-test:1.99.1}
CARGO_HOME_VOLUME=${LATCHIX_TEST_CARGO_HOME_VOLUME:-latchix-test-cargo-home}
TARGET_VOLUME=${LATCHIX_TEST_TARGET_VOLUME:-latchix-test-target}
PODMAN=${PODMAN:-podman}
PORT=${LATCHIX_SFTP_DEV_PORT:-2222}
DATA_DIR="$REPO_ROOT/.ci/sftp-dev"

if ! command -v "$PODMAN" >/dev/null 2>&1; then
    echo "error: $PODMAN is not installed" >&2
    exit 1
fi

if ! "$PODMAN" image exists "$IMAGE"; then
    echo "building $IMAGE from $REPO_ROOT/Containerfile.test" >&2
    "$PODMAN" build -f "$REPO_ROOT/Containerfile.test" -t "$IMAGE" "$REPO_ROOT"
fi

mkdir -p "$DATA_DIR"
"$PODMAN" rm -f latchix-sftp-dev >/dev/null 2>&1 || true

echo "Host data dir: $DATA_DIR" >&2
echo "FileZilla: 127.0.0.1:$PORT, user demo, key $DATA_DIR/id_ed25519" >&2

args=(
    run --rm
    --name latchix-sftp-dev
    --read-only
    --userns=keep-id
    --user "$(id -u):$(id -g)"
    --security-opt=no-new-privileges
    --cap-drop=ALL
    --tmpfs /tmp:rw,nosuid,nodev,size=1g
    -e HOME=/tmp
    -e CARGO_HOME=/cargo
    -e CARGO_TARGET_DIR=/target
    -p "127.0.0.1:${PORT}:2222"
    -v "$REPO_ROOT:/src:ro,Z"
    -v "$CARGO_HOME_VOLUME:/cargo:U"
    -v "$TARGET_VOLUME:/target:U"
    -v "$DATA_DIR:/data:U,Z"
    -w /src
)

if [[ -t 0 && -t 1 ]]; then
    args+=(-it)
fi

exec "$PODMAN" "${args[@]}" "$IMAGE" \
    cargo run --locked --example dev_server -- \
    --bind 0.0.0.0:2222 --data-dir /data
