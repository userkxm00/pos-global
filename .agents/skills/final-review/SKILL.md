---
name: final-review
description: Execute pre-merge verification audit: query live remote CI check suite, verify all tests, inspect git diff, and confirm zero unresolved review issues.
---

# /final-review Workflow — Pre-Merge Verification Audit

Follow this comprehensive audit before declaring any task complete or PR merge-ready.

## Checklist

### 1. Remote CI Verification
- [ ] Query GitHub Actions check runs API for the current `HEAD` commit SHA.
- [ ] Confirm 100% of required check runs have concluded with status `completed` and conclusion `success`:
  - Rust matrix (Ubuntu, Windows, macOS)
  - Clippy linter (`-D warnings`)
  - Formatting (`cargo fmt --check`)
  - Frontend test & build
  - SonarCloud Quality Gate (0 new issues)
  - CodeQL security analysis

### 2. Local Quality Gate Confirmation
- [ ] `cargo test --all` (100% passing)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` (Clean)
- [ ] `cargo fmt --all -- --check` (Clean)
- [ ] `npm run build` (Clean)

### 3. Review Thread Resolution
- [ ] All valid review comments resolved with verified code fixes.
- [ ] Out-of-scope/invalid comments documented with technical reasoning.

### 4. Git Diff Audit
- [ ] `git diff origin/main...HEAD` inspected. Zero unintended changes.
