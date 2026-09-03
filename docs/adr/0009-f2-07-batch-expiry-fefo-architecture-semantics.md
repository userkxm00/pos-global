# ADR-0009 — F2.07 Batches, Expiry Dates & FEFO Architecture & Semantics

Status: Accepted
Date: 2026-09-03

## Context

Phase 2 milestone `F2.07 — Batches / expiry / FEFO` establishes the backend domain model, database migration, validation invariants, and Tauri IPC interfaces for structured lot tracking, expiration date validation, and First-Expire, First-Out (FEFO) allocation planning in POS Global.

During the pre-implementation audit and authoritative source reconciliation, key architectural tensions around stock ledger boundaries, capability orthogonality, SQLite table rebuild semantics, and quantity precision were analyzed and resolved. This ADR records the approved architectural decisions governing F2.07 execution.

---

## Authoritative Existing Facts vs Architectural Decisions Made Now

### Authoritative Existing Facts

1. **Orthogonal Capabilities Seeded**:
   - `003_global_commerce_foundation.sql:122-124` seeded three independent capabilities in the `capabilities` table:
     - `('BATCH', 'Batch/Lot', 'product', 'Batch tracking')`
     - `('EXPIRY', 'Expiry', 'product', 'Expiry tracking')`
     - `('FEFO', 'FEFO', 'product', 'First-expire-first-out')`
   - `CAPABILITY_MATRIX.md:12-13` establishes `Batch/lot` and `Expiry/FEFO` as distinct capabilities across retail presets (e.g. Fashion, Hardware, and Furniture track lots without expiry).
2. **Product Core Expiry Flag**:
   - `001_initial.sql:61` and `src-tauri/src/product/mod.rs:44,63` established `products.requires_expiry INTEGER NOT NULL DEFAULT 0`.
3. **Legacy Initial Batch Schema**:
   - `001_initial.sql:106-115` defined `product_batches` as an initial stub containing `id`, `product_id`, `branch_id`, `batch_number`, `quantity REAL`, `expiry_date TEXT NOT NULL`, `received_at TEXT`, and index `idx_batches_expiry`.
   - No other table in the database contains foreign keys referencing `product_batches(id)`.
   - Zero existing code in `src-tauri/` or `src/` references `product_batches`.
4. **Quantity Precision Standard**:
   - `006_quantity_precision_hardening.sql` and `DATABASE_RULES.md:30-32` mandate exact integer milli-units (`quantity_milli INTEGER`) as the sole authoritative representation for physical quantities.
5. **Stock Movement Ledger Invariant**:
   - `DOMAIN_CONTRACTS.md:15` states: *"Every quantity change creates a stock movement in the same atomic operation."*
   - `EXECUTION_PLAN_DETAILED.md:39` places `movement ledger (F2.11)` sequentially after `batch/expiry (F2.07)`.

---

### Approved Architectural Decisions

#### Decision 1 — Read-Only FEFO Allocation Planning (D.1 Option 1A)
- F2.07 provides deterministic, read-only FEFO allocation planning via `plan_fefo_allocation`.
- F2.07 does **NOT** decrement inventory quantities and does **NOT** insert rows into `stock_movements`.
- Actual stock deduction and double-entry movement ledger posting are strictly deferred to **F2.11 (Stock Movement Ledger)** and **Phase 3 (Sales Checkout)**.
- This ensures full compliance with `DOMAIN_CONTRACTS.md:15` and `EXECUTION_PLAN_DETAILED.md:41` (all mutations must be ledger-backed).

#### Decision 2 — Capability Orthogonality & Enablement Rules (D.2 Option 2A)
The domain explicitly separates three orthogonal capability rules:
1. **Batch Tracking Eligibility (`is_batch_tracked`):**
   $$\text{is\_batch\_tracked}(P) \iff \text{has\_capability}(P, \text{'BATCH'}) \lor \text{products.requires\_expiry} = 1 \lor \text{has\_capability}(P, \text{'EXPIRY'}) \lor \text{has\_capability}(P, \text{'FEFO'})$$
   A product can only create and hold batch records if it satisfies this rule.
2. **Expiry Requirement (`is_expiry_required`):**
   $$\text{is\_expiry\_required}(P) \iff \text{products.requires\_expiry} = 1 \lor \text{has\_capability}(P, \text{'EXPIRY'}) \lor \text{has\_capability}(P, \text{'FEFO'})$$
   If true, `expiry_date` is strictly mandatory (`YYYY-MM-DD`). If false (pure `BATCH` tracking), `expiry_date` is optional and may be `NULL`.
