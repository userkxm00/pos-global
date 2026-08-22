# MILESTONE CHECKPOINTS — Zylo

Milestones define completion conditions, not calendar promises. A milestone is closed only when its listed tasks, acceptance criteria, and evidence are complete.

## Rules

1. A milestone is condition-based, never time-based.
2. A phase is not complete because its code exists; required evidence must pass.
3. The orchestrator/user controls progression to the next phase.
4. A checkpoint does not authorize a new implementation task by itself.
5. Exact evidence must be recorded in `AGENT_STATE.md` and task evidence.

## Foundation / Phase 0

### Checkpoint 0.A — Architecture and repository coherence

Required:

- architecture/schema/execution plan reconciled;
- security and dependency contracts internally consistent;
- no unresolved critical architecture conflict;
- required verification evidence recorded.

Gate: `FOUNDATION_ARCHITECTURE_VERIFIED`

### Checkpoint 0.B — Domain/commercial/regulatory contracts ready

Required:

- shared domain contracts frozen for the implementation scope;
- commercial/provider boundaries explicit;
- regulatory halt points active;
- unresolved critical business/security/regulatory decisions are either resolved or correctly marked blocked.

Gate: `FOUNDATION_CONTRACTS_READY`

### Checkpoint 0.C — Foundation verification

Required:

- required Foundation CI checks pass for the exact branch head;
- migration tests and required baseline tests pass;
- security/secret checks pass or have explicit approved disposition;
- evidence artifact references the exact verified commit.

Gate: `FOUNDATION_VERIFIED`

### Checkpoint 0.D — Agent implementation readiness

Required:

- `FOUNDATION_VERIFIED` is established;
- agent contracts are current;
- reading/testing/blocking/evidence protocols are available;
- first implementation task is explicitly assigned by the orchestrator/user.

Gate: `AGENT_IMPLEMENTATION_READY`

## Phase progression rule

For each later phase, define its checkpoint in the same pattern:

```text
Phase N
  → required tasks complete
  → required tests/evidence pass
  → critical decisions resolved
  → checkpoint artifact/evidence recorded
  → orchestrator authorizes next phase/task
```

Do not manufacture dates or story-point estimates as completion gates.

## Phase 1 — Identity / organization / permissions

Suggested closure evidence:

- auth flow works on the supported surfaces;
- session/security behavior is tested;
- authorization/tenant boundaries are verified outside the UI;
- RLS evidence exists for cloud paths;
- offline/session behavior meets the approved contract.

Gate: `PHASE_1_VERIFIED`

## Phase 2 — Product / units / inventory baseline

Suggested closure evidence:

- product and unit rules implemented;
- inventory mutations are ledger-traceable;
- stock adjustment and oversell behavior tested;
- migration and integration evidence complete.

Gate: `PHASE_2_VERIFIED`

## Phase 3 — Sales / payments / cash / customer debt / refunds

Suggested closure evidence:

- sale transaction atomicity verified;
- payment lifecycle and retries verified;
- customer/debt rules verified;
- return/refund uses approved compensating behavior;
- golden sale flow passes.

Gate: `PHASE_3_VERIFIED`

## Remaining phases

The authoritative task list remains `EXECUTION_PLAN.md` + `TASK_DEPENDENCY_GRAPH.md`. Each phase must close with the same evidence-first checkpoint pattern before the orchestrator authorizes the next phase.

Do not create duplicate phase schedules in this file. This file defines the checkpoint contract only.
