# Foundation Evidence Contract

The Foundation Gate is evaluated against one exact Git commit. Evidence from another commit is stale and cannot satisfy the gate.

## Authoritative evidence

For automated verification, the authoritative record is the successful `foundation-gate-evidence` GitHub Actions job and its uploaded `foundation-gate-evidence-<commit-sha>` artifact for the exact repository head being evaluated.

`FOUNDATION_EVIDENCE.md` is the human-readable contract. It must not be edited to copy an older run onto a newer commit.

## Required evidence

| Gate | Required evidence |
|---|---|
| Frontend | CI job URL + successful build result |
| Rust | CI job URL + successful `cargo check` |
| Rust tests | CI job URL + successful `cargo test` |
| Formatting | CI job URL + successful `cargo fmt --check` |
| Migrations | CI job URL + fresh-database migration/test result |
| Dependency security | audit report + disposition of every high/critical finding |
| Secrets | secret scan result + disposition of any finding |
| Spec consistency | automated/manual consistency check recorded |
| Exact-head evidence | successful `foundation-gate-evidence` job + artifact naming the same commit SHA |

## Evidence record format

```text
commit_sha: <40-char SHA>
verified_at_utc: <timestamp>
ci_run: <GitHub Actions URL>
frontend: PASS | FAIL
rust_check: PASS | FAIL
rust_test: PASS | FAIL
rust_fmt: PASS | FAIL
migrations: PASS | FAIL
security: PASS | FAIL | REVIEWED
secrets: PASS | FAIL | REVIEWED
spec_consistency: PASS | FAIL
exact_head_artifact: PASS | FAIL
foundation_state: FOUNDATION_VERIFIED | BLOCKED
notes: <short evidence summary>
```

## Rules

1. Never copy a result from an older commit and mark the current commit green.
2. `queued` is not `pass`.
3. `mergeable` is not `pass`.
4. A skipped critical test is not a pass.
5. A security finding must have an explicit disposition; it must not be hidden by `continue-on-error`.
6. A merge creates a new commit; that exact post-merge head must receive its own CI/evidence run.
7. Evidence is part of the deliverable, but exact-head automated evidence is recorded by CI artifacts to avoid circular self-referential commits.
