# ADR-0010 — F2.08 Serial, IMEI & Tracked Assets Architecture & Semantics

Status: Accepted
Date: 2026-09-03

---

## 1. Context

Phase 2 milestone `F2.08 — Serial / IMEI / Assets` establishes the backend domain model, database migration specification, validation invariants, and Tauri IPC interfaces for tracking discrete, unit-level inventory items in POS Global.

Serialized tracking differs fundamentally from bulk or batch tracking:
- A batch or bulk inventory record tracks a continuous or aggregated quantity of interchangeable goods (e.g. 50 kg of rice or 200 boxes of tiles).
- A serialized record tracks an individual, unique physical instance (e.g. a specific laptop, a smartphone with an IMEI, or a tracked power tool with an asset tag), where each recorded instance represents exactly one physical unit.

During the pre-implementation discovery and architectural reconciliation for F2.08, the authoritative repository corpus was audited against legacy schema definitions, capability matrices, domain contracts, and milestone dependencies. This ADR records the approved architectural decisions governing F2.08 execution.

---

## 2. Authoritative Existing Facts vs Explicit Decisions Approved Now

To maintain complete architectural integrity, this document explicitly delineates existing authoritative facts from new decisions approved for F2.08.

### A. Authoritative Existing Facts

1. **Legacy Schema Stub (`001_initial.sql:116-124`):**
   - An initial table `serial_numbers` was created in Migration 001 with the following schema:
     ```sql
     CREATE TABLE serial_numbers (
         id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
         product_id TEXT NOT NULL REFERENCES products(id),
         branch_id TEXT NOT NULL REFERENCES branches(id),
         serial_number TEXT NOT NULL UNIQUE,
         status TEXT NOT NULL DEFAULT 'in_stock',
         sold_in_sale_id TEXT,
         warranty_expires_at TEXT
     );
     ```
   - In this legacy stub, `serial_number` was defined as `TEXT NOT NULL UNIQUE`.
   - Columns for `imei`, `asset_tag`, `variant_id`, `cost_price_minor`, `created_at`, and `updated_at` were absent.
   - Zero application code in `src-tauri/` or `src/` currently accesses this table.
2. **Product Core Serial Flag (`001_initial.sql:62`, `src-tauri/src/product/mod.rs`):**
   - `products.requires_serial INTEGER NOT NULL DEFAULT 0` is an established, first-class column on the `products` table.
3. **Distinct Capabilities Seeded (`003_global_commerce_foundation.sql:125-127`):**
   - Independent capability codes were seeded in the platform:
     - `('SERIAL', 'Serial Number', 'product', 'Unique serial tracking')`
     - `('IMEI', 'IMEI', 'product', 'IMEI tracking')`
     - `('WARRANTY', 'Warranty', 'product', 'Warranty tracking')`
4. **Capability Composability (`DOMAIN_CONTRACTS.md:11`, `docs/FOUNDATION_GATE.md:22`):**
   - *"Variant/matrix, weighted, batch, expiry, serial, IMEI and warranty behavior are capabilities, not mutually exclusive global product categories."*
   - Capabilities are composable across all industry presets (`CAPABILITY_MATRIX.md:33`).
5. **Stock Movement Ledger Invariant (`DOMAIN_CONTRACTS.md:15`, `EXECUTION_PLAN_DETAILED.md:39`):**
   - *"Every quantity change creates a stock movement in the same atomic operation."*
   - Milestone `F2.11 (Stock Movement Ledger)` is sequenced after F2.08. F2.08 does not post to `stock_movements`.
6. **Append-Only Migration Policy (`DATABASE_RULES.md:9-10`, `SCHEMA.md:31`):**
   - Applied migrations 001–016 are immutable. The schema evolution for F2.08 must be delivered exclusively via Migration 017.

---

### B. Explicit Architectural Decisions Approved for F2.08

#### Decision D.1 — Flexible Triple-Identifier Model (Option B)
- **Approved Decision:** An instance-level tracked record supports three distinct, optional identifier attributes: `serial_number`, `imei`, and `asset_tag`.
- **Nullability:**
  - `serial_number` is **nullable** (`TEXT NULL`).
  - `imei` is **nullable** (`TEXT NULL`).
  - `asset_tag` is **nullable** (`TEXT NULL`).
- **Presence Constraint:** At least **ONE** identifier must be present on every record:
  ```sql
  CHECK (
      serial_number IS NOT NULL
      OR imei IS NOT NULL
      OR asset_tag IS NOT NULL
  )
  ```
