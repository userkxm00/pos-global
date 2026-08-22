# REGULATORY HALT POINTS — Zylo

## Purpose

Research files are baselines, not legal certification. The agent must not turn a research statement into a jurisdiction-specific compliance rule without authoritative evidence and explicit project approval.

## Hard-stop rule

A task is `BLOCKED` when it requires implementing or asserting jurisdiction-specific tax, invoicing, reporting, e-invoicing, fiscalization, retention, privacy, payment, or other regulated behavior and the required authoritative decision package is missing.

## Required before implementation

1. Identify the exact jurisdiction and effective date.
2. Use authoritative primary sources where available (government, official regulator, official EU/legal publication, or an explicitly approved legal/compliance source).
3. Record the source, publication/version/date, and the exact rule being implemented.
4. Record an explicit project approval/sign-off for translating the source into a product rule.
5. Define the jurisdiction adapter or configuration boundary; do not hard-code a country rule into shared core logic.
6. Add executable tests for the rule and its effective-date/boundary behavior.

## Agent behavior

- Never infer a legal requirement from another country's implementation.
- Never treat ERPNext/Frappe localization repositories or other external projects as legal authority.
- Never claim "compliant" based only on tests or a research note.
- If evidence is missing or contradictory, stop with `BLOCKED` or `DECISION REQUIRED`.

## Release implication

Production/launch readiness for a jurisdiction requires the approved evidence package and applicable release review. Development of a generic provider-neutral adapter may proceed when it does not assert unapproved jurisdiction-specific rules.
