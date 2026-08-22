# ADR SYSTEM

## Purpose
Architectural decisions must be explicit, reviewable, and durable across agents.

## When an ADR is required

Create an ADR before implementation when changing:

- technology or core framework;
- database ownership/model;
- authentication/authorization boundary;
- financial arithmetic/model;
- inventory/cash/debt ledger semantics;
- sync/conflict semantics;
- licensing/security model;
- update-signing/release model;
- public domain/API contracts;
- dependency with material architectural/security impact.

## ADR format

```markdown
# ADR-NNNN — Title

Status: Proposed | Accepted | Superseded | Rejected
Date:

## Context

## Decision

## Alternatives considered

## Consequences

## Security impact

## Data/migration impact

## Testing/evidence

## Rollback/reversal
```

## Agent rule

An agent may propose an ADR, but must not silently treat a material architectural choice as accepted. Accepted decisions must be referenced by affected implementation tasks.