- **Rationale:** F2.08 models three distinct identifier categories (manufacturer serial, cellular telecommunications IMEI, and merchant asset tag). Forcing a non-null `serial_number` would require merchants to fabricate dummy serial numbers for IMEI-only items (e.g. mobile devices recorded only by IMEI) or internal assets (e.g. tools identified solely by an organization asset tag).
- **Provenance:** This is an explicit **new** architectural decision for F2.08, modifying the legacy `NOT NULL` constraint from `001_initial.sql`.

#### Decision D.2 — Single IMEI Model (Option A)
- **Approved Decision:** The schema and domain engine define exactly one IMEI attribute: `imei TEXT NULL`.
- **Prohibition:** No `imei2`, `dual_imei`, or secondary IMEI columns are introduced in F2.08.
- **Rationale:** Authoritative project sources (`003_global_commerce_foundation.sql:126`, `ARCHITECTURE.md:117`, `BACKLOG.md:121`) specify singular IMEI tracking. No dual-IMEI requirement exists in the shared core contracts. Dual-SIM or accessory handling belongs to future specialized industry workflows if ever prioritized.

#### Decision D.3 — Global Serial Uniqueness (Option A)
- **Approved Decision:** When `serial_number` is present, it is **globally unique** across the table, evaluated case-insensitively (`COLLATE NOCASE`).
- **Partial Unique Index:**
  ```sql
  CREATE UNIQUE INDEX idx_serial_numbers_serial_active
      ON serial_numbers(serial_number COLLATE NOCASE)
      WHERE serial_number IS NOT NULL;
  ```
- **Rationale:** Preserves the strongest legacy integrity contract from `001_initial.sql:120` (`serial_number TEXT NOT NULL UNIQUE`). While F2.08 makes `serial_number` optional to accommodate IMEI-only and asset-only items (Decision D.1), it **does not relax uniqueness** when a serial number is provided.
- **Provenance:** The global uniqueness scope is an explicit architectural decision for F2.08. It avoids duplicate serial registrations across products in the local database.

---

## 3. Detailed Semantics of the Three Identifiers

F2.08 explicitly distinguishes between three instance-level identifiers:

1. **Serial Number (`serial_number`):**
   - An alphanumeric identifier assigned by a manufacturer or merchant to uniquely designate a single manufactured physical unit.
   - Evaluated case-insensitively (`COLLATE NOCASE`).
   - Globally unique across all tracked instances when present.
   - Trimmed string, max 100 characters.
2. **IMEI (`imei`):**
   - International Mobile Equipment Identity: a standardized 15-digit decimal string identifying cellular communications hardware.
   - Validated at the domain boundary:
     - Exactly 15 decimal digits (`^[0-9]{15}$`).
     - Must pass the standard **Luhn checksum algorithm** (Mod 10). Invalid checksums are rejected fail-closed.
   - Globally unique across all tracked instances when present (`WHERE imei IS NOT NULL`).
3. **Asset Tag (`asset_tag`):**
   - An internal organizational identifier assigned by a merchant to track capital assets, rental items, or internal store equipment.
   - Scoped to the branch or organization: unique per branch when present (`UNIQUE (branch_id, asset_tag COLLATE NOCASE) WHERE asset_tag IS NOT NULL`).
   - Trimmed string, max 100 characters.

---

## 4. Capability Composability & Enablement Rules

### A. Capability Evaluation
A product is eligible to hold serialized instance records if and only if it satisfies the serial tracking capability rule:
$$\text{is\_serial\_tracked}(P) \iff \text{products.requires\_serial} = 1 \lor \text{has\_capability}(P, \text{'SERIAL'}) \lor \text{has\_capability}(P, \text{'IMEI'})$$

Attempting to register a serialized instance for a product that does not satisfy this condition is rejected with a strongly-typed domain validation error.

### B. Weighted and Serial Coexistence
- In accordance with `DOMAIN_CONTRACTS.md:11` (*"capabilities, not mutually exclusive global product categories"*) and `docs/FOUNDATION_GATE.md:22` (*"Matrix, weighted, batch, expiry, serial/IMEI and warranty capabilities are composable"*), the platform architecture **does not forbid** a product from having both weighted and serial capabilities.
- While standard retail rarely serializes variable-weight goods, the core engine does not hardcode an artificial mutual exclusion. Each capability independently governs its respective domain invariants.

