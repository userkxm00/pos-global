# ADR-0014 — F2.12 Transfers Architecture & Semantics

Status: Accepted — Authoritative
Date: 2026-09-11

---

## 1. Context

Milestones `F2.01` through `F2.11` established the catalog, units, matrix variants, weighted items, batches, serialized assets, warranties, macroscopic/microscopic physical storage topography (`locations` and `bins` via `019_locations_bins.sql`), and the three-tier inventory ledger with spatial balances (`location_inventory` and `stock_movements` via `020_stock_ledger_spatial.sql`).

Prior to milestone `F2.12`:
1. Inventory balances were confined to static physical locations and bins within individual branches.
2. The system lacked transfer document structures to initiate, authorize, dispatch, track, and receive stock moving between locations or across branches.
3. [ADR-0012](file:///c:/Users/user0/Desktop/pos%20global/pos-global/docs/adr/0012-f2-10-locations-bins-architecture-semantics.md#L306) explicitly deferred inter-branch and intra-branch inventory transfers, transit status, and virtual in-transit locations to `F2.12 Transfers`.
4. [ADR-0013](file:///c:/Users/user0/Desktop/pos%20global/pos-global/docs/adr/0013-f2-11-stock-ledger-spatial-balances-architecture-semantics.md#L239) explicitly deferred transfer workflows to `F2.12 Transfers` and restricted write mutations in `post_stock_movement` to adjustment primitives (`opening_balance`, `adjustment`, `damage`, `loss`), while defining `StockMovementReason::Transfer` as a system reason.

This document formalizes the authoritative architecture, data models, state machines, mutation authorities, transaction boundaries, and invariants for milestone **F2.12 — Transfers**.

---

## 2. Separation of Architectural Concerns

### A. Authoritative Existing Facts
1. **Ledger Mutation Authority ([ADR-0013](file:///c:/Users/user0/Desktop/pos%20global/pos-global/docs/adr/0013-f2-11-stock-ledger-spatial-balances-architecture-semantics.md)):**
   - `StockLedgerService` (`src-tauri/src/stock_ledger/mod.rs`) is the sole authority for quantity mutations across `inventory` (aggregate) and `location_inventory` (spatial).
   - Raw SQL updates to inventory balance tables are strictly prohibited.
   - `stock_movements` is append-only and enforced immutable via database triggers `trg_stock_movements_immutable_update` and `trg_stock_movements_immutable_delete`.
2. **Physical Topography ([ADR-0012](file:///c:/Users/user0/Desktop/pos%20global/pos-global/docs/adr/0012-f2-10-locations-bins-architecture-semantics.md), `019_locations_bins.sql`):**
   - Locations are physically bound to branches (`branch_id`, `id`). Bins are physically bound to locations (`location_id`, `id`).
   - Composite foreign keys enforce strict same-branch and same-location relational hierarchy.
3. **Serial Lifecycle State Machine ([ADR-0010](file:///c:/Users/user0/Desktop/pos%20global/pos-global/docs/adr/0010-f2-08-serial-imei-assets-architecture-semantics.md), `src-tauri/src/serial/mod.rs`):**
   - Allowed statuses include `in_stock` and `transferred`.
   - Valid transitions already include `in_stock -> transferred` and `transferred -> in_stock`.
   - Each serial is bound to `(branch_id, product_id)` with unit delta strictly $\pm 1000$ milli.
4. **Batch Lifecycle ([ADR-0009](file:///c:/Users/user0/Desktop/pos%20global/pos-global/docs/adr/0009-f2-07-batch-expiry-fefo-architecture-semantics.md), `016_batches_and_expiry.sql`):**
   - Batches are tracked per `(branch_id, product_id, batch_number)`.
   - Depleted batches are strictly terminal.
5. **Permission Model (`src-tauri/src/permission/mod.rs`):**
   - `Permission::InventoryTransfer` (`"inventory.transfer"`) is the dedicated permission seeded in `004_exact_money_and_identity.sql` and assigned to Manager and Admin roles.
6. **Append-Only Migration Rule (`DATABASE_RULES.md`, `V2_RULES.md`):**
   - Migrations 001–020 are immutable. All F2.12 schema additions must be introduced in forward migration `021_stock_transfers.sql`.

---

## 3. Explicit Architectural Decisions

### Decision 1 — Transfer Scope & Dual Transfer Topologies

F2.12 establishes two distinct transfer topologies with separate lifecycle semantics:

#### 1. Intra-Branch (Internal / Spatial) Relocation
- **Definition:** Moving stock between physical locations and/or bins within the *same* branch (`source_branch_id == destination_branch_id`).
- **Aggregate Balance Invariant:** Net aggregate branch stock change is strictly zero ($\Delta_{\text{aggregate}} = 0$).
- **Spatial Balance Invariant:** Source spatial balance is decremented by $\Delta$; destination spatial balance is incremented by $\Delta$.
- **Execution Model:** Immediate, single-transaction atomic relocation. No intermediate in-transit state is permitted because physical custody remains within the same premises.
- **Lifecycle Restriction:** Intra-branch transfers must NEVER enter `in_transit`. Permitted lifecycle states are strictly `draft`, `completed`, and `cancelled`. Immediate physical relocations are recorded directly as `completed`. Enforced at the schema level:
  `CHECK (transfer_type != 'intra_branch' OR status IN ('draft', 'completed', 'cancelled'))`.

#### 2. Inter-Branch Inventory Transfer
- **Definition:** Moving stock across two distinct physical branches (`source_branch_id != destination_branch_id`).
- **Two-Stage Document Lifecycle:**
  $$\text{Draft} \xrightarrow{\text{dispatch}} \text{In-Transit} \xrightarrow{\text{receive}} \text{Completed}$$
  $$\text{Draft} \xrightarrow{\text{cancel}} \text{Cancelled}$$
- **Dispatch (Outbound from Source Branch):** Deducts stock from source branch aggregate and spatial balances via `StockLedgerService` (`reason = 'transfer'`). Status becomes `in_transit`. Serials transition `in_stock -> transferred`.
- **Receive (Inbound to Destination Branch):** Credits stock to destination branch aggregate and spatial balances via `StockLedgerService` (`reason = 'transfer'`). Status becomes `completed`. Serials transition `transferred -> in_stock` and are re-homed to the destination branch.
- **Strict All-or-Nothing Receipt (No Partial Receipt / No Transit Variance):**
  - F2.12 does NOT support partial receipt.
  - F2.12 does NOT support transit variance.
  - `received_quantity_milli` must either be `NULL` (while `draft` or `in_transit`) or strictly equal dispatched `quantity_milli` (`CHECK (received_quantity_milli IS NULL OR received_quantity_milli = quantity_milli)`).
  - Any physical shortage, shrinkage, damage, or discrepancy discovered upon arrival is not absorbed by the transfer document. The transfer document is received in full into the destination staging area, and any variance is handled subsequently by `F2.13 Adjustments` (Damage or Loss write-offs) to preserve end-to-end ledger conservation and auditability.
  - **Completion Integrity Enforcement:** Completion integrity is enforced by:
    1. `TransferService` domain validation during receipt.
    2. Database trigger `trg_stock_transfers_completed_receipt`, which rejects updating a transfer to `completed` if any child item has `received_quantity_milli IS NULL`.

---

### Decision 2 — Transfer Document Ownership vs. Mutation Authority

To prevent duplicate authorities or fragmented ledger logic:

1. **Document Ownership (`TransferService`):**
   - Owns transfer headers (`stock_transfers`) and line items (`stock_transfer_items`).
   - Validates transfer business rules, branch routing, location accessibility, line item quantities, and document state transitions.
   - **Absence of Independent Item Status (Document Normalization):** `stock_transfer_items` has NO independent status. Transfer lifecycle belongs exclusively to `stock_transfers.status`. Item state is derived by joining to the parent transfer (`stock_transfers`). This eliminates split-brain status drift, preserves normalized 3NF document modeling, and matches existing POS Global document structures such as `sale_items` and `purchase_order_items`.
   - **Cross-Entity Relational Validation:** Because SQLite simple foreign keys do not enforce branch, product, or variant consistency for tracked entities (`product_variants`, `product_batches`, `serial_numbers`), `TransferService` must perform fail-closed domain validation prior to insertion and dispatch for:
     - `variant.product_id == item.product_id`
     - `batch.product_id == item.product_id`
     - `batch.variant_id` matches `item.variant_id` where applicable
     - `batch.branch_id == source_branch_id`
     - `serial.product_id == item.product_id`
     - `serial.variant_id` matches `item.variant_id` where applicable
     - `serial.branch_id == source_branch_id`
2. **Mutation Authority (`StockLedgerService`):**
   - Remains the **sole authority** for updating `inventory` and `location_inventory`.
   - `TransferService` does NOT update inventory tables directly with raw SQL.
   - All physical inventory deductions and intakes are executed by invoking `StockLedgerService::post_movement`.
   - Every stock movement record is generated with `reason = StockMovementReason::Transfer`, `source_type = 'transfer'`, and `source_id = transfer_id`.

---

### Decision 3 — Security Decision on Transfer Movement Posting

**Problem:** Should the public Tauri IPC command `post_stock_movement` allow arbitrary callers to pass `reason: "transfer"`?

**Authoritative Decision:**
- **NO.** Public callers must NOT be permitted to post arbitrary transfer movements through the generic `post_stock_movement` endpoint.
- If generic callers could invoke `post_stock_movement` with `reason = "transfer"`, users could generate "naked" transfer movements without an underlying transfer document, bypassing source/destination branch pairing, audit trails, and transit reconciliation.
- Therefore:
  1. `post_stock_movement` continues to reject `reason: "transfer"` for generic client calls.
  2. `TransferService` has authorized internal capability to post movements with `StockMovementReason::Transfer`, strictly binding each movement to a verified `stock_transfers` row (`source_type = 'transfer'`, `source_id = transfer_id`).

---

### Decision 4 — In-Transit Stock Modeling (No Synthetic Virtual Locations)

**Problem ([ADR-0012:340](file:///c:/Users/user0/Desktop/pos%20global/pos-global/docs/adr/0012-f2-10-locations-bins-architecture-semantics.md#L340)):** Does in-transit stock require synthetic "virtual in-transit location" rows in the `locations` table?

**Authoritative Decision:**
- **NO synthetic virtual locations.**
- `locations` rows in `019_locations_bins.sql` represent physical, addressable storage topography strictly scoped to a single `branch_id`. Injecting synthetic virtual branches or virtual locations would corrupt tenant foreign keys, spatial uniqueness indexes, and branch isolation.
- Instead, in-transit stock is tracked **at the document layer**:
  - Outbound stock has departed the source branch (deducted from source `inventory` and `location_inventory`).
  - Outbound stock has not yet arrived at the destination branch.
  - The in-transit inventory quantity is deterministically queryable by aggregating active `stock_transfer_items` for transfers where `status = 'in_transit'`.
  - Serialized items in transit hold `status = 'transferred'` with `location_id = NULL` and `bin_id = NULL`.

---

### Decision 5 — Tracked Unit Continuity (Batches & Serials)

#### 1. Batch Continuity
- During inter-branch dispatch:
  - Source batch quantity is deducted via `StockLedgerService` using the source `batch_id`.
  - If source batch reaches 0, it transitions to `depleted`.
- During inter-branch receipt:
  - The destination branch intakes the batch.
  - **Destination Batch Resolution:** `TransferService` resolves destination batch identity by matching:
    `(destination_branch_id, product_id, variant_id, batch_number COLLATE NOCASE)`
  - **Expiry Alignment:** After resolving an existing batch at the destination branch, its `expiry_date` must match the incoming batch's `expiry_date` exactly. Any discrepancy fails-closed as a data integrity anomaly.
  - **Terminal & Inactive Status Protection:** If an existing destination batch is found in `depleted`, `recalled`, or `quarantined` status, intake is strictly rejected fail-closed (`TransferError::InvalidBatchStatus`). Depleted batches are terminal and cannot accept new intake.
  - **New Destination Batch Creation:** If no matching batch exists at the destination branch, a new batch record is created before ledger intake:
    - `status = 'active'`
    - Initial `quantity_milli = 0` (metadata registration only, per F2.07 domain rule)
    - Metadata copied from source batch: `cost_price_minor`, `manufactured_date`, and `expiry_date`.
    - Stock is subsequently intaked into this destination batch via `StockLedgerService::post_movement`.
  - No new batch identity rules or schema constraints are invented beyond existing F2.07/F2.11 contracts.

#### 2. Serial Asset Continuity & Re-homing
- During inter-branch dispatch:
  - The serial instance must belong to `source_branch_id` and have `status = 'in_stock'`.
  - Quantity delta is strictly $-1000$ milli (1 unit).
  - Serial transitions `status: in_stock -> transferred`, and its spatial coordinates (`location_id`, `bin_id`) are cleared (`NULL`).
- During inter-branch receipt:
  - The serial instance must have `status = 'transferred'`.
  - Quantity delta is strictly $+1000$ milli (1 unit).
  - Inside the atomic receive transaction:
    - Serial is re-homed to the destination branch (`branch_id = destination_branch_id`).
    - Destination `location_id` and `bin_id` are assigned in the same transaction.
    - Status transitions `transferred -> in_stock`.
- **F2.12 Transfer-Aware Ledger Capability:**
  - Existing F2.11 `StockLedgerService` blocks generic transfer reasons, maps outbound serials only for damage/loss/adjustment, and rejects serial intake unless status is `reserved` or `defective` from the same branch.
  - Therefore, F2.12 requires transfer-aware ledger support for `StockMovementReason::Transfer`:
    - Outbound: `in_stock -> transferred`
    - Inbound: `transferred -> in_stock`
    - Destination branch re-homing occurs inside the atomic receive transaction alongside destination location/bin assignment.
  - This is explicitly recognized as a new narrow internal capability required by F2.12. (Do NOT implement the capability yet).
- Serials in `sold`, `defective`, `recalled`, or `disposed` status are strictly rejected from transfer dispatch.

---

### Decision 6 — Transaction Boundaries

1. **Intra-Branch Relocation (Single Transaction):**
   ```text
   BEGIN TRANSACTION;
     Verify source location/bin stock >= requested;
     Post negative movement at source location/bin (StockLedgerService, idempotency_key = None);
     Post positive movement at destination location/bin (StockLedgerService, idempotency_key = None);
     Insert stock_transfers (status = 'completed', transfer_type = 'intra_branch');
     Insert stock_transfer_items (received_quantity_milli = quantity_milli);
     Record transfer idempotency key;
   COMMIT;
   ```
2. **Inter-Branch Dispatch (Single Transaction at Source):**
   ```text
   BEGIN TRANSACTION;
     Verify transfer status is 'draft';
     Verify source branch stock >= requested;
     Post negative movement at source branch/location (StockLedgerService, idempotency_key = None);
     Update serial status to 'transferred' (if serialized);
     Update stock_transfers (status = 'in_transit', dispatched_at, dispatched_by);
     Record transfer idempotency key;
   COMMIT;
   ```
3. **Inter-Branch Receive (Single Transaction at Destination):**
   ```text
   BEGIN TRANSACTION;
     Verify transfer status is 'in_transit';
     Post positive movement at destination branch/location (StockLedgerService, idempotency_key = None);
     Reassign serial branch_id and set status to 'in_stock' (if serialized);
     Update or create destination batch (if batched);
     Update stock_transfer_items (received_quantity_milli = quantity_milli);
     Update stock_transfers (status = 'completed', received_at, received_by);
     Record transfer idempotency key;
   COMMIT;
   ```

Any failure at any step aborts and rolls back the transaction completely.

---

### Decision 7 — Idempotency Ownership & Replay Semantics

- Both `dispatch_stock_transfer` and `receive_stock_transfer` support an optional `idempotency_key`.
- **Idempotency Key Ownership:** The client's `idempotency_key` is owned and evaluated exclusively by `TransferService` at the outer transaction boundary.
- **No Secondary Idempotency System:** Transfer operations use the existing `idempotency_keys` table with SHA-256 canonical hashing of the operation and payload (`operation = 'dispatch_stock_transfer'` or `operation = 'receive_stock_transfer'`).
- **Internal Ledger Isolation Invariant:** All internal invocations of `StockLedgerService::post_movement` made by `TransferService` MUST pass `idempotency_key: None`. Subordinate ledger movements participate in the parent transfer transaction; passing the client key down to `StockLedgerService` would cause a primary key collision on `idempotency_keys.key`.
- **Replay:** If the key exists and the payload hash matches, the previously persisted transfer document is returned without executing secondary inventory deductions or receipts.
- **Conflict:** If the key exists but the payload hash differs, the operation fails-closed with `TransferError::IdempotencyConflict`.

---

### Decision 8 — Authorization & Tenancy Boundaries

1. **Permission:** All transfer operations require `Permission::InventoryTransfer` (`"inventory.transfer"`).
2. **Branch Tenancy Enforcement:**
   - `create_stock_transfer`: Caller must be authorized for `source_branch_id`.
   - `dispatch_stock_transfer`: Caller must be authorized for `source_branch_id`.
   - `receive_stock_transfer`: Caller must be authorized for `destination_branch_id`.
   - `cancel_stock_transfer`: Caller must be authorized for `source_branch_id`.
   - `list_stock_transfers`: Caller can only query transfers where their authorized branch is either source or destination.
3. Unprivileged roles (e.g. Cashier) are rejected fail-closed outside the UI.

---

## 4. Migration 021 Specification

Forward migration `021_stock_transfers.sql` introduces:

```sql
-- 021_stock_transfers.sql
-- F2.12 — Transfers Architecture
-- Append-only migration. Never modify applied migrations.

CREATE TABLE stock_transfers (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    transfer_number TEXT NOT NULL UNIQUE,
    transfer_type TEXT NOT NULL CHECK (transfer_type IN ('intra_branch', 'inter_branch')),
    source_branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    destination_branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    source_location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
    destination_location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
    source_bin_id TEXT,
    destination_bin_id TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'in_transit', 'completed', 'cancelled')),
    dispatched_at TEXT,
    dispatched_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    received_at TEXT,
    received_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (source_location_id, source_branch_id) REFERENCES locations(id, branch_id) ON DELETE RESTRICT,
    FOREIGN KEY (destination_location_id, destination_branch_id) REFERENCES locations(id, branch_id) ON DELETE RESTRICT,
    FOREIGN KEY (source_bin_id, source_location_id) REFERENCES bins(id, location_id) ON DELETE RESTRICT,
    FOREIGN KEY (destination_bin_id, destination_location_id) REFERENCES bins(id, location_id) ON DELETE RESTRICT,
    CHECK (transfer_type != 'intra_branch' OR status IN ('draft', 'completed', 'cancelled')),
    CHECK ((transfer_type = 'intra_branch' AND source_branch_id = destination_branch_id) OR (transfer_type = 'inter_branch' AND source_branch_id != destination_branch_id)),
    CHECK (transfer_type != 'intra_branch' OR NOT (source_location_id = destination_location_id AND (source_bin_id = destination_bin_id OR (source_bin_id IS NULL AND destination_bin_id IS NULL))))
);

CREATE INDEX idx_stock_transfers_source_branch ON stock_transfers(source_branch_id, status);
CREATE INDEX idx_stock_transfers_destination_branch ON stock_transfers(destination_branch_id, status);
CREATE INDEX idx_stock_transfers_status ON stock_transfers(status);

CREATE TABLE stock_transfer_items (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    transfer_id TEXT NOT NULL REFERENCES stock_transfers(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    variant_id TEXT REFERENCES product_variants(id) ON DELETE RESTRICT,
    batch_id TEXT REFERENCES product_batches(id) ON DELETE RESTRICT,
    serial_id TEXT REFERENCES serial_numbers(id) ON DELETE RESTRICT,
    quantity_milli INTEGER NOT NULL CHECK (quantity_milli > 0),
    received_quantity_milli INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (NOT (batch_id IS NOT NULL AND serial_id IS NOT NULL)),
    CHECK (serial_id IS NULL OR quantity_milli = 1000),
    CHECK (received_quantity_milli IS NULL OR received_quantity_milli = quantity_milli)
);

CREATE INDEX idx_stock_transfer_items_transfer ON stock_transfer_items(transfer_id);
CREATE INDEX idx_stock_transfer_items_product ON stock_transfer_items(product_id);

CREATE TRIGGER trg_stock_transfers_completed_receipt
BEFORE UPDATE OF status ON stock_transfers
FOR EACH ROW
WHEN NEW.status = 'completed' AND EXISTS (
    SELECT 1 FROM stock_transfer_items
    WHERE transfer_id = NEW.id AND received_quantity_milli IS NULL
)
BEGIN
    SELECT RAISE(ABORT, 'Cannot complete transfer with unreceived items');
END;
```

---

## 5. Explicit Exclusions & Protected Scope

The following areas are strictly outside the scope of F2.12:
- **Frontend UI:** All screens, buttons, and user interaction workflows are deferred to `F2.25: locations/transfers/adjustments UI`.
- **F2.13 Adjustments:** Multi-step approval workflows for inventory adjustments.
- **F2.14 Stock Count & Reconciliation:** Physical inventory counting and cycle count reconciliation.
- **Phase 3 Sales & POS:** Register checkout and sales orders (`src-tauri/src/commands/sales.rs` remains frozen).
- **Phase 4 Purchasing:** Supplier purchase orders and goods receipt notes (GRN).
- **Phase 6 Offline Sync:** Distributed multi-device transfer synchronization protocols.
- **Phase 10 Hardware:** Barcode scanners, printers, and scale protocols.
- **Protected Migrations:** Migrations 001–020 are immutable.

---

## 6. Known Risks & Mitigations

1. **Serial Branch Re-homing Consistency:**
   - *Risk:* Updating `serial_numbers.branch_id` across branches could violate foreign keys or fail if concurrent queries access the serial.
   - *Mitigation:* Re-homing executes strictly inside the atomic receive transaction, validating that the serial is in `transferred` status before assigning `destination_branch_id`.
2. **Concurrency / Double-Dispatch / Double-Receive:**
   - *Risk:* Two concurrent dispatch or receive requests on the same transfer document could double-mutate stock.
   - *Mitigation:* State-guarded SQL updates (`UPDATE stock_transfers SET status = 'in_transit' WHERE id = ? AND status = 'draft'` and `UPDATE stock_transfers SET status = 'completed' WHERE id = ? AND status = 'in_transit'`) ensure that exactly one execution succeeds.
3. **Insufficient Stock at Dispatch:**
   - *Risk:* Stock balance changed between draft creation and dispatch.
   - *Mitigation:* `StockLedgerService` enforces non-negative checks (`CHECK (quantity_milli >= 0)`). Any deduction exceeding available balance immediately fails and rolls back the entire transaction.
