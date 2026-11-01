# Latchix Blueprint

**Status:** draft
**Owner:** Musa Yohanes
**Started:** 2026-10-31
**License:** Apache-2.0
**Repository:** `Muyoouu/Latchix`

---

## 1. Summary

Latchix is a permissively licensed, API-first SFTP server written in Rust.

It is built for platform teams that need to give customers, partners, or internal teams a managed SFTP endpoint without adopting an AGPL server, a heavyweight MFT product, or a pile of custom OpenSSH scripts.

Latchix has two modes:

1. **Standalone server.** A single binary with a REST API, a small admin UI, SQLite metadata, and a local filesystem backend.
2. **Embedded library.** The same core can be mounted inside another Rust service to expose per-tenant SFTP endpoints.

The first public release is intentionally narrow: SFTP only, local filesystem only, public key authentication only. The differentiator is not feature breadth. It is the license, the API-first control plane, the embeddable core, and the audit-first design.

Multi-tenancy is a schema decision, not a later feature. Every user, group, key, grant, session, root, and audit event is tenant-scoped from v0.1. The first release seeds a default tenant and exposes tenant CRUD through the API. Tenant self-service UI, deny rules, and alternate storage backends arrive later without a schema rewrite.

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
2. **API-first control plane.** Tenants, users, groups, grants, quotas, and audit are managed through a documented API, not a config file and a restart.
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
6. **Boring operations.** Static binary, SQLite metadata, optional OCI image, systemd unit, clear logs, Prometheus metrics.
7. **Permissive core.** Apache-2.0.
8. **Compatibility over cleverness.** Follow the SFTP v3 specification and match OpenSSH client behavior.
9. **No feature parity races.** Compete on the wedge, not on the checklist.
10. **Security claims require evidence.** Fuzzing, audits, and reproducible benchmarks before saying "production-ready."
11. **Tenant-scoped by construction.** Every row, query, path, and audit event carries a tenant. A missing tenant predicate is a bug, not a default.
12. **Schema now, surface later.** Expensive-to-retrofit schema decisions land in v0.1 even when the UI or the operation arrives later.
13. **One authority for access.** The database ACL decides. POSIX mode bits are not authorization.
14. **Keep the database off the data path.** No metadata query per SFTP packet. ACL snapshots, quota counters, and audit batching are in memory.

---

## 6. Scope

### 6.1 v0.1 MVP

The first release proves the SFTP core with real clients and ships the multi-tenant schema, ACL, and control plane that later releases extend. It targets a single host, a small team, and a staging or single-node production deployment.

**Protocol**

- SFTP v3 over SSH
- Public key authentication only
- Host key algorithms: Ed25519, RSA, ECDSA
- Modern KEX and cipher defaults
- Read, write, list, stat, mkdir, rmdir, remove, rename
- Resumable transfers if the SFTP client supports them
- File size and quota enforcement
- Path traversal protection enforced with a root dirfd and `openat2`
- Symlinks denied by default
- Hardlinks denied by default. Allowed only in an explicit single-tenant mode.
- Rename restricted to the same root

**Identity and access**

- Tenants, with one seeded default tenant
- Tenant CRUD through the API
- Per-tenant storage root with an opaque storage key
- Users, groups, and group membership
- Per-user home root
- Grants scoped to a tenant, root, and path anchor
- Grant scope: subtree or exact
- Grant effect: allow in v0.1. Deny is reserved in the schema and evaluator, rejected by the API
- Grant lifecycle: `not_before`, `expires_at`, `revoked_at`
- Rights bitmask: `TRAVERSE`, `LIST`, `READ`, `CREATE`, `WRITE`, `OVERWRITE`, `DELETE`, `RENAME`
- Per-tenant and per-user quotas: bytes, file count, concurrent sessions
- Enable and disable accounts
- Soft delete for tenants, users, groups, and roots. Names are unique only among live rows.
- SSH public key management with expiry
- API keys with roles and tenant scope
- Optional read-only users

**Storage**

- Local filesystem backend only
- Root directory per tenant, home root per user
- Database ACL is the only authorization authority
- POSIX mode bits are defense in depth, never authorization
- Quota reservation at open, reconciliation on close or disconnect
- Partial-file policy: keep, delete, or quarantine
- Atomic rename where the filesystem supports it

**Control plane**

- REST API with OpenAPI
- Tenant-scoped credentials. Tenant selection comes from the token, never from an unauthenticated header
- Unix socket and optional TCP listener
- Single-use bootstrap admin token
- CRUD for tenants, users, groups, members, keys, grants, quotas, roots, and API keys
- Effective-permissions endpoint with `explain`
- Session list and revoke
- Audit query with filters and cursor pagination
- Health and readiness endpoints
- Prometheus metrics without unbounded tenant labels
- `problem+json` errors with stable codes
- ETag and `If-Match` on mutations
- Idempotency keys scoped to a tenant

