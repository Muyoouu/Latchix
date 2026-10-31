# Latchix v0.1 plan

- Status: Active
- Date: 2026-11-01
- Owner: Musa Yohanes
- Architecture: [BLUEPRINT.md](../BLUEPRINT.md)
- Decisions: [docs/decisions](decisions)

This plan turns the accepted architecture into an execution order. The blueprint is the source of truth for scope. This file is the source of truth for sequence and acceptance.

## Goal

Ship a single-binary, permissively licensed SFTP server that passes the client matrix and proves tenant isolation, ACL enforcement, and the API-first control plane on one host with SQLite metadata.

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

## Non-negotiables

- SFTP v3 works with real clients before UI polish or alternate backends.
- Every tenant-owned row, query, path, and audit event carries a tenant.
- The database ACL is the only authorization authority.
- Every filesystem operation is root-descriptor relative. No string prefix checks.
- SQLite only. No Postgres, DuckDB, or DuckLake in the metadata path.
- The UI is an opt-in feature. API-only and embedded builds do not pay for it.
- No feature ships without its test and its audit event.

## Phase 0: Decisions and scaffolding

Weeks 1-2.

Deliverables:

- Tenancy, ACL, metadata, and control-plane ADRs accepted.
- Security policy and code of conduct.
- Repository, CI, issue templates, `cargo-deny`, `cargo-audit`, pinned lockfile.
- `DESIGN.md` started before any template work.
- Default tenant and storage root conventions agreed.

Exit criteria:

- ADRs merged.
- CI builds an empty workspace and runs format, clippy, test, audit, deny.
- No open question blocks the schema.

## Phase 1: Protocol and containment

Weeks 3-6.

Deliverables:

- `russh` + `russh-sftp` server with public key authentication and a local filesystem.
- OpenSSH `sftp`, FileZilla, and rclone smoke tests pass.
- Root descriptor opened once per session.
- `openat2` containment with `RESOLVE_BENEATH`, `RESOLVE_NO_SYMLINKS`, and `RESOLVE_NO_MAGICLINKS`.
- Canonical path parser with rejection tests for `..`, absolute paths, empty segments, and trailing slash.
- Symlink denied. Hardlink denied by default, allowed only in an explicit single-tenant mode.
- Rename restricted to one root.
- Fuzz targets for the packet parser and path handling.
- Real-client integration harness.

Exit criteria:

- Gate 1: client matrix for the three launch clients passes.
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

- Docker image and systemd unit.
- Reproducible benchmark against a stock SSH server and a mature open-source SFTP server.
- SQLite control-plane benchmark published.
- SBOM, signed binaries, checksums.
- Demo video and launch post.
- Design partner feedback and v0.2 scope.

Exit criteria:

- Gate 3: backup restores, migrations apply forward, and the server survives a restart with active session drain.
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
| Release | TBD | Packaging, benchmarks, SBOM | All |

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

- Client matrix passes for OpenSSH, FileZilla, and rclone.
- Tenant isolation suite and ACL matrix pass.
- Revocation, expiry, and quota reservation tests pass.
- Backup restores into the current schema version.
- No unresolved critical or high security finding.
- The API is documented by OpenAPI and every mutation is audited.
- The UI is opt-in and passes its security review.
- The benchmark is reproducible from the published script.

## Learning checkpoints

Personal understanding is a delivery constraint, not an afterthought. Run the phase checkpoints in [LEARNING.md](LEARNING.md) before moving to the next phase. A phase is not done if the decision owner cannot explain the phase's decisions without AI.

## Delivery pressure

If the schedule slips, cut in this order:

1. UI pages beyond users, keys, grants, and audit.
2. Tenant switcher polish.
3. Docker and systemd packaging.
4. Benchmark breadth.

Never cut: tenant isolation suite, ACL matrix, backup and restore, audit triggers, `openat2` containment, or the launch client matrix.

## Change control

- A scope change requires a blueprint edit and, if it is expensive or irreversible, an ADR.
- A deferred item moves earlier only when its trigger is met or a design partner commits.
- A new database backend is not a v0.1 or v0.2 task.
