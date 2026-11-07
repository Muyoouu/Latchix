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
3. A tenant owns one storage tree. Within that tree, each `roots` row is an explicit storage boundary with its own opaque `storage_key`. v0.1 seeds exactly one default root per tenant. Users point at a root through `home_root_id`; roots are not per user by default. The schema permits multiple roots per tenant; multiple roots per user is deferred. Never derive the on-disk path from a tenant or root name.
4. `users.home_root_id` replaces `users.root_path` and is required in v0.1. Absolute host paths are never accepted from an API caller. Multiple users may reference the same root. Creating a user does not create a root or a directory tree.
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
- Roots are storage boundaries, not per-user folders. A user is assigned to a root; the root is not created for that user.

## Root model clarification

Amended 2026-11-07. This clarifies the relationship between tenants, roots, and
users. It does not change the v0.1 seed: one default root per tenant.

```text
tenant
  └── storage tree
        ├── root: default      <- seeded in v0.1, shared by users
        └── root: finance      <- allowed by the schema, later
```

- A **tenant** is the isolation boundary and owns the storage tree.
- A **root** is a named storage boundary inside that tree.
- A **user** is a login identity. `home_root_id` points at a root. It does not
  own the root.
- v0.1 creates one default root per tenant. All users in that tenant point at
  it unless an operator assigns otherwise.
- Multiple roots per tenant are allowed by the schema. Multiple roots per user
  is deferred.
- Each root has its own opaque storage key because it is a first-class storage
  boundary. The key supports root-level lifecycle, backup, and future multiple
  roots. It is not a per-user directory.

If the product later guarantees exactly one root per tenant forever, removing
the root storage key is a separate schema decision. Until then, the extra
directory level is deliberate future-proofing.

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
- When to expose additional roots per tenant through the API and UI. v0.1
  seeds one default root per tenant; the schema already permits more.
- Tenant hierarchy or nesting. Not planned.
- Hosted and HA thresholds that would change the tenancy deployment model.
