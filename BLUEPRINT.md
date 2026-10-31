# Latchix Blueprint

**Status:** draft
**Owner:** Musa Yohanes
**Started:** 2026-10-31
**License:** Apache-2.0
**Repository:** `Muyoouu/latchix`

---

## 1. Summary

Latchix is a permissively licensed, API-first SFTP server written in Rust.

It is built for platform teams that need to give customers, partners, or internal teams a managed SFTP endpoint without adopting an AGPL server, a heavyweight MFT product, or a pile of custom OpenSSH scripts.

Latchix has two modes:

1. **Standalone server.** A single binary with a REST API, a small admin UI, SQLite metadata, and a local filesystem backend.
2. **Embedded library.** The same core can be mounted inside another Rust service to expose per-tenant SFTP endpoints.

The first public release is intentionally narrow: SFTP only, local filesystem only, public key authentication only. The differentiator is not feature breadth. It is the license, the API-first control plane, the embeddable core, and the audit-first design.

---

## 2. Problem

SFTP is still the default file exchange protocol for finance, healthcare, logistics, payroll, and B2B integrations. It is boring, universal, and difficult to replace.

The current options are uneven:

- **OpenSSH** is secure and ubiquitous, but it has no control plane. User, key, and chroot changes require config edits and reloads. Building a management layer around it means writing the same scripts every team writes.
- **Mature open-source servers** are often copyleft, and the commercial editions are paid. Some companies will not adopt AGPL software, cannot embed it, or do not want the copyleft obligations.
- **Commercial MFT products** solve the problem but are expensive, closed, and heavy for small teams.
- **Custom scripts** are common, fragile, and hard to audit.

There is room for a focused, permissively licensed server with a first-class API and an embeddable core.

---

## 3. Target users

### Primary

- **Platform teams at SaaS companies** that need per-tenant SFTP endpoints for their customers.
- **Small infrastructure teams** that want a self-hosted SFTP server with an API instead of hand-written OpenSSH automation.
- **Rust shops** that want to embed an SFTP endpoint in an existing service.

### Secondary

- **Finance and operations teams** that receive bank or payment files over SFTP and need an auditable drop point.
- **Managed service providers** that run small SFTP deployments for multiple clients.
- **Homelab and self-hosted users** who want a modern, single-binary SFTP server.

### Not the target for v0.1

- Large enterprises replacing a full MFT suite.
- Organizations that require SOC 2, HIPAA, or FedRAMP certification on day one.
- Teams that need FTP, FTPS, WebDAV, AS2, or EDI gateways.

---

## 4. Positioning

**The API-first SFTP server for platform teams.**

The buying reasons, in order:

1. **Permissive license.** Apache-2.0 core. No AGPL network clause.
2. **API-first control plane.** Users, groups, permissions, quotas, and audit are managed through a documented API, not a config file and a restart.
3. **Embeddable core.** A Rust crate that can expose an SFTP endpoint inside another service.
4. **Single binary.** Low memory, fast startup, easy deployment.
5. **Audit-first.** Authentication, transfers, and admin changes are recorded.
6. **Rust.** Memory safety and a modern async stack, without making Rust the marketing message.

Rust is a credibility signal, not the product. The product is controlled, auditable SFTP without a copyleft license.

---

## 5. Principles

1. **Narrow before broad.** SFTP only. Do not add FTP, WebDAV, or an S3 gateway until the SFTP core is excellent.
2. **API first, UI second.** Every admin action must be available through the API before it appears in the UI.
3. **Secure by default.** Key authentication only, modern algorithms, no password auth in v0.1, least privilege, explicit host key verification.
4. **Audit everything that matters.** Auth, transfer, permission change, user change, key change, config change.
5. **No hidden state.** The database and the filesystem are the source of truth. No generated config files that drift.
6. **Boring operations.** Single binary, SQLite by default, Postgres later, Docker image, systemd unit, clear logs, Prometheus metrics.
7. **Permissive core.** Apache-2.0. Enterprise features can live in a separate commercial repository later.
8. **Compatibility over cleverness.** Follow the SFTP v3 specification and match OpenSSH client behavior.
9. **No feature parity races.** Compete on the wedge, not on the checklist.
10. **Security claims require evidence.** Fuzzing, audits, and reproducible benchmarks before saying "production-ready."

---

## 6. Scope

### 6.1 v0.1 MVP

The first release should be useful for a small team and safe enough for a staging environment.

**Protocol**

