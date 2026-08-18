# DEFINITION OF READY

A task may enter implementation only when all applicable conditions below are true.

## Required

- [ ] Task has a unique ID.
- [ ] Objective is unambiguous.
- [ ] Dependencies are identified and complete.
- [ ] Affected domain is identified.
- [ ] Acceptance criteria are observable.
- [ ] Security impact is considered.
- [ ] Data/schema impact is considered.
- [ ] Existing implementation has been inspected.
- [ ] Required tests are named.
- [ ] Rollback/recovery is understood.
- [ ] No unresolved architectural decision blocks implementation.

## Mandatory stop conditions

The agent must mark the task `BLOCKED` rather than guessing if:

- financial behavior is ambiguous;
- authorization boundaries are ambiguous;
- a migration would require rewriting already-applied history;
- sync conflict semantics are undefined;
- a private key or secret is required but no approved secret-management path exists;
- the requested change contradicts an approved ADR/architecture rule.

## Exception policy

Minor implementation details may be chosen by the agent when they do not alter contracts, security, financial truth, schema semantics, or public behavior. The choice must be documented in the task handoff.
