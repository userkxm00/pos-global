# Foundation Evidence Contract

The Foundation Gate is evaluated against one exact Git commit. Evidence from another commit is stale and cannot satisfy the gate.

## Authoritative evidence

The normal `CI` workflow validates the exact pull-request head SHA, not GitHub's synthetic pull-request merge ref. It runs only for `pull_request` events and checks out `github.event.pull_request.head.sha` explicitly.

After a successful merge into `foundation/v2`, the separate `Foundation Gate Evidence` workflow is triggered by a `push` to `foundation/v2`. It checks out `github.sha`, verifies that `refs/heads/foundation/v2` still points to that exact SHA, runs the complete foundation verification suite on that post-merge branch head, and uploads `foundation-gate-evidence-<commit-sha>` as the authoritative machine-readable evidence.

`FOUNDATION_EVIDENCE.md` is the human-readable contract. It must not be edited to copy an older run onto a newer commit.

## Required evidence

| Gate | Required evidence |
|---|---|
| Frontend | Foundation Gate Evidence workflow + successful build result |
| Rust | Foundation Gate Evidence workflow + successful `cargo check` |
| Rust tests | Foundation Gate Evidence workflow + successful `cargo test` |
| Formatting | Foundation Gate Evidence workflow + successful `cargo fmt --check` |
| Migrations | Foundation Gate Evidence workflow + fresh-database migration/test result |
| Dependency security | audit report + disposition of every high/critical finding |
| Secrets | secret scan result + disposition of any finding |
| Spec consistency | automated/manual consistency check recorded |
| Exact-head evidence | successful `Foundation Gate Evidence` workflow + artifact naming the same commit SHA |

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
6. A pull-request merge ref is not the authoritative Foundation head evidence target.
7. A successful PR CI run is evidence for the tested PR head only. It is not post-merge exact-head evidence by itself.
8. The authoritative post-merge exact-head evidence is produced by `Foundation Gate Evidence` on a `push` to `foundation/v2`, after it verifies that the live branch ref still equals `github.sha`.
9. Exact-head evidence must run the mandatory build, test, migration, security, secret-scan, and specification checks on that exact post-merge commit before the artifact can be considered authoritative.
10. Evidence is part of the deliverable, but exact-head automated evidence is recorded by CI artifacts to avoid circular self-referential commits.
