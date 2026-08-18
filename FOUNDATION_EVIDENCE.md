# Foundation Evidence Contract

The Foundation Gate is evaluated against one exact Git commit. Evidence from another commit is stale and cannot satisfy the gate.

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
foundation_state: FOUNDATION_VERIFIED | BLOCKED
notes: <short evidence summary>
```

## Rules

1. Never copy a result from an older commit and mark the current commit green.
2. `queued` is not `pass`.
3. `mergeable` is not `pass`.
4. A skipped critical test is not a pass.
5. A security finding must have an explicit disposition; it must not be hidden by `continue-on-error`.
6. Evidence is part of the deliverable and must be updated after material foundation changes.