**Admin UI**

- Opt-in Cargo feature. Off for core and embedded builds, on for the standalone binary
- Minimal server-rendered UI
- Cookie session with CSRF protection
- Login exchanges an API key for a session. No separate admin password in v0.1.
- Tenant switcher for server admins. Tenant admins see only their tenant
- Users, groups, keys, grants, quota usage, and audit
- No JavaScript framework in v0.1
- A `DESIGN.md` direction is required before template work

**Operations**

- Static musl binary, primary. glibc binary as fallback.
- Optional OCI image, Docker-compatible, built with rootless Podman.
- TOML config, environment variables, CLI flags
- SQLite metadata only
- WAL mode, one writer, read pool
- Foreign keys enabled on every connection
- Embedded forward-only migrations
- Backup through the SQLite backup API or `VACUUM INTO`
- systemd unit and Podman Quadlet example
- CLI subcommands: `serve`, `migrate`, `backup`, `health`, `bootstrap-token`, `doctor`.
- Structured logs with `tracing`
- Graceful shutdown
- Single-process file lock on the database

**Quality**

- Unit tests
- Integration tests with OpenSSH `sftp`, FileZilla, and rclone
- Fuzz targets for the SFTP packet parser, path handling, and config parser
- Tenant isolation test suite, including cross-tenant references and traversal
- ACL matrix tests for every right and operation
- Revocation and expiry tests during an active session
- Quota reservation and concurrency tests
- Backup, restore, and migration tests
- `cargo-audit`, `cargo-deny`, pinned lockfile
- Reproducible benchmark harness

### 6.2 v0.2

- Multiple virtual roots per user
- S3-compatible storage backend behind the `VirtualFs` trait
- Deny rules enabled at the API
- Read-only share links as a separate credential
- Per-user bandwidth limits
- Audit export to JSONL and webhook
- OpenTelemetry traces
- OIDC for the admin UI
- mTLS for the API
- Bulk operations with per-item results
- Richer admin UI, including bulk grant editing and saved audit views
- Terraform provider
- Embedded library API made public and stable

### 6.3 v0.3

- Tenant self-service UI for tenant admins
- Event hooks on upload, download, delete, and auth
- File lifecycle rules: archive, move, delete after N days
- Cross-tenant reporting for server admins
- Metrics dashboard
- SCIM provisioning

### 6.4 v1.0

- Security audit completed
- Stable API and config format
- Upgrade and migration guarantees
- Packaging for major Linux distributions
- Documented compatibility matrix
- Published performance benchmarks
- HA and Postgres only if a documented trigger is met: multiple writers across hosts, a hosted offering, sustained durable audit writes beyond the SQLite ceiling, row-level security as a second isolation layer, or read replicas

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
- Postgres in v0.1 or v0.2
- DuckDB or DuckLake as the control-plane metadata store
- Per-tenant SQLite database files
- Deny rules at the API in v0.1
- Multiple roots per user in v0.1

---

## 7. Architecture

### 7.1 Workspace layout

```text
Latchix/
├── Cargo.toml                 # workspace
├── crates/
│   ├── latchix-core/          # domain, tenant context, ACL engine, ports
│   ├── latchix-sftp/          # SSH and SFTP adapters
│   ├── latchix-api/           # REST API and OpenAPI
│   ├── latchix-store/         # SQLite metadata and migrations
│   ├── latchix-audit/         # audit events and exporters
│   ├── latchix-cli/           # standalone binary
│   └── latchix-ui/            # server-rendered admin UI, opt-in feature
├── docs/
│   └── decisions/             # architecture decision records
├── migrations/                # SQL migrations
├── tests/                     # integration tests and client matrix
├── benches/                   # Criterion benchmarks
├── fuzz/                      # cargo-fuzz targets
└── xtask/                     # development tasks
```

Dependency rule: `latchix-core` depends on neither `axum`, SQLite, nor `russh`. Adapters depend inward. The core defines ports as traits: `MetadataStore`, `VirtualFs`, `Clock`, `IdGen`, `Authenticator`, `Authorizer`, and `AuditSink`.

### 7.2 Runtime components

**SSH listener**

- `russh` handles the SSH transport, host keys, session channels, and authentication.
- One listener per configured bind address.
- Graceful shutdown drains active sessions.
- Per-tenant, per-IP, and per-user connection limits.
- One process-wide host key set in v0.1. SSH has no tenant hint before key exchange, so a per-tenant host key requires a per-tenant listener.

**SFTP subsystem**

- `russh-sftp` handles the SFTP v3 packet layer.
- The subsystem adapter maps SFTP operations to `VirtualFs`.
- Unknown or unsupported extensions return a clear error, not a panic.
- Every file handle is scoped to a session, tenant, root, and grant revision.
- `hardlink@openssh.com` is denied by default. Allowed only in an explicit single-tenant mode.

**Authentication**

