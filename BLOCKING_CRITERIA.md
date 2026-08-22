# BLOCKING CRITERIA — Zylo

This document defines when the agent must stop, when it may continue with an explicit documented gap, and when it must self-block for quality. It supplements `AGENT_SYSTEM.md` and `REGULATORY_HALT_POINTS.md`.

## 1. Hard blocks — must stop

| Category | Example | Required unblocker |
|---|---|---|
| Missing contract | Task depends on an undefined domain/API/schema contract | Explicit contract or approved ADR |
| Security decision missing | Auth boundary or secret handling is ambiguous | Explicit security decision |
| Financial rule missing | Tax, money, costing, debt, refund, or ledger behavior is ambiguous | Approved financial/domain rule |
| Regulatory gap | Jurisdiction-specific behavior lacks approved authoritative evidence | Approved regulatory evidence package |
| Provider decision missing | Provider-specific implementation required but provider is not approved | Provider/commercial decision or provider-neutral task |
| Schema conflict | Task conflicts with applied schema/migration contract | Approved schema decision + append-only migration plan |
| Sync conflict rule missing | Financial/stock conflict cannot be resolved from the sync contract | Approved sync rule |
| Licensing conflict | External code/asset reuse is unclear | License review and explicit approval |
| Required dependency unavailable | Environment/dependency prevents safe verification or implementation | Dependency/environment repair or explicit bounded exception |
| Prior mandatory task failed/blocked | Assigned task requires an output that does not exist | Resolve prerequisite or explicit re-planning by orchestrator |

## 2. Soft gaps — may continue only when safe

A soft gap may be used only when the task contract explicitly allows it and no critical invariant is affected.

Examples:

- cosmetic UI detail not yet finalized;
- missing non-critical synthetic test data that can be generated locally;
- performance tuning that is outside the task acceptance criteria;
- optional research detail that does not alter the contract.

The agent must record the gap and mark the task `PARTIAL` or `UNVERIFIED` as appropriate. A soft gap is never permission to invent a business/security/regulatory rule.

## 3. Self-blocks — agent must not declare success

Self-block when:

- required tests for changed behavior cannot be written or executed;
- a financial/stock invariant fails;
- authorization can be bypassed;
- a required migration test fails;
- idempotency/retry produces duplicate side effects;
- security verification fails;
- the implementation violates an approved contract;
- the diff contains unexplained unrelated changes.

Do not lower the quality bar to make the task green.

## 4. Blocking message

Use the protocol in `AGENT_COMMUNICATION_PROTOCOL.md`:

```text
[BLOCKED] <task ID>
Blocker: <precise blocker>
Category: <category>
Required to unblock: <exact input/decision/evidence>
Cannot safely proceed because: <reason>
```

## 5. Decision hierarchy when blocked

1. Existing explicit repository contract
2. Approved ADR
3. Approved jurisdiction/provider package
4. Explicit orchestrator/user decision
5. Implementation judgment only for non-critical details

Never use an external repository, generic web knowledge, or agent preference to override a critical project decision.

## 6. No autonomous task switching

A blocked task does **not** authorize the agent to choose another implementation task. The orchestrator/user decides the next task explicitly.

## 7. Unblocking

After the unblocker is provided:

1. Record the new evidence/decision.
2. Re-validate dependencies.
3. Re-read only affected contracts.
4. Resume the assigned task.
5. Re-run affected verification.
