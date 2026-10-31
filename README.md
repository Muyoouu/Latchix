# Latchix

**The API-first SFTP server.**

Latchix is a Rust SFTP server with a first-class control plane. Tenants, users, groups, grants, quotas, audit logs, and metrics are managed through an HTTP API and an opt-in admin UI. The same core is designed to run as a standalone binary or be embedded in another Rust service.

Status: **blueprint**. There is no runtime code yet.

- [Project blueprint](BLUEPRINT.md)
- [v0.1 plan](docs/PLAN.md)
- [Design notes](docs/README.md)
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
- Single binary, Docker image, systemd unit
- Apache-2.0 core

## What it will not be

- An FTP, FTPS, or WebDAV server
- An S3 gateway
- A high-availability clustered platform in v0.1
- An LDAP, OIDC, or MFA product in v0.1
- A clone of another SFTP server
- A drop-in replacement for every feature of every MFT product

## Contributing

The project is not ready for contributions yet. The blueprint is the first artifact. Once the v0.1 scope and architecture are settled, contribution guidelines and good first issues will follow.

## Security

Do not run early builds in production. A security policy and disclosure process will be published before the first public release.