- Public key authentication.
- Keys are stored in the metadata database and parsed with `ssh-key`.
- A public key fingerprint is globally unique and resolves to exactly one tenant and user.
- Group-bound keys are not supported. Group access comes from membership and grants.
- Disabled users, disabled tenants, and expired keys fail closed.
- The authenticated principal carries `tenant_id`, `user_id`, `home_root_id`, and the current ACL revision.
- Authentication failures return uniform errors and timing that does not reveal tenant or user existence.

**Tenancy**

- A tenant owns one storage root, one key namespace, one group namespace, and one limit set.
- Tenant IDs are never reused. Tenants are soft-deleted with a tombstone.
- The v0.1 database seeds a `default` tenant. Standalone installs may keep it invisible in the UI.
- Tenant roots use an opaque storage key, never the tenant slug or a username.
- Root creation canonicalizes the path and rejects a root nested inside another tenant root.

**Virtual filesystem**

- `VirtualFs` is a trait with operations for open, read, write, stat, list, mkdir, remove, rename, and metadata.
- `LocalFs` is the first implementation.
- `S3Fs` comes later.
- A session opens its root once with `O_PATH | O_DIRECTORY` and performs every operation relative to that dirfd.
- Path resolution uses `openat2` with `RESOLVE_BENEATH`, `RESOLVE_NO_SYMLINKS`, and `RESOLVE_NO_MAGICLINKS`.
- String prefix checks are never the isolation boundary.
- Symlinks are denied. Hardlinks are denied by default and allowed only in an explicit single-tenant mode.
- Rename requires source and destination in the same root.
- No path escapes through `..`, absolute paths, encoded separators, symlinks, or hardlinks.

**ACL engine**

- Rights are a bitmask: `TRAVERSE`, `LIST`, `READ`, `CREATE`, `WRITE`, `OVERWRITE`, `DELETE`, `RENAME`.
- Evaluation is default deny. Subjects are the user plus group memberships.
- A grant matches a path on segment boundaries, never raw `LIKE` matching.
- Every ancestor requires `TRAVERSE`.
- v0.1 is allow-only. Deny rules are reserved in the schema and evaluator, rejected by the API.
- `users.read_only` and read-only group membership mask the effective rights.
- Each session holds an `ArcSwap` snapshot keyed by `acl_revision`.
- The revision is bumped in the same transaction as any grant, membership, user, key, or root change.
- Revocation is visible on the next operation. Optional session kill closes active channels.

**Metadata store**

- `latchix-store` owns tenants, roots, users, groups, keys, grants, quotas, API keys, sessions, and audit events.
- SQLite is the only backend in v0.1 and v0.2.
- WAL mode, one writer connection or writer actor, and a read pool.
- `busy_timeout` is set and write transactions are short and retried on `SQLITE_BUSY`.
- Foreign keys are enabled on every pooled connection.
- Migrations are embedded, forward-only, versioned, and tested.
- Backup uses the SQLite backup API or `VACUUM INTO` inside a read transaction.
- The database file is local disk only. Network filesystems are rejected.
- A process lock prevents two Latchix processes from writing the same database.
- The store is the source of truth. No generated config files.

**REST API**

- `axum` with `utoipa` for OpenAPI.
- `problem+json` errors with stable error codes and `request_id`.
- API key authentication.
- Tenant identity comes from the credential. A server-admin key may target another tenant through an explicit tenant path.
- Optional Unix socket listener.
- Optional TCP listener bound to localhost or a private interface.
- Rate limiting per IP, per API key, and per tenant.
- ETag and `If-Match` on mutable resources.
- `Idempotency-Key` on creates and replacements, scoped to a tenant.
- Cursor pagination on a stable `(created_at, id)` order.

**Admin UI**

- Opt-in Cargo feature. The standalone binary enables it, core and embedded builds do not.
- Server-rendered in v0.1 to keep the binary simple.
- Cookie session with `HttpOnly`, `Secure`, `SameSite=Lax`, CSRF tokens, and an Origin check.
- Login exchanges an API key for a session. There is no separate admin password or password table in v0.1.
- Uses the same API internally or calls the service layer directly. It never bypasses authorization.
- Tenant admins see only their tenant. Server admins get a tenant switcher.
- No JavaScript framework in v0.1.
- A richer SPA can come later if needed.

**Audit**

- Append-only event table.
- Append-only is enforced with `BEFORE UPDATE` and `BEFORE DELETE` triggers that raise and abort.
- Event types: auth success, auth failure, upload, download, delete, rename, tenant created, tenant updated, user created, user updated, user disabled, key added, key removed, grant created, grant updated, grant revoked, quota changed, config changed.
- Each event includes tenant, actor, target, timestamp, request ID, source IP, result, and, where relevant, `acl_revision`.
- Denied operations are audited with rate-limited suppression per session and path.
- Export to JSONL and webhook in v0.2.

**Observability**