### C. Variant Compatibility
- In accordance with `EXECUTION_PLAN_DETAILED.md:41` (*"variant/weight/batch/serial combinations work without hardcoded industry forks"*), serialized instances support optional variant association via `variant_id REFERENCES product_variants(id)`.
- If a product has active variants, an instance may be associated with a specific variant SKU. When provided, `variant_id` is validated to ensure it belongs to the target `product_id`.

---

## 5. Quantity, Inventory & Ledger Boundaries

F2.08 adheres strictly to the quantity and ledger boundary established across Phase 2:

1. **Invariant Unit Quantity:** Every tracked instance represents exactly **one physical unit** (`1000` milli-units). A serial number cannot represent fractional or multi-unit quantities.
2. **Zero Mutation of Inventory Balances:** F2.08 does **NOT** decrement or increment `inventory.quantity_milli`.
3. **Zero Movement Ledger Posting:** F2.08 does **NOT** insert rows into `stock_movements`.
4. **Architectural Rationale:**
   - Double-entry stock movements and inventory balance updates belong strictly to **F2.11 (Stock Movement Ledger)**.
   - Physical count reconciliation between registered active serials and cached balance quantities belongs strictly to **F2.14 (Stock Count & Reconciliation)**.
   - Checkout deduction belongs strictly to **Phase 3 (Sales Checkout)**.
   - F2.08 is the instance identity, validation, and lifecycle registry only.

---

## 6. Lifecycle Status & State Transitions

A tracked instance maintains an explicit operational status:
- Allowed statuses:
  - `in_stock`: Available at the branch for normal operations.
  - `reserved`: Held for a pending customer order or quote.
  - `sold`: Dispatched/sold to a customer (historical reference to `sold_in_sale_id`).
  - `transferred`: Transferred to another branch or location.
  - `defective`: Flagged as damaged, defective, or awaiting repair.
  - `recalled`: Subject to manufacturer or safety recall (terminal state).
  - `disposed`: Written off, scrapped, or decommissioned (terminal state).
- Status updates must be validated through the domain engine (`update_serial_status`), enforcing valid transition paths and preventing mutations on terminal records.

---

## 7. Security, Tenancy & Authorization

1. **Branch Tenancy Boundary:** Every serialized record is tied to a specific branch (`branch_id REFERENCES branches(id)`).
2. **Scoped Mutation Permission:** Creating or updating a serial instance requires the caller to hold `Permission::InventoryAdjust` (`code: 'inventory.adjust'`) scoped to the target branch.
3. **Existence-Leakage Protection:** Queries and lookups enforce branch-scoped authorization via `AuthorizeRequest`. If an instance exists but belongs to a branch inaccessible to the caller's session, the system returns a fail-closed error: `Serial instance not found or inaccessible for this session`. Entity existence is never leaked across tenant or branch boundaries.

---

## 8. Migration 017 Implications (Documentation Only)

Migration 017 will evolve the legacy `serial_numbers` table from `001_initial.sql`.

### A. Required Schema Evolution
To safely transition `serial_numbers` to the approved F2.08 architecture, Migration 017 must:
1. Rebuild the table (following the verified Migration 016 table-rebuild pattern) to make `serial_number TEXT NULL`.
2. Add and retain strictly Category A/B columns:
   - `variant_id TEXT REFERENCES product_variants(id)`
   - `imei TEXT`
   - `asset_tag TEXT`
   - `cost_price_minor INTEGER CHECK (cost_price_minor IS NULL OR cost_price_minor >= 0)`
   - `sold_in_sale_id TEXT` (retained passive nullable historical reference for Phase 3 compatibility)
   - `warranty_expires_at TEXT` (retained passive nullable historical reference for F2.09 compatibility)
   - `created_at TEXT NOT NULL DEFAULT ''`
   - `updated_at TEXT NOT NULL DEFAULT ''`
   *(Note: Category C field `notes` is explicitly excluded to maintain strict scope firewalls).*
3. Enforce the triple-identifier check constraint:
   `CHECK (serial_number IS NOT NULL OR imei IS NOT NULL OR asset_tag IS NOT NULL)`
4. Enforce the status check constraint:
   `CHECK (status IN ('in_stock', 'reserved', 'sold', 'transferred', 'defective', 'recalled', 'disposed'))`
