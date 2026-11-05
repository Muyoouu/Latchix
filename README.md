# Latchix

**The API-first SFTP server.**

Latchix is a Rust SFTP server with a first-class control plane. Tenants, users, groups, grants, quotas, audit logs, and metrics are managed through an HTTP API and an opt-in admin UI. The same core is designed to run as a standalone binary or be embedded in another Rust service.

Status: **Phase 0 scaffold**. The binary builds and serves health endpoints.
SFTP, metadata, the control plane, and the admin UI are not implemented yet.

- [Project blueprint](BLUEPRINT.md)
- [v0.1 plan](docs/PLAN.md)
- [CLI contract](docs/CLI.md)
- [Admin UI design](DESIGN.md)
- [Design notes](docs/README.md)
- [Packaging](packaging/README.md)
- [Security policy](SECURITY.md)
- [Contributing](CONTRIBUTING.md)
- License: Apache-2.0

## Why

SFTP remains the default file exchange protocol for finance, healthcare, logistics, and B2B integrations. The mature open-source servers are either copyleft or commercial. Some teams need a permissive license, a single binary, and an embeddable core.

Latchix is for those teams.

## What it will be

- SFTP over SSH with public key authentication
- Local filesystem first, S3-compatible storage later
- Tenant-scoped users, groups, roots, grants, and quotas from v0.1
- Allow-only ACL with ancestor traversal, expiry, and revocation
- SQLite metadata with a single-binary deployment
- REST API with OpenAPI, plus a Unix socket
- Audit log and Prometheus metrics
- Opt-in admin UI
- Static binary, optional OCI image, systemd unit
- Apache-2.0 core

## Current state

Phase 0 provides:

- A Cargo workspace with core, SFTP, API, store, audit, CLI, and UI crates.
- `latchix serve` with `/healthz` and `/readyz`.
- `latchix health` with path checks and an API readiness probe.
- `latchix doctor` with build, SQLite, host key, and path checks.
- Explicit `migrate`, `backup`, and `bootstrap-token` stubs for Phase 2 and 3.
- A static musl target, a glibc fallback, a distroless Containerfile, a systemd
  unit, and a rootless Podman Quadlet.
- CI for format, Clippy, tests, docs, cargo-deny, cargo-audit, both binary
  targets, and a read-only container smoke test.

The SSH/SFTP listener is the next implementation step.

## What it will not be

- An FTP, FTPS, or WebDAV server
- An S3 gateway
- A high-availability clustered platform in v0.1
- An LDAP, OIDC, or MFA product in v0.1
- A clone of another SFTP server
- A drop-in replacement for every feature of every MFT product

## Contributing

The project is pre-release. Contributions are limited to docs, tests, and
reproducible issues before v0.2. Read [CONTRIBUTING.md](CONTRIBUTING.md) first.

## Security

Do not run early builds in production. Report vulnerabilities according to
[SECURITY.md](SECURITY.md). Do not open a public issue for a vulnerability.
