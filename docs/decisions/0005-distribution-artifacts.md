# ADR 0005: Distribution artifacts

- Status: Accepted
- Date: 2026-11-01
- Owners: Musa Yohanes
- Supersedes: none

## Context

Latchix can ship as a native binary, an OCI image, or a Rust library. The blueprint promises a single binary and a future embedded core. Users include systemd hosts, container platforms, and Rust services that want to mount an SFTP endpoint inside their own process.

A container-only product would undermine the single-binary promise and the embedded core. A library-first product would create a semver contract before the embedded API is proven. Neither is acceptable for v0.1.

## Decision

1. The primary artifact is a static Linux binary. Ship `x86_64-unknown-linux-musl` first and `x86_64-unknown-linux-gnu` as a fallback.
2. The secondary artifact is an OCI image. It is Docker-compatible and built with rootless Podman. It is a packaging channel, not the product architecture.
3. The runtime image uses `distroless/static-debian12:nonroot`, runs as non-root, has a read-only root filesystem, and receives data, config, and secrets through mounts.
4. Never bake the database, host keys, bootstrap token, or config into the image.
5. Publish the systemd unit and a Podman Quadlet example as first-class deployment paths.
6. The CLI exposes `serve`, `migrate`, `backup`, `health`, `bootstrap-token`, and `doctor` from day one.
7. Library crates stay internal in v0.1. Publish `latchix-core` or a `latchix` facade in v0.2 after the embedded API stabilizes.
8. Multi-arch images are post-v0.1. Build amd64 first, then arm64 after the client matrix and static build pass.
9. `russh` default crypto is `aws-lc-rs`. If it blocks the static musl build, switch to the `ring` feature instead of weakening the static target.

## Consequences

- The release pipeline builds two binary targets and one image.
- API-only and embedded users are not forced into a container.
- Container users get a small, shell-free runtime with a narrow attack surface.
- The library is a later product with a deliberate public API and semver policy.
- The `health` subcommand exists because the distroless image has no shell or curl.
- The `backup` and `migrate` subcommands make container operations possible without `podman exec` into a shell.

## Alternatives considered

- Container-only. Rejected. Contradicts the single-binary promise and blocks embedded users.
- Library-first. Rejected. Freezes an unproven API and adds support debt before the SFTP core is stable.
- Docker-only build. Rejected. Rootless Podman is installed and sufficient. The Containerfile stays Docker-compatible.
- Full distro base image. Rejected. Larger image, package manager, shell, and a wider CVE surface for no runtime benefit.
- Alpine runtime. Rejected. A static musl binary does not need a userland. Distroless gives a smaller surface.
- `scratch` runtime. Rejected for the default image. Distroless static includes CA certificates and a non-root user, which later webhooks and OIDC need.
- Publishing all workspace crates. Rejected. Internal crates are not a public API.

## Evidence

- `cargo` and `rustc` 1.99.1 are installed through rustup.
- A scratch project compiled `russh` 0.65.0, `russh-sftp` 3.0.1, `rusqlite` 0.40.2 with `bundled`, `rusqlite_migration` 2.6.0, `deadpool-sqlite` 0.14.0, and `rustix` 1.1.6.
- A minimal SSH server and SFTP handler using `VerifiedPublicKey`, `Channel::into_stream`, and `russh_sftp::server::run` compiled cleanly.
- A Rust `openat2` containment test matched the C test: dotdot, absolute paths, and symlinks denied, hardlink opened.
- Podman is installed. Docker is not. Rootless Podman is the supported build and test path.

## Open items

- Verify the static musl build with `aws-lc-rs`. Fall back to `ring` only if needed.
- Measure binary and image size after the first real build.
- Add arm64 after v0.1.
- Add image signing, SBOM, and provenance after the release pipeline stabilizes.
