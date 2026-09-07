# ADR-0013 — F2.11 Stock Ledger Architecture & Semantics

Status: Accepted — Approved for Planning & Contract Freeze  
Date: 2026-09-07  

---

## 1. Context

Following the completion and merge of milestone `F2.10 — Locations / Bins` (`019_locations_bins.sql`), which established the physical topography master data (`locations` and `bins`) with zero quantity tracking, milestone `F2.11 — Stock Ledger` operationalizes spatial inventory tracking.

In retail, wholesale, and multi-facility commercial operations:
- **At the aggregate level**, inventory balances track total physical ownership per product and variant within a branch.
- **At the spatial level**, inventory must be physically located in specific zones (`locations`) and addressable storage slots (`bins`).
- **At the ledger level**, all quantity changes must be audited by an append-only, immutable transaction ledger (`stock_movements`) providing end-to-end historical reconstruction.
- **At the batch and serial level**, lots (`product_batches`) and uniquely identified assets (`serial_numbers`) must link seamlessly to physical bins without data fragmentation or dual-source-of-truth conflicts.

This document establishes the authoritative architectural decisions, entity models, database invariants, ledger engine semantics, authorization boundaries, and milestone partitioning governing `F2.11`.

---

## 2. Separation of Architectural Concerns

To maintain absolute architectural clarity and prevent specification drift, this ADR categorizes all architectural statements into distinct tiers:

### A. Authoritative Existing Facts
1. **Aggregate Inventory Source of Truth (`001_initial.sql:96`, `006_quantity_precision_hardening.sql:5`):**
   - The `inventory` table stores aggregate stock per `(branch_id, product_id, variant_id)` using integer milli-units (`quantity_milli INTEGER NOT NULL DEFAULT 0`).
   - Prior to Migration 020, `inventory` contains no `location_id` or `bin_id` columns.
2. **Batch Source of Truth (`016_batches_and_expiry.sql:32`):**
   - The `product_batches` table tracks batch lots per `(branch_id, product_id, variant_id, batch_number)` using `quantity_milli INTEGER NOT NULL DEFAULT 0 CHECK (quantity_milli >= 0)`.
   - Prior to Migration 020, `product_batches` contains no `location_id` or `bin_id` columns.
3. **Serial Number Source of Truth (`017_serial_imei_assets.sql:51`):**
   - The `serial_numbers` table tracks individual units with `status CHECK (status IN ('in_stock', 'reserved', 'sold', 'transferred', 'defective', 'recalled', 'disposed'))`.
   - Prior to Migration 020, `serial_numbers` contains no `location_id` or `bin_id` columns.
4. **Existing Stock Movement Ledger (`003_global_commerce_foundation.sql:73`, `006_quantity_precision_hardening.sql:15`):**
   - `stock_movements` records historical movements with `quantity_delta_milli`, `quantity_before_milli`, and `quantity_after_milli`.
   - Existing historical rows have `location_id IS NULL` and `bin_id IS NULL`.
5. **Existing Idempotency Table (`003_global_commerce_foundation.sql:89`):**
   - `idempotency_keys` stores `(key TEXT PRIMARY KEY, operation TEXT, result_json TEXT, created_at TEXT)`.
6. **Physical Topography Master Data (`019_locations_bins.sql`):**
   - `locations` and `bins` exist as physical storage entities scoped to branches.
7. **Append-Only Migration Rule (`DATABASE_RULES.md:9`):**
   - Applied migrations 001–019 are immutable. All schema modifications for F2.11 must be delivered strictly via Migration 020.

### B. Explicit Architectural Decisions
1. **Decision D1 — Discrete Spatial Inventory Balance Entity (`location_inventory`):**
   - Spatial on-hand quantities are tracked in a dedicated `location_inventory` table representing slot balances at `(branch_id, location_id, bin_id, product_id, variant_id, batch_id)`.
   - Aggregate inventory remains in `inventory`. The `inventory` table is NOT replaced or deprecated.
2. **Decision D2 — Single Write Authority (`StockLedgerService`):**
   - `StockLedgerService` in `src-tauri/src/stock/mod.rs` is the sole write authority for F2.11 inventory mutations.
   - All balance updates (`inventory`, `location_inventory`, `product_batches`, `serial_numbers`) must be performed atomically inside the same database transaction that appends to `stock_movements`.
   - Direct, non-ledger balance mutations are strictly forbidden.