- `tracing` for structured logs.
- Every span carries `tenant_id`, `request_id`, and `session_id`.
- Paths are logged relative to the root.
- Prometheus metrics at `/metrics`.
- No unbounded tenant label. Tenant labels are opt-in and bounded.
- `/healthz` and `/readyz`.
- Optional OpenTelemetry in v0.2.

### 7.3 Request flow

**SFTP login**

1. Client connects to the SSH listener.
2. `russh` presents the host key.
3. Client authenticates with a public key.
4. The authenticator looks up the key fingerprint and resolves exactly one tenant and user.
5. Disabled tenants, disabled users, expired keys, and over-quota users are rejected.
6. The session opens the tenant root by descriptor and binds `tenant_id`, `user_id`, `root_id`, and `acl_revision`.
7. The ACL snapshot loads and the SFTP subsystem starts.

**File upload**

1. Client opens a file for write.
2. The ACL engine checks the operation against the session snapshot, including `TRAVERSE` on every ancestor.
3. Quota bytes are reserved atomically for the tenant and user.
4. `VirtualFs` resolves the path relative to the root dirfd with `openat2` and no symlinks.
5. Data is streamed to the backend.
6. On close, quota usage is reconciled and the audit event is recorded.
7. If the client disconnects, the reservation is released and partial files follow the configured policy: keep, delete, or quarantine.

**Admin change**

1. Admin calls the REST API with an API key.
2. The API resolves the tenant from the credential and the role from the key or session.
3. The request is validated, including canonical paths and cross-tenant references.
4. The store applies the change in one transaction and bumps `acl_revision` when access changes.
5. The audit event is written in the same transaction.
6. The change is visible on the next SFTP operation. No reload and no restart.

**Grant revocation**

1. Admin revokes a grant or disables a user.
2. The store deletes or tombstones the grant and bumps `acl_revision` in one transaction.
3. The audit event records the actor, target, and resulting revision.
4. Sessions compare their snapshot revision on the next operation and reload.
5. With `revoke_sessions=true`, the session registry closes matching channels and releases quota reservations.

### 7.4 Data model

```text
tenants
  id, slug, name, status, storage_key, max_bytes, max_files,
  max_sessions, acl_revision, created_at, updated_at, deleted_at
  unique(slug), unique(storage_key)

roots
  id, tenant_id, name, storage_key, enabled, created_at, deleted_at
  partial unique(tenant_id, name) where deleted_at is null
  unique(storage_key)

users
  id, tenant_id, username, home_root_id, enabled, read_only,
  quota_bytes, quota_files, max_sessions, version,
  created_at, updated_at, deleted_at
  partial unique(tenant_id, username COLLATE NOCASE) where deleted_at is null

groups
  id, tenant_id, name, enabled, created_at, updated_at, deleted_at
  partial unique(tenant_id, name COLLATE NOCASE) where deleted_at is null

group_members
  tenant_id, group_id, user_id, access_level
  primary key(group_id, user_id)
  composite foreign keys to groups and users on (tenant_id, id)

ssh_keys
  id, tenant_id, user_id, name, public_key, fingerprint,
  created_at, last_used_at, expires_at, revoked_at
  partial unique(fingerprint) where revoked_at is null

grants
  id, tenant_id, root_id, user_id, group_id, anchor, scope,
  effect, rights, not_before, expires_at, revoked_at,
  created_by, created_at, updated_at, version
  exactly one of user_id or group_id
  composite foreign keys to roots, users, and groups on (tenant_id, id)
  partial unique indexes per subject kind

api_keys
  id, tenant_id, name, key_prefix, key_hash, role, scopes,
  created_at, last_used_at, expires_at, revoked_at
  tenant_id is null only for server admins
  unique(key_prefix)

sessions
  id, tenant_id, user_id, root_id, grant_revision,
  peer_ip, started_at, last_seen_at, ended_at

usage
  tenant_id, root_id, user_id, bytes_used, files_used,
  reserved_bytes, sessions_active, updated_at
  primary key(tenant_id, root_id, user_id)

idempotency_keys
  tenant_id, key, route, request_hash, status, response_json, expires_at
  primary key(tenant_id, key)

audit_events
  id, tenant_id, actor_type, actor_id, action, target_type,
  target_id, source_ip, request_id, result, details_json,
  acl_revision, created_at

share_links
  id, tenant_id, root_id, anchor, token_hash, rights,
  created_by, created_at, expires_at, max_downloads,
  download_count, revoked_at
  unique(token_hash)
```

Rules:

- `tenant_id` is the first column of every composite index.
- Tenant IDs are never reused. Soft delete tenants and never hard-delete audit history.
- Tenant and audit IDs use `AUTOINCREMENT` or an application UUID. A deleted row ID must never be reused.
- `tenants.status` is `active`, `disabled`, or `deleted`.
- `users.home_root_id` is required in v0.1. A user without a live root cannot authenticate.
- `DELETE` on a user, group, or root is a soft delete. Unique names apply only to live rows, so a name can be reused after deletion.
- Deleting a user disables it, revokes keys, memberships, grants, and sessions, then tombstones the row.
- Deleting a group revokes memberships and grants, then tombstones the row.
- Deleting a root requires no live users and is an audited lifecycle operation.
- A revoked SSH key no longer reserves its fingerprint. Re-adding the key is a new audited row or clears `revoked_at`.
- `users.home_root_id`, `sessions.root_id`, and `grants.root_id` reference `roots(tenant_id, id)`.
- `grants.user_id` and `grants.group_id` reference `users(tenant_id, id)` and `groups(tenant_id, id)`.
- `sessions.user_id` references `users(tenant_id, id)`.
- `usage` rows are per root and aggregate to user and tenant quotas.
- `audit_events.tenant_id` intentionally has no cascade delete. Tenants are soft-deleted so audit history survives.
- `share_links` is reserved for v0.2 and unused in v0.1.
- Path anchors are canonical, root-relative, UTF-8 NFC, and matched on segment boundaries.
- Timestamps that drive expiry and ordering are integer epoch seconds, or an enforced UTC format. Not free-form ISO strings with offsets.
- Foreign keys are enabled on every connection.
- Unique indexes that include a nullable subject column use partial indexes. SQLite treats `NULL` values as distinct in a plain `UNIQUE`.
- `grants.effect = 'deny'` is reserved for v0.2 and rejected by the v0.1 API.
- `api_keys.tenant_id` may be null only when `role = 'server_admin'`. A CHECK enforces it.
- `share_links.rights` is restricted to `TRAVERSE | LIST | READ`. Write, overwrite, delete, rename, and create are rejected by a CHECK.
- A grant carries at least one right. When both `not_before` and `expires_at` are set, `expires_at` is later.
- A soft-deleted user, group, or root is disabled. `deleted_at` implies `enabled = 0`.
- `tenants.status = 'deleted'` exactly when `deleted_at` is set.
- SSH fingerprints use the OpenSSH SHA256 base64 form and are normalized on write.
- `audit_events.details_json` carries a schema version so consumers can parse it stably.
- Expired idempotency keys and revoked grants are removed by a maintenance task, and the cleanup is audited.
- Indexes: `grants(tenant_id, root_id, user_id)`, `grants(tenant_id, root_id, group_id)`, `sessions(tenant_id, user_id)`, `audit_events(tenant_id, created_at, id)`, `idempotency_keys(expires_at)`.

### 7.5 API surface

```text
GET    /api/v1/whoami

GET    /api/v1/tenants
POST   /api/v1/tenants
GET    /api/v1/tenants/{id}
PATCH  /api/v1/tenants/{id}
DELETE /api/v1/tenants/{id}
GET    /api/v1/tenants/{id}/usage

GET    /api/v1/roots
POST   /api/v1/roots
GET    /api/v1/roots/{id}
PATCH  /api/v1/roots/{id}
DELETE /api/v1/roots/{id}

GET    /api/v1/users
POST   /api/v1/users
GET    /api/v1/users/{id}
PATCH  /api/v1/users/{id}
DELETE /api/v1/users/{id}
POST   /api/v1/users/{id}/enable
POST   /api/v1/users/{id}/disable
GET    /api/v1/users/{id}/usage

GET    /api/v1/groups
POST   /api/v1/groups
GET    /api/v1/groups/{id}
PATCH  /api/v1/groups/{id}
DELETE /api/v1/groups/{id}
GET    /api/v1/groups/{id}/members
PUT    /api/v1/groups/{id}/members/{user_id}
DELETE /api/v1/groups/{id}/members/{user_id}

GET    /api/v1/users/{id}/keys
POST   /api/v1/users/{id}/keys
GET    /api/v1/users/{id}/keys/{key_id}
DELETE /api/v1/users/{id}/keys/{key_id}
POST   /api/v1/users/{id}/keys/{key_id}/rotate

GET    /api/v1/grants
POST   /api/v1/grants
GET    /api/v1/grants/{id}
PATCH  /api/v1/grants/{id}
DELETE /api/v1/grants/{id}
GET    /api/v1/users/{id}/effective-permissions

GET    /api/v1/sessions
DELETE /api/v1/sessions/{id}
GET    /api/v1/users/{id}/sessions
DELETE /api/v1/users/{id}/sessions

GET    /api/v1/api-keys
POST   /api/v1/api-keys
GET    /api/v1/api-keys/{id}
DELETE /api/v1/api-keys/{id}
POST   /api/v1/api-keys/{id}/rotate

GET    /api/v1/audit
GET    /api/v1/audit/{id}

POST   /api/v1/auth/session
DELETE /api/v1/auth/session

GET    /api/v1/openapi.json
GET    /api/v1/openapi.yaml

GET    /healthz
GET    /readyz
GET    /metrics
```