- SFTP v3 over SSH
- Public key authentication only
- Host key algorithms: Ed25519, RSA, ECDSA
- Modern KEX and cipher defaults
- Read, write, list, stat, mkdir, rmdir, remove, rename
- Resumable transfers if the SFTP client supports them
- File size and quota enforcement
- Path traversal protection
- Symlink policy: deny by default, optional allowlist later

**Identity and access**

- Users
- Groups
- Per-user virtual root
- Per-user and per-group permissions
- Quotas: bytes, file count, concurrent sessions
- Enable and disable accounts
- SSH public key management
- API keys for the REST API

**Storage**

- Local filesystem backend
- Root directory per user or per group
- POSIX permission checks
- Optional read-only users
- Atomic rename where the filesystem supports it

**Control plane**

- REST API with OpenAPI
- Unix socket and optional TCP listener
- Bootstrap admin token
- CRUD for users, groups, keys, quotas, and permissions
- Audit log
- Health and readiness endpoints
- Prometheus metrics

**Admin UI**

- Minimal server-rendered UI
- Login with an admin token or session
- Users, groups, keys, quotas, audit
- No design system in v0.1

**Operations**

- Single binary
- TOML config, environment variables, CLI flags
- SQLite metadata
- Docker image
- systemd unit
- Structured logs with `tracing`
- Graceful shutdown

**Quality**

- Unit tests
- Integration tests with OpenSSH `sftp`, FileZilla, WinSCP, rclone, Paramiko, and `ssh2`
- Fuzz targets for the SFTP packet parser, path handling, and config parser
- `cargo-audit`, `cargo-deny`, pinned lockfile
- Reproducible benchmark harness

### 6.2 v0.2

- Postgres metadata backend
- S3-compatible storage backend behind the `VirtualFs` trait
- Multiple virtual roots per user
- Per-user bandwidth limits
- Audit export to JSONL and webhook
- OpenTelemetry traces
- Better admin UI
- Terraform provider
- Embedded library API stabilized

### 6.3 v0.3

- Event hooks on upload, download, delete, and auth
- File lifecycle rules: archive, move, delete after N days
- Read-only share links
- Multi-tenant organizations
- OIDC for the admin UI
- mTLS for the API
- Metrics dashboard

### 6.4 v1.0

- Security audit completed
- Stable API and config format
- Upgrade and migration guarantees
- HA story with Postgres and shared storage
- Packaging for major Linux distributions
- Documented compatibility matrix
- Published performance benchmarks

### 6.5 Non-goals

- FTP, FTPS, and WebDAV
- AS2, EDI, or MFT suite features
- S3 gateway or object storage proxy
- Windows service support in v0.1
- LDAP, OIDC, and MFA in v0.1
- A visual workflow builder
- Billing and subscription management
- A drop-in replacement for every feature of an incumbent server
- Password authentication in v0.1

---

## 7. Architecture

### 7.1 Workspace layout

```text
latchix/
├── Cargo.toml                 # workspace
├── crates/
│   ├── latchix-core/          # domain types, auth, permissions, VirtualFs
│   ├── latchix-sftp/          # SSH and SFTP adapters
│   ├── latchix-api/           # REST API and OpenAPI
│   ├── latchix-store/         # SQLite and Postgres metadata
│   ├── latchix-audit/         # audit events and exporters
│   ├── latchix-cli/           # standalone binary
│   └── latchix-ui/            # server-rendered admin UI
├── docs/
│   └── decisions/             # architecture decision records
├── migrations/                # SQL migrations
├── tests/                     # integration tests and client matrix
├── benches/                   # Criterion benchmarks
├── fuzz/                      # cargo-fuzz targets
└── xtask/                     # development tasks
```

### 7.2 Runtime components

**SSH listener**

- `russh` handles the SSH transport, host keys, session channels, and authentication.
- One listener per configured bind address.
- Graceful shutdown drains active sessions.
- Per-IP and per-user connection limits.

**SFTP subsystem**

- `russh-sftp` handles the SFTP v3 packet layer.
- The subsystem adapter maps SFTP operations to `VirtualFs`.
- Unknown or unsupported extensions return a clear error, not a panic.
- File handles are scoped to a session and a virtual root.

**Authentication**

- Public key authentication.
- Keys are stored in the metadata database.
- Key parsing uses `ssh-key`.
- A key can be bound to one user and optionally one group.
- Disabled users and expired keys fail closed.

**Virtual filesystem**

- `VirtualFs` is a trait with operations for open, read, write, stat, list, mkdir, remove, rename, and metadata.
- `LocalFs` is the first implementation.
- `S3Fs` comes later.
- Every path is normalized and checked against the user's root.
- Symlinks are denied by default.
- No path escapes through `..`, encoded separators, or symlink tricks.