3. **Decision D3 — Preservation of Legacy Stock as Unallocated:**
   - Pre-020 legacy stock recorded in `inventory`, `product_batches`, and `serial_numbers` remains untouched and unallocated during Migration 020.
   - Migration 020 does NOT backfill rows into `location_inventory` and does NOT synthesize virtual locations or dummy bins.
4. **Decision D4 — Strict Non-Zero Quantity Delta Invariant:**
   - Every movement executed through `StockLedgerService` represents a real physical quantity change (`quantity_delta_milli != 0`).
   - Zero-delta movements (`quantity_delta_milli == 0`) and zero-net spatial putaway operations are strictly prohibited in F2.11.
5. **Decision D5 — Formal Milestone Partitioning with Transfers and Reconciliation:**
   - Intra-branch zero-net relocations (including moving stock between bins or from unallocated staging into bins) belong strictly to **F2.12 Transfers**.
   - Physical bin auditing and reconciling legacy unallocated stock into bins belongs strictly to **F2.14 Stock Count / Reconciliation**.
   - Deferred convergence for sales prototype write path (`sales.rs`) to Phase 3 (**F3.03**).

---

## 3. Truth Hierarchy & Inventory Balance Architecture

The inventory subsystem maintains a strict, non-redundant truth hierarchy across four tiers:

```
+---------------------------------------------------------------------------------+
|                                 BRANCH CONTEXT                                  |
+---------------------------------------------------------------------------------+
                                         |
         +-------------------------------+-------------------------------+
         |                                                               |
         v                                                               v
+------------------------------------+          +------------------------------------+
|             inventory              |          |          product_batches           |
|------------------------------------|          |------------------------------------|
| (branch_id, product_id, variant_id)|          | (id, branch_id, prod_id, var_id,   |
| quantity_milli: Aggregate On-Hand  |          |  batch_number)                     |
|                                    |          | quantity_milli: Batch Lot On-Hand  |
+------------------------------------+          +------------------------------------+
         |                                                               |
         +-------------------------------+-------------------------------+
                                         |
                                         v
         +---------------------------------------------------------------+
         |                      location_inventory                       |
         |---------------------------------------------------------------|
         | (branch_id, location_id, bin_id, product_id, variant_id,     |
         |  batch_id)                                                    |
         | quantity_milli: Addressable Physical Slot On-Hand             |
         +---------------------------------------------------------------+
                                         ^
                                         | (Spatial Reference)
         +---------------------------------------------------------------+
         |                        serial_numbers                         |
         |---------------------------------------------------------------|
         | (id, branch_id, product_id, variant_id, serial_number)        |
         | location_id, bin_id: Instance Physical Coordinates            |
         | status: 'in_stock' | 'reserved' | 'sold' | ...                |
         +---------------------------------------------------------------+
```

1. **Branch Aggregate Inventory (`inventory`):**
   - Source of truth for total unreserved physical inventory owned by the branch for a product/variant.
2. **Spatial Slot Inventory (`location_inventory`):**
   - Source of truth for physical stock positioned inside a specific macroscopic location or terminal bin.
3. **Batch Lot Inventory (`product_batches`):**
   - Source of truth for total quantity belonging to a specific batch lot within the branch.
4. **Serial Asset Inventory (`serial_numbers`):**
   - Source of truth for discrete physical instances. Each in-stock serialized item holds instance-level spatial coordinates (`location_id`, `bin_id`).

---

## 4. Invariant Equations & Balance Formulas

### 4.1 Non-Batch Stock Invariant
For any branch, product, and variant:
$$\text{inventory.quantity\_milli} = \text{Unallocated Stock} + \sum \text{location\_inventory.quantity\_milli}$$

$$\text{unallocated\_quantity\_milli} = \text{inventory.quantity\_milli} - \sum \text{location\_inventory.quantity\_milli} \ge 0$$

### 4.2 Batch-Tracked Stock Invariant
For any branch, product, and batch:
$$\text{product\_batches.quantity\_milli} = \text{Unallocated Batch Stock} + \sum_{\text{for that batch}} \text{location\_inventory.quantity\_milli}$$

