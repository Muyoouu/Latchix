# Latchix v0.1 plan

- Status: Active
- Date: 2026-11-01
- Owner: Musa Yohanes
- Architecture: [BLUEPRINT.md](../BLUEPRINT.md)
- Decisions: [docs/decisions](decisions)

This plan turns the accepted architecture into an execution order. The blueprint is the source of truth for scope. This file is the source of truth for sequence and acceptance.

## Goal

Ship a static single-binary, permissively licensed SFTP server that passes the client matrix and proves tenant isolation, ACL enforcement, and the API-first control plane on one host with SQLite metadata. Provide an optional OCI image and keep the Rust library internal until v0.2.

## Artifacts

| Artifact | v0.1 | Role |
|---|---|---|
| Static musl binary | Yes, primary | Portable single file, systemd, downloads |
| glibc binary | Yes, fallback | Distro compatibility |
| OCI image | Yes, optional | Demo, container deployments, CI |
| systemd unit | Yes | Native Linux host |
| Podman Quadlet | Yes | Rootless systemd |
| Rust library crates | Internal only | Public `latchix-core` in v0.2 |
| Multi-arch image | Deferred | arm64 after v0.1 |
| Helm chart, deb, rpm, brew | Deferred | Later distribution |

Packaging rules:

- The binary is the product. The container is a packaging channel.
- Build `x86_64-unknown-linux-musl` first, `x86_64-unknown-linux-gnu` as fallback.
- The OCI image is Docker-compatible, built with rootless Podman.
- Runtime image is `distroless/static-debian12:nonroot`, non-root, read-only rootfs.
- Mount data, config, host keys, and bootstrap token. Never bake them into the image.
- `health`, `backup`, and `migrate` are CLI subcommands because the image has no shell.
- Keep `ring` as a fallback feature if `aws-lc-rs` blocks the musl build.

## Requirement coverage

| Requirement | Design record | Plan phase |
|---|---|---|
| Hard multi-tenant isolation | ADR 0001, blueprint sections 6.1, 7.4, 8.2 | Phase 2 |
| Multiple users per tenant | ADR 0001, blueprint section 7.4 | Phase 2 |
| ACL and effective rights | ADR 0002 | Phase 2 |
| Dynamic grants, expiry, revocation | ADR 0002, blueprint section 7.3 | Phase 2 |
| Per-tenant root and chroot CRUD | ADR 0001, blueprint section 7.4 and 7.5 | Phase 2, Phase 3 |
| API-first control plane | ADR 0004, blueprint section 7.5 | Phase 3 |
| Opt-in admin UI | ADR 0004, blueprint section 6.1 | Phase 4 |
| SQLite single-host metadata | ADR 0003 | Phase 2 |
| Static binary and optional OCI image | ADR 0005 | Phase 0, Phase 5 |

## Non-negotiables

- SFTP v3 works with real clients before UI polish or alternate backends.
- Every tenant-owned row, query, path, and audit event carries a tenant.
- The database ACL is the only authorization authority.
- Every filesystem operation is root-descriptor relative. No string prefix checks.
- SQLite only. No Postgres, DuckDB, or DuckLake in the metadata path.
- The UI is an opt-in feature. API-only and embedded builds do not pay for it.
- The primary artifact is the static binary. The container is optional.
- No database, host key, bootstrap token, or config is baked into an image.
- Library crates stay internal in v0.1.
- No feature ships without its test and its audit event.

## Phase 0: Decisions and scaffolding

Weeks 1-2.

Deliverables:

- Tenancy, ACL, metadata, control-plane, and distribution ADRs accepted.
- Security policy and code of conduct.
- Repository, CI, issue templates, `cargo-deny`, `cargo-audit`, pinned lockfile.
- `DESIGN.md` started before any template work.
- Default tenant and storage root conventions agreed.
- Build spike for musl static and glibc binaries.
- Podman image spike with a non-root, read-only runtime and mounted volumes.
- CLI command contract for `serve`, `migrate`, `backup`, `health`, `bootstrap-token`, `doctor`.

Exit criteria:

- ADRs merged.
- CI runs format, clippy, test, doc, audit, and deny.
- Both binary targets build in CI.
- The image builds and starts with a mounted data volume.
- No open question blocks the schema.

### Phase 0 status

- [x] ADRs 0001 to 0006 accepted.
- [x] Security policy, code of conduct, contributing guide, and issue templates.
- [x] Workspace, pinned toolchain, `Cargo.lock`, `cargo-deny`, `cargo-audit`, and CI workflow.
- [x] `DESIGN.md` started before template work.
- [x] Default tenant and storage layout recorded in ADR 0006.
- [x] CLI contract and command stubs.
- [x] musl static and glibc release builds verified locally.
- [x] Podman image builds and starts read-only, non-root, with mounted data.
- [ ] First GitHub Actions run.

