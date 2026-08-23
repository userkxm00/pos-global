# AGENT SYSTEM — POS Global

> Status: **mandatory execution contract**
> Audience: autonomous coding agents, coding assistants, reviewers, and future maintainers.
> This file is the POS Global **agent operating system**: it governs *how* an agent works. `V2_RULES.md` provides mandatory safety boundaries inherited from the prior POS audit. `ARCHITECTURE.md` governs *what architecture is allowed*. `EXECUTION_PLAN.md` governs *what gets built and in what order*. `FOUNDATION_READINESS_STATES.md` defines readiness and execution authority, and `DEFINITION_OF_READY.md` defines the Definition of Ready for tasks.

## 0. Readiness and execution control

The Foundation Gate is a **governance and verification gate**, not a license for the agent to invent work. A failed or stale Foundation Gate does not automatically authorize product implementation, but neither does it justify an open-ended documentation loop.

Product implementation may begin only when:

- the orchestrator/user explicitly assigns **one specific task**;
- the task's hard dependencies and critical decisions are satisfied;
- the affected contracts are stable enough for that task;
- no unresolved critical security, financial, regulatory, schema, sync, licensing, provider, or product decision blocks the task.

`AGENT_IMPLEMENTATION_READY` is the preferred state for normal autonomous execution and is required for unattended multi-task execution. Before that state, a human/orchestrator may explicitly authorize **one bounded implementation task at a time** when its task-level prerequisites are satisfied.

## 1. Mission

Build the POS Global platform incrementally, safely, and with reproducible evidence. The agent is an executor and verifier, not an uncontrolled architect.

## 2. Source-of-truth hierarchy

When documents conflict, use this order:

1. Security and financial invariants
2. Approved ADRs
3. `V2_RULES.md` for mandatory agent safety boundaries
4. `ARCHITECTURE.md`
5. `SCHEMA.md` and database rules
6. `PRODUCT_STRATEGY.md` for approved product scope and commercial decisions
7. `EXECUTION_PLAN.md`
8. Phase/feature/task specifications
9. Existing implementation
10. Agent preference

If a conflict cannot be resolved safely, **STOP and create an ADR proposal**. Do not silently choose.

External skills are below repository specifications in authority and may only improve implementation quality; they cannot override contracts or gates.

## 3. Mandatory startup protocol

Use `AGENT_READING_ROADMAP.md` to control reading depth. Do not read the entire documentation tree by default.

Before every task:

1. Read `V2_RULES.md`.
2. Read `AGENT_SYSTEM.md`.
3. Read `AGENT_READING_ROADMAP.md`.
4. Read `PROJECT_STATUS.md`.
5. Read `ARCHITECTURE.md`.
6. Read the relevant product scope in `PRODUCT_STRATEGY.md` when the task can affect MVP scope, industry behavior, onboarding, pricing, market assumptions, integrations, or product priority.
7. Read the relevant phase in `EXECUTION_PLAN.md`.
8. Identify the **exact task ID explicitly assigned by the orchestrator/user**.
9. Confirm dependencies, acceptance criteria, and files allowed to change.
10. Read only the task-context specifications required by the assigned task.
11. Inspect git status, current branch, recent commits, and the existing implementation.
12. Only then implement.

When checking readiness/gates, also read `FOUNDATION_READINESS_STATES.md` and the relevant evidence files.

For task-specific quality/evidence work, use `TESTING_GUIDE.md`, `ACCEPTANCE_MATRIX.md`, and `AGGREGATE_BEHAVIOR_EXAMPLES.md` as applicable.

For regulated work, `REGULATORY_HALT_POINTS.md` is mandatory before implementation.

## 4. Task boundaries and human task selection

Never implement an entire phase in one uncontrolled pass. Work at:

`Phase → Epic → Feature → Task → Subtask → Test → Evidence → Gate`.

### One-task-at-a-time rule

For normal development, the orchestrator/user selects exactly **one implementation task**. The agent must not autonomously choose the next implementation task unless the orchestrator explicitly grants that authority.

