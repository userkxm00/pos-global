---
name: learn
description: Capture, verify, and store a durable engineering lesson in the persistent memory system. Trigger after bug fixes, CI remediations, review triages, or non-obvious technical discoveries.
---

# /learn Workflow — Engineering Knowledge Capture

Follow this structured procedure to record a verified engineering lesson.

## Step 1: Verification & Root Cause Analysis
Before writing anything:
1. Verify the problem against actual code and test executions.
2. Formulate the precise root cause (not just surface symptoms).
3. Identify why standard checks missed it initially.
4. Confirm that a regression test was added and is passing.

## Step 2: Scope Classification
- **GLOBAL:** Broadly applicable across languages/frameworks (e.g. CI verification, port bounds, cognitive complexity decomposition).
- **PROJECT:** Specific to `pos-global` (e.g. SQLite partial unique index for soft-deleted categories).

## Step 3: Format & Store Lesson
Assign the next sequential ID (`ENG-NNN`) and create `.agents/memory/lessons/ENG-NNN.md`:

```markdown
# LESSON: ENG-NNN — [Title]

---
- **LESSON ID:** ENG-NNN
- **DATE:** YYYY-MM-DD
- **PROJECT:** pos-global
- **PHASE:** [Active Phase]
- **CATEGORY:** [CI / Database / Security / Complexity / Scope]
- **SEVERITY:** [CRITICAL / HIGH / MEDIUM / LOW]
- **SCOPE:** [GLOBAL / PROJECT]
- **STATUS:** ACTIVE
---

## 1. Problem
[Exact description of what went wrong]

## 2. Root Cause
[Technical root cause]

## 3. Why Checks Missed It
[Why earlier checks or tests did not catch it]

## 4. Evidence
[CI error, compiler output, or test failure logs]

## 5. Correct Fix
[Minimal, clean fix applied]

## 6. Regression Test
[Specific test verifying the fix]

## 7. Prevention Rule
[Concrete rule to prevent future recurrence]
```

## Step 4: Update Index & Prevention Rules
1. Update `.agents/memory/lessons/INDEX.md` with the new entry.
2. If high-impact or recurring, add a checklist item to `.agents/memory/prevention_rules.md`.