Notes:

- The local musl build ran in the `rust:1.99.1-alpine` container because
  `musl-gcc` is not installed on the development host. CI installs
  `musl-tools`.
- `cargo-deny` and `cargo-audit` pass with the documented
  `RUSTSEC-2023-0071` mitigation. RSA host keys are rejected at config
  validation.

## Phase 1: Protocol and containment

Weeks 3-6.

Deliverables:

- `russh` + `russh-sftp` server with public key authentication and a local filesystem.
- OpenSSH `sftp` and rclone automated smoke tests pass.
- FileZilla manual compatibility check is recorded.
- Root descriptor opened once per session.
- `openat2` containment with `RESOLVE_BENEATH`, `RESOLVE_NO_SYMLINKS`, and `RESOLVE_NO_MAGICLINKS`.
- Canonical path parser with rejection tests for `..`, absolute paths, empty segments, and trailing slash.
- Symlink denied. Hardlink denied by default, allowed only in an explicit single-tenant mode.
- Rename restricted to one root.
- Fuzz targets for the packet parser and path handling.
- Real-client integration harness.

Exit criteria:

- Gate 1: automated client matrix for OpenSSH `sftp` and rclone passes.
  FileZilla manual checklist is recorded.
- Escape tests fail closed for `..`, absolute paths, symlinks, and hardlinks.
- No panic on malformed SFTP packets.

## Phase 2: Metadata and ACL

Weeks 7-10.

Deliverables:

- SQLite schema: `tenants`, `roots`, `users`, `groups`, `group_members`, `ssh_keys`, `grants`, `api_keys`, `sessions`, `usage`, `idempotency_keys`, `audit_events`, `share_links`.
- Embedded forward-only migrations.
- Composite foreign keys and `UNIQUE(tenant_id, id)` on parents.
- Partial unique indexes for grant subjects.
- Schema invariants for grant rights and expiry, soft deletes, API key scope, and share-link rights.
- Soft-delete lifecycle for users, groups, and roots with partial unique indexes over live rows.
- Writer actor, read pool, `busy_timeout`, `synchronous=FULL`, `foreign_keys=ON` on every connection.
- Process lock on the database file.
- Backup and restore path with `VACUUM INTO`.
- ACL engine with the rights bitmask, ancestor `TRAVERSE`, subtree and exact scope, expiry, revocation, and revision snapshots.
- Read-only user and read-only group membership masks.
- Effective-permissions resolver with `explain`.
- Quota reservation and reconciliation.
- Tenant isolation test suite and ACL matrix.

Exit criteria:

- Gate 2: tenant isolation suite and ACL matrix green.
- Cross-tenant user, group, root, and grant references are rejected by the database.
- Revocation is visible on the next operation in an active session.
- Permission changes and quota reservations survive a crash and restart.
- Backup restores into the current schema version.

## Phase 3: Control plane

Weeks 7-10, parallel with Phase 2 after the schema lands.

Deliverables:

- `axum` + `utoipa` API with the tenant-scoped surface from the blueprint.
- `problem+json` error registry.
- ETag and `If-Match` on mutations.
- Idempotency keys scoped to tenant.
- Cursor pagination on `(created_at, id)`.
- API key issue, rotate, revoke, prefix index, SHA-256 or HMAC, constant-time compare.
- Single-use bootstrap token.
- Roles and tenant scoping.
- Unix socket with `SO_PEERCRED`.
- Rate limits per IP, key, and tenant.
- Audit append-only triggers and tenant-scoped read path.
- OpenAPI JSON and YAML, with a CI drift check.

Exit criteria:

- Every mutation appears in the audit log in the same transaction.
- A tenant-scoped key cannot read or write another tenant.
- A missing `If-Match` returns 428 and a stale `If-Match` returns 412.
- A retried create returns the stored response, not a duplicate row.
- OpenAPI covers every route and error code.

## Phase 4: Admin UI

Weeks 11-12.

Deliverables:

- `ui` Cargo feature, on for the standalone binary only.
- Server-rendered pages: login, tenant switcher, users, groups, keys, grants, quota usage, audit.
- Login exchanges an API key for a server-side session. No admin password table.
- Cookie session with `HttpOnly`, `Secure`, `SameSite=Lax`, CSRF, Origin check, logout.
- Effective-permissions inspector and grant editor with expiry.
- Destructive actions with typed confirmation for tenant and user deletion.
- Empty, loading, error, and permission-denied states.
- Accessibility basics: labels, focus order, keyboard navigation, contrast, table semantics.
- No JavaScript framework.