Design rules:

- Tenant identity comes from the credential, never from an unauthenticated header.
- Server-admin keys use `/api/v1/tenants/{tenant_id}/...` for cross-tenant management. Tenant-scoped keys use the flat routes.
- Errors use `application/problem+json` with `type`, `title`, `status`, `detail`, `code`, `request_id`, and field errors.
- Cursor pagination uses an opaque `(created_at, id)` cursor. Limit is 1 to 200, default 50.
- `ETag` and `If-Match` are required on `PATCH` and `DELETE`. Missing preconditions return 428. Mismatch returns 412.
- `Idempotency-Key` is required on create and replace. The key is scoped to `(tenant_id, key)`, has a 24 hour TTL, and is bound to the request hash.
- A reused idempotency key with a different route or request hash returns 409 `idempotency_mismatch`.
- Quotas are fields on tenant and user resources. There is no separate quota collection.
- No secrets in responses. API key material is returned exactly once at creation.
- API keys are stored as a prefix index plus SHA-256 or HMAC-SHA-256, compared in constant time.
- Rate limits apply per IP, per API key, and per tenant.
- Every mutation writes an audit event in the same transaction.
- Cross-tenant reference checks are enforced by composite foreign keys and by the API.

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
busy_timeout_ms = 5000
synchronous = "full"
max_read_connections = 8

[tenancy]
default_slug = "default"

[acl]
snapshot_refresh_seconds = 5

[audit]
jsonl_path = "/var/log/latchix/audit.jsonl"

[limits]
max_sessions_per_user = 10
max_connections_per_ip = 50

[ui]
enabled = true
```

Notes:

- `ui.enabled` only has effect when the binary is compiled with the `ui` feature.
- `database.synchronous = "full"` is the default for durable audit commits. `"normal"` is allowed only when the operator accepts losing the last commits on power loss.
- `acl.snapshot_refresh_seconds` is a safety net for out-of-band database edits. Normal invalidation uses `acl_revision`.

### 7.7 Deployment

- Static Linux binary. `x86_64-unknown-linux-musl` is primary, `x86_64-unknown-linux-gnu` is fallback.
- Optional OCI image, Docker-compatible, built with rootless Podman.
- Image runtime is `distroless/static-debian12:nonroot`, non-root, with a read-only root filesystem.
- Data, config, host keys, and bootstrap token are mounted. They are never baked into the image.
- The image has no shell. `latchix health`, `latchix backup`, and `latchix migrate` handle operations.
- systemd unit with `ProtectSystem`, `PrivateTmp`, and a dedicated user.
- Podman Quadlet example for rootless container installs.
- The SFTP listener runs on a configurable port.
- The API should bind to a Unix socket or a private interface, not the public internet.
- SQLite is the only metadata backend. The database file and storage root live on local disk, never on NFS or SMB.
- One Latchix process owns a database file. A process lock fails startup if a second process tries to write it.
- Backups use the SQLite backup API or `VACUUM INTO` inside a read transaction, plus a consistent snapshot of the storage root. Copying a live database file is not a backup.
- Restore is tested with the same migration version. Migrations are forward-only.

---

## 8. Security

### 8.1 Threat model

Assume:

- The SFTP client is untrusted.
- The network is hostile.
- The API may be exposed to internal users with stolen tokens.
- The filesystem may contain symlinks and hardlinks.
- The host may have other tenants or services.
- A tenant, or a tenant admin, may be hostile toward another tenant.
- The database file is readable by the host operator. Host compromise is out of scope.

Protect against:

- Path traversal and chroot escape
- Symlink and hardlink escapes
- Authentication bypass
- Key confusion or key substitution
- Cross-tenant references, queries, and audit reads
- Predictable tenant storage paths
- ACL path matching that crosses segment boundaries
- Stale authorization after a grant change or revocation
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
- `tenant_id` is a required argument in the store and `VirtualFs` APIs. Unscoped queries are rejected in review and by a SQL lint.
- Composite foreign keys on `(tenant_id, id)` prevent cross-tenant references. Foreign keys are enabled on every connection.
- Per-tenant root and per-session root descriptor enforced in `VirtualFs`, not only in the SSH layer.
- Path resolution uses `openat2` with `RESOLVE_BENEATH`, `RESOLVE_NO_SYMLINKS`, and `RESOLVE_NO_MAGICLINKS`.
- Paths are canonical before matching. No `..`, no empty segments, no trailing slash except `/`.
- ACL path matching uses byte-exact segment comparison. Bare `LIKE` is forbidden because it is case-insensitive and treats `_` and `%` as wildcards.
- Symlinks are denied. Hardlinks are denied by default and allowed only in an explicit single-tenant mode. Rename must stay inside one root.
- The database ACL is the only authorization authority. POSIX mode bits are never read for access decisions.
- `chmod` and `chown` over SFTP are denied in v0.1.
- Quotas are reserved atomically at open, reconciled on close, and released on disconnect. `statvfs` is virtualized to the tenant quota.
- Connection and session limits apply per tenant, per IP, and per user.
- API keys are stored as a prefix index plus SHA-256 or HMAC-SHA-256. Argon2 is not used for high-entropy bearer tokens.
- Rate limits apply per IP, per API key, and per tenant, including pre-authentication traffic.
- Audit is append-only, enforced by triggers. Every auth and mutation event is recorded with `tenant_id`.
- Authorization is re-evaluated per operation against `acl_revision`. Revocation does not wait for the next login.
- No secrets in logs.
- Structured errors that do not leak paths, tenant existence, or internal state.
- Metrics do not carry an unbounded tenant label.
- Expiry and ordering columns use epoch seconds or an enforced UTC format.
- Database and storage files are `0600` and `0700`, owned by the service user.

### 8.3 Release security

- `cargo-audit` and `cargo-deny` in CI.
- `cargo-fuzz` targets for the packet parser, path handling, and config parser.
- Integration tests with real clients.
- Tenant isolation suite in CI: cross-tenant references, cross-tenant queries, cross-tenant audit reads, and traversal attempts.
- Filesystem escape tests for `..`, absolute paths, symlinks, and hardlinks against a real root descriptor.
- Property tests for ACL path matching and effective-rights resolution.
- Backup and restore drills with the current schema version.
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

Launch gate for v0.1:

- OpenSSH `sftp`
- FileZilla
- rclone

Full matrix before v1.0:

- WinSCP
- Cyberduck
- Paramiko
- `ssh2` for Node.js
- Termius
- Transmit

### 9.2 Protocol coverage

- SFTP v3
- `limits@openssh.com`
- `fsync@openssh.com`
- `hardlink@openssh.com` denied by default, allowed only in an explicit single-tenant mode.
- `statvfs@openssh.com`
- `expand-path@openssh.com`

### 9.3 Test strategy

- Unit tests for path normalization, ACL evaluation, quotas, and key parsing.
- Integration tests that start a real server and connect with real clients.
- Property tests for path handling, ACL segment matching, and effective-rights resolution.
- Fuzz tests for packet parsing and config parsing.
- Concurrency tests for uploads, renames, and deletes.
- Tenant isolation tests for every table with a `tenant_id`, including cross-tenant foreign keys and audit reads.
- Revocation and expiry tests against an active session.
- Quota reservation tests for concurrent opens, disconnects, and over-cap writes.
- Backup, restore, and forward-only migration tests.
- Failure tests for disk full, permission denied, and backend timeouts.
- Benchmark tests in CI, but not as a merge gate until stable.

### 9.4 CI

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo audit`
- `cargo deny check`
- `cargo doc`
- Container image build with Podman
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
- Contributions use DCO, not a CLA.

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
- Package names: `latchix`, `latchix-core`, `latchix-sftp`, `latchix-api`, `latchix-store`, `latchix-audit`, `latchix-ui`, `latchix-cli`.

