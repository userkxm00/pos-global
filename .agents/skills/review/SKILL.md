---
name: review
description: Triage automated reviewer findings (Codex, Gitar, CodeRabbit, SonarCloud, CodeQL) against authoritative project specifications and live code evidence.
---

# /review Workflow — Evidence-Based Review Triage

Follow this protocol whenever reviewing bot or human feedback.

## Step 1: Inspect Current PR & HEAD State
1. Check if the review thread is attached to the current `HEAD` commit or an outdated commit.
2. Check if the alleged defect actually exists in the current codebase.

## Step 2: Cross-Check Authoritative Specifications
1. Check `BACKLOG.md`: Does this requirement belong to the current phase or a future milestone?
2. Check `SCHEMA.md` and `DATABASE_RULES.md`: Does this violate schema immutability or architectural invariants?
3. Check `PHASE-*.md`: Does this touch protected files from a previously merged phase?

## Step 3: Classify Finding
- **VALID (Active Scope):** Real defect in current scope -> proceed to `/remediate`.
- **OUT-OF-SCOPE / DEFERRED:** Suggestion belongs to a future milestone (e.g. F2.17) -> Reject for current PR, document in review record.
- **INVALID / FALSE POSITIVE:** Finding is factually incorrect or contradicts project design -> Reject with technical explanation.
- **ALREADY RESOLVED:** Already addressed in a subsequent commit -> Mark resolved.

## Step 4: Record Review Outcome
Create or update a record in `.agents/memory/reviews/REV-[PHASE]-[REVIEWER].md` and update `.agents/memory/reviews/INDEX.md`.
