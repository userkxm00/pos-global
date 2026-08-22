# EVIDENCE TEMPLATE — Zylo

Every completed task must produce evidence proportionate to its acceptance criteria. This template standardizes the record; it does not replace Git history or CI.

## Task identity

- Task ID:
- Phase / Epic / Feature:
- Objective:
- Agent:
- Date:
- Final status: `DONE | PARTIAL | BLOCKED | UNVERIFIED | REJECTED`

## Acceptance criteria

List the exact observable acceptance criteria from the task.

- [ ] Given / When / Then: ...
- [ ] Given / When / Then: ...

## Changes

### Files changed

- `path/to/file` — summary

### Migrations

- None, or list migration(s) and compatibility impact.

### Decisions

- Existing contract/ADR used:
- New decision required: none / reference to approved ADR.

## Verification

| Layer | Command / method | Result | Evidence |
|---|---|---|---|
| Format | ... | PASS/FAIL/UNVERIFIED | ... |
| Lint/typecheck | ... | PASS/FAIL/UNVERIFIED | ... |
| Unit | ... | PASS/FAIL/UNVERIFIED | ... |
| Integration | ... | PASS/FAIL/UNVERIFIED | ... |
| Migration | ... | PASS/FAIL/UNVERIFIED | ... |
| Security | ... | PASS/FAIL/UNVERIFIED | ... |
| E2E/golden flow | ... | PASS/FAIL/UNVERIFIED/N/A | ... |
| Offline/sync | ... | PASS/FAIL/UNVERIFIED/N/A | ... |
| Hardware | ... | PASS/FAIL/UNVERIFIED/N/A | ... |

Do not mark a check `PASS` unless it actually ran and passed.

## Invariant evidence

For financial/stock/auth/sync tasks, list the critical invariants verified and their observable results.

Examples:

- Same idempotency key → same logical result.
- Side effects occur exactly once.
- Stock movement before/after balances match.
- Authorization deny path is enforced outside the UI.
- Money remains exact integer/exact-decimal.

## Security / regulatory

- Secrets present in repository: no / investigated
- Security impact:
- Regulatory applicability: none / applicable
- Regulatory evidence reference: none / link or artifact

## Known limitations

List only real remaining limitations or use `None`.

## Blockers / decisions

- Blockers: none / list
- Decisions required: none / list

## Evidence links

- Commit:
- Pull request:
- CI run(s):
- Artifacts/logs:
- Screenshots/manual review where applicable:

## Handoff

```text
STATUS: <status>
TASK: <task ID>
SUMMARY: <one paragraph>
EVIDENCE: <links>
NEXT_TASK: <recommendation only — not authorization>
```