---

## 13. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Feature creep | Delays v0.1 indefinitely | Enforce non-goals. SFTP only. |
| Security incident | Destroys trust | Threat model, fuzzing, audit, disclosure policy |
| Tenant isolation bug | Cross-tenant data exposure | Composite foreign keys, descriptor-based containment, isolation suite in CI |
| SQLite write ceiling | Stalled control plane under audit load | One writer, short transactions, batch audit, JSONL fallback, measured trigger for Postgres |
| Single-process constraint | No HA or multi-writer | Documented limit. Postgres only when a documented trigger is met |
| Database choice churn | Rewrites and lost time | `MetadataStore` trait, SQLite only, no speculative backend |
| ACL semantics drift | Silent privilege expansion | ADR, property tests, effective-permissions inspector |
| UI scope | Delays v0.1 | Opt-in feature flag, `DESIGN.md` first, no JavaScript framework |
| `russh-sftp` gaps | Blocks protocol work | Contribute upstream, keep a fallback plan, test early |
| Weak adoption | No users, no feedback | Build in public, design partners, benchmarks, launch posts |
| Incumbent comparison | Unfavorable feature checklist | Compete on license, API, embeddability, not breadth |
| Support burden | Burnout | Clear support policy, community forum, maintainer boundaries |
| License confusion | Adoption friction | Apache-2.0, clear FAQ, no AGPL dependencies |
| Trademark conflict | Legal risk | Name search, domain registration, trademark policy |
| Performance claims | Credibility loss | Reproducible benchmarks, honest results |
| Windows support | Scope explosion | Linux first, Windows later |

---

## 14. Open questions

### Resolved

