# Contributing

Latchix is pre-release. Before v0.2, contributions are limited to docs, tests,
and issues with a clear reproduction. Do not send large feature branches
without discussing the scope first.

## Before you start

- Read [BLUEPRINT.md](BLUEPRINT.md) and [docs/PLAN.md](docs/PLAN.md).
- Read the relevant ADR in [docs/decisions](docs/decisions).
- Check existing issues before opening a new one.
- For a security issue, follow [SECURITY.md](SECURITY.md). Do not open a public
  issue.

## Development

The pinned toolchain is in `rust-toolchain.toml`.

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps --all-features
cargo xtask check
```

Containerized tests:

```bash
scripts/test-container.sh
scripts/test-container.sh -- cargo test --locked -p latchix-sftp
```

The source mount is read-only. Cargo target and registry data live in named
Podman volumes.

Policy checks:

```bash
cargo install --locked --root .tools cargo-deny cargo-audit
./.tools/bin/cargo-deny check
./.tools/bin/cargo-audit audit
```

Build both binary targets:

```bash
cargo xtask build
```

The musl target needs a C toolchain for native dependencies. On Fedora, install
`musl-gcc`, `cmake`, `make`, and `perl`. On Debian or Ubuntu, install
`musl-tools`, `cmake`, `make`, and `perl`.

If the target-prefixed musl compiler is missing, `cargo xtask build` falls back
to the `rust:1.99.1-alpine` container through Podman. The container build is
slower but needs no host musl package.

## Rules

- Keep tenant context explicit. Never read a tenant or root from a process
  global.
- Every tenant-owned query and path carries a tenant.
- Every filesystem operation is root-descriptor relative. No string prefix
  checks.
- The database ACL is the only authorization authority.
- Add a test for every behavior change and an audit event for every mutation.
- Do not add `unsafe` outside a reviewed syscall adapter.
- Do not add a dependency without checking its license and advisory status.
- Do not commit secrets, host keys, bootstrap tokens, database files, or tenant
  data.

## Commits

Use Conventional Commits:

```text
type(scope): imperative summary
```

Common types: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `build`, `ci`,
`chore`, `revert`.

Keep the subject under 72 characters. Add a body when the reason is not
obvious. Do not add agent attribution, co-author trailers, or generated-by
lines.

## Licensing

The project is Apache-2.0. By contributing, you agree that your contribution is
licensed under the same terms. There is no CLA. A DCO may be adopted later.

## Review

Review focuses on correctness, tenant isolation, error handling, test coverage,
and whether the change matches the plan. A change that works but expands scope
without an accepted decision will be sent back.
