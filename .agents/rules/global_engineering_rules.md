# Global Engineering Rules

> **Scope:** Universal Engineering Invariants (Applies across all repositories & technologies)  
> **Status:** ACTIVE  
> **Source:** Validated cross-project engineering practices

---

## 1. Verification & Completion Invariants

### 1.1 Evidence-First Verification
- **Rule:** Never claim a PR or task is "ready for merge" without inspecting live remote CI execution.
- **Evidence Requirement:** Local compilation or test passes do NOT guarantee green GitHub Actions CI. The exact remote `HEAD` commit SHA must have all required check runs in `success` state.
- **Anti-Pattern:** Trusting a local test run or an LLM summary without querying the GitHub Checks API / CI run status.

### 1.2 Authoritative Specification Priority
- **Rule:** Automated reviewers (Gitar, CodeRabbit, Codex, SonarCloud, CodeQL) and AI tools are evidence sources, not absolute authorities.
- **Procedure:**
  1. Compare the reviewer finding against authoritative project specifications (`BACKLOG.md`, `SCHEMA.md`, `ARCHITECTURE.md`).
  2. Verify whether the suggestion belongs to the current scope or a planned future phase.
  3. Verify whether the change violates existing invariants or merged code.
  4. Fix only if factually valid within the current scope. If invalid or out-of-scope, reject with clear technical rationale.

### 1.3 Scope Discipline & Anti-Scope Drift
- **Rule:** Never expand the scope of an active task to satisfy out-of-scope reviewer feedback.
- **Anti-Pattern:** Adding foreign keys, new entities, or extra database columns that are scheduled for future milestones just because an automated review bot suggested them.
- **Protection:** Merged code from prior phases is strictly protected. Do NOT edit merged modules unless addressing an explicit, verified regression.

### 1.4 Database & Migration Immutability
- **Rule:** Applied and merged migrations are strictly immutable.
- **Procedure:**
  - Never edit an existing applied migration file.
  - All schema evolutions must be performed via new, sequentially numbered forward migrations.
  - Always verify both fresh database installation and sequential upgrade paths.

### 1.5 Mandatory Regression Testing
- **Rule:** Every fixed defect, edge case, and review remediation must include a dedicated automated unit or integration regression test.
- **Requirement:** The test must explicitly reproduce the exact failure condition and prove that the fix prevents future recurrence.
- **Anti-Pattern:** Deleting or weakening existing tests to make CI pass.

### 1.6 Pre-Commit & Pre-Push Diff Inspection
- **Rule:** Always inspect the complete `git diff` before staging and pushing commits.
- **Checklist:**
  - Zero unrelated file modifications.
  - Zero accidental formatting changes across unaffected files.
  - Zero leftover temporary debug scripts or sensitive tokens.

### 1.7 Fail-Closed Security & Input Validation
- **Rule:** Validation, authorization, and data integrity must fail-closed.
- **Requirement:** Invalid inputs (e.g., malformed URLs, port numbers > 65535, invalid foreign keys) must be rejected immediately at the boundary with structured errors.

### 1.8 Cognitive Complexity & Code Hygiene
- **Rule:** Keep functions focused with low cognitive complexity (SonarCloud <= 15).
- **Technique:** Decompose complex multi-branch validation/parsing logic into small, single-responsibility private helper functions.
