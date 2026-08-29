# Quick Prevention Rules Hub

> **Operational Quick Checklists for High-Impact Tasks**  
> **Source:** Consolidated from Verified Engineering Lessons (ENG-001 to ENG-006)

---

## 1. Before Saying "Ready for Merge" (The Pre-Completion Gate)

- [ ] **Inspect Live Remote CI Matrix:** Query the exact `HEAD` commit SHA check suite on GitHub Actions. Verify 100% green status on all required check runs (Rust tests, Clippy, Format, Frontend, SonarCloud, CodeQL).
- [ ] **Run Full Local Test Matrix:**
  ```bash
  cargo test --all
  cargo clippy --all-targets --all-features -- -D warnings
  cargo fmt --all -- --check
  npm run build
  ```
- [ ] **Inspect Git Diff:** Run `git diff origin/main...HEAD` to verify zero unrelated modifications or unintended file touches.
- [ ] **Verify Review Threads:** Ensure all valid reviewer comments are resolved and invalid/out-of-scope comments have written technical justifications.

---

## 2. Before Implementing Any Review Comment (The Review Triage Gate)

- [ ] **Locate in Authoritative Spec:** Does the suggestion exist in the active milestone in `BACKLOG.md` or `SCHEMA.md`?
- [ ] **Check Phase Boundaries:** Does this touch a merged, protected phase (e.g. F1.*, F2.01)? If yes, reject unless it fixes a proven bug.
- [ ] **Check Future Roadmap:** Is this feature scheduled for a future milestone (e.g. F2.17)? If yes, mark OUT-OF-SCOPE and defer.
- [ ] **Verify Actual Code:** Does the alleged issue actually exist on the current `HEAD`? If already fixed or invalid, mark RESOLVED/INVALID.

---

## 3. Before Altering Database Schemas or Writing Tests (The Database Gate)

- [ ] **Migrations are Immutable:** Never edit an existing migration file. Create the next sequential migration `NNN_description.sql`.
- [ ] **Check Partial Unique Indexes:** Use `WHERE deleted_at IS NULL AND is_active = 1` for entities supporting soft delete.
- [ ] **Handle Foreign Keys in Test Fixtures:** Establish parent entities first before referencing them in child records or creating cyclic graph fixtures.
- [ ] **Map SQLite Constraint Errors:** Map SQLite error 2067 (unique violation) or 787 (foreign key violation) to domain enum errors.

---

## 4. Before Submitting URL / Input Parsing Logic (The Security/Complexity Gate)

- [ ] **Strict Port Range:** Enforce `1..=65535` for network ports; reject `:0`, `:65536`, `:abc`, and trailing colons `:`.
- [ ] **Cognitive Complexity:** Decompose multi-branch parsing logic into small private helper functions to stay under Cognitive Complexity 15.
