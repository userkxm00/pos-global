# ADR-0013 — F2.11 Stock Ledger & Spatial Balances Architecture & Semantics

Status: Accepted — Authoritative
Date: 2026-09-09

---

## 1. Context

Milestone `F2.11 — Stock Ledger & Spatial Balances` establishes the foundational inventory accounting engine for POS Global. Prior to this milestone:
- Milestone `F2.01` through `F2.09` established products, categories, units, variants, weighted items, batches, serialized assets, and warranties.
- Milestone `F2.10` established the macroscopic and microscopic physical topography via `locations` (zones/areas) and `bins` (terminal pick/put storage slots) scoped to branches (`019_locations_bins.sql`).
- Aggregate stock balances were stored in the legacy `inventory` table at `(branch_id, product_id, variant_id)` grain (`001_initial.sql`, `006_quantity_precision_hardening.sql`, `008_inventory_and_schema_hardening.sql`), with historical sales movements logged to `stock_movements` (`003_global_commerce_foundation.sql`).

However, the system lacked:
1. A unified, authoritative ledger service governing every inventory quantity mutation.
2. Spatial attribution of stock balances to physical locations and bins.
3. Batch and serial attribution within spatial storage coordinates.
4. Guaranteed transaction-boundary atomicity and strict idempotency for stock movements.
5. Database-level immutability enforcement for stock movement audit records.

This document defines the authoritative architecture, data models, state machines, invariants, and boundaries for F2.11.

---

## 2. Separation of Architectural Concerns

To maintain structural clarity, all statements are categorized into authoritative existing facts, locked decisions, and milestone boundaries.

### A. Authoritative Existing Facts
1. **Aggregate Inventory Authority (`001_initial.sql:96-105`, `006_quantity_precision_hardening.sql:5-8`, `008_inventory_and_schema_hardening.sql:56-65`):**
   - The `inventory` table aggregates current branch stock at `(branch_id, product_id, variant_id)` using `quantity_milli INTEGER NOT NULL DEFAULT 0`.
   - Partial unique indexes enforce exactly one non-variant row and one variant row per product/branch.
2. **Physical Topography (`019_locations_bins.sql:6-52`):**
   - `locations` defines physical areas within a branch (`branch_id`, `name`, `code`, `is_active`). Composite uniqueness exists on `(id, branch_id)`.
   - `bins` defines addressable storage slots (`location_id`, `name`, `code`, `is_active`).
3. **Batch Master Data (`016_batches_and_expiry.sql:32-46`, ADR-0009):**
   - `product_batches` stores batch identity, expiry dates, and batch quantities in `quantity_milli INTEGER`.
   - Batch status lifecycle: `active`, `quarantined`, `recalled`, `depleted`. ADR-0009 defines `depleted` as strictly terminal.
4. **Tracked Serialized Assets (`017_serial_imei_assets.sql:51-66`, ADR-0010):**
   - `serial_numbers` tracks individual physical units with `serial_number`, `imei`, `asset_tag`, and `status`.
   - Allowed statuses: `in_stock`, `reserved`, `sold`, `transferred`, `defective`, `recalled`, `disposed`.
5. **Historical Stock Movements (`003_global_commerce_foundation.sql:73-87`, `006_quantity_precision_hardening.sql:15-36`):**
   - `stock_movements` stores movement records with `branch_id`, `product_id`, `variant_id`, `quantity_delta_milli`, `quantity_before_milli`, `quantity_after_milli`, `reason`, `source_type`, `source_id`, `user_id`.
6. **Append-Only Migration Rule (`DATABASE_RULES.md`, `V2_RULES.md`):**
   - Applied migrations 001–019 are strictly immutable. All F2.11 schema extensions must be delivered in Migration 020.
7. **Permission Model (`src-tauri/src/permission/mod.rs`):**
   - `Permission::InventoryAdjust` (`"inventory.adjust"`) is the existing dedicated permission for inventory mutations.

---

## 3. Explicit Architectural Decisions

### Decision 1 — Three-Tier Inventory Authority Model
F2.11 establishes a clear three-tier separation of concerns for inventory:
1. **Aggregate Authority (`inventory` table):**
   - Represents total branch-level stock for a product/variant.
   - Remains the fast lookup authority for sales, catalog, and replenishment.
   - No parallel aggregate inventory table is introduced.
