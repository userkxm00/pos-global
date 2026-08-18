# AGENT STATE

> This file is operational state, not a substitute for Git history or CI evidence.

## Current

- Phase: 0 — Foundation Gate
- Status: FOUNDATION VERIFICATION IN PROGRESS
- Current task: verify the exact `foundation/v2` head after the migration-test merge
- Last verified commit: see the latest successful `foundation-gate-evidence` artifact for the exact `foundation/v2` head
- Next task: close Foundation Gate, then initialize Phase 1 only after `AGENT_IMPLEMENTATION_READY`

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
| 2026-08-18 | Post-merge exact-head verification | foundation-gate-evidence | PENDING | `foundation/v2` push CI |

## Known blockers

- Exact post-merge `foundation/v2` CI/evidence must be green before `FOUNDATION_VERIFIED`.
- Dependency vulnerability findings must have explicit disposition; do not use blind force upgrades.
- Dependency lockfiles are a reproducibility requirement to close before production/release gating; do not fabricate them.
- Production signing secrets are intentionally absent until release infrastructure is configured.
- Production Supabase must remain separate from development/staging.

## Handoff

When a task ends, update Current, Evidence ledger, Known blockers, and Next task before declaring the task complete.
