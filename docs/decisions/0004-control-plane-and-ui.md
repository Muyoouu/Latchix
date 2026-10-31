# ADR 0004: Control plane contract and admin UI packaging

- Status: Accepted
- Date: 2026-11-01
- Owners: Musa Yohanes
- Supersedes: none

## Context

The original API surface was flat and single-tenant. It had no grant routes, no tenant scoping, no concurrency control, and no admin identity model. The admin UI was described as "server-rendered" with no feature gate, session model, CSRF defense, or design direction.

The control plane is the product differentiator. It must be usable by automation first and by humans second. The embedded core and API-only deployments must not pay for UI code.

## Decision

1. Tenant identity comes from the credential. Tenant-scoped keys use flat routes. Server-admin keys use `/api/v1/tenants/{tenant_id}/...` for cross-tenant management.
2. Roles are server admin, tenant admin, operator, and viewer. A key or session cannot mint a credential with more privilege than its own.
3. Errors use `application/problem+json` with a stable code registry and `request_id`.
4. Mutations require `ETag` and `If-Match`. Missing preconditions return 428, mismatches return 412.
5. Creates and replacements require `Idempotency-Key`, scoped to `(tenant_id, key)`, with a 24 hour TTL and a request hash binding.
6. Pagination uses an opaque `(created_at, id)` cursor. Limit is 1 to 200, default 50.
7. API keys are returned once at creation, stored as a prefix index plus SHA-256 or HMAC-SHA-256, and compared in constant time. Argon2 is not used for bearer tokens.
8. The bootstrap token is single use, `0600`, expires, is consumed on first admin creation, and is audited.
9. The Unix socket uses `SO_PEERCRED` with a uid allowlist before routing.
10. The admin UI is an opt-in Cargo feature. It is on for the standalone binary and off for core and embedded builds.
11. UI login exchanges an API key for a server-side session. There is no separate admin password or password table in v0.1.
12. The UI session uses `HttpOnly`, `Secure`, `SameSite=Lax`, CSRF tokens, and an Origin check. Logout deletes the session row. Privilege changes bump a session version.
13. The UI is server-rendered with no JavaScript framework in v0.1. Forms, POST-redirect-GET, and server-rendered errors are the contract.
14. A `DESIGN.md` direction is required before template work. The visual review uses the antislop and impeccable guidance after a mode is chosen.
15. Every UI action maps to an API operation. The UI never bypasses authorization or talks to the database directly.

## Consequences

- Automation and UI share one contract.
- Concurrent admins cannot silently overwrite each other.
- Retries cannot duplicate creates or audit events.
- API-only and embedded builds stay small.
- The UI has a clear security model instead of an ad hoc token in a cookie.
- UI work cannot start without a design direction, which avoids rework.

## Alternatives considered

- Flat routes with a tenant header. Rejected. Headers are easy to spoof or forget, and tenant identity must come from the credential.
- Nested routes for every tenant-scoped call. Rejected. It adds noise for tenant-scoped tokens. The hybrid keeps both cases explicit.
- Last-write-wins PATCH. Rejected. Silent clobbering is unacceptable for access control.
- Argon2 for API keys. Rejected. High-entropy bearer tokens do not need a password KDF, and per-request Argon2 is a CPU DoS vector.
- SPA in v0.1. Rejected. It adds a build toolchain, a second auth model, and a larger attack surface.
- UI always compiled in. Rejected. It fights the embedded core and API-only deployments.

## Evidence

- The v0.1 API originally exposed no grant, root, member, quota, API key, or session routes, even though the data model implied them.
- The original UI description had no feature gate, session store, CSRF defense, or logout.
- SQLite `LIKE` and case-sensitive uniqueness tests showed that path and identity matching need explicit normalization.
- The ACL evaluator prototype confirmed that effective-permission inspection is feasible without a database read per operation.

## Open items

- `DESIGN.md` content and antislop mode.
- Exact role capability matrix.
- UI route inventory and no-JavaScript contract.
- OIDC and mTLS in v0.2.