$$\text{unallocated\_batch\_milli} = \text{product\_batches.quantity\_milli} - \sum_{\text{for that batch}} \text{location\_inventory.quantity\_milli} \ge 0$$

Across all batches of a product:
$$\text{inventory.quantity\_milli} \ge \sum_{\text{all batches}} \text{product\_batches.quantity\_milli}$$

### 4.3 Fail-Closed Non-Negative Stock Policy
Unless an explicit business policy permits negative stock, the system enforces non-negative stock fail-closed:
- `inventory.quantity_milli >= 0`
- `location_inventory.quantity_milli >= 0`
- `product_batches.quantity_milli >= 0`

---

## 5. Legacy Unallocated Stock vs. New Opening Balance

### 5.1 Semantic Clarification
The system strictly distinguishes between historical pre-020 data and new post-020 operational movements:

| Dimension | Legacy Existing Stock (Pre-020) | New F2.11 Opening Balance |
| :--- | :--- | :--- |
| **Origin** | Created prior to Migration 020. | Created post-020 via `StockLedgerService`. |
| **Spatial Coordinates** | None (`location_id IS NULL`, `bin_id IS NULL`). | Mandatory (`location_id NOT NULL`, optional `bin_id`). |
| **Quantity Delta** | N/A (historical static balance). | Strict non-zero positive delta ($\Delta > 0$). |
| **Ledger Row** | Historical rows in `stock_movements`. | New row inserted into `stock_movements`. |
| **Balance Effect** | Preserved in `inventory` and `product_batches`. | Atomically increments aggregate AND spatial balances by $\Delta$. |
| **Unallocated Effect** | Forms initial `unallocated` quantity ($> 0$). | Leaves `unallocated` quantity **strictly unchanged**. |

### 5.2 Proof of Unallocated Invariance
When a new spatial opening balance with $\Delta > 0$ is posted to location $L$ and bin $B$:
$$\text{aggregate}_{\text{after}} = \text{aggregate}_{\text{before}} + \Delta$$
$$\text{spatial}_{\text{after}} = \text{spatial}_{\text{before}} + \Delta$$
$$\begin{aligned}
\text{unallocated}_{\text{after}} &= \text{aggregate}_{\text{after}} - \text{spatial}_{\text{after}} \\
&= (\text{aggregate}_{\text{before}} + \Delta) - (\text{spatial}_{\text{before}} + \Delta) \\
&= \text{aggregate}_{\text{before}} - \text{spatial}_{\text{before}} \\
&= \text{unallocated}_{\text{before}}
\end{aligned}$$

A new F2.11 opening balance adds equal quantity to aggregate and spatial state simultaneously. The unallocated quantity remains strictly unchanged.

### 5.3 Prohibition on Automated Legacy Allocation
- `opening_balance` does **NOT** mean *"automatically allocate or re-label historical inventory during migration"*.
- Migration 020 and `StockLedgerService` shall never backfill synthetic spatial records or automatically allocate historical inventory.
- Spatial attribution of legacy unallocated stock without net quantity change represents an intra-branch transfer or count reconciliation, strictly sequenced in **F2.12 Transfers** and **F2.14 Stock Count / Reconciliation**.

---

## 6. Serialized Asset Spatial Semantics

1. **Legacy Serials (Pre-020):**
   - Existing rows in `serial_numbers` with `status = 'in_stock'` and `location_id IS NULL, bin_id IS NULL` remain preserved exactly as legacy unallocated serials.
   - Migration 020 does not assign or fabricate locations for legacy serials.
2. **New Serialized Opening Balance (Post-020):**
   - Establishing a new serialized asset requires a physical location (`location_id NOT NULL`).
   - Every serialized movement represents an indivisible single unit: $|\Delta_{\text{milli}}| = 1000$.
   - Atomically updates:
     - `serial_numbers.location_id = LOC`
     - `serial_numbers.bin_id = BIN`
     - `serial_numbers.status = 'in_stock'`
     - `location_inventory.quantity_milli += 1000`
     - `inventory.quantity_milli += 1000`
     - `stock_movements(serial_id = S, location_id = LOC, bin_id = BIN, delta_milli = 1000)`

---

## 7. Ledger Engine Semantics (`StockLedgerService`)