3. **FEFO Enablement (`is_fefo_enabled`):**
   $$\text{is\_fefo\_enabled}(P) \iff \text{has\_capability}(P, \text{'FEFO'})$$
   FEFO allocation calculation is enabled **strictly** by the active `'FEFO'` capability in `product_capabilities`. `requires_expiry = 1` alone denotes perishability, not automated FEFO allocation.

#### Decision 3 — Nullable `expiry_date` & Table Rebuild in Migration 016
- Migration `016_batches_and_expiry.sql` rebuilds `product_batches` so that `expiry_date TEXT NULL` is nullable.
- This allows non-perishable lots (e.g. textile dye lots, ceramic tile batches, paint mixes) to exist without fabricating fake/dummy expiration dates.
- FEFO allocation planning operates strictly on batches where `expiry_date IS NOT NULL`. Non-perishable lots (`expiry_date IS NULL`) are excluded from FEFO.
- The rebuild pattern safely preserves existing IDs, product links, branch links, batch numbers, converted quantities, and timestamps.

#### Decision 4 — Sole Quantity Source of Truth (`quantity_milli` Only)
- The legacy column `quantity REAL` is **removed** during the Migration 016 rebuild.
- `quantity_milli INTEGER NOT NULL DEFAULT 0` is the **sole authoritative source of truth** for batch quantity balances.
- Migration 016 includes a fail-closed pre-validation check via `RAISE(ABORT, ...)` that halts and rolls back the migration if any existing row has negative quantities or precision beyond 3 decimal places.
- Valid legacy quantities are converted exactly: `quantity_milli = CAST(ROUND(quantity * 1000.0) AS INTEGER)`.

#### Decision 5 — Derived Expiration Model (No Persisted 'expired' Status)
- Persisted `status` values are strictly restricted by a database CHECK constraint:
  `CHECK (status IN ('active', 'quarantined', 'recalled', 'depleted'))`
- "Expired" is **dynamically derived** at query time:
  $$\text{expired} \iff \text{expiry\_date} < \text{strftime}('\%Y\text{-\%m-\%d}', '\text{now}')$$
- `plan_fefo_allocation` queries `WHERE status = 'active' AND expiry_date >= strftime('%Y-%m-%d', 'now') AND quantity_milli > 0`.
- Expired lots are dynamically excluded without requiring daily asynchronous cron jobs to mutate row status.

#### Decision 6 — Status Lifecycle State Machine & Terminal Depleted State
- Lifecycle state transitions are strictly governed:
  - `active` $\to$ `quarantined`, `recalled`, `depleted`
  - `quarantined` $\to$ `active`, `recalled`
  - `recalled` $\to$ *(terminal)*
  - `depleted` $\to$ *(terminal)*
- In F2.07, `'depleted'` is **strictly terminal**. Reopening a depleted lot via `update_batch_status` is rejected fail-closed to preserve lot auditability.

#### Decision 7 — Batch / Variant Relationship & Partial Uniqueness
- `product_batches` includes `variant_id TEXT REFERENCES product_variants(id)` (nullable).
- If `variant_id` is supplied, the domain strictly validates that `variant.product_id == batch.product_id` and `variant.deleted_at IS NULL`.
- Uniqueness is enforced per `(branch_id, product_id, variant_id, batch_number COLLATE NOCASE)` using two partial unique indexes to properly handle SQLite `NULL` semantics for non-variant products:
  - `idx_product_batches_unique_prod ... WHERE variant_id IS NULL`
  - `idx_product_batches_unique_var ... WHERE variant_id IS NOT NULL`

#### Decision 8 — Supplier Boundary Isolation
- Supplier association is **NOT** included in `product_batches` in F2.07.
- Goods receiving, purchase orders, and supplier lot linkages are strictly assigned to **Phase 4 (Purchasing & Inventory Costing — F4.02)**.

---

## Technical Specifications

### 1. Database Schema (`016_batches_and_expiry.sql`)

