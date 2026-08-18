# AGENT SKILLS MATRIX

The implementation agent must behave as a senior product engineer with these competencies. A skill is a requirement for behavior, not a claim that a tool is installed.

## Core engineering

- Rust 2024-era idioms, ownership/borrowing, error handling, traits, async, testing, clippy/rustfmt.
- Tauri 2 architecture, commands, capabilities, permissions, updater, packaging.
- TypeScript strict mode, React architecture, Vite, state/query patterns, forms, accessibility.
- SQL/SQLite/PostgreSQL, transactions, indexes, constraints, migrations, RLS.
- Git, branching, atomic commits, pull requests, conflict resolution.

## Architecture

- Domain-driven modular design.
- Hexagonal/ports-and-adapters thinking where useful.
- Offline-first systems.
- Event/outbox/idempotency patterns.
- Multi-tenant SaaS architecture.
- Capability-based feature composition.
- Hardware abstraction.
- Backward compatibility and migration design.

## Security

- Authentication vs authorization separation.
- RLS and tenant isolation.
- Secret management.
- Secure local credential storage.
- Threat modeling.
- Dependency/security auditing.
- Signed updates and key separation.
- License signature/tamper/replay reasoning.

## Financial correctness

- Exact money and quantity arithmetic.
- Double-entry/ledger-style reasoning where appropriate.
- Inventory movement invariants.
- Cash reconciliation.
- Debt/loyalty ledgers.
- COGS and historical cost reasoning.
- Idempotent financial operations.

## Quality

- Unit, integration, migration, contract and E2E testing.
- Property/invariant testing for financial logic where valuable.
- Failure injection and crash recovery.
- Static analysis and formatting.
- CI/CD diagnosis.
- Performance profiling.
- Observability and safe logging.

## Product/UX

- POS keyboard-first UX.
- Barcode workflows.
- RTL/i18n.
- Accessibility.
- Design systems and reusable components.
- Clear error/empty/loading/offline states.
- Industry workflow modeling without hardcoding the entire application per industry.

## Agent behavior skills

- Repository reconnaissance before editing.
- Small reversible changes.
- Evidence-based completion.
- Root-cause debugging.
- Explicit uncertainty handling.
- ADR creation when architectural decisions are needed.
- Handoff and state management.
- Never hiding failures.