**Metadata store**

- `latchix-store` owns users, groups, keys, quotas, permissions, API keys, and audit events.
- SQLite for v0.1.
- Postgres for v0.2.
- Migrations are versioned and tested.
- The store is the source of truth. No generated config files.

**REST API**

- `axum` with `utoipa` for OpenAPI.
- JSON errors with stable error codes.
- API key authentication.
- Optional Unix socket listener.
- Optional TCP listener bound to localhost or a private interface.
- Rate limiting per API key.

**Admin UI**

- Server-rendered in v0.1 to keep the binary simple.
- Uses the same API internally.
- No JavaScript framework in v0.1.
- A richer SPA can come later if needed.

**Audit**

- Append-only event table.
- Event types: auth success, auth failure, upload, download, delete, rename, user created, user updated, user disabled, key added, key removed, permission changed, quota changed, config changed.
- Each event includes actor, target, timestamp, request ID, source IP, and result.
- Export to JSONL and webhook in v0.2.

**Observability**

- `tracing` for structured logs.
- Prometheus metrics at `/metrics`.
- `/healthz` and `/readyz`.
- Optional OpenTelemetry in v0.2.

### 7.3 Request flow

**SFTP login**

1. Client connects to the SSH listener.
2. `russh` presents the host key.
3. Client authenticates with a public key.
4. The auth provider looks up the key and user.
5. Disabled, expired, or over-quota users are rejected.
6. The session is bound to the user's virtual root.
7. The SFTP subsystem starts.

**File upload**

1. Client opens a file for write.
2. `VirtualFs` normalizes the path and checks the root.
3. Permission and quota checks run.
4. Data is streamed to the backend.
5. On close, the audit event is written.
6. If the client disconnects, partial files follow the configured policy: keep, delete, or quarantine.

**Admin change**

1. Admin calls the REST API with an API key.
2. The API validates the request and checks the admin role.
3. The store applies the change in a transaction.
4. The audit event is written.
5. The change is visible to the next SFTP login immediately.
6. No config reload and no restart.

### 7.4 Data model

```text
users
  id, username, enabled, root_path, quota_bytes, quota_files,
  max_sessions, created_at, updated_at

groups
  id, name, created_at, updated_at

group_members
  group_id, user_id

ssh_keys
  id, user_id, name, public_key, fingerprint, created_at, last_used_at

permissions
  id, subject_type, subject_id, path, can_read, can_write,
  can_delete, can_rename, can_mkdir, can_list

api_keys
  id, name, key_hash, role, created_at, last_used_at, expires_at

audit_events
  id, timestamp, actor_type, actor_id, action, target_type,
  target_id, source_ip, request_id, result, details_json
```

### 7.5 API surface

```text
GET    /api/v1/health
GET    /api/v1/ready

GET    /api/v1/users
POST   /api/v1/users
GET    /api/v1/users/{id}
PATCH  /api/v1/users/{id}
DELETE /api/v1/users/{id}

GET    /api/v1/groups
POST   /api/v1/groups
PATCH  /api/v1/groups/{id}
DELETE /api/v1/groups/{id}

GET    /api/v1/users/{id}/keys
POST   /api/v1/users/{id}/keys
DELETE /api/v1/users/{id}/keys/{key_id}

GET    /api/v1/audit
GET    /api/v1/metrics

GET    /api/v1/openapi.json
```

Design rules:

- Stable error codes.
- Cursor pagination.
- Idempotency keys for create operations.
- No secrets in responses.
- API keys are hashed at rest.
- Every mutation writes an audit event.

### 7.6 Configuration

Precedence: CLI flags, environment variables, TOML file, defaults.

```toml
[server]
bind = "0.0.0.0:2222"
host_keys = ["/etc/latchix/host_ed25519_key"]

[api]
unix_socket = "/run/latchix/api.sock"
tcp = "127.0.0.1:8080"
bootstrap_token_file = "/run/secrets/latchix_bootstrap_token"

[storage]
backend = "local"
root = "/var/lib/latchix/files"

[database]
url = "sqlite:///var/lib/latchix/latchix.db"

[audit]
jsonl_path = "/var/log/latchix/audit.jsonl"

[limits]
max_sessions_per_user = 10
max_connections_per_ip = 50
```

### 7.7 Deployment

- Static Linux binary, glibc and musl builds.
- Docker image with a non-root user where possible.
- systemd unit with `ProtectSystem`, `PrivateTmp`, and a dedicated user.
- The SFTP listener runs on a configurable port.
- The API should bind to a Unix socket or a private interface, not the public internet.
- SQLite is the default. Postgres is optional.
- Backups: database plus storage root.