### 7.1 Single Write Authority
All F2.11 inventory mutations must execute through `StockLedgerService::post_movement`. Direct SQL updates or inserts to `inventory` or `location_inventory` bypassing this service are strictly prohibited.

### 7.2 Permitted Movement Reasons in F2.11
`StockLedgerService` accepts exactly four operational reasons in F2.11:
1. `opening_balance`: Initializing physical stock in a bin ($\Delta > 0$).
2. `adjustment`: Modifying physical stock in a bin ($\Delta \ne 0$).
3. `damage`: Writing off damaged stock from a bin ($\Delta < 0$).
4. `loss`: Writing off lost/stolen stock from a bin ($\Delta < 0$).

All other movement reasons (`sale`, `refund`, `purchase_receipt`, `transfer_out`, `transfer_in`, `count_reconciliation`) are reserved for future milestones.

### 7.3 Strict Delta Invariant
- Any movement request with `quantity_delta_milli == 0` is rejected immediately with `StockLedgerError::ZeroQuantityDelta`.
- Any movement request with `location_id IS NULL` is rejected immediately with `StockLedgerError::MissingLocation`.
- Synthetic movement pairs and zero-net transfer operations are rejected in F2.11.

### 7.4 Atomic Mutation Sequence (Single SQLite Transaction)
1. **Idempotency Gate:** Validate key and canonical hash against `idempotency_keys`. Replay cached response on match; abort on conflict.
2. **Lock & Read:** Fetch current `inventory.quantity_milli` and slot `location_inventory.quantity_milli` within the transaction.
3. **Validate Invariants:**
   - Verify `location_id` belongs to `branch_id` and is active.
   - If `bin_id` provided, verify it belongs to `location_id` and is active.
   - Check non-negative constraints: `quantity_after >= 0` and `slot_after >= 0`.
   - If batch: verify batch belongs to product and branch; check batch balance $\ge 0$.
   - If serial: verify serial belongs to product and branch; verify $|\Delta| = 1000$.
4. **Mutate Balances:**
   - Update `inventory.quantity_milli += delta`.
   - Upsert `location_inventory.quantity_milli += delta`.
   - If batch: update `product_batches.quantity_milli += delta`.
   - If serial: update `serial_numbers` status, `location_id`, and `bin_id`.
5. **Append Movement:**
   - Insert row into `stock_movements`:
     `quantity_delta_milli = delta`,
     `quantity_before_milli = aggregate_before`,
     `quantity_after_milli = aggregate_before + delta`,
     `location_id = LOC`,
     `bin_id = BIN`.
6. **Persist Idempotency:** Write execution result to `idempotency_keys`.

---

## 8. Idempotency Architecture

Canonical idempotency persistence uses the existing `idempotency_keys` table (`003_global_commerce_foundation.sql:89`), extended via Migration 020 with:
```sql
ALTER TABLE idempotency_keys ADD COLUMN request_hash TEXT;
```

### Canonical Request Hash
$$\text{request\_hash} = \text{SHA-256}(\text{branch\_id} \mid \text{product\_id} \mid \text{variant\_id} \mid \text{location\_id} \mid \text{bin\_id} \mid \text{batch\_id} \mid \text{serial\_id} \mid \text{delta\_milli} \mid \text{reason})$$

### Resolution Rules
1. **New Key:** Execute ledger transaction, persist key, operation, hash, and JSON result atomically.
2. **Matching Key + Matching Hash:** Return cached `result_json` without re-executing ledger mutations.
3. **Matching Key + Different Hash:** Reject immediately with `StockLedgerError::IdempotencyConflict` (HTTP 409 / conflict error).

---

## 9. Historical Reconstruction & Audit Streams

The append-only ledger `stock_movements` provides four distinct audit streams, strictly preserving the boundary between legacy pre-020 baselines and post-020 spatial tracking:

1. **Attributed Spatial History (100% Reconstructable from Movements):**
   Because `location_inventory` is instantiated empty in Migration 020 and mutated solely through `StockLedgerService`, all spatial slot balances are strictly reconstructable from spatial movement rows:
   $$\text{location\_inventory.quantity\_milli}(L, B) = \sum_{\substack{\text{location\_id} = L \\ \text{bin\_id} = B}} \text{quantity\_delta\_milli}$$

