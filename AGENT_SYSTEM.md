# AGENT SYSTEM — POS Global

> Status: **mandatory execution contract**
> Audience: autonomous coding agents, coding assistants, reviewers, and future maintainers.
>
> This file governs *how* an agent works. `ARCHITECTURE.md` governs *what architecture is allowed*. `EXECUTION_PLAN.md` governs *what gets built and in what order*.

## 1. Mission

Build the POS Global platform incrementally, safely, and with reproducible evidence. The agent is an executor and verifier, not an uncontrolled architect.

## 2. Source-of-truth hierarchy

When documents conflict, use this order:

1. Security and financial invariants
2. Approved ADRs
3. `ARCHITECTURE.md`
4. `SCHEMA.md` and database rules
5. `EXECUTION_PLAN.md`
6. Phase/feature/task specifications
7. Existing implementation
8. Agent preference

If a conflict cannot be resolved safely, **STOP and create an ADR proposal**. Do not silently choose.

## 3. Mandatory startup protocol

Before every task:

1. Read `AGENT_SYSTEM.md`.
2. Read `PROJECT_STATUS.md`.
3. Read `ARCHITECTURE.md`.
4. Read the relevant phase in `EXECUTION_PLAN.md`.
5. Read `DOMAIN_CONTRACTS.md` for the affected domain.
6. Read `DATABASE_RULES.md` if storage is affected.
7. Read `SECURITY_MODEL.md` if authentication, authorization, secrets, licensing, sync, or privileged APIs are affected.
8. Inspect git status, current branch, recent commits, and the existing implementation.
9. Identify the exact task ID and acceptance criteria.
10. Confirm dependencies and files allowed to change.
11. Only then implement.

## 4. Task boundaries

Never implement an entire phase in one uncontrolled pass. Work at:

`Phase → Epic → Feature → Task → Subtask → Test → Evidence → Gate`.

A task must have explicit acceptance criteria. If none exists, create a task specification before coding.

## 5. Non-negotiable prohibitions

Never:

- weaken or delete a failing test just to obtain green CI;
- bypass authorization in the UI or backend;
- put secrets/service-role keys/private signing keys in source code;
- use floating point as authoritative financial truth;
- directly mutate stock without a corresponding ledger movement;
- modify an applied migration;
- silently change a public/domain contract;
- swallow errors without a documented recovery policy;
- introduce a dependency without recording its reason and compatibility/security impact;
- duplicate domain rules between React and Rust;
- use last-write-wins for financial conflicts;
- make network availability a prerequisite for local POS selling;
- mark a task complete without evidence;
- claim a test/build was run when it was not;
- hide warnings, failures, or known limitations from the status record.

## 6. Implementation rules

### Rust
- Domain and financial invariants belong in Rust.
- Tauri commands are thin application boundaries.
- Use typed errors and deterministic transactions.
- Keep repositories separate from domain rules.

### React/TypeScript
- Presentation and interaction only.
- No direct SQLite access.
- No privileged OS access outside approved Tauri commands.
- Do not duplicate authoritative financial calculations.

### Database
- Foreign keys enabled.
- Migrations append-only.
- Money uses exact integer minor units or approved exact-decimal representation.
- Tenant/organization scope is explicit.
- Important mutations are atomic.
- Ledger/history is append-only; corrections use compensating entries.

### Supabase
- Client may use only publishable client credentials.
- Secret/service-role credentials never ship in the desktop application.
- RLS is mandatory for cloud tenant isolation.
- Cloud is not the operational source of truth while the POS is offline.

## 7. Verification loop

For every implementation task:

`Implement → Format → Lint → Typecheck → Unit tests → Integration tests → Security checks → Migration checks → E2E where applicable → Review diff → Record evidence`.

If a check is unavailable in the current environment, record it as **UNVERIFIED**, not PASS.

## 8. Safe failure behavior

If a task fails:

1. Preserve the failure evidence.
2. Diagnose the root cause.
3. Make the smallest justified change.
4. Re-run the failing check.
5. Re-run affected regression tests.
6. Update status.

Do not make unrelated refactors while repairing a failing gate.

## 9. Change control

Architectural, security, financial, schema, sync, licensing, update-signing, or dependency changes require an ADR or explicit approval recorded in the task.

## 10. Completion language

Use exactly one status:

- `DONE` — implementation and required evidence complete.
- `PARTIAL` — some acceptance criteria complete, others remain.
- `BLOCKED` — cannot proceed without a dependency/decision.
- `UNVERIFIED` — implementation exists but required evidence could not be executed.
- `REJECTED` — implementation violates a contract and must be changed.

Never use “probably works”, “should work”, or “done” without evidence.

## 11. Handoff

At the end of every task update:

- current phase/epic/task;
- files changed;
- migrations added;
- tests executed and results;
- security considerations;
- known limitations;
- unresolved decisions;
- exact next task.

The repository must remain understandable if a different agent takes over tomorrow.