2. **Spatial Authority (`location_inventory` table):**
   - Represents physical stock attributed to specific locations and bins.
   - Enforces non-negative balances: `quantity_milli >= 0`.
   - Grain: `(branch_id, location_id, product_id, bin_id, variant_id, batch_id)`.
3. **Immutable History (`stock_movements` table):**
   - The append-only, auditable ledger of every inventory mutation.
   - Enforced immutable via database triggers aborting UPDATE and DELETE operations.

**Quantity Conservation Invariant:**
For every post-020 spatially attributed movement with delta $\Delta$:
$$\text{aggregate\_after} = \text{aggregate\_before} + \Delta$$
$$\text{spatial\_after} = \text{spatial\_before} + \Delta$$
All state updates and the movement record MUST be committed in the exact same atomic transaction.

---

### Decision 2 — Single Transaction Boundary & Sequence
Every movement operation processed by `StockLedgerService` executes within a single SQLite transaction with the following strict sequence:

1. **Idempotency Lookup:** Check `idempotency_keys` for `key`.
2. **Canonical Request Comparison:**
   - If key exists and `request_hash` matches: replay persisted successful result safely without duplicate mutation.
   - If key exists and `request_hash` differs: reject immediately fail-closed as an idempotency conflict.
3. **Request Validation:**
   - Verify non-zero quantity (`quantity_delta_milli != 0`).
   - Validate reason against the 4 approved F2.11 movement reasons and directional rules.
4. **Topography Integrity Validation:**
   - Verify `location_id` exists in the target `branch_id` and is active.
   - If `bin_id` is supplied, verify it belongs to `location_id` and is active.
   - Reject virtual or cross-branch locations.
5. **Product & Variant Validation:**
   - Verify product exists and is active.
   - If variant is supplied, verify variant belongs to product and is active.
6. **Batch Validation (when batch_id supplied):**
   - Verify batch exists, belongs to target product, branch, and variant (null-safe match).
   - Verify batch status is eligible for intake or deduction.
7. **Serial Validation (when serial_id supplied):**
   - Verify exact unit quantity: $+1000$ for positive intake, $-1000$ for negative deduction.
   - Verify serial belongs to target product, branch, and variant.
   - Validate lifecycle status transition according to approved F2.08 state machine.
8. **Mutate Aggregate Inventory:**
   - Upsert or update `inventory` row for `(branch_id, product_id, variant_id)`.
   - Enforce non-negative result for deductions (underflow rejection).
9. **Mutate Spatial Inventory:**
   - Upsert or update `location_inventory` row for `(branch_id, location_id, product_id, bin_id, variant_id, batch_id)`.
   - Database `CHECK (quantity_milli >= 0)` guarantees no spatial underflow.
10. **Mutate Batch / Serial State:**
    - For batch: update `quantity_milli`. If new quantity reaches 0, transition status to `depleted`.
    - For serial: update `status`, `location_id`, and `bin_id`.
11. **Append Stock Movement:**
    - Insert immutable row into `stock_movements` with spatial, batch, and serial attribution columns.
12. **Persist Idempotency Key:**
    - Insert into `idempotency_keys` with `operation`, `request_hash`, and serialized `result_json`.
13. **COMMIT.**

Any error or invariant violation at any step rolls back the entire transaction. No partial movements are permitted.

---

### Decision 3 — Authoritative Movement Reasons and Directionality
F2.11 restricts movement reasons strictly to four primitives:

| Reason | Direction | Delta Sign | Description |
|---|---|---|---|
| `opening_balance` | Inbound only | Positive ($> 0$) | Initial stock establishment during migration or setup. |
| `adjustment` | Inbound or Outbound | Positive or Negative ($\neq 0$) | Stock corrections, counts reconciliation primitive. |
| `damage` | Outbound only | Negative ($< 0$) | Damaged, spoiled, or broken goods written off. |
| `loss` | Outbound only | Negative ($< 0$) | Shrinkage, missing items, or theft written off. |

- No other movement reasons are permitted in F2.11.
- `adjustment` is an atomic ledger primitive. The future `F2.13 Adjustment Business Workflow` (approvals, two-step workflows) is strictly out of scope.

