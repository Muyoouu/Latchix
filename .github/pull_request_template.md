## Summary

Describe the change and why it is needed.

## Tests

List the commands you ran and their results.

## Checklist

- [ ] The change matches the v0.1 plan or has an accepted ADR.
- [ ] Tenant context is explicit.
- [ ] Tenant-owned queries and paths carry a tenant.
- [ ] Filesystem operations are root-descriptor relative.
- [ ] Authorization comes from the database ACL.
- [ ] Every mutation has an audit event.
- [ ] A test covers the new behavior or the bug.
- [ ] No secrets, host keys, bootstrap tokens, databases, or tenant data are committed.
- [ ] `cargo xtask check` passes.