1. Password authentication in v0.1: no.
2. Windows in v0.1: no. Linux first.
3. S3 in v0.1: no. Local filesystem first.
4. Admin UI in v0.1: yes, minimal server-rendered, opt-in Cargo feature.
5. API compatibility with existing servers: no. Clean tenant-scoped API.
6. Embedded library: design for it in v0.1, expose publicly in v0.2.
7. Metadata backend: SQLite only. Postgres is not scheduled.
8. Contributions before v0.2: docs, tests, and good first issues only.
9. DuckDB and DuckLake: audit and usage analytics only, never the control-plane store.
10. Postgres trigger: multiple writers across hosts, a hosted offering, sustained durable audit writes beyond the SQLite ceiling, row-level security, or read replicas.

### Open

1. Should the project join a foundation? Not before v1.0.
2. Host keys: shared process-wide key in v0.1. Per-tenant host key only if a design partner requires a per-tenant listener.
3. Deny rules: reserved for v0.2. Confirm the API and UI shape before enabling.
4. Admin UI direction: a `DESIGN.md` is required before template work.
5. HA threshold: define the exact write rate, host count, and deployment model that triggers Postgres.
6. Audit retention defaults and per-tenant export boundaries.
7. Tenant hierarchy or nesting: not planned. Revisit only with a design partner.
8. SCIM priority relative to OIDC.

---

## 15. First 90 days

### Weeks 1-2

- 10 interviews with platform and infrastructure teams
- Register `latchix.com`
- Publish the blueprint
- Set up the repository, CI, and issue templates
- Write the security policy and code of conduct
- Write the tenancy, ACL, and metadata ADRs
- Start `DESIGN.md` before any UI work

### Weeks 3-6

- Spike: `russh` + `russh-sftp` server with public key auth and local filesystem
- Pass OpenSSH `sftp`, FileZilla, and rclone smoke tests
- Implement the root descriptor and `openat2` containment
- Add traversal, symlink, and hardlink escape tests
- Add fuzz targets for path handling

### Weeks 7-10

- SQLite schema with tenants, roots, users, groups, keys, grants, sessions, and usage
- Migrations, foreign keys, writer actor, read pool, backup path
- ACL engine with the rights bitmask, `TRAVERSE`, expiry, revocation, and revision snapshots
- Tenant-scoped REST API with OpenAPI, problem+json, ETag, and idempotency
- Audit log with append-only triggers
- Quota reservation and reconciliation
- Containerfile and systemd unit scaffolding

### Weeks 11-12

- Minimal opt-in admin UI for users, keys, grants, and audit
- Tenant isolation suite and ACL matrix green
- SQLite control-plane benchmark published
- Static musl and glibc binaries built
- OCI image, systemd unit, and Podman Quadlet built
- Reproducible benchmark against a stock SSH server and a mature open-source SFTP server
- Demo video
- Launch post on Hacker News, r/rust, r/selfhosted, r/sysadmin
- Collect feedback and decide v0.2 scope

### Gate

1. If the spike cannot pass the client matrix, stop and reassess. The protocol layer is the risk.
2. Do not build the UI or the full API before the SFTP core works with real clients.
3. Do not ship v0.1 until the tenant isolation suite and ACL matrix pass, and a backup restores into the current schema version.

---

## 16. Decision log

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
| 2026-11-01 | Tenancy in v0.1 schema | Tenant scoping is expensive to retrofit. Seed one default tenant and expose tenant CRUD early |
| 2026-11-01 | Root is a boundary, not a grant | Explicit audited grants make access auditable and avoid an implicit full-access root |
| 2026-11-01 | Rights bitmask with `TRAVERSE` and `OVERWRITE` | SFTP needs ancestor traversal and overwrite is distinct from write |
| 2026-11-01 | Allow-only ACL in v0.1 | Deny rules are reserved in the schema and evaluator, enabled later |
| 2026-11-01 | Per-operation authorization with `acl_revision` | Login-time checks cannot revoke access or expire grants |
| 2026-11-01 | Descriptor-based filesystem containment | String prefix checks are not isolation. Use a root dirfd and `openat2` |
| 2026-11-01 | Hardlinks denied by default | A hardlink can escape the root even with `openat2` |
| 2026-11-01 | SQLite-only metadata, no scheduled Postgres | Control-plane write volume fits SQLite. Postgres only on a documented trigger |
| 2026-11-01 | DuckDB and DuckLake are analytics only | Columnar lakehouse engines are wrong for row-level control-plane metadata |
| 2026-11-01 | API keys hashed with SHA-256, not Argon2 | Bearer keys are high entropy and Argon2 per request is a CPU DoS vector |
| 2026-11-01 | Admin UI is an opt-in Cargo feature | Keeps core and embedded builds small and API-only |
| 2026-11-01 | Shared host key in v0.1 | SSH has no tenant hint before key exchange. Per-tenant host keys need per-tenant listeners |
| 2026-11-01 | Binary primary, OCI image secondary, library internal | Keeps the single-binary promise while supporting container users. Library waits for a stable embedded API |