Exit criteria:

- Every UI action maps to an API operation.
- CSRF and session tests pass.
- An API-only build has no UI assets or routes.
- Design review against `DESIGN.md`.

## Phase 5: Release

Weeks 11-12.

Deliverables:

- Static musl binary and glibc fallback.
- OCI image, Docker-compatible, built with rootless Podman.
- Distroless static runtime, non-root, read-only rootfs, mounted data, config, and secrets.
- systemd unit and Podman Quadlet example.
- CLI subcommands: `serve`, `migrate`, `backup`, `health`, `bootstrap-token`, `doctor`.
- Packaging docs for binary, systemd, Podman, backup, restore, and upgrade.
- Reproducible benchmark against a stock SSH server and a mature open-source SFTP server.
- SQLite control-plane benchmark published.
- SBOM, signed binaries, checksums.
- Demo video and launch post.
- Design partner feedback and v0.2 scope.

Exit criteria:

- Gate 3: backup restores, migrations apply forward, and the server survives a restart with active session drain.
- `latchix health` works from inside the distroless image.
- The image runs rootless with a read-only rootfs and a mounted data volume.
- The image contains no database, host key, bootstrap token, or config.
- The systemd unit and Quadlet start the same binary with the same config.
- Performance claims include hardware, kernel, filesystem, client version, script, and raw results.

## Workstreams

| Workstream | Owner | Deliverable | Depends on |
|---|---|---|---|
| Protocol | TBD | SFTP v3, client matrix | russh spike |
| Containment | TBD | `openat2`, path parser, escape tests | Protocol |
| Store | TBD | SQLite schema, migrations, backup | Tenancy ADR |
| ACL | TBD | Rights engine, revision snapshots, effective permissions | Store |
| API | TBD | Tenant-scoped REST, OpenAPI, concurrency | Store, ACL |
| UI | TBD | Opt-in server-rendered admin | API, `DESIGN.md` |
| Security | TBD | Threat model tests, fuzz, audit triggers | All |
| Packaging | TBD | musl and glibc binaries, OCI image, systemd, Quadlet, CLI | All |
| Release | TBD | Benchmarks, SBOM, signing, checksums | Packaging |

## Deferred items and triggers

| Item | Trigger to start |
|---|---|
| Postgres | Multiple writers across hosts, hosted offering, sustained audit writes beyond the SQLite ceiling, RLS, or read replicas |
| DuckDB or DuckLake audit analytics | Audit retention or analytics demand that SQLite queries cannot serve |
| Multiple roots per user | A design partner needs more than one root per user |
| Deny rules | v0.2, after the allow-only semantics are stable |
| Share links | v0.2, as a separate credential table |
| S3 backend | v0.2, behind `VirtualFs` |
| OIDC and mTLS | v0.2 |
| HA | Only with a documented trigger and a Postgres decision |
| Tenant hierarchy | Only with a design partner |
| Per-tenant host key | A design partner requires it, which implies per-tenant listeners |

## Definition of done for v0.1

- Client matrix passes for OpenSSH, FileZilla, and rclone. OpenSSH and rclone
  are automated. FileZilla is a recorded manual check.
- Tenant isolation suite and ACL matrix pass.
- Revocation, expiry, and quota reservation tests pass.
- Backup restores into the current schema version.
- No unresolved critical or high security finding.
- The API is documented by OpenAPI and every mutation is audited.
- The UI is opt-in and passes its security review.
- musl and glibc binaries build and start on a clean host.
- The OCI image runs rootless, non-root, read-only, with mounted data and config.
- The systemd unit and Quadlet start the server.
- `latchix health`, `latchix backup`, and `latchix migrate` work.
- No secrets are baked into the image.
- The benchmark is reproducible from the published script.

## Learning checkpoints

Personal understanding is a delivery constraint, not an afterthought. Run the phase checkpoints in [LEARNING.md](LEARNING.md) before moving to the next phase. A phase is not done if the decision owner cannot explain the phase's decisions without AI.

## Delivery pressure

If the schedule slips, cut in this order:

1. UI pages beyond users, keys, grants, and audit.
2. Tenant switcher polish.
3. Container image and Quadlet packaging.
4. Benchmark breadth.

Never cut: static binary, systemd unit, tenant isolation suite, ACL matrix, backup and restore, audit triggers, `openat2` containment, or the launch client matrix.

## Change control

- A scope change requires a blueprint edit and, if it is expensive or irreversible, an ADR.
- A deferred item moves earlier only when its trigger is met or a design partner commits.
- A new database backend is not a v0.1 or v0.2 task.
- A distribution change that affects the primary artifact requires an ADR update.