---

### Decision 4 — Locked Batch Decision & Batch Lifecycle
**Authoritative Rule:**
A newly registered batch with zero stock MUST use:
$$\text{quantity\_milli} = 0$$
$$\text{status} = \text{active}$$

**Rationale & Lifecycle Invariants:**
1. ADR-0009 explicitly defines `depleted` as a **strictly terminal** lifecycle state.
2. If a newly registered empty batch were set to `depleted`, it could never receive stock intake because `depleted` cannot be reopened.
3. Therefore, an empty batch starts `active`.
4. The batch lifecycle under F2.11 is:
   - `create_batch` (F2.07): creates metadata record with `quantity_milli = 0` and `status = active`. No inventory is mutated, no movement recorded.
   - Positive intake (`StockLedgerService`): `active` $+$ positive quantity $\to$ `active`.
   - Negative deduction (`StockLedgerService`): `active` $+$ negative quantity $\to$ `active` while $\text{quantity} > 0$; transitions to `depleted` when $\text{quantity} == 0$.
   - `depleted` $\to$ terminal.
5. **No Resurrection:** F2.11 NEVER introduces or executes a transition `depleted -> active`.
6. **Non-Active Batches:** If a batch is in `recalled` or `quarantined`, F2.11 rejects stock movements and must never overwrite or reset the status to `active`.

---

### Decision 5 — Serial Boundary & Spatial Attribution
1. **Separation of Registration and Stock Ownership:**
   - Registration (`create_serial_instance` in F2.08):
     $$\text{status} = \text{reserved}, \quad \text{location\_id} = \text{NULL}, \quad \text{bin\_id} = \text{NULL}$$
     No inventory quantity is changed; no stock movement is created.
   - Serialized Intake (`StockLedgerService` in F2.11):
     - Requires physical location (`location_id != NULL`).
     - Allowed unit quantity: **exactly $+1000$ milli** (1 unit) for positive intake, or **$-1000$ milli** for negative movement. Comparison MUST NOT use `abs(delta) == 1000`.
     - Permitted positive lifecycle sources: existing F2.08 transitions, specifically `reserved -> in_stock` and `defective -> in_stock`.
     - Atomically updates: `inventory` $(+1000)$, `location_inventory` $(+1000)$, `serial_numbers` ($\text{status} = \text{in\_stock}, \text{location\_id}, \text{bin\_id}$), and appends `stock_movements`.
2. **Direct Outbound Mutation Blocked:**
   - Direct manual status updates from `in_stock` to outbound states outside the ledger are blocked.
   - The Recall workflow is out of scope for F2.11.

---

### Decision 6 — Location & Bin Structural Relational Integrity
1. **Macroscopic & Microscopic Topography:**
   - `locations` represents macroscopic physical storage areas.
   - `bins` represents terminal pick/put slots within a location.
2. **Composite Relational Integrity:**
   - `locations` has composite uniqueness on `(id, branch_id)` via `idx_locations_id_branch_id` (`019_locations_bins.sql`).
   - Migration 020 creates unique index `idx_bins_id_location_id` on `bins(id, location_id)`.
   - `location_inventory` enforces:
     - `FOREIGN KEY (location_id, branch_id) REFERENCES locations(id, branch_id)`
     - `FOREIGN KEY (bin_id, location_id) REFERENCES bins(id, location_id)`
   - This ensures at the database level that a bin cannot be referenced under the wrong location, and a location cannot be referenced under the wrong branch.

---

### Decision 7 — Location Inventory Grain & Nullable Uniqueness
1. **Relational Grain:**
   $$( \text{branch\_id}, \text{location\_id}, \text{product\_id}, \text{bin\_id}, \text{variant\_id}, \text{batch\_id} )$$