---

## 8. Security

### 8.1 Threat model

Assume:

- The SFTP client is untrusted.
- The network is hostile.
- The API may be exposed to internal users with stolen tokens.
- The filesystem may contain symlinks and hardlinks.
- The host may have other tenants or services.

Protect against:

- Path traversal and chroot escape
- Symlink and hardlink escapes
- Authentication bypass
- Key confusion or key substitution
- Resource exhaustion: connections, sessions, file handles, disk, memory
- TOCTOU races on file operations
- Command injection through paths or metadata
- Log injection
- Quota bypass
- Privilege escalation through the API

### 8.2 Controls

- Public key authentication only in v0.1.
- Host key verification on the client side.
- Modern algorithms by default. No SHA-1, no CBC, no weak DH groups.
- Path normalization before every filesystem operation.
- Deny symlinks by default.
- Per-user virtual root enforced in `VirtualFs`, not only in the SSH layer.
- Quotas enforced at open time and during writes.
- Connection and session limits.
- API keys hashed with Argon2 or a comparable password hash.
- API rate limiting.
- Audit every auth and mutation event.
- No secrets in logs.
- Structured errors that do not leak paths or internal state.

### 8.3 Release security

- `cargo-audit` and `cargo-deny` in CI.
- `cargo-fuzz` targets for the packet parser, path handling, and config parser.
- Integration tests with real clients.
- Signed release binaries and checksums.
- SBOM for each release.
- Security policy with a private disclosure channel.
- 90-day disclosure window.
- External security audit before v1.0.

### 8.4 Claims discipline

Do not call Latchix "secure" or "production-ready" until the threat model, fuzzing, integration tests, and an external audit are complete. Memory safety is not a security guarantee.

---

## 9. Compatibility and testing

### 9.1 Client matrix

Test at least:

- OpenSSH `sftp`
- FileZilla
- WinSCP
- Cyberduck
- rclone
- Paramiko
- `ssh2` for Node.js
- Termius
- Transmit

### 9.2 Protocol coverage

- SFTP v3
- `limits@openssh.com`
- `fsync@openssh.com`
- `hardlink@openssh.com` only if the backend supports it
- `statvfs@openssh.com`
- `expand-path@openssh.com`

### 9.3 Test strategy

- Unit tests for path normalization, permissions, quotas, and key parsing.
- Integration tests that start a real server and connect with real clients.
- Property tests for path handling and permission checks.
- Fuzz tests for packet parsing and config parsing.
- Concurrency tests for uploads, renames, and deletes.
- Failure tests for disk full, permission denied, and backend timeouts.
- Benchmark tests in CI, but not as a merge gate until stable.

