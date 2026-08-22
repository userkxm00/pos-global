# Foundation Readiness States

This document defines the valid readiness states for the repository and how they control agent autonomy. A higher state must never be inferred from a lower state.

## 1. FOUNDATION_DESIGNED

The architecture, domain contracts, execution plan, backlog, agent operating system, security model, sync contract, product/UI contracts, release contract, and required foundation scaffolding exist and are internally reviewable.

**Meaning:** designed, not yet proven by Foundation CI/evidence.

## 2. FOUNDATION_VERIFIED

`FOUNDATION_DESIGNED` plus all Foundation Gate evidence is green for the exact head commit:

- frontend build passes
- Rust check passes
- Rust tests pass
- migration verification passes
- dependency/security review is recorded
- secret scanning is clean or every finding is explicitly dispositioned
- repository/spec consistency check passes

**Meaning:** the shared foundation is technically verified.

## 3. AGENT_IMPLEMENTATION_READY

`FOUNDATION_VERIFIED` plus:

- Definition of Ready is satisfied
- Phase 1 backlog is unambiguous
- agent state is initialized
- no unresolved decision can change the Phase 1 architecture
- implementation/review/evidence prompts are available

**Meaning:** the orchestrator may authorize unattended multi-task autonomous implementation.

## 4. BOUNDED_TASK_AUTHORIZED

A human/orchestrator explicitly assigns **one specific implementation task** before `AGENT_IMPLEMENTATION_READY`, provided that:

- the task has explicit acceptance criteria;
- all hard prerequisites for that task are complete;
- critical security, financial, regulatory, schema, sync, licensing, and provider decisions required by the task are resolved;
- the affected foundation contracts are stable enough for the task;
- the task can be verified independently.

`BOUNDED_TASK_AUTHORIZED` does **not** authorize the agent to select another task, begin another phase, or run an unattended multi-task loop.

**Meaning:** one bounded implementation task may proceed under explicit orchestration even while broader Foundation verification is still pending.

## 5. PRODUCTION_READY

All required product capabilities are implemented and verified, including security, data integrity, sync, licensing, update, backup/recovery, hardware, E2E, performance, and applicable jurisdiction requirements.

**Meaning:** the software can enter controlled production release.

## 6. LAUNCH_READY

`PRODUCTION_READY` plus commercial, operational, support, release-signing, website, billing, legal, documentation, monitoring, and rollout gates are complete for the selected launch markets.

**Meaning:** public launch is approved.

## Execution control rules

1. `AGENT_IMPLEMENTATION_READY` is required for unattended multi-task autonomous execution.
2. `BOUNDED_TASK_AUTHORIZED` may be used for a single explicitly assigned task when its task-level prerequisites are satisfied.
3. The agent must never infer permission to start another task from `TASK_DEPENDENCY_GRAPH.md`, `BACKLOG.md`, or `AGENT_STATE.md`.
4. `FOUNDATION_VERIFIED` remains the canonical evidence that the shared foundation itself has passed its full verification gate.
5. A failed Foundation Gate does not become a PASS through GitHub mergeability, a successful unrelated task, or manual assertion.

## Non-negotiable rule

`mergeable=true` on GitHub is not evidence of `FOUNDATION_VERIFIED`. GitHub mergeability only describes whether the branch can be merged mechanically. The repository's Foundation Gate remains authoritative for the foundation's verified state.
