# ADR 0003: Metadata store and database choice

- Status: Accepted
- Date: 2026-11-01
- Owners: Musa Yohanes
- Supersedes: none

## Context

Latchix metadata is an OLTP workload: point reads, small writes, unique constraints, foreign keys, and low write volume from the control plane. The data path is bulk file transfer and must not query the database per packet.

The blueprint previously scheduled Postgres for v0.2. That is not justified by the workload and it fights the single-binary and embedded-core goals. DuckDB 2.0 and Quack server mode are real options now, but they are analytics-optimized and Quack is beta.

## Decision

1. SQLite is the only metadata backend in v0.1 and v0.2.
2. SQLite runs in WAL mode with one writer connection or writer actor and a pool of read connections.
3. Foreign keys are enabled on every pooled connection. `busy_timeout` is set. Write transactions are short and retried on `SQLITE_BUSY`.
4. `synchronous = "full"` is the default for durable audit commits. `"normal"` is opt-in for operators who accept losing the last commits on power loss.
5. The database file is on local disk only. Network filesystems are rejected. A process lock prevents two Latchix processes from writing the same database.
6. Migrations are embedded, forward-only, versioned, and tested. Backup uses the SQLite backup API or `VACUUM INTO` inside a read transaction.
7. Postgres is not scheduled. It is added only when a documented trigger is met.
8. DuckDB and DuckLake are analytics tools for audit and usage export, never the control-plane store.
9. The core defines a `MetadataStore` trait. SQLite is the first implementation. The trait exists for embedded hosts and tests, not as speculative support for a second backend.
10. The database stays off the data path. ACL checks use an in-memory snapshot. Quota counters reserve in memory and flush in batch. Audit writes are batched or appended to JSONL and indexed asynchronously.

## Triggers for Postgres

- More than one Latchix process must write the same metadata, or HA and zero-downtime upgrades are required.
- A hosted service cannot be pinned to one VM.
- Sustained durable audit writes exceed the measured SQLite ceiling after batching.
- Row-level security, read replicas, online schema changes, or cross-region replication are required.
- Metadata outgrows a single node.

## Consequences

- Single-host deployments stay simple and dependency-free.
- The write ceiling is real and must be measured. Audit volume is the first thing to fix, not the engine.
- Multi-instance and HA are explicitly out of scope until a trigger fires.
- A future Postgres adapter is a swap behind the trait, not a rewrite.
- DuckDB can still be used for audit analytics without touching the control plane.

## Measured evidence

Python driver, WAL, small database, on the development host. Rust with prepared statements should be faster.

| Workload | Durability | Rate |
|---|---|---|
| Per-row commit | `synchronous=FULL` | 725 writes/sec |
| Batch 200 | `synchronous=FULL` | 123,000 writes/sec |
| Per-row commit | `synchronous=NORMAL` | 51,000 writes/sec |
| Batch 200 | `synchronous=NORMAL` | 813,000 writes/sec |
| Point reads | read transaction | 1,135,000 reads/sec |
| Hot-row updates | `synchronous=FULL` | 713 writes/sec |

Concurrent write test: a second writer was blocked immediately. A reader in WAL succeeded while the writer held the transaction.

## Alternatives considered

- Postgres in v0.2. Deferred. No measured need, and it adds an external service to a single-binary product.
- DuckDB with Quack server mode as the control-plane store. Rejected for v0.1. Quack is beta with expected breaking changes, has no row-level security, and uses optimistic concurrency that aborts on hot-row conflicts. Revisit if Quack stabilizes.
- DuckLake with a Postgres catalog. Rejected for metadata. Parquet snapshots and compaction are wrong for small row updates.
- One SQLite file per tenant. Rejected for v0.1. Parallel writers per tenant do not justify N migrations, N backups, and no cross-tenant queries.
- Embedded key-value stores. Rejected. No SQL, no foreign keys, weaker operational tooling.

## Open items

- Exact audit batching policy and JSONL fallback threshold.
- Independent benchmark on the target deployment disk.
- Postgres trigger thresholds by write rate, host count, and deployment model.
