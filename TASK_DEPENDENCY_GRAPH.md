# POS Global — Agent Task Dependency Graph

This graph defines implementation order. A task is unblocked only when all listed prerequisites are complete and accepted by its phase gate.

## Rules

- `→` means hard dependency.
- Tasks in the same row separated by `+` may proceed independently after their prerequisite.
- A phase gate is a hard barrier; later phases cannot start early unless the explicit exception below applies.
- Documentation-only decisions may run in parallel, but implementation tasks cannot depend on an unapproved critical decision.
- The agent must not infer missing dependencies from code structure.
- Phase 0.65 has two purposes: establish the approved MVP/global product boundary and prevent premature commercial/industry expansion. Unresolved commercial decisions block MVP-specific commercial scope, but do not block an explicitly authorized generic foundation/core task that does not depend on those decisions.
- No Fashion-specific implementation may begin until the approved MVP scope and its task-level acceptance criteria are available.

## Foundation

```text
F0.01 → F0.02 → F0.03 → F0.04 → F0.05 → F0.06
                    ↘     ↘      ↘
                      F0.07 → F0.08 → F0.09 → F0.10

F0.5.01 + F0.5.02 + F0.5.03 + F0.5.04
              + F0.5.05 + F0.5.06 + F0.5.07
              + F0.5.08 + F0.5.09 + F0.5.10
              + F0.5.11 + F0.5.12 + F0.5.13 + F0.5.14
                    → F0.5.15 → F0.5.16

F0.6.01 + F0.6.02 + F0.6.03 + F0.6.05
      → F0.6.04
F0.6.06 + F0.6.07 + F0.6.08 + F0.6.09
F0.6.10 + F0.6.11
      → F0.6.12
```

## Phase 0.65 — Product Strategy & MVP

```text
F0.65.01
   → F0.65.03 + F0.65.04 + F0.65.05 + F0.65.06 + F0.65.07
   → F0.65.08

F0.65.02 = explicit human product-decision gate.
It is required before final MVP/commercial scope is treated as approved.
It does not block an explicitly authorized generic foundation/core task.

F0.65.03 + F0.65.04 + F0.65.05 + F0.65.06 + F0.65.07
   → MVP-specific implementation authorization
```

`F0.65.01` records that POS Global remains a global multi-industry platform while Fashion/Clothing/Footwear/Accessories is the first validation scope. The industry roadmap remains intact; only approved MVP scope may be implemented for the first commercial release.

## Phase 0.7 — Agent Readiness

```text
F0.7.01 + F0.7.02 + F0.7.03 + F0.7.04
      + F0.7.05 + F0.7.06 + F0.7.07 + F0.7.08 + F0.7.09
      → F0.7.10 + F0.7.11 + F0.7.12 + F0.7.13
      → F0.7.14
```

The Agent Readiness Gate must include verification that the product-strategy guardrails are visible to the agent and that unresolved product decisions are recorded as decisions/blockers rather than silently invented.

## Phase 1 — Identity & tenancy

```text
F1.01 → F1.02 → F1.03
F1.04 → F1.05
F1.06 → F1.07 → F1.09
F1.08 → F1.10
F1.01 + F1.02 + F1.03 + F1.04 + F1.05 + F1.06 + F1.07 + F1.08
      → F1.09 → F1.10 → Phase 1 Gate
```

Phase 1 remains generic/global. It must not embed Fashion-specific assumptions.

## Phase 2 — Product & inventory

```text
F1 Gate
  → F2.01 → F2.02 → F2.03 → F2.04
  → F2.05 + F2.06 + F2.07 + F2.08 + F2.09
  → F2.10 → F2.11 → F2.12 + F2.13 + F2.14 → F2.15
```

Matrix/weight/batch/serial features depend on the shared product identity and canonical-unit contracts, but remain independently testable. MVP-specific UI should consume these generic capabilities rather than redefine them.

## Phase 3 — Sales & cash

```text
F2 Gate
  → F3.01
  → F3.02
  → F3.03
  → F3.04 → F3.05
  → F3.06
  → F3.07
  → F3.08
  → F3.09 + F3.10 + F3.11
  → F3.12 → F3.13 → Phase 3 Gate
```

## Phase 4 — Purchasing & profitability

```text
F2 Gate + F3.03
  → F4.01 → F4.02 → F4.03 → F4.04
  → F4.05 → F4.06 → F4.07 → F4.08
```

## Phase 5 — Customers & loyalty

```text
F3.08
  → F5.01 → F5.02 → F5.03 → F5.04 → F5.05
  → F5.06 → F5.07
```

## Phase 6 — Offline & sync

```text
F1.03 + F2.11 + F3.03
  → F6.01 → F6.02 → F6.03 → F6.04 → F6.05
  → F6.06 → F6.07 → F6.08 → F6.09
```

All cross-client contracts must remain compatible with future mobile clients.

## Phase 7 — Industry modules

All industry workflows require Phase 2 inventory + Phase 3 sales/cash + Phase 4 purchasing where relevant + Phase 6 sync foundations. Shared capabilities are implemented once, then composed by preset.

```text
Core commerce gate
  → MVP product-decision approval
  → F7.*.01 capability composition
  → F7.*.02 workflow model
  → F7.*.03 screens/commands
  → F7.*.04 transactions/invariants
  → F7.*.05 tests
  → F7.*.06 acceptance/evidence
```

For the first commercial release, only the approved Fashion/Clothing/Footwear/Accessories scope may advance through this chain. Other families remain roadmap-only until explicitly promoted by product decision and evidence.

Each industry family has its own dependency chain in `INDUSTRY_EXECUTION_PLAN.md`.

## Phase 8 — Licensing

```text
F1.04 + commercial gate
  → F8.01 → F8.02 → F8.03 → F8.04
  → F8.05 → F8.06 + F8.07 + F8.08
  → F8.09
```

## Phase 9 — Website & billing

```text
Commercial gate + F8.01
  → F9.01 → F9.02 → F9.03
  → F9.04 → F9.05 → F9.06 → F9.07
```

## Phase 10 — Hardware

```text
F3.07 + hardware contracts
  → F10.01 + F10.02 + F10.04
  → F10.03 + F10.05 + F10.06
  → F10.07
```

Only hardware required by the approved first commercial workflow is in MVP scope; abstractions must remain extensible.

## Phase 11 — Reports

```text
F3 Gate + F4 Gate + F5 Gate
  → F11.01 + F11.02 + F11.05 + F11.07
  → F11.03 + F11.04 + F11.06
  → F11.08 + F11.09 + F11.10
```

## Phase 12–14

```text
Core feature gates
  → F12.01/F12.02/F12.03
  → F12.04 → F12.05 → F12.06 → F12.07 + F12.08 + F12.09
  → F13.01..F13.09
  → F14.01 → F14.02 → F14.03 → F14.04 → F14.05
```

## Phase 15 — Mobile Companion & Unified Distribution

```text
Shared contract stability + Phase 6 sync foundations + product promotion
  → F15.01
  → F15.02 + F15.03
  → F15.04 + F15.05 + F15.06 + F15.07 + F15.08
  → F15.09
  → F15.10 + F15.11 + F15.12
  → F15.13
```

## Agent scheduling policy

The agent may select the next task only from tasks whose prerequisites are satisfied, whose Definition of Ready is true, and whose critical decisions are approved. If two tasks are independent, the agent may choose either one but must record the choice in `AGENT_STATE.md`.

The MVP product scope is not permission to start an entire phase. The orchestrator/user still assigns exactly one implementation task at a time unless explicit autonomous multi-task authority has been granted.
