# Implementation Recovery — Phase 0/1 Boundary

This branch restores executable implementation that existed in the earlier implementation snapshot and reconciles it with the current Foundation branch.

## Recovered

- Tauri command modules for auth, inventory, licence, and sales
- Tauri command registration in `main.rs`
- Local SQLite `DbState` wiring
- Licence security boundary with no fake signature implementation
- Eight executable sales regression tests covering input validation, shift ownership/open state, overselling, stock decrement, stock ledger, payment/idempotency/outbox records, and minor-unit money columns

## Deliberate limitations

- Auth/login/PIN, inventory CRUD, and licence activation remain explicit stubs until their approved Phase tasks are executed.
- The sales implementation is a foundation recovery, not a claim that the full POS financial domain is complete. Taxes, discounts, COGS, debt, loyalty, refunds, returns, and provider-specific payment workflows remain governed by their contracts/backlog tasks.
- Tests have been added but have not been claimed as CI-verified while GitHub Actions quota is exhausted.

## Recovery rule

The recovered code must be reviewed against the current Foundation contracts before further expansion. Do not replace working implementation with documentation-only scaffolding, and do not skip ahead to later phases.
