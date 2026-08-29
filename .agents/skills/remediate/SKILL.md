---
name: remediate
description: Apply focused, minimal remediations for verified review findings or test failures with zero scope drift and dedicated regression tests.
---

# /remediate Workflow — Minimal-Impact Remediation

Follow this protocol when applying fixes for verified review findings or CI failures.

## Step 1: Define Narrow Fix Boundary
1. Identify the exact minimal lines of code requiring change.
2. Prohibit broad refactoring of unrelated functions or modules.
3. Prohibit modifying merged/protected files from earlier phases.

## Step 2: Implement Fix & Regression Test
1. Apply the minimal fix.
2. Add a dedicated regression unit/integration test specifically proving the edge case or failure mode is handled.
3. Run local test suite: `cargo test --all`.

## Step 3: Verify Local Quality Gates
1. `cargo clippy --all-targets --all-features -- -D warnings`
2. `cargo fmt --all -- --check`
3. `npm run build`

## Step 4: Final Diff Audit
Run `git diff` to ensure zero stray modifications before committing and pushing.
