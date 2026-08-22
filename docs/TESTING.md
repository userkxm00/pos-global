# Testing Policy

## Required layers
1. SQL migration tests on a clean SQLite database.
2. Rust unit tests for domain invariants.
3. Rust integration tests for transactions and rollback.
4. TypeScript typecheck and frontend build.
5. RLS tests against non-production Supabase.
6. Sync retry/idempotency tests.
7. License signature/tamper tests.
8. Updater verification and safe-install tests.
9. E2E flows for sale, refund, purchase and cash close.

## Core transaction cases
- missing/closed shift rejected
- insufficient stock rejected
- multi-line sale commits atomically
- payment total reconciles with sale total
- rollback leaves no partial sale
- retry with same idempotency key returns the original result
- outbox event is created in the same transaction
- refund cannot exceed refundable quantity
- unauthorized users cannot perform privileged actions

Green CI is necessary but not sufficient. Each phase needs documented invariants and acceptance evidence.