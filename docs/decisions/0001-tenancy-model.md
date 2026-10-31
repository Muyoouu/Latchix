# ADR 0001: Tenancy model

- Status: Accepted
- Date: 2026-11-01
- Owners: Musa Yohanes
- Supersedes: none

## Context

The primary user is a platform team that gives customers or partners a managed SFTP endpoint. The blueprint previously deferred multi-tenant organizations to v0.3 while stabilizing the embeddable API in v0.2. A schema without a tenant concept cannot represent per-tenant isolation, and retrofitting it after clients exist is a breaking migration.

A tenant shares one process, one SQLite database, and one storage filesystem with other tenants. Isolation must survive hostile paths, hostile tenant admins, and application bugs.

## Decision

1. Add `tenants` and `roots` tables. Every tenant-owned entity carries a non-null `tenant_id`.
2. Seed one `default` tenant in v0.1. Tenant CRUD is available through the API in v0.1. Tenant self-service UI waits for v0.3.
3. A tenant owns one storage root with an opaque `storage_key`. Never derive the on-disk path from the tenant slug or a username.
4. `users.home_root_id` replaces `users.root_path` and is required in v0.1. Absolute host paths are never accepted from an API caller.
5. Tenant identity comes from the credential. An SSH key fingerprint resolves to exactly one tenant and user. An API key carries a tenant. Only server-admin keys may target another tenant, through an explicit tenant path.
6. Enforce tenant references with composite foreign keys on `(tenant_id, id)`. Enable foreign keys on every SQLite connection.
7. Tenant namespaces are per tenant: partial unique indexes on `(tenant_id, username)` and `(tenant_id, group_name)` where `deleted_at` is null, both case-folded.
8. Tenant IDs are never reused. Tenants are soft-deleted with a tombstone. Audit history is never hard-deleted with a tenant.
9. User, group, and root deletion is also a soft delete. Unique names are partial indexes over live rows, so a name can be reused after deletion.
10. Deleting a user disables it, revokes keys, memberships, grants, and sessions, then tombstones the row. Deleting a group revokes memberships and grants.
11. One process-wide host key in v0.1. SSH has no tenant hint before key exchange, so per-tenant host keys require per-tenant listeners.

## Consequences

- Every store query requires a `TenantId`. An unscoped query is a defect.
- The schema, migrations, and test fixtures are larger in v0.1.
- Tenant deletion is a lifecycle operation: drain sessions, revoke keys, tombstone, schedule storage garbage collection.
- Cross-tenant reads are restricted to a server-admin principal.
- The embedded core must accept an explicit tenant context and must not read tenant identity from process globals.

## Alternatives considered

- Single-tenant v0.1, tenancy in v0.3. Rejected. It freezes a single-tenant public API and schema, then breaks both.
- One SQLite database per tenant. Rejected for v0.1. It gives parallel writers but multiplies migrations, backups, and cross-tenant administration.
- Tenant as a database schema or file prefix. Rejected. It still requires a tenant registry and does not provide row-level enforcement.
- Rely on application filters alone. Rejected. Composite foreign keys are a cheap second line of defense.

## Evidence

- Single-column foreign keys allowed a tenant 1 grant to reference a tenant 2 user, group, or root, and a tenant 1 user to reference a tenant 2 root.
- Composite foreign keys on `(tenant_id, id)` blocked all four cases.
- `unique(tenant_id, username)` allowed the same username in two tenants and blocked a duplicate inside one tenant.
- `COLLATE NOCASE` blocked case-variant duplicates.
- `INTEGER PRIMARY KEY` reused a deleted tenant ID, which would misattribute retained audit rows.
- `openat2(RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS)` blocked `..`, absolute paths, and symlink escapes. A hardlink still escaped, so hardlinks must be denied separately.

## Open items

- Exact per-tenant host key design if a design partner requires it.
- Tenant hierarchy or nesting. Not planned.
- Hosted and HA thresholds that would change the tenancy deployment model.
