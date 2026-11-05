# Latchix design notes

The main project blueprint lives at [BLUEPRINT.md](../BLUEPRINT.md).

The v0.1 execution plan lives at [PLAN.md](PLAN.md).

The command line contract lives at [CLI.md](CLI.md).

Container testing lives at [TESTING.md](TESTING.md).

The client matrix lives at [CLIENT-MATRIX.md](CLIENT-MATRIX.md).

The admin UI design gate lives at [DESIGN.md](../DESIGN.md).

The personal learning checkpoints live at [LEARNING.md](LEARNING.md).

The Phase 0 self-study reader lives at
[learning/PHASE-0-READER.md](learning/PHASE-0-READER.md).

Packaging and deployment notes live at [packaging/README.md](../packaging/README.md).

## Decisions

- [0001: Tenancy model](decisions/0001-tenancy-model.md)
- [0002: ACL and dynamic grants](decisions/0002-acl-and-grants.md)
- [0003: Metadata store and database choice](decisions/0003-metadata-store.md)
- [0004: Control plane and UI packaging](decisions/0004-control-plane-and-ui.md)
- [0005: Distribution artifacts](decisions/0005-distribution-artifacts.md)
- [0006: IDs and storage layout](decisions/0006-ids-and-storage-layout.md)

## Conventions

- `decisions/` holds one ADR per irreversible or expensive decision.
- Keep ADRs short: context, decision, consequences.
- Update the blueprint when a decision changes the scope or architecture.