```sql
-- 016_batches_and_expiry.sql
-- F2.07 — Batches, Expiry Dates & FEFO Schema Hardening

-- 1. Pre-validation assertion: fail closed on invalid historical legacy quantities
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM product_batches
        WHERE quantity < 0
           OR quantity IS NULL
           OR CAST(ROUND(quantity * 1000.0) AS INTEGER) / 1000.0 != quantity
    ) THEN RAISE(ABORT, 'Migration 016 aborted: legacy product_batches contains negative, NULL, or non-exact fractional quantities')
END;

-- 2. Rebuild product_batches with exact columns, nullable expiry_date, and integer milli precision
CREATE TABLE product_batches_new (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL REFERENCES products(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    variant_id TEXT REFERENCES product_variants(id),
    batch_number TEXT NOT NULL,
    quantity_milli INTEGER NOT NULL DEFAULT 0 CHECK (quantity_milli >= 0),
    cost_price_minor INTEGER CHECK (cost_price_minor IS NULL OR cost_price_minor >= 0),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'quarantined', 'recalled', 'depleted')),
    manufactured_date TEXT,
    expiry_date TEXT,
    received_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT ''
);

-- 3. Copy legacy data with exact integer conversion (dropping legacy quantity REAL)
INSERT INTO product_batches_new (
    id, product_id, branch_id, batch_number, quantity_milli, expiry_date, received_at, created_at, updated_at
)
SELECT
    id, product_id, branch_id, batch_number,
    CAST(ROUND(quantity * 1000.0) AS INTEGER),
    expiry_date, received_at, received_at, received_at
FROM product_batches;

-- 4. Swap tables
DROP TABLE product_batches;
ALTER TABLE product_batches_new RENAME TO product_batches;

-- 5. Indexes
CREATE INDEX idx_batches_expiry ON product_batches(expiry_date);
CREATE INDEX idx_product_batches_fefo ON product_batches(branch_id, product_id, status, expiry_date);
CREATE INDEX idx_product_batches_variant_fefo ON product_batches(branch_id, product_id, variant_id, status, expiry_date);

CREATE UNIQUE INDEX idx_product_batches_unique_prod
    ON product_batches(branch_id, product_id, batch_number COLLATE NOCASE)
    WHERE variant_id IS NULL;

CREATE UNIQUE INDEX idx_product_batches_unique_var
    ON product_batches(branch_id, product_id, variant_id, batch_number COLLATE NOCASE)
    WHERE variant_id IS NOT NULL;
```

### 2. FEFO Allocation Planning Algorithm

For a requested quantity `R` of product `P` (with optional variant `V`) in branch `B`:

1. **Verification:** Validate that `is_fefo_enabled(P) == true`.
2. **Candidate Selection Query:**
   ```sql
   SELECT id, batch_number, expiry_date, quantity_milli
   FROM product_batches
   WHERE branch_id = ?1
     AND product_id = ?2
     AND (variant_id = ?3 OR (?3 IS NULL AND variant_id IS NULL))
     AND status = 'active'
     AND quantity_milli > 0
     AND expiry_date IS NOT NULL
     AND expiry_date >= strftime('%Y-%m-%d', 'now')
   ORDER BY expiry_date ASC, received_at ASC, id ASC;
   ```
3. **Allocation Computation:**
   - Iterate candidate lots in deterministic order.
   - For each lot: $\text{take} = \min(\text{remaining\_demand}, \text{batch.quantity\_milli})$.
   - Record allocation line: `batch_id`, `batch_number`, `expiry_date`, `allocated_quantity_milli`.
   - $\text{remaining\_demand} \leftarrow \text{remaining\_demand} - \text{take}$.
   - Stop when $\text{remaining\_demand} = 0$ or candidate lots are exhausted.
4. **Result:** Return `FefoAllocationPlan` containing requested quantity, total allocated quantity, shortfall quantity, and line breakdowns. The database remains completely unmutated.

---

## Scope Firewalls

The following boundaries are strictly enforced:
- **F2.08 Serial / IMEI / Assets:** `serial_numbers` table remains untouched.
- **F2.09 Warranty:** Warranty registration and policies remain untouched.
- **F2.10 Locations / Bins:** Storage bins and aisle locations remain untouched.
- **F2.11 Stock Movement Ledger:** Zero rows inserted into `stock_movements`.
- **F2.12 Transfers:** Zero transfer orders or receiving workflows.
- **F2.23 Frontend UI:** React batch/expiry UI components deferred.
- **Phase 3 Sales Integration:** Checkout cart line deductions deferred.
- **Phase 4 Purchasing:** Supplier association and purchase order linkage deferred.
