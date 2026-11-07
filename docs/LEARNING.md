# Latchix learning checkpoints

- Status: Active
- Date: 2026-11-01
- Owner: Musa Yohanes
- Related: [PLAN.md](PLAN.md), [decisions](decisions)
- Reader: [Phase 0 reader](learning/PHASE-0-READER.md)
- Reader: [Phase 1 reader](learning/PHASE-1-READER.md)

The goal is not to read more code. The goal is to keep owning the decisions while agents write most of the code. Checkpoints test retrieval, prediction, and debugging, not familiarity.

## Operating rules

1. **You own the decision, the agent drafts.** An ADR is not accepted until you can defend its Decision and Consequences sections without AI.
2. **Explain before merge.** For every nontrivial diff, write five lines: what changed, why, what could break, how it was tested, and what you still do not understand.
3. **Predict before run.** Before asking the agent or running a test, write what you expect to happen. Compare. This is the fastest learning loop.
4. **One no-AI session per week.** Thirty minutes. No chat, no autocomplete. Docs, compiler, tests, and debugger are allowed.
5. **One no-AI bug per phase.** Timeboxed. Struggle first, then use AI.
6. **Trace one request per phase.** Follow it through logs, SQL, syscalls, and code.
7. **Keep an unknowns log.** Write "I do not know" explicitly. Unanswered questions become the next checkpoint.
8. **Review the agent like an adversary.** Assume it changed something you did not ask for. Find it.

## Checkpoint schedule

| Phase | Week | Exercise | Time | Pass condition |
|---|---|---|---|---|
| 0 Decisions | 2 | Decision defense | 60 min | Draw the system from memory. Defend five decisions and one opposing view. |
| 1 Containment | 6 | Escape lab | 90 min | Run four escape attempts. Explain every result, including the hardlink. |
| 2 Schema and ACL | 10 | Schema from memory | 90 min | Draw the schema and ACL order. Predict ten constraint outcomes. |
| 3 Control plane | 10 | Request trace | 60 min | Trace one mutation end to end. Find three planted bugs. |
| 4 Admin UI | 12 | Threat model | 60 min | Find one authz gap and write the regression test. |
| 5 Release | 12 | Operate from docs | 90 min | Restore a backup, rotate a key, revoke a grant during a session. |
| Monthly | ongoing | ADR re-derive | 60 min | Argue the opposite. Confirm or change the decision. |

## Phase 0: Decision defense

Before coding starts.

1. Draw the component diagram on paper. Include SFTP listener, session context, root descriptor, ACL snapshot, store, API, UI, audit.
2. Write the five decisions from memory:
   - Why tenancy is in the v0.1 schema.
   - Why SQLite is enough and Postgres is not scheduled.
   - Why the root is a descriptor, not a string.
   - Why the ACL is allow-only in v0.1.
   - Why the UI is an opt-in feature.
3. For each decision, write the strongest opposing argument. Then write why it loses.
4. Compare against the ADRs. Mark every gap in the unknowns log.

Pass: ten minutes without notes, then a coherent defense. If you need the ADR to start, you are not ready.

## Phase 1: Escape lab

After the SFTP spike and containment work.

1. Start the server. Connect with OpenSSH `sftp`.
2. Trace the open path with `strace`. Identify the root dirfd, the `openat2` call, and the flags.
3. Attempt four escapes: `..`, absolute path, symlink, hardlink. Record the errno for each.
4. Explain why `RESOLVE_BENEATH` does not stop the hardlink, and why the deny policy exists.
5. Find one place in the code where a string prefix check would have passed a malicious path.

Pass: you can explain the containment model without opening the code. You can predict the errno before running the attempt.

## Phase 2: Schema from memory

After the SQLite schema and ACL engine.

1. Draw the tables, the tenant-owned columns, the composite foreign keys, the partial unique indexes, and the audit triggers.
2. Write ten scenarios and predict the constraint result before running them:
   - cross-tenant grant
   - duplicate live grant
   - duplicate revoked key fingerprint
   - reused username after soft delete
   - grant with zero rights
   - reversed grant expiry
   - unscoped tenant admin key
   - writable share link
   - audit update
   - tenant hard delete
3. Implement one small migration without AI. Run it forward and restore the backup.
4. Explain the ACL evaluation order and one denial: account gates, subject mask, allow union, ancestor traverse, operation rights, deny.

Pass: eight of ten predictions correct. You can explain why a partial index is used instead of a plain unique.

## Phase 3: Request trace

After the control plane.

1. Pick one mutation, for example revoke a grant.
2. Trace it end to end: credential, tenant resolution, role check, transaction, `acl_revision` bump, audit event, session behavior on the next operation.
3. Draw the error mapping for 404, 409 idempotency mismatch, 412 stale ETag, 428 missing precondition.
4. Ask the agent to plant three bugs in a throwaway branch: a missing tenant predicate, a missing `If-Match`, and an idempotency replay. Find all three without AI.
5. Write one integration test for the bug you missed.

Pass: find all three planted bugs in thirty minutes. Explain each fix.

## Phase 4: Admin UI

After the UI.

1. Threat model the UI without AI: CSRF, session fixation, XSS, clickjacking, open redirect, privilege change during a session.
2. Find one route or form that could bypass API authorization.
3. Write the regression test that proves the gap.
4. Explain why the UI cannot talk to the database directly.

Pass: one real finding and one test. A clean review is acceptable only if you can show the tests that make it clean.

## Phase 5: Operate from docs

Before release.

1. Restore a backup into a clean directory. Start the server. Verify users and audit.
2. Rotate an API key. Confirm the old key fails.
3. Revoke a grant during an active SFTP session. Confirm the next operation fails.
4. Read the audit trail and reconstruct the sequence.
5. Do this using only the docs. No AI.

Pass: complete all four without asking the agent where anything is.

## Monthly: re-derive a decision

1. Pick one ADR. Write the decision from memory.
2. Argue the opposite for fifteen minutes.
3. Decide: still correct, needs amendment, or needs reversal.
4. If amended, update the ADR. If reversed, write a new ADR.

This prevents decisions from becoming inherited folklore.

## Unknowns log

Keep this table in your notes. Review it weekly.

| Date | Question | Answer | Source |
|---|---|---|---|
|  |  |  |  |

Examples: "Why does `acl_revision` live on the tenant?" "What happens to open file handles when a grant is revoked?" "Which index serves the audit cursor?"

## Warning signs

- You cannot explain a diff without opening the file.
- You accept "tests pass" as understanding.
- You cannot predict a failure mode before running it.
- You ask the agent what your own code does.
- You cannot draw the data flow from memory.
- You only review the happy path.

When one appears, stop feature work and run a checkpoint.

## What not to do

- Do not read AI summaries and call it learning.
- Do not passively scroll a diff.
- Do not let the agent write the final ADR, test, or security decision.
- Do not memorize file names instead of data flow.
- Do not say "I will understand it later."
- Do not use AI to debug your own mental model before trying.

## Time budget

- Weekly no-AI session: 30 minutes.
- Phase checkpoint: 60 to 90 minutes.
- Monthly ADR re-derive: 60 minutes.

Roughly two to three hours per month, plus the phase gates. Small cost, high return.
