# CLI contract

- Status: Active
- Date: 2026-11-05
- Applies to: v0.1

The `latchix` binary is the primary artifact. The container has no shell, so
every operational action must be a CLI subcommand.

## Global options

| Option | Default | Purpose |
|---|---|---|
| `--config <PATH>` | `/etc/latchix/config.toml` | TOML configuration file. Also `LATCHIX_CONFIG`. |
| `--log-format <text\|json>` | `text` | Log output format. Logs go to stderr. |
| `--quiet` | off | Suppress informational logs. `RUST_LOG` still wins. |
| `--version` | | Print the binary version. |
| `--help` | | Print help for a command. |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Operational failure |
| `2` | Usage error from the argument parser |
| `3` | Command is not implemented in this build |
| `4` | Configuration missing, unreadable, or invalid |
| `5` | Health check failed |

## Commands

### `serve`

Starts the long-running process. In Phase 0 it starts the HTTP health listener
only. The SSH/SFTP listener arrives in Phase 1.

```bash
latchix serve --config /etc/latchix/config.toml
```

Options:

- `--shutdown-grace-seconds <SECONDS>`: default `30`. Wait this long for active
  listeners to stop after `SIGTERM` or `SIGINT`.

Behavior:

- Loads and validates config.
- Creates the storage root, database directory, and API socket directory if
  missing. Existing directories keep their permissions.
- Rejects a bad host key before binding. RSA host keys are rejected.
- Binds `api.tcp` and `api.unix_socket` when configured. Both can run at once.
- Exposes `GET /healthz` and `GET /readyz` on the API listener.
- Logs a warning that the SFTP listener is not implemented in Phase 0.
- Handles `SIGTERM` and `SIGINT` with a bounded graceful shutdown.

### `health`

Checks configuration and runtime paths, then probes the running API. This is
the container healthcheck.

```bash
latchix health --config /etc/latchix/config.toml
```

Checks:

- Config parses and validates.
- Storage root exists, is a directory, and is not a symlink.
- Database directory exists. The database file is checked when present.
- Each host key exists, is readable, parses, and is Ed25519 or ECDSA.
- API socket directory exists when a Unix socket is configured.
- Audit directory exists when a JSONL path is configured.

- Probes `GET /readyz` over `api.tcp`, with a fallback to `api.unix_socket`.
- A non-200 response or a connection failure is a health failure.

`doctor` is the preflight command. `health` is the liveness command.

### `doctor`

Prints build and environment details, then runs the same path checks as
`health`.

```bash
latchix doctor --config /etc/latchix/config.toml
```

Output includes the binary version, SFTP protocol version, audit schema
version, bundled SQLite version, UI feature state, bind addresses, database
path, and every path check.

Exit code `1` means at least one check failed.

### `migrate`

Applies embedded forward-only migrations.

```bash
latchix migrate --config /etc/latchix/config.toml
```

Phase 0 behavior: exit `3`, not implemented. Phase 2 implements it.

### `backup`

Writes a consistent SQLite backup and a storage snapshot.

```bash
latchix backup --config /etc/latchix/config.toml --output /backups/latchix.tar.zst
```

Phase 0 behavior: exit `3`, not implemented. Phase 2 implements it.

### `bootstrap-token`

Prints a one-time bootstrap token for the first server admin.

```bash
latchix bootstrap-token --config /etc/latchix/config.toml --ttl-seconds 3600
```

Options:

- `--ttl-seconds <SECONDS>`: default `3600`.
- `--json`: emit a JSON object instead of plain text.

The token is written to stdout once and never logged. Phase 0 behavior: exit
`3`, not implemented. Phase 3 implements it.

## Output rules

- Logs go to stderr.
- Machine-readable command output goes to stdout.
- Errors include the failing check and the path or option, never a secret.
- `--log-format json` emits one JSON object per line.
- `--quiet` does not suppress command output or errors.

## Container contract

- Entrypoint: `/usr/local/bin/latchix`.
- Default command: `serve --config /etc/latchix/config.toml`.
- Healthcheck: `latchix health --config /etc/latchix/config.toml`.
- The image has no shell. Do not depend on `sh`, `curl`, or `wget` inside it.
- Config, host keys, and bootstrap tokens are mounted, never baked in.
- The process runs as UID/GID `65532`.

## Signals

- `SIGTERM` and `SIGINT` trigger graceful shutdown.
- The process exits `0` after a clean shutdown.
- A shutdown that exceeds `--shutdown-grace-seconds` exits `1`.

## Configuration rules

- Paths in config are absolute.
- `database.url` must be `sqlite:///absolute/path.db`. In-memory databases are
  rejected.
- `storage.backend` must be `local` in v0.1.
- At least one of `api.tcp` or `api.unix_socket` must be set.
- `tenancy.default_slug` must be a lowercase DNS label.
- Unknown config fields are rejected so typos fail fast.