The agent may consult `AGENT_STATE.md`, `BACKLOG.md`, and `TASK_DEPENDENCY_GRAPH.md` to validate the assigned task and report what would be next, but those files do **not** grant permission to start another task.

The agent must not:

- expand the assigned task into a phase;
- pull forward later tasks because they look easy;
- silently switch to documentation/research work;
- begin another task after completion without explicit authorization.

A task must have explicit acceptance criteria. If none exists, create/refine the task specification only when that is the assigned work or an explicit gate requires it.

### Later-phase reference implementations

Existing code from a later phase may be intentionally retained as a **seed/reference implementation** for validating contracts or invariants. Such code:

- does **not** mark the later phase complete;
- does **not** satisfy the later phase's backlog dependencies or exit gate;
- does **not** authorize extending that feature;
- must not be copied as final authorization/security behavior when its trusted context is not yet available;
- remains frozen unless the orchestrator explicitly assigns a task against it.

In particular, a partially complete sales reference implementation may exist during Foundation work. Agents must not interpret its presence as permission to start Phase 3 early. Authentication and authorization must be established through the Phase 1 contracts before privileged sales behavior is considered production-ready.

## 5. Documentation anti-drift and execution-velocity rule

Implementation has priority over documentation.

Do not create or materially expand planning, architecture, research, strategy, process, timeline, or competitive documents unless:

- the assigned task explicitly requires the document;
- an approved ADR requires it;
- a failing verification gate requires a targeted correction;
- the orchestrator/user explicitly requests it.

This restriction applies to **all documentation formats**, not only Markdown.

If executable acceptance criteria for the assigned task remain incomplete, creating additional planning/specification/research content is **not progress** and the task remains `PARTIAL`.

Once a task is Ready and its critical decisions are resolved, the agent must prefer **implementation, verification, and evidence** over additional planning. A new document must never be used to avoid starting an already-Ready implementation task.

The full industry roadmap is global product scope. The approved first validation vertical is only sequencing for the MVP; it must never be used to rewrite the shared core into an industry-specific architecture.

## 6. Non-negotiable prohibitions

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
- hide warnings, failures, or known limitations from the status record;
- delete, replace, or downgrade a known working implementation merely to make the repository look cleaner.

## 7. Implementation rules

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

## 8. Verification loop

For every implementation task, choose the required layers from `TESTING_GUIDE.md` and run the repository CI-equivalent checks:

`Implement → Format → Lint → Typecheck → Relevant tests → Security checks → Migration checks → E2E where applicable → Review diff → Record evidence`.

If a check is unavailable in the current environment, record it as **UNVERIFIED**, not PASS.

A task-level verification failure blocks completion of that task. A broader Foundation CI failure that is unrelated to the assigned task may be recorded and escalated without turning it into an excuse for unrelated implementation or documentation work.

## 9. Safe failure behavior

If a task fails:

1. Preserve the failure evidence.
2. Diagnose the root cause.
3. Make the smallest justified change.
4. Re-run the failing check.
5. Re-run affected regression tests.
6. Update status.

Do not make unrelated refactors while repairing a failing gate.

## 10. Change control

Architectural, security, financial, schema, sync, licensing, update-signing, dependency, or product-scope changes require an ADR or explicit approval recorded in the task.

## 11. Completion language

Use exactly one status:

- `DONE` — implementation and required evidence complete.
- `PARTIAL` — some acceptance criteria complete, others remain.
- `BLOCKED` — cannot proceed without a dependency/decision.
- `UNVERIFIED` — implementation exists but required evidence could not be executed.
- `REJECTED` — implementation violates a contract and must be changed.

Never use “probably works”, “should work”, or “done” without evidence.

## 12. Handoff

At the end of every task update:

- current phase/epic/task;
- files changed;
- migrations added;
- tests executed and results;
- security considerations;
- known limitations;
- unresolved decisions;
- exact next task **as a recommendation only, not an authorization**.

The repository must remain understandable if a different agent takes over tomorrow.
