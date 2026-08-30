# POS Global — Gemini Operating Rules & Engineering Memory

> **Authoritative Gemini/Antigravity Rule Hub**  
> **Workspace:** `pos-global` (Tauri v2 + Rust + SQLite + React)  
> **Memory & Rules Root:** `.agents/`

---

## 1. Operating Rules & Protocols

All Agent actions in this workspace are governed by the Engineering Memory System and strict quality gates:

1. **Start-of-Task Retrieval:**
   - Always load authoritative requirements from `BACKLOG.md`, `SCHEMA.md`, and `ARCHITECTURE.md`.
   - Inspect [.agents/rules/global_engineering_rules.md](.agents/rules/global_engineering_rules.md).
   - Inspect [.agents/rules/project_engineering_rules.md](.agents/rules/project_engineering_rules.md).
   - Check relevant historical lessons in [.agents/memory/lessons/INDEX.md](.agents/memory/lessons/INDEX.md).

2. **Automated Review Triage:**
   - Treat automated reviewers (Codex, Gitar, CodeRabbit, SonarCloud) as evidence inputs, never unquestioned authorities.
   - Verify findings against authoritative project documents before implementing fixes.
   - Reject out-of-scope suggestions to prevent scope drift and protect merged phases.

3. **Database Immutability:**
   - Never modify merged migrations.
   - Always apply schema changes via new sequentially numbered migrations.
   - Always verify SQLite foreign keys, cyclic references, and soft deletion indices.

4. **Continuous Learning:**
   - Use `/learn` at the end of critical tasks to store durable engineering lessons in `.agents/memory/lessons/`.
   - Maintain `.agents/memory/reviews/` and `.agents/memory/phases/` records.

---

## 2. Directory Reference

- **Rules:** [.agents/rules/](.agents/rules/)
- **Lessons:** [.agents/memory/lessons/](.agents/memory/lessons/)
- **Reviews:** [.agents/memory/reviews/](.agents/memory/reviews/)
- **Phases:** [.agents/memory/phases/](.agents/memory/phases/)
- **Workflows:** [.agents/skills/](.agents/skills/)
