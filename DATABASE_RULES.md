# DATABASE RULES

## 1. General

SQLite is the offline operational database. PostgreSQL/Supabase is the cloud data store. Domain semantics must remain provider-neutral where practical.

## 2. Migrations

- Applied migrations are immutable.
- New changes require a new migration.
- Every migration must be deterministic and idempotent at the migration-runner level.
- Destructive changes require an explicit migration plan, backup consideration, and compatibility window.
- Test migrations on an empty database and on a representative upgraded database.

## 3. IDs and timestamps

- Use stable opaque IDs for domain entities.
- Never use display names as identifiers.
- Record creation/update timestamps explicitly.
- Financial/ledger records must preserve original event time and processing time where needed.

## 4. Tenant isolation

Every organization-owned record must have an explicit organization/tenant boundary or a provable parent boundary. Cloud tables must have RLS policies that enforce this boundary.

## 5. Money

Authoritative money is integer minor units or an approved exact-decimal type plus ISO currency. Never use FLOAT/REAL for authoritative monetary fields.

## 6. Quantities

Quantity precision must be explicit. Weighted/decimal quantities use a documented exact representation. Unit conversion must be deterministic and tested.

## 7. Ledger policy

Stock, cash, debt, loyalty and other balances that require auditability must have an append-only movement/ledger model. A balance is a derived/current state, not the only historical truth.

## 8. Transactions

A business operation must update all required records atomically. Example sale: sale header + lines + payments + stock movements + cash/debt effects + outbox event.

## 9. JSON/custom attributes

JSON is allowed for genuinely extensible metadata. It must not replace structured columns for money, stock, tax, identity, permissions, serials, batches, or other core invariants.

## 10. Indexing

Add indexes for primary lookup paths, organization/branch scoping, foreign keys, unique business identifiers, synchronization queues, and date-based reporting. Every non-trivial index should have a query/use-case justification.

## 11. Deletion

Do not hard-delete financial history. Prefer status/void/archive/compensating records. Hard deletion is allowed only for data explicitly classified as disposable and after security/privacy review.

## 12. Concurrency

Use database transactions and constraints rather than application-only assumptions. Inventory/cash operations must be safe under retry and concurrent terminals.

## 13. Cloud RLS

Every exposed cloud table must have deliberate RLS policies. “Table exists” is not evidence of secure access.
