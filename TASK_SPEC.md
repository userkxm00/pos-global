# TASK SPECIFICATION CONTRACT

Every implementation task must be representable with this structure.

## Identity

- Task ID:
- Phase:
- Epic:
- Feature:
- Priority:
- Status:
- Owner/Agent:

## Objective

One precise sentence describing the outcome.

## Context

Why the task exists and which existing behavior it depends on.

## Dependencies

List required prior tasks, migrations, APIs, providers, or decisions.

## Files

- Allowed to create/change:
- Read-only/reference:
- Forbidden to change:

## Contracts

- Domain contract:
- Database contract:
- API/Tauri contract:
- UI contract:
- Security contract:

## Business rules

List every invariant and edge case. Do not leave critical rules implicit.

## Data impact

Tables, columns, indexes, migrations, ledger/outbox effects, and rollback behavior.

## Acceptance criteria

Use observable statements beginning with “Given / When / Then” where practical.

## Tests required

- unit
- integration
- migration
- authorization/security
- offline/sync
- E2E/golden flow where applicable

## Evidence

Commands, CI runs, screenshots/logs/artifacts, and exact results.

## Failure/recovery

Expected failures and safe recovery behavior.

## Rollback

How the implementation can be reverted without corrupting data.

## Definition of Done

All acceptance criteria pass, required tests pass, docs are updated, no secrets are present, and evidence is recorded.
