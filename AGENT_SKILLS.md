# AGENT SKILLS MATRIX

The implementation agent must behave as a senior product engineer with these competencies. A skill is a requirement for behavior, not a claim that a tool is installed. When a required tool is unavailable, the agent must use an approved alternative or explicitly mark the task blocked.

## Core engineering

- Modern stable Rust idioms: ownership/borrowing, lifetimes, traits, error handling, async, concurrency, testing, clippy/rustfmt.
- Tauri 2 architecture: commands, capabilities, permissions, IPC boundaries, secure local storage, updater, packaging and platform integration.
- TypeScript strict mode, React architecture, Vite, typed API boundaries, state/query patterns, forms, accessibility and error boundaries.
- SQL/SQLite/PostgreSQL: transactions, constraints, indexes, query plans, foreign keys, WAL, migrations, backups, restore and RLS.
- Git/GitHub: branches, commits, pull requests, reviews, rebases/merges, conflict resolution, release tags and protected branches.
- HTTP/API fundamentals, JSON schema validation, webhooks, retries, idempotency and timeout-after-commit handling.

## Architecture

- Domain-driven modular design and bounded contexts.
- Ports-and-adapters/hexagonal architecture where provider isolation is required.
- Offline-first desktop systems and local-first data ownership.
- Event/outbox/idempotency patterns and exactly-once business-effect reasoning.
- Multi-tenant SaaS architecture and tenant/branch/device isolation.
- Capability-based feature composition and industry presets.
- Hardware abstraction and failure containment.
- Backward compatibility, append-only migrations and safe upgrade paths.
- Explicit state machines for lifecycle-heavy workflows such as licenses, rentals, repairs and subscriptions.

## Backend / Supabase

- Supabase Auth concepts, sessions/tokens, MFA/OTP where enabled, and identity-to-application-user mapping.
- Postgres schema design, RLS, policies, indexes and least-privilege roles.
- Supabase Edge/server functions as adapters, not as a replacement for domain rules.
- Storage security, signed URLs where appropriate, webhook verification and audited server actions.
- Realtime/sync patterns where useful without making cloud availability a prerequisite for local POS operation.

## Security

- Authentication vs authorization separation.
- Rust-side authorization and secure Tauri command boundaries.
- RLS and tenant isolation with negative tests.
- Secret management and safe environment-variable handling.
- Secure local credential/session storage appropriate to the platform.
- Threat modeling, abuse cases and trust-boundary analysis.
- Dependency/security auditing, vulnerability triage and explicit disposition.
- Secret scanning and prevention of credential leakage.
- Signed updates, key separation, artifact integrity and secure release workflows.
- License signature, tamper, replay, clock manipulation and device-entitlement reasoning.
- OWASP-style web/API security fundamentals and secure webhook processing.
- Least privilege, secure logging, redaction and safe error reporting.

## Financial correctness

- Exact money and quantity arithmetic without floating-point financial truth.
- Deterministic rounding and currency minor-unit models.
- Ledger/double-entry-style reasoning where appropriate.
- Inventory movement invariants and stock reconciliation.
- Cash shift/open-close/count and variance reconciliation.
- Debt and loyalty ledgers with compensating events.
- COGS, valuation, historical cost and return-cost reasoning.
- Idempotent financial operations and duplicate-payment protection.
- Auditability and immutable historical truth.

## Domain / regulatory reasoning

- Tax-engine architecture with jurisdiction/version/effective-date data.
- Provider-neutral payment and billing adapters.
- Regulatory research workflow: authoritative source first, source date/version, implementation mapping and tests.
- Explicit separation between generic POS behavior and jurisdiction-specific compliance.
- Regulated-industry gating: never claim legal compliance without jurisdiction evidence.
- Commercial lifecycle reasoning for trials, subscriptions, entitlements, activation, renewal, grace and cancellation.

## Data / Sync

- Transactional outbox and idempotency stores.
- Retry, backoff and timeout-after-commit reconciliation.
- Conflict strategies by aggregate rather than one global last-write-wins rule.
- Device identity, sync checkpoints, quarantine/recovery and observability.
- Schema evolution and compatibility across offline clients and cloud projections.

## Quality

- Unit, integration, migration, contract, property/invariant and E2E testing.
- UI component and workflow testing for critical POS flows.
- Failure injection, crash recovery and power-loss style reasoning for local transactions.
- Static analysis, formatting and type checking.
- CI/CD diagnosis and reproducible build reasoning.
- Performance profiling and database/query optimization.
- Observability, metrics, tracing where appropriate and safe structured logging.
- Accessibility and keyboard-first UX verification.
- RTL/i18n/locale-aware formatting and localization testing.
- Backup/restore drills and disaster-recovery verification.

## Release engineering

- Windows/macOS/Linux packaging and platform-specific considerations.
- Application code signing and platform verification/notarization workflows where applicable.
- Tauri updater metadata/signatures, staged rollout and rollback/recovery.
- Protected GitHub environments and secret separation for release signing.
- Reproducible dependency installs and lockfile discipline.
- Artifact integrity, provenance/traceability and release evidence.
- Semantic versioning and database migration compatibility.

## Product / UX

- POS keyboard-first UX and fast cashier paths.
- Barcode/scanner workflows and resilient device feedback.
- Product matrix/variant UX, weighted-item UX and serial/batch workflows.
- Cash/payment/refund/exchange UX with clear totals and explanations.
- RTL/i18n and locale-aware money/date/number formatting.
- Accessibility and reusable design-system components.
- Clear loading/empty/error/offline/retry/conflict states.
- Industry workflow modeling without hardcoding the whole product per industry.
- Explainable pricing/tax/promotion results and auditable user actions.

## Agent behavior skills

- Repository reconnaissance before editing.
- Read the authoritative specs and current agent state before selecting work.
- Use the task dependency graph and capability matrix instead of inventing sequencing.
- Small, reversible changes with focused commits.
- Evidence-based completion and exact-head verification.
- Root-cause debugging rather than disabling checks.
- Explicit uncertainty handling and STOP/BLOCKED behavior for critical unknowns.
- ADR creation when architectural/business/security decisions are needed.
- Handoff and persistent state management.
- Never hiding failures, stale evidence or skipped required checks.
- Never fabricating lockfiles, test evidence, regulatory claims or provider eligibility.
