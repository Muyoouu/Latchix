#!/usr/bin/env bash
# Run Latchix tests in a rootless Podman container.
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

IMAGE=${LATCHIX_TEST_IMAGE:-localhost/latchix-test:1.99.1}
CARGO_HOME_VOLUME=${LATCHIX_TEST_CARGO_HOME_VOLUME:-latchix-test-cargo-home}
TARGET_VOLUME=${LATCHIX_TEST_TARGET_VOLUME:-latchix-test-target}
PODMAN=${PODMAN:-podman}

usage() {
    cat <<EOF
Usage: $(basename "$0") [--build] [--] [command ...]

Run the Phase 1 test environment in rootless Podman.

The repository is mounted read-only at /src. Cargo's target directory and
CARGO_HOME live in named Podman volumes, so the host checkout is not modified.
Any files created by the tests stay inside Podman storage.

Arguments after -- (or the first non-option) replace the default command.

Examples:
  $(basename "$0")                              # cargo test --workspace --all-features
  $(basename "$0") --build                      # rebuild image, then test
  $(basename "$0") -- cargo test -p latchix-sftp
  $(basename "$0") -- bash
  $(basename "$0") -- cargo +nightly fuzz list

Environment:
  LATCHIX_TEST_IMAGE               image tag (default: ${IMAGE})
  LATCHIX_TEST_CARGO_HOME_VOLUME   cargo home volume (default: ${CARGO_HOME_VOLUME})
  LATCHIX_TEST_TARGET_VOLUME       target volume (default: ${TARGET_VOLUME})
  PODMAN                           podman binary (default: ${PODMAN})
EOF
}

build=0
while (($# > 0)); do
    case "$1" in
        --build) build=1; shift ;;
        --help | -h) usage; exit 0 ;;
        --) shift; break ;;
        -*) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
        *) break ;;
    esac
done

command=("$@")
if ((${#command[@]} == 0)); then
    command=(cargo test --locked --workspace --all-features)
fi

if ! command -v "$PODMAN" >/dev/null 2>&1; then
    echo "error: $PODMAN is not installed" >&2
    exit 1
fi

if ((build)) || ! "$PODMAN" image exists "$IMAGE"; then
    echo "building $IMAGE from $REPO_ROOT/Containerfile.test" >&2
    "$PODMAN" build -f "$REPO_ROOT/Containerfile.test" -t "$IMAGE" "$REPO_ROOT"
fi

args=(
    run --rm
    --read-only
    --userns=keep-id
    --user "$(id -u):$(id -g)"
    --security-opt=no-new-privileges
    --cap-drop=ALL
    --tmpfs /tmp:rw,nosuid,nodev,size=4g
    -e HOME=/tmp
    -e CARGO_HOME=/cargo
    -e CARGO_TARGET_DIR=/target
    -e CARGO_TERM_COLOR=always
    -v "$REPO_ROOT:/src:ro,Z"
    -v "$CARGO_HOME_VOLUME:/cargo:U"
    -v "$TARGET_VOLUME:/target:U"
    -w /src
)

if [[ -t 0 && -t 1 ]]; then
    args+=(-it)
fi

exec "$PODMAN" "${args[@]}" "$IMAGE" "${command[@]}"
