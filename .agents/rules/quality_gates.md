# POS Global — Quality Gates & Verification Standards

> **Scope:** Code Quality, Static Analysis, and CI Gates  
> **Status:** ACTIVE

---

## 1. Rust Quality Gates
- **Tests:** `cargo test --all` must pass 100% (zero failures, zero ignored critical tests).
- **Clippy:** `cargo clippy --all-targets --all-features -- -D warnings` must produce zero warnings.
- **Formatting:** `cargo fmt --all -- --check` must produce zero formatting diffs.
- **Cognitive Complexity:** Functions must keep cognitive complexity <= 15 (SonarCloud standard). Refactor complex parsing/validation into single-responsibility private helper functions.

---

## 2. Frontend Quality Gates
- **Type Checking:** `npm run build` / `tsc --noEmit` must produce zero TypeScript errors.
- **Unit/Integration Tests:** `npm run test` (Vitest/Jest) must pass completely.
- **ESLint:** Zero lint errors or unhandled warnings.

---

## 3. Remote CI Quality Gates
- **Required Check Matrix:**
  - Rust build & test matrix (Ubuntu/Windows/macOS)
  - Clippy linting
  - Cargo formatting
  - Frontend test & build
  - SonarCloud Quality Gate (0 new bugs, 0 new vulnerabilities, 0 new security hotspots, 0 new code smells)
  - CodeQL security analysis
- **Rule:** Never mark a task or PR complete until the exact commit SHA has passed all remote CI checks.