2. **Aggregate Branch History:**
   Reflects the pre-020 legacy unallocated baseline plus all recorded movement deltas for that branch and product:
   $$\text{inventory.quantity\_milli} = \text{legacy\_unallocated\_baseline} + \sum_{\text{movements}} \text{quantity\_delta\_milli}$$
   (For products initialized post-020 via `opening_balance`, `legacy_unallocated_baseline = 0`, making aggregate inventory fully derived from movements).

3. **Legacy Unattributed History:**
   Historical movement rows with `location_id IS NULL` (including pre-convergence sales from `sales.rs`) remain permanently unattributed to physical locations. They document historical volume changes without retroactive spatial attribution.

4. **Attributed Batch History:**
   Reflects the pre-020 legacy batch baseline plus all subsequent batch movement deltas:
   $$\text{product\_batches.quantity\_milli}(X) = \text{legacy\_batch\_baseline}(X) + \sum_{\text{batch\_id} = X} \text{quantity\_delta\_milli}$$

### Immutability Enforcement

Historical movement records are protected by database triggers that abort any `UPDATE` or `DELETE` on `stock_movements`.

---

## 10. Database Schema Specification (Migration 020)

### 10.1 Schema Enhancements
Migration 020 (`020_stock_ledger_and_spatial_balances.sql`) implements:

```sql
-- 1. Idempotency extension
ALTER TABLE idempotency_keys ADD COLUMN request_hash TEXT;

-- 2. Stock movements spatial & entity references
ALTER TABLE stock_movements ADD COLUMN location_id TEXT REFERENCES locations(id);
ALTER TABLE stock_movements ADD COLUMN bin_id TEXT REFERENCES bins(id);
ALTER TABLE stock_movements ADD COLUMN batch_id TEXT REFERENCES product_batches(id);
ALTER TABLE stock_movements ADD COLUMN serial_id TEXT REFERENCES serial_numbers(id);

-- 3. Serial numbers spatial attribution
ALTER TABLE serial_numbers ADD COLUMN location_id TEXT REFERENCES locations(id);
ALTER TABLE serial_numbers ADD COLUMN bin_id TEXT REFERENCES bins(id);

-- 4. Spatial inventory balances table
CREATE TABLE location_inventory (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
    bin_id TEXT REFERENCES bins(id) ON DELETE RESTRICT,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    variant_id TEXT REFERENCES product_variants(id) ON DELETE RESTRICT,
    batch_id TEXT REFERENCES product_batches(id) ON DELETE RESTRICT,
    quantity_milli INTEGER NOT NULL DEFAULT 0 CHECK (quantity_milli >= 0),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (location_id, branch_id) REFERENCES locations(id, branch_id),
    FOREIGN KEY (bin_id, location_id) REFERENCES bins(id, location_id),
    FOREIGN KEY (batch_id, product_id, branch_id) REFERENCES product_batches(id, product_id, branch_id)
);
```

### 10.2 Logical Key & 8 Mutually Exclusive Partial Unique Indexes
To guarantee uniqueness across nullable permutations of `(bin_id, variant_id, batch_id)` without NULL-collapsing bugs, exactly 8 partial unique indexes are created:

1. `(branch_id, location_id, product_id)` WHERE `bin_id IS NULL AND variant_id IS NULL AND batch_id IS NULL`
2. `(branch_id, location_id, bin_id, product_id)` WHERE `bin_id IS NOT NULL AND variant_id IS NULL AND batch_id IS NULL`
3. `(branch_id, location_id, product_id, variant_id)` WHERE `bin_id IS NULL AND variant_id IS NOT NULL AND batch_id IS NULL`
4. `(branch_id, location_id, product_id, batch_id)` WHERE `bin_id IS NULL AND variant_id IS NULL AND batch_id IS NOT NULL`
5. `(branch_id, location_id, bin_id, product_id, variant_id)` WHERE `bin_id IS NOT NULL AND variant_id IS NOT NULL AND batch_id IS NULL`
6. `(branch_id, location_id, bin_id, product_id, batch_id)` WHERE `bin_id IS NOT NULL AND variant_id IS NULL AND batch_id IS NOT NULL`
7. `(branch_id, location_id, product_id, variant_id, batch_id)` WHERE `bin_id IS NULL AND variant_id IS NOT NULL AND batch_id IS NOT NULL`
8. `(branch_id, location_id, bin_id, product_id, variant_id, batch_id)` WHERE `bin_id IS NOT NULL AND variant_id IS NOT NULL AND batch_id IS NOT NULL`

