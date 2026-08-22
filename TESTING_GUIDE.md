# TESTING GUIDE — Zylo

## Purpose

Tests are evidence for task completion. The required test depth depends on the behavior changed; line coverage alone is not a completion criterion.

## Test layers

### 1. Unit

Use for pure domain rules, parsers, state transitions, validation and calculations.

Minimum expectation: happy path + boundary/invalid cases for changed rules.

### 2. Integration

Use when a command changes SQLite state, crosses repositories, or coordinates multiple domain components.

Verify persisted state and all important side effects, not only the returned value.

### 3. Migration

For every schema migration, verify:

- fresh database application;
- repeatability/idempotent migration runner behavior;
- expected schema objects and constraints;
- rollback behavior where the migration contract requires it;
- compatibility with the existing migration chain.

Never edit an already-applied migration; add a new migration.

### 4. Idempotency and retry

For every retryable command:

- first execution succeeds;
- same idempotency key returns the same logical result;
- side effects occur only once;
- different keys create independent operations;
- conflict/mismatch behavior is deterministic.

Where concurrency matters, add a concurrent/race-oriented test when the environment permits it.

### 5. Financial and stock invariants

For money/inventory changes, assert:

- exact integer/exact-decimal behavior;
- atomic commit/rollback;
- no overselling;
- ledger/stock movement traceability;
- compensating-entry behavior for corrections/returns;
- no floating-point authoritative truth.

### 6. Security

For auth/authorization/privileged operations, test both allow and deny paths.

Do not test security only through the UI. Verify the trusted Rust/service boundary and tenant/organization scope where applicable.

### 7. E2E / golden flows

Use for user-visible workflows that cross UI + Tauri/service + database/cloud boundaries.

A golden flow should verify observable business outcomes, for example:

`login → open shift → create sale → payment → stock decrement → receipt → evidence`

Golden flows should cover at least one happy path and the most important rejection/recovery path for the feature family.

### 8. Offline/sync

Test:

- normal offline operation;
- queue/outbox creation;
- retry after reconnect;
- duplicate delivery;
- conflict handling according to the sync contract;
- financial/stock conflicts without naive last-write-wins.

### 9. Hardware

Hardware integrations should be tested through adapters/mocks in CI and through a small set of real-device acceptance tests before hardware release. Printer/device failure must not duplicate the underlying transaction.

## Verification commands

Run the commands required by the changed layer and the repository CI. Do not claim a check was run when it was unavailable.

If a required check cannot run in the current environment, report `UNVERIFIED` and include the reason.

## Coverage policy

Do not use a single global percentage as the definition of quality. Critical business invariants require behavior-focused tests even when overall line coverage is high.

## Completion rule

A test task is complete only when acceptance criteria are met, required regression layers pass, and evidence identifies exactly what was executed.