5. Execute pre-rebuild fail-closed guard (`migration_017_guard`):
   - Aborts if any case-insensitive duplicate serial numbers exist (`COUNT(*) > 1` grouped by `trim(serial_number) COLLATE NOCASE`).
   - Aborts if any legacy row contains an empty or whitespace-only serial (`length(trim(serial_number)) = 0`). An empty string is not a valid identifier.
   - Aborts if any legacy row contains an invalid status not in the allowed whitelist.
   - Aborts if any orphaned `product_id` or `branch_id` exists.
6. Copy and Normalization Semantics:
   - For valid legacy rows, leading and trailing whitespace is trimmed: `trim(serial_number)`. This is an intentional normalization decision to eliminate spurious whitespace anomalies from legacy input while preserving the exact canonical alphanumeric identifier.
7. Create partial unique indexes:
   - `idx_serial_numbers_serial_active` ON `serial_numbers(serial_number COLLATE NOCASE) WHERE serial_number IS NOT NULL`
   - `idx_serial_numbers_imei_active` ON `serial_numbers(imei) WHERE imei IS NOT NULL`
   - `idx_serial_numbers_asset_tag_branch` ON `serial_numbers(branch_id, asset_tag COLLATE NOCASE) WHERE asset_tag IS NOT NULL`
8. Create lookup indexes on `(branch_id, product_id, status)` and `(branch_id, product_id, variant_id, status)`.
9. Preserve any existing legacy rows and backfill audit timestamps deterministically.

### B. Implementation Boundary Notice
> **IMPORTANT:** This section serves as documentation of the architectural specification for Migration 017. **NO SQL file is created and NO database changes are executed under this ADR.**

---

## 9. Scope Firewalls

The following boundaries are strictly enforced and protected against scope creep:

| Milestone / Subsystem | Boundary Invariant | Strictly Forbidden in F2.08 |
| :--- | :--- | :--- |
| **F2.09 Warranty** | Warranty terms, duration calculation, and claims | Computing warranty expiration dates, managing warranty claims |
| **F2.10 Locations / Bins** | Warehouse bin/shelf location tracking | Assigning serials to specific bin IDs or location hierarchies |
| **F2.11 Stock Ledger** | Double-entry inventory movement ledger | Inserting rows into `stock_movements`, altering valuation ledgers |
| **F2.12 Transfers** | Inter-branch transfer workflows | Transfer order generation, transit manifests |
| **F2.13 Adjustments** | Inventory write-offs and variance | Generating inventory adjustment movements |
| **F2.14 Reconciliation** | Physical count & reconciliation | Count variance reporting between physical serials and ledger totals |
| **F2.19 Barcode Scanning** | GS1-128 / DataMatrix AI (21) parsing | Raw scanner hardware parsing, Application Identifier decomposition |
| **F2.24 Serial UI** | Frontend React user interface | Writing or modifying React components, dialogs, forms, or views |
| **Phase 3 Sales** | POS checkout, tender, receipts | Checkout cart serial selection, deduction upon sale completion |
| **Phase 4 Purchasing** | Purchase Order Receiving (GRN) | Generating purchase receipt documents, vendor intake workflows |
| **Phase 7 Industry** | Repair service tickets, Rental contracts | Repair ticket creation (`F7.11`), rental booking calendar (`F7.12`) |
| **Phase 10 Hardware** | Device drivers for scale / scanner | Scanner device drivers, USB/RS232 serial communication |

---

## 10. Consequences

### Positive
- Accommodates modern retail electronics (IMEI tracking), internal tools (Asset Tag tracking), and standard manufactured products (Serial tracking) in a unified, normalized table structure without dummy data.
- Prevents invalid cellular equipment entry by enforcing standard 15-digit Luhn algorithm verification.
- Preserves full tenancy isolation, auditability, and backwards compatibility with existing Phase 0/1/2 contracts.
- Strictly protects the stock movement ledger from unverified out-of-order mutations.

### Negative / Trade-offs
- Table rebuild required in Migration 017 because SQLite cannot alter a column from `NOT NULL` to `NULL` in-place.
- Global uniqueness on `serial_number` means two different manufacturers producing identical serial numbers cannot be entered under the same database without prefixing or qualification.

---

## 11. Revisit Triggers

This architecture shall be revisited only if:
1. An explicit business requirement is approved for dual-SIM mobile devices requiring simultaneous tracking of primary and secondary IMEIs (`imei` + `imei2`).
2. An industry requirement demands relaxing serial number uniqueness from global catalog scope to strictly product-scoped `(product_id, serial_number)`.
3. Phase 7 Rental or Service modules introduce separate external asset registers that necessitate foreign-key linkage beyond the inventory asset tag.
