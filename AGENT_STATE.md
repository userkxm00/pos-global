# AGENT STATE

> This file is operational state, not a substitute for Git history or CI evidence.

## Current

- Phase: 0 — Foundation Gate
- Status: FOUNDATION VERIFIED
- Current task: Foundation Gate closed; prepare Phase 1 task authorization
- Last verified commit: `8f5cdfe9e96c7a2fab5d4afaeef36f3bc1499098` via `foundation-gate-evidence-8f5cdfe9e96c7a2fab5d4afaeef36f3bc1499098`
- Next task: authorize exactly one Phase 1 task, starting with `F1.01` only after the state-synchronization commit receives its own current green Foundation Gate evidence

## Rules

- Update this file at every phase/task gate.
- Never claim a task is verified without CI/test evidence.
- Record blocked dependencies explicitly.
- Preserve the exact next task so another agent can resume without guessing.
- A pull-request head and the post-merge branch head are different commits and require separate verification.

## Evidence ledger

| Date | Task | Check | Result | Evidence |
|---|---|---|---|---|
| 2026-08-18 | Foundation documentation | Repository/PR review | PASS | GitHub PR #1 |
| 2026-08-18 | Frontend baseline | npm build | PASS | PR CI run |
| 2026-08-18 | Rust baseline | cargo check/test | PASS on migration-test PR head | PR #3 CI run |
| 2026-08-18 | Migration verification | fresh DB + repeatability + rollback + exact-money column tests | PASS on PR #3 head | PR #3 CI run |
| 2026-08-22 | Foundation evidence branch alignment | authoritative workflow updated from foundation/v2 to main | PASS | merged alignment changes |
| 2026-08-23 | Post-merge exact-head verification | authoritative foundation-gate-evidence | PASS | Run #79; exact main head `8f5cdfe9e96c7a2fab5d4afaeef36f3bc1499098`; artifact `foundation-gate-evidence-8f5cdfe9e96c7a2fab5d4afaeef36f3bc1499098` |

## Known blockers

- The state-synchronization commit must receive its own current green Foundation Gate evidence before the repository can advance to `AGENT_IMPLEMENTATION_READY`.
- Dependency vulnerability findings must have explicit disposition; do not use blind force upgrades.
- Dependency lockfiles are a reproducibility requirement to close before production/release gating; do not fabricate them.
- Production signing secrets are intentionally absent until release infrastructure is configured.
- Production Supabase must remain separate from development/staging.

## Handoff

When a task ends, update Current, Evidence ledger, Known blockers, and Next task before declaring the task complete.
