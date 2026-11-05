# Testing in containers

Phase 1 tests run in a rootless Podman container so the host needs no Rust,
LLVM, fuzz, or SFTP client installs. `Containerfile.test` is test-only and is
not part of the release image.

## Requirements

- Rootless Podman with user namespaces enabled (`podman info` shows
  `rootless: true`).
- `id -u` must not be 0.
- Roughly 5 GiB of free space in Podman storage for the image and Cargo
  caches.

Everything written by the container lives in Podman storage:

- `/src` is the repository mounted read-only (`:ro,Z`).
- `latchix-test-target` is a named volume mounted at `/target`.
- `latchix-test-cargo-home` is a named volume mounted at `/cargo`.
- `/tmp` is a 4 GiB tmpfs.
- The container runs as the host UID/GID with `--userns=keep-id`,
  `--read-only`, `--security-opt=no-new-privileges`, and `--cap-drop=ALL`.

The named volumes are chowned to the host UID on use (`:U`), so no host-root
owned files are produced.

## Build the image

The runner builds the image automatically if it is missing. To force a rebuild
after changing `Containerfile.test`:

```bash
scripts/test-container.sh --build -- true
```

Or build directly:

```bash
podman build -f Containerfile.test -t localhost/latchix-test:1.99.1 .
```

The image contains the Debian build toolchain, Clippy and rustfmt, the musl
target pinned by `rust-toolchain.toml`, a nightly toolchain, and `cargo-fuzz`.
The first build takes several minutes because it downloads the toolchains and
compiles `cargo-fuzz`.

## Run the tests

Default command:

```bash
scripts/test-container.sh
# cargo test --locked --workspace --all-features
```

Run a subset or another command by passing arguments after `--`:

```bash
scripts/test-container.sh -- cargo test -p latchix-sftp
scripts/test-container.sh -- cargo test --workspace --all-features -- --nocapture
scripts/test-container.sh -- cargo clippy --workspace --all-targets --all-features -- -D warnings
scripts/test-container.sh -- bash
```

The image is not rebuilt unless `--build` is given. Environment overrides:

```bash
LATCHIX_TEST_IMAGE=localhost/latchix-test:dev \
LATCHIX_TEST_TARGET_VOLUME=latchix-target-dev \
scripts/test-container.sh
```

To start from clean Cargo state:

```bash
podman volume rm -f latchix-test-target latchix-test-cargo-home
```

Then the next run repopulates the volumes from the image.

## Fuzz smoke test

Cargo fuzzing writes corpora, artifacts, and a separate target directory, so it
cannot run against the read-only `/src` mount. Copy the tree into the tmpfs
first, then run a time-boxed smoke test:

```bash
scripts/test-container.sh -- bash -c '
  set -e
  mkdir -p /tmp/latchix
  tar -C /src \
    --exclude=./target --exclude=./.ci --exclude=./.tools \
    --exclude=./.git --exclude=./.cargo-home \
    --exclude=./fuzz/target --exclude=./fuzz/corpus --exclude=./fuzz/artifacts \
    -cf - . | tar -C /tmp/latchix -xf -
  cd /tmp/latchix
  cargo +nightly fuzz run fuzz_path -- -max_total_time=60
'
```

List the available targets without running them:

```bash
scripts/test-container.sh -- bash -c '
  mkdir -p /tmp/latchix
  tar -C /src \
    --exclude=./target --exclude=./.ci --exclude=./.tools \
    --exclude=./.git --exclude=./.cargo-home \
    --exclude=./fuzz/target --exclude=./fuzz/corpus --exclude=./fuzz/artifacts \
    -cf - . | tar -C /tmp/latchix -xf -
  cd /tmp/latchix
  cargo +nightly fuzz list
'
```

Targets: `fuzz_path` for the virtual path parser and `fuzz_packet` for the
`russh-sftp` packet decoder. A smoke run passes when libFuzzer reports no
crashes for the time limit.

Artifacts under `/tmp` are discarded when the container exits. Copy any
reproducer out with `podman cp` before it stops if needed.

## Dev SFTP server

Run the Phase 1 dev server in a rootless container and publish only localhost:

```bash
scripts/sftp-dev-container.sh
```

The script keeps the source read-only, stores the root and generated keys in
`.ci/sftp-dev`, publishes `127.0.0.1:2222`, and prints the `sftp` command.
FileZilla uses the same host, port, user `demo`, and key file. The manual
checklist lives in [CLIENT-MATRIX.md](CLIENT-MATRIX.md).

The automated client matrix uses the `openssh-client` and `rclone` binaries
installed in the image. Tests that need a client write their config, batch
files, and downloaded data under `/tmp`; nothing touches the checkout.

## Limitations

- The host checkout is read-only. Commands that need to create files in the
  repository (`cargo fmt`, `cargo update`, fuzz output) must write to a copy or
  to a mounted volume.
- Network access is needed for the first image build and for Cargo dependency
  downloads. There is no offline mode.
- The test runner publishes no ports. `sftp-dev-container.sh` is the separate
  loopback-only dev entry point.
