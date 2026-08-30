# POS Global — Agent Engineering System & Operating Rules

> **Authoritative Agent Operating Guide**  
> **Workspace:** `pos-global` (Tauri v2 + Rust + SQLite + React)  
> **Memory & Rules Root:** `.agents/`

---

## 1. Core Operating Protocols

### 1.1 Start-of-Task Protocol (Mandatory)
Before planning or executing any task (feature implementation, bugfix, PR review, or remediation):
1. **Load Authoritative Specifications:**
   - Read `BACKLOG.md` for current phase scope and explicit milestone boundaries.
   - Read `SCHEMA.md` and `DATABASE_RULES.md` for database schemas and invariants.
   - Read `ARCHITECTURE.md` and `SECURITY_MODEL.md` for architectural boundaries.
2. **Load Global & Project Engineering Rules:**
   - Check [.agents/rules/global_engineering_rules.md](.agents/rules/global_engineering_rules.md).
   - Check [.agents/rules/project_engineering_rules.md](.agents/rules/project_engineering_rules.md).
   - Check [.agents/rules/database_rules.md](.agents/rules/database_rules.md).
   - Check [.agents/rules/quality_gates.md](.agents/rules/quality_gates.md).
3. **Retrieve Historical Lessons:**
   - Query [.agents/memory/lessons/INDEX.md](.agents/memory/lessons/INDEX.md) for active lessons relevant to the task category (CI, Database, Complexity, Scope, Auth, etc.).
   - Check [.agents/memory/prevention_rules.md](.agents/memory/prevention_rules.md) for quick-check checklists.
4. **Enforce Scope & Protected Boundaries:**
   - Strictly verify which files are protected (e.g., previous merged phases like `F1.*` and `F2.01`).
   - Prohibit out-of-scope edits or premature feature introduction.

### 1.2 Implementation & Verification Protocol
1. **Minimal, Focused Changes:** Implement only what is specified for the active task.
2. **Mandatory Regression Tests:** Write automated tests reproducing fixed issues or validating new edge cases.
3. **Local Quality Gates:**
   - `cargo test --all` (All unit and integration tests must pass)
   - `cargo clippy --all-targets --all-features -- -D warnings` (Zero warnings)
   - `cargo fmt --check` (Strict formatting)
   - `npm run test` / `npm run build` (Frontend integrity)
4. **Live CI Verification:**
   - Never declare completion until the exact remote `HEAD` commit SHA has passed all required GitHub Actions CI checks (Rust, Frontend, SonarCloud, CodeQL).

### 1.3 End-of-Task Learning Protocol (/learn)
At the conclusion of any task where unexpected failures, non-obvious root causes, or reviewer findings occurred:
1. **Analyze:** What was the problem? Root cause? Why was it missed? How was it fixed and tested?
2. **Classify:** Determine if the lesson is `GLOBAL` (applies to all codebases) or `PROJECT-SPECIFIC` (applies only to `pos-global`).
3. **Record:** Append a structured entry to `.agents/memory/lessons/` and update `INDEX.md`.
4. **Update Prevention Rules:** If the issue represents a recurring high-risk pattern, update `.agents/memory/prevention_rules.md`.

---

## 2. Interactive Agent Skills & Workflows

The following skills are available to invoke during development workflows:

| Skill / Command | Purpose | Location |
| :--- | :--- | :--- |
| **`/learn`** | Capture a new evidence-based engineering lesson | [.agents/skills/learn/SKILL.md](.agents/skills/learn/SKILL.md) |
| **`/review`** | Triage automated reviewer comments against authoritative spec | [.agents/skills/review/SKILL.md](.agents/skills/review/SKILL.md) |
| **`/remediate`** | Execute minimal fixes for valid review findings with zero drift | [.agents/skills/remediate/SKILL.md](.agents/skills/remediate/SKILL.md) |
| **`/final-review`** | Perform pre-merge verification audit (CI, tests, diff, gates) | [.agents/skills/final-review/SKILL.md](.agents/skills/final-review/SKILL.md) |
| **`engineering-memory`** | Complete engineering memory lifecycle management | [.agents/skills/engineering-memory/SKILL.md](.agents/skills/engineering-memory/SKILL.md) |

---

## 3. Directory Map of Engineering Knowledge

```
.agents/
├── rules/
│   ├── global_engineering_rules.md   # Universal engineering principles
│   ├── project_engineering_rules.md  # POS Global domain & architecture rules
│   ├── database_rules.md             # Migration & SQLite rules
│   └── quality_gates.md              # Rust, Clippy, Sonar, Test standards
├── memory/
│   ├── prevention_rules.md           # Quick prevention rules & checklists
│   ├── lessons/                      # Persistent structured lessons (ENG-*.md)
│   │   ├── INDEX.md                  # Master catalog & search index
│   │   ├── ENG-001.md ... ENG-006.md # Seeded verified lessons
│   ├── reviews/                      # Review triage records (Gitar, Codex, Sonar)
│   │   ├── INDEX.md                  # Review log index
│   │   ├── REV-F201-GITAR.md
│   │   ├── REV-F202-CODEX.md
│   │   └── REV-F202-SONAR.md
│   └── phases/                       # Phase summaries & memory records
│       ├── INDEX.md                  # Phase timeline index
│       ├── PHASE-F1.md
│       ├── PHASE-F201.md
│       └── PHASE-F202.md
└── skills/                           # Executable workflow skills
    ├── engineering-memory/SKILL.md
    ├── learn/SKILL.md
    ├── review/SKILL.md
    ├── remediate/SKILL.md
    └── final-review/SKILL.md
```
