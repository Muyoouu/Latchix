# ADR 0002: ACL and dynamic grants

- Status: Accepted
- Date: 2026-11-01
- Owners: Musa Yohanes
- Supersedes: none

## Context

The blueprint defined six allow-only booleans on a `permissions` row and promised that admin changes are visible on the next SFTP login. That model cannot express ancestor traversal, overwrite, time-bound grants, or revocation during an active session. It also left four security-sensitive questions unanswered: path matching, whether the virtual root grants access, whether POSIX or SQLite wins, and when authorization is re-evaluated.

## Decision

1. Rights are a bitmask: `TRAVERSE`, `LIST`, `READ`, `CREATE`, `WRITE`, `OVERWRITE`, `DELETE`, `RENAME`.
2. `TRAVERSE` is required on every ancestor of the target. SFTP needs resolve permission on each parent.
3. The virtual root is a boundary, not a grant. Creating a user may insert one explicit, audited `allow FULL` grant at `/`.
4. v0.1 is allow-only and default deny. `effect = 'deny'` is reserved in the schema and evaluator but rejected by the API until v0.2.
5. Grants are scoped to a tenant, root, and anchor. Scope is `subtree` or `exact`.
6. Subtree matching compares whole path segments. `/a` matches `/a/b`, never `/ab`. Bare `LIKE` is forbidden.
7. Paths are canonical, root-relative, UTF-8 NFC, with no `..`, no empty segments, and no trailing slash except `/`.
8. Subjects are a user or a group. Exactly one subject is set. Grants use partial unique indexes per subject kind.
9. Groups provide grants. Group membership can be downgraded to `read`, which masks effective rights to `TRAVERSE | LIST | READ`.
10. `users.read_only` masks effective rights after the allow union.
11. Each session holds an ACL snapshot keyed by `acl_revision`. Every grant, membership, user, key, or root mutation bumps the revision in the same transaction.
12. Sessions compare the revision on every operation and reload on mismatch. Revocation takes effect on the next operation, not the next login.
13. A `revoke_sessions=true` option closes matching channels and releases quota reservations.
14. The database ACL is the only authority. POSIX mode bits are never read for authorization. `chmod` and `chown` over SFTP are denied in v0.1.
15. Share links are a separate credential table. They are never grant subjects.
16. An effective-permissions endpoint with `explain=true` returns the matched rules and the missing right.
17. A grant carries at least one right. When both `not_before` and `expires_at` are set, the expiry is later.

## Evaluation order

1. Account and tenant gates: enabled, key valid, key not expired, root enabled.
2. Subject mask: read-only user or read-only group membership.
3. Union of matching, time-valid allow grants for the user and groups.
4. `TRAVERSE` on every ancestor above the target.
5. Operation-specific rights from the SFTP operation table.
6. Deny on failure and record the matched rule IDs.

## Consequences

- Grant changes are visible immediately, without a restart or a new login.
- Every session operation is an integer revision comparison plus an in-memory check. No database read on the hot path.
- Deny rules require a specificity algebra later. The schema and evaluator already reserve the branch.
- Clients cannot use `chmod` or `chown` to create a second source of truth.
- Operators get a debugging surface instead of reading SQLite by hand.

## Alternatives considered

- Six booleans per row. Rejected. It cannot express `TRAVERSE`, `OVERWRITE`, or new rights without migrations.
- Globs or regex in anchors. Rejected. Unreadable precedence and unbounded matching cost.
- Deny rules in v0.1. Deferred. The schema and evaluator reserve them, the API does not expose them yet.
- POSIX mode bits as the authority. Rejected. The service runs as one uid, so mode bits cannot mirror per-user grants.
- Login-time authorization. Rejected. It cannot revoke access or expire grants.
- ABAC or an external policy engine. Rejected for v0.1. The only attribute needed is ownership, and that can wait.

## Evidence

- An ACL evaluator prototype passed 22 of 22 cases: ancestor traverse, subtree boundaries, exact scope, expiry, read-only mask, group union, default deny, deny tie, non-canonical rejection, rename source and destination rights, overwrite, and root overlap.
- SQLite `LIKE` matched `/A/file` against grant `/a`, and `/axb/file` against grant `/a_b`. Byte-exact segment matching rejected both.
- Two identical user grants with a `NULL` group column were stored twice. Partial unique indexes fixed it.
- Expired grants failed closed at the expiry instant and passed one second earlier.
- `openat2` blocked `..`, absolute paths, and symlinks. A hardlink still escaped, so hardlinks are denied by default and allowed only in an explicit single-tenant mode.

## Open items

- Deny rule precedence and UI in v0.2.
- Ownership model for drop-box delete and overwrite policies.
- Share-link rights and download accounting in v0.2.