### 10.3 Engine Integrity Triggers
```sql
-- Immutability triggers
CREATE TRIGGER trg_stock_movements_no_update
BEFORE UPDATE ON stock_movements
BEGIN
    SELECT RAISE(ABORT, 'Historical stock_movements rows are immutable and cannot be updated');
END;

CREATE TRIGGER trg_stock_movements_no_delete
BEFORE DELETE ON stock_movements
BEGIN
    SELECT RAISE(ABORT, 'Historical stock_movements rows are immutable and cannot be deleted');
END;

-- Spatial consistency trigger
CREATE TRIGGER trg_stock_movements_spatial_guard
BEFORE INSERT ON stock_movements
WHEN NEW.location_id IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'Movement location branch does not match movement branch')
    WHERE NOT EXISTS (
        SELECT 1 FROM locations
        WHERE id = NEW.location_id AND branch_id = NEW.branch_id
    );

    SELECT RAISE(ABORT, 'Movement bin does not belong to movement location')
    WHERE NEW.bin_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM bins
        WHERE id = NEW.bin_id AND location_id = NEW.location_id
    );
END;
```

---

## 11. Subsystem Boundaries & Scope Protections

| Subsystem / Milestone | Responsibility in F2.11 | Boundary Contract |
| :--- | :--- | :--- |
| **Sales Prototype (`sales.rs`)** | None (Preserved untouched). | Option B: Deferred Sales Spatial Convergence. `sales.rs` continues to write legacy unlocated movements until Phase 3 (**F3.03**). |
| **F2.10 Locations / Bins** | Target FK entities. | F2.11 consumes `locations` and `bins` as foreign keys without modifying F2.10 master data rules. |
| **F2.11 Stock Ledger** | Spatial stock ledger, slot balances, single write authority. | Owns physical quantity movements (`opening_balance`, `adjustment`, `damage`, `loss`). |
| **F2.12 Transfers** | Deferred to F2.12. | Owns all inter-branch and intra-branch transfers, zero-net relocations, transit states, and transfer documents. |
| **F2.13 Adjustments** | Deferred to F2.13. | Owns shrinkage taxonomies, adjustment approval workflows, and loss classification hierarchies. |
| **F2.14 Stock Count / Reconciliation** | Deferred to F2.14. | Owns physical barcode/bin counting and reconciling unallocated legacy stock into physical bins. |
| **Protected Inventions** | Strictly Forbidden. | Zero virtual locations, zero default warehouses, zero receiving locations, zero synthetic putaway balances. |

---

## 12. Consequences

### Positive Consequences
- **Absolute Immutability:** Historical ledger rows cannot be tampered with via SQL `UPDATE` or `DELETE`.
- **Zero Data Fabrication:** Legacy pre-020 inventory is preserved accurately as unallocated stock without guessing physical locations.
- **Single Write Authority:** Prevents drift between aggregate, spatial, batch, and serial balance tables.
- **Relational Integrity:** Composite foreign keys and 8 mutually exclusive partial indexes prevent branch contamination and slot collisions at the database engine level.
- **Full Historical Auditability:** Every unit of spatial inventory can be reconstructed from immutable ledger movements with signed arithmetic quantity deltas.

### Negative & Neutral Trade-Offs
- **Dual Write Overhead:** Posting a spatial stock movement requires updating both `inventory` and `location_inventory`.
- **Deferred Legacy Relocation:** Legacy stock cannot be relocated into bins until F2.12 Transfers or F2.14 Reconciliation are implemented.

---

## 13. Revisit Triggers

This architecture shall be revisited only if one of the following authoritative triggers occurs:
1. **Cloud Multi-Write Synchronization:** If cloud offline sync conflicts require a CRDT-based ledger model rather than strict local serializability.
2. **Sales Spatial Requirement Prior to Phase 3:** If commercial requirements mandate bin-level deduction during sales checkout prior to milestone F3.03.