### 9.4 CI

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo audit`
- `cargo deny check`
- `cargo doc`
- Docker build
- Integration tests with OpenSSH client

---

## 10. Performance

Performance is a differentiator only if it is real and reproducible.

Measure:

- Upload throughput
- Download throughput
- Small-file latency
- Concurrent sessions
- Memory per session
- Startup time
- CPU under encryption

Compare against:

- A stock SSH server's SFTP subsystem
- A mature open-source SFTP server baseline
- A minimal `russh` + `russh-sftp` baseline

Publish:

- Hardware
- Kernel
- Filesystem
- Client version
- Test script
- Raw results

Do not optimize before correctness. Do not publish cherry-picked numbers.

---

## 11. Licensing and governance

### 11.1 License

- Core: Apache-2.0.
- Rationale: permissive, patent grant, commercial-friendly, compatible with the Rust ecosystem.
- Do not copy AGPL code into the core.
- Enterprise features, if any, live in a separate repository with a commercial license.
- Contributions use DCO, not a CLA, unless a CLA becomes necessary for open-core relicensing. Prefer DCO.

### 11.2 Governance

- Benevolent maintainer for v0.x.
- Public roadmap.
- RFC process for major changes.
- Architecture decision records in `docs/decisions/`.
- Security policy and code of conduct before public release.
- No corporate-controlled roadmap in v0.x.

### 11.3 Trademark

- Latchix is the project name.
- Do not use any third-party project name in the Latchix name, domain, or logo.
- Comparative references are allowed if truthful and not misleading.
- Third-party product names, when used, are for technical identification and compatibility only. No affiliation or endorsement is implied.
- Publish a trademark policy before v1.0.

---

## 12. Brand

- Name: **Latchix**
- Domain: `latchix.com` (register before public launch)
- Tagline: **The API-first SFTP server.**
- Tone: technical, direct, calm. No hype, no security theater.
- Visual identity: simple wordmark, no lock clipart, no shield clichés.
- Package names: `latchix`, `latchix-core`, `latchix-sftp`, `latchix-api`, `latchix-store`, `latchix-cli`.

---

## 13. Success metrics

### 90 days

- 10 customer or platform-team interviews
- 3 design partners
- 100 GitHub stars
- 1 working demo with OpenSSH, FileZilla, and rclone
- 1 reproducible benchmark post

### 12 months

- 1,000 GitHub stars
- 5 production deployments
- 3 external contributors
- Stable v0.3
- Postgres and S3 support
- 1 security audit scheduled

### 24 months

- v1.0
- 10 paying enterprise or support customers, if commercialized
- Documented compatibility matrix
- Published security audit
- Sustainable maintenance model

---

## 14. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Feature creep | Delays v0.1 indefinitely | Enforce non-goals. SFTP only. |
| Security incident | Destroys trust | Threat model, fuzzing, audit, disclosure policy |
| `russh-sftp` gaps | Blocks protocol work | Contribute upstream, keep a fallback plan, test early |
| Weak adoption | No users, no feedback | Build in public, design partners, benchmarks, launch posts |
| Incumbent comparison | Unfavorable feature checklist | Compete on license, API, embeddability, not breadth |
| Support burden | Burnout | Clear support policy, community forum, paid support later |
| License confusion | Adoption friction | Apache-2.0, clear FAQ, no AGPL dependencies |
| Trademark conflict | Legal risk | Name search, domain registration, trademark policy |
| Performance claims | Credibility loss | Reproducible benchmarks, honest results |
| Windows support | Scope explosion | Linux first, Windows later |

---

## 15. Open questions

1. Should v0.1 support password authentication? Recommendation: no.
2. Should v0.1 support Windows? Recommendation: Linux first.
3. Should v0.1 include S3? Recommendation: no, local filesystem first.
4. Should v0.1 include an admin UI? Recommendation: a minimal server-rendered UI, not a SPA.
5. Should the API be compatible with existing servers? Recommendation: no, design a clean API.
6. Should the embedded library be public in v0.1? Recommendation: design for it, expose in v0.2.
7. Should Postgres be in v0.1? Recommendation: no, SQLite first.
8. Should the project accept contributions before v0.2? Recommendation: yes, but limit to docs, tests, and good first issues.
9. Should there be a commercial edition? Recommendation: decide after 3 design partners.
10. Should the project join a foundation? Recommendation: not before v1.0.

---

## 16. First 90 days

### Weeks 1-2

- 10 interviews with platform and infrastructure teams
- Register `latchix.com`
- Publish the blueprint
- Set up the repository, CI, and issue templates
- Write the security policy and code of conduct

### Weeks 3-6

- Spike: `russh` + `russh-sftp` server with public key auth and local filesystem
- Pass OpenSSH `sftp`, FileZilla, and rclone smoke tests
- Add path normalization and traversal tests
- Add fuzz targets for path handling

### Weeks 7-10

- SQLite metadata store
- Users, groups, keys, quotas
- REST API with OpenAPI
- Audit log
- Docker image and systemd unit

### Weeks 11-12

- Reproducible benchmark against a stock SSH server and a mature open-source SFTP server
- Demo video
- Launch post on Hacker News, r/rust, r/selfhosted, r/sysadmin
- Collect feedback and decide v0.2 scope

### Gate

If the spike cannot pass the client matrix, stop and reassess. The protocol layer is the risk. Do not build the UI or the API before the SFTP core works with real clients.

---

## 17. Decision log

| Date | Decision | Rationale |
|---|---|---|
| 2026-10-31 | Name: Latchix | Short, readable, security-adjacent, distinct from existing projects |
| 2026-10-31 | Language: Rust | Memory safety, performance, strong async ecosystem |
| 2026-10-31 | License: Apache-2.0 | Permissive, patent grant, commercial-friendly |
| 2026-10-31 | SSH: `russh` | Mature Rust SSH implementation, Apache-2.0 |
| 2026-10-31 | SFTP: `russh-sftp` | Server and client support, Apache-2.0, active |
| 2026-10-31 | API: `axum` + `utoipa` | Simple, typed, OpenAPI-friendly |
| 2026-10-31 | Metadata: SQLite first | Single binary, zero external dependencies |
| 2026-10-31 | Storage: local filesystem first | Smallest useful server, S3 later |
| 2026-10-31 | Scope: SFTP only | Focus is the differentiator |
| 2026-10-31 | Repository visibility: private | Blueprint stage, public before launch |
