# Phase 0.7 — Agent Readiness Gate

This gate answers one question:

> Can a fresh autonomous implementation agent start building Phase 1 without inventing critical architecture or business rules?

## Gate checklist

### Architecture
- [x] Tauri/React/Rust/SQLite boundary documented
- [x] Supabase cloud boundary documented
- [x] Offline-first principle documented
- [x] capability/module architecture documented
- [x] licensing and updater trust boundaries documented

### Domain
- [x] exact money policy
- [x] quantity/unit policy
- [x] inventory ledger principle
- [x] costing strategy interface
- [x] tax engine contract
- [x] pricing/promotion contract
- [x] payment abstraction
- [x] refund/exchange contract
- [x] cash ledger
- [x] debt ledger
- [x] loyalty ledger
- [x] industry capability taxonomy

### Sync
- [x] outbox/idempotency foundation
- [x] conflict strategy matrix
- [x] financial/stock last-write-wins prohibition

### Commercial
- [x] license/entitlement boundary
- [x] SaaS billing abstraction
- [x] POS payment abstraction
- [x] provider selection checklist
- [x] website customer lifecycle

### Regulatory
- [x] jurisdiction adapter architecture
- [x] Algeria research baseline
- [x] France/EU research baseline
- [x] authoritative-source policy
- [x] no-global-compliance-claim rule

### Agent system
- [x] master agent prompt
- [x] planner/implementer/reviewer roles
- [x] granular backlog
- [x] task specification
- [x] Definition of Ready/Done
- [x] ADR protocol
- [x] evidence protocol
- [x] persistent state
- [x] golden E2E flows
- [x] acceptance matrix

## Explicit unresolved decisions

These are intentionally not guessed:

1. Final legal company/entity and billing seller-of-record arrangement.
2. Final public pricing and plan limits.
3. Final POS payment terminal/provider per launch market.
4. Final legal/compliance scope for each regulated industry.
5. Exact accounting/tax treatment that depends on the merchant's accountant/legal regime.

An agent may implement provider-neutral interfaces and test doubles for these. It may not silently select production providers or claim legal compliance.

## Ready condition

Phase 1 may start only when:
- CI is green;
- the Foundation Gate is accepted;
- the product owner approves the launch-market sequence;
- the unresolved decisions above are either intentionally deferred behind adapters or explicitly decided;
- `AGENT_STATE.md` points to the next unblocked task.

## Exit evidence

The gate requires links to:
- CI run(s)
- migration test result
- security/dependency audit
- acceptance checklist
- current commit SHA

Never mark this gate green merely because documentation exists.