2. **SQLite NULL Handling:**
   - Because standard SQLite `UNIQUE` constraints treat each `NULL` as distinct, a standard composite unique index would allow duplicate rows when `bin_id`, `variant_id`, or `batch_id` are `NULL`.
   - Migration 020 defines 8 mutually exclusive partial unique indexes covering all permutations of `(bin_id, variant_id, batch_id)`:
     1. `bin_id IS NULL AND variant_id IS NULL AND batch_id IS NULL`
     2. `bin_id IS NOT NULL AND variant_id IS NULL AND batch_id IS NULL`
     3. `bin_id IS NULL AND variant_id IS NOT NULL AND batch_id IS NULL`
     4. `bin_id IS NOT NULL AND variant_id IS NOT NULL AND batch_id IS NULL`
     5. `bin_id IS NULL AND variant_id IS NULL AND batch_id IS NOT NULL`
     6. `bin_id IS NOT NULL AND variant_id IS NULL AND batch_id IS NOT NULL`
     7. `bin_id IS NULL AND variant_id IS NOT NULL AND batch_id IS NOT NULL`
     8. `bin_id IS NOT NULL AND variant_id IS NOT NULL AND batch_id IS NOT NULL`

---

### Decision 8 — Preservation of Legacy Pre-020 Stock
1. **Strict Non-Destructive Policy:**
   - Legacy pre-020 aggregate stock in `inventory` is preserved exactly as-is.
   - No synthetic locations, synthetic bins, or synthetic historical movements will be generated.
   - `location_inventory` is not backfilled for legacy stock; legacy stock remains unallocated spatially.
   - Legacy serialized items with `status = in_stock` and `location_id = NULL` remain untouched.
2. **Future Reconciliation Ownership:**
   - Spatially attributing legacy inventory belongs to the future `F2.14 Stock Count/Reconciliation` milestone.

---

### Decision 9 — Database vs Service Responsibility Division
1. **Database Enforcement (Structural Invariants):**
   - Foreign key integrity and composite relational hierarchy (`branch_id`, `location_id`, `bin_id`).
   - Non-negative balance constraint: `location_inventory.quantity_milli >= 0`.
   - 8-fold partial unique index coverage for spatial stock grain.
   - Immutability triggers on `stock_movements` preventing UPDATE and DELETE.
2. **Service Enforcement (`StockLedgerService`):**
   - Whitelist of 4 movement reasons and directional validations.
   - Serial unit delta (+1000 / -1000) and lifecycle transition enforcement.
   - Batch status progression (depletion when 0; no reactivation of depleted/recalled/quarantined).
   - Canonical request SHA-256 hashing and idempotency replay/conflict detection.
   - Business authorization checks.

---

### Decision 10 — Authorization Model
1. **Mutations:**
   - All quantity-changing operations require `Permission::InventoryAdjust` (`"inventory.adjust"`).
   - Branch tenancy is strictly enforced; users cannot mutate stock for a branch outside their authorized session.
2. **Reads:**
   - Reading spatial stock, ledger movements, and batch/serial coordinates requires an active authenticated session scoped to the requested branch.
   - No fabricated `inventory.view` permission is introduced.

---

### Decision 11 — Milestone Scope Boundaries & Protected Scope
The following areas are strictly out of scope for F2.11:
- `F2.12 Transfers`: Inter-branch and intra-branch transfers.
- `F2.13 Adjustment Workflow`: Multi-step approval workflows for inventory adjustments.
- `F2.14 Stock Count / Reconciliation`: Physical stocktaking, cycle counting, and legacy reconciliation.
- `Phase 3 Sales & Checkout`: Sales order processing and point-of-sale register transactions.
- `Phase 4 Purchasing & Goods Receiving`: Purchase orders, vendor shipments, and receiving docks.
- Protected files: `src-tauri/src/commands/sales.rs`, migrations 001..019, and the permission catalog.

---

## 4. Migration 020 Specification

Migration `020_stock_ledger_spatial.sql` delivers:
1. `CREATE UNIQUE INDEX IF NOT EXISTS idx_bins_id_location_id ON bins(id, location_id);`
2. `CREATE TABLE location_inventory (...)` with composite foreign keys and non-negative check.
3. 8 partial unique indexes on `location_inventory`.
4. Spatial columns on `stock_movements`: `location_id`, `bin_id`, `batch_id`, `serial_id` + performance indexes.
5. Spatial columns on `serial_numbers`: `location_id`, `bin_id` + performance indexes.
6. Idempotency request hash column: `ALTER TABLE idempotency_keys ADD COLUMN request_hash TEXT;`
7. Immutability triggers `trg_stock_movements_immutable_update` and `trg_stock_movements_immutable_delete`.
