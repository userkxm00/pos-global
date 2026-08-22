# Sales Hardening Verification

This change addresses two reviewed findings in the seed sales path:

- Quantity source-of-truth is now integer thousandths (`*_milli`) for inventory, sale items, and stock movements. Legacy REAL columns remain as derived compatibility projections.
- Sales reports require an explicit `branch_id` and aggregate only rows for that branch.

The migration is additive and does not edit previously applied migrations.

Verification expectations:

- All database migrations apply to a fresh database.
- Integer quantity columns are declared `INTEGER`.
- Overselling cannot decrement stock.
- Successful sales decrement `quantity_milli` atomically.
- Stock movements record integer before/after/delta quantities.
- Idempotent retries produce no duplicate side effects.
- Sales reporting excludes other branches.
