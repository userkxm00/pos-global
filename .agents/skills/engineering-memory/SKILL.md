---
name: engineering-memory
description: Comprehensive workflow for managing, retrieving, and updating the persistent engineering memory system. Use to query past lessons, triage reviews, and record new verified knowledge.
---

# Engineering Memory Management Skill

This skill governs the persistent external engineering knowledge system.

## Available Slash Workflows

1. **`/learn`**: Capture a newly discovered, verified engineering lesson at the end of a task or post-incident.
2. **`/review`**: Triage automated or human review comments against authoritative project specifications (`BACKLOG.md`, `SCHEMA.md`) and live code evidence.
3. **`/remediate`**: Apply minimal, focused fixes with zero scope drift and dedicated regression tests.
4. **`/final-review`**: Conduct a pre-merge verification audit (live CI check suite, full test matrix, diff review, and quality gates).

---

## 1. Start-of-Task Retrieval Procedure
When starting any feature, bugfix, or migration:
1. Load `BACKLOG.md` and check current milestone boundaries.
2. Load `.agents/rules/global_engineering_rules.md` and `.agents/rules/project_engineering_rules.md`.
3. Query `.agents/memory/lessons/INDEX.md` for active lessons matching the task category.
4. Verify protected files from prior phases in `.agents/memory/phases/INDEX.md`.

---

## 2. End-of-Task Knowledge Recording Procedure
1. Verify if an unexpected defect, non-obvious failure, or valuable review triage occurred.
2. Structure the lesson using the mandatory standard format (`ENG-NNN`).
3. Classify the scope as `GLOBAL` or `PROJECT`.
4. Append to `.agents/memory/lessons/ENG-NNN.md` and update `INDEX.md`.
5. Update `.agents/memory/prevention_rules.md` if a recurring risk pattern is identified.
