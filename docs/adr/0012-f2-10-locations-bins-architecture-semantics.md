# ADR-0012 — F2.10 Locations / Bins Architecture & Semantics

Status: Accepted — User Approved
Date: 2026-09-04

---

## 1. Context

Phase 2 milestone `F2.10 — Locations / Bins` establishes the physical spatial topography and storage layout master data for POS Global.

In retail, wholesale, and warehouse operations, physical facilities are organized into discrete spatial zones and addressable storage slots:
- **At the facility/branch level**, physical storage is divided into macroscopic zones, departments, rooms, aisles, or warehouse bays.
- **At the granular storage level**, items are placed into addressable physical slots, shelves, racks, or bins where goods can be physically deposited and picked.
- **At the organizational level**, warehouse and store topography is strictly isolated by branch, preventing physical cross-branch contamination.

Following completed milestones `F2.01` through `F2.09` (which established catalog metadata, variants, weights, batches, serials/assets, and warranties), milestone `F2.10` establishes the foundational physical layout model. It provides the necessary master-data substrate for subsequent inventory ledger milestones (`F2.11` Stock Ledger, `F2.12` Transfers, `F2.13` Adjustments, `F2.14` Stock Count/Reconciliation, and `F2.15` Inventory Tests).

This document establishes the authoritative architectural decisions, entity models, hierarchy invariants, validation rules, authorization requirements, and subsystem boundaries governing `F2.10`.

---

## 2. Separation of Architectural Concerns

To maintain absolute architectural clarity and prevent spec drift, this ADR explicitly categorizes all architectural statements into five distinct tiers:

### A. Authoritative Existing Facts
1. **Branch Scoping Foundation (`001_initial.sql:12-19`):**
   - The `branches` table defines the physical operating sites of an organization (`id TEXT PRIMARY KEY`, `name TEXT`, `currency TEXT`, `is_active INTEGER DEFAULT 1`).
2. **Current Inventory Table Structure (`001_initial.sql:96-105`, `006_quantity_precision_hardening.sql:5-9`):**
   - The `inventory` table aggregates stock balances at `(branch_id, product_id, variant_id)` using `quantity_milli INTEGER`. It has no `location_id` or `bin_id` columns.
3. **Current Batches Table Structure (`016_batches_and_expiry.sql:32-46`):**
   - The `product_batches` table tracks batches per `(branch_id, product_id, variant_id)` with `quantity_milli INTEGER`. It has no `location_id` or `bin_id` columns.
4. **Current Serial Numbers Table Structure (`017_serial_imei_assets.sql:51-66`):**
   - The `serial_numbers` table tracks individual units per `(branch_id, product_id, variant_id)` with `serial_number`, `imei`, `asset_tag`, `status`, and `warranty_expires_at`. It has no `location_id` or `bin_id` columns.
5. **Existing Permission Catalog (`004_exact_money_and_identity.sql`, `src-tauri/src/permission/mod.rs:101-104`):**
   - `Permission::SettingsManage` (`"settings.manage"`) is seeded and granted by default to `Role::Admin` and `Role::Manager`, but denied to `Role::Cashier`.
   - `Permission::InventoryAdjust` (`"inventory.adjust"`) is dedicated to stock quantity adjustments.
   - `Permission::InventoryTransfer` (`"inventory.transfer"`) is dedicated to inventory transfers.
6. **Append-Only Migration Rule (`DATABASE_RULES.md:9-10`, `V2_RULES.md:60`):**
   - Applied migrations 001–018 are immutable. All database modifications for `F2.10` must be delivered strictly via Migration 019.

### B. Explicit Architectural Decisions (D1 / D2 / D3)
1. **Decision D1 — Discrete Two-Entity Model (Option 1A):**
   - The system introduces two distinct relational entities: `locations` (zones/areas) and `bins` (slots/shelves).
   - Bins are NOT modeled as generic `locations` rows with `location_type = 'bin'`.
2. **Decision D2 — Defer Serial and Batch Linkage (Option 2A):**
   - `F2.10` is strictly a master-data milestone and MUST NOT modify `serial_numbers` or `product_batches`.
   - Migration 019 must NOT add `location_id` or `bin_id` columns to `serial_numbers` or `product_batches`. Spatial stock attribution belongs to `F2.11+`.
3. **Decision D3 — Master-Data Authorization via `Permission::SettingsManage` (Preferred Option 3B):**
   - Location and bin creation, modification, and deactivation are treated as branch facility configuration and master-data management, authorized strictly by `Permission::SettingsManage`.
   - `F2.10` does NOT invent new unseeded permissions and does NOT use an `OR` model with `InventoryAdjust`.

### C. Directly Implied Invariants
1. **Same-Branch Hierarchy:** A parent location and all its child locations MUST share the identical `branch_id`. Cross-branch parenting is strictly impossible.
2. **Acyclic Hierarchy (DAG):** Self-parenting (`parent_id == id`) and transitive cyclic relationships are strictly forbidden.
3. **Scoped Code Uniqueness:**
   - Location `code` must be unique per branch (`UNIQUE(branch_id, code COLLATE NOCASE)`).
   - Bin `code` must be unique per parent location (`UNIQUE(location_id, code COLLATE NOCASE)`).
4. **String Sanitization:** Leading/trailing whitespace must be trimmed; empty or whitespace-only names and codes must be rejected.
5. **Inventory Boundary:** `F2.10` must NOT alter `inventory.quantity_milli`, write to `stock_movements`, or calculate stock balances.
6. **Physical Reality Only:** Locations in `F2.10` represent tangible, physical storage spaces; virtual or transit locations are strictly excluded.

### D. Recommendations (Architectural Guidance, Not Requirements)
1. **Implementation Traversal Limit:** A defensive limit of 50 steps during cycle detection is an engineering safety mechanism to guard against runaway loops on corrupted legacy data (matching `category/mod.rs`), not a business invariant.
2. **Open Classification:** If classification is required, use an open text field `location_type TEXT NULL` rather than an enum or closed set of hardcoded values.
3. **Deactivation Cascading Guard:** Before deactivating a location, verify that no active child locations or active bins exist.

### E. Deferred Future Scope
1. Default putaway bin configuration on `products` or `product_variants`.
2. Spatial coordinates (3D dimensions, X/Y/Z, pick sequence routes).
3. Hazardous material, environmental, or temperature zone classifications.
4. Virtual in-transit locations for inter-branch transfers (deferred to `F2.12`).
5. Real-time stock balance tracking per location or bin (deferred to `F2.11`).

---

## 3. Location Semantics

A **Location** represents a macroscopic physical storage area, zone, room, aisle, department, or warehouse bay within a branch.

1. **Branch Confinement:** A location exists exclusively within the context of a single physical branch (`branch_id`). It cannot span multiple branches.
2. **Tangible Physical Space:** A location represents a physical boundary where inventory may be stored. It is neither a financial account nor a virtual transfer bucket.
3. **Hierarchical Nesting:** Locations can optionally form a parent-child hierarchy within the same branch (e.g., `Main Warehouse` $\to$ `Aisle 2` $\to$ `Rack B`).
4. **Open Classification:** Locations do not enforce a closed enum for type. If callers supply an optional category or type tag, it is treated as open text. Arbitrary classifications (such as `sales_floor`, `backroom`, `quarantine`) are NOT hardcoded into the schema.

---

## 4. Bin Semantics

A **Bin** represents a specific, addressable pick/put storage compartment, shelf partition, slot, or drawer contained within a Location.

1. **Location Confinement:** Every bin belongs to exactly one parent location (`location_id`).
2. **Terminal Storage Slot:** Bins are leaf-level addressable slots designed for physical putaway and picking. Bins do not have sub-bins or child bins.
3. **Addressable Identity:** A bin is uniquely identified within its parent location by its code (e.g., `A-01`, `BIN-104`, `TOP-SHELF`).
4. **Branch Derivation:** A bin derives its branch association directly from its parent location. A bin cannot be reassigned across locations belonging to different branches.

---

## 5. Entity Model Decision (D1 — Option 1A)

### Approved Decision
The system adopts **Option 1A: Discrete Two-Entity Model**, establishing separate relational tables:
1. `locations` for macroscopic physical zones and hierarchical areas.
2. `bins` for addressable pick/put slots belonging to a location.

```
+-------------------------------------------------------------+
|                         branches                            |
+-------------------------------------------------------------+
                              | 1
                              |
                              | *
+-------------------------------------------------------------+
|                        locations                            |
|-------------------------------------------------------------|
| id: TEXT PRIMARY KEY                                        |
| branch_id: TEXT NOT NULL (FK -> branches)                   |
| parent_id: TEXT NULL (FK -> locations)                      |
| name: TEXT NOT NULL                                         |
| code: TEXT NOT NULL                                         |
| location_type: TEXT NULL                                    |
| is_active: INTEGER NOT NULL DEFAULT 1                       |
| created_at: TEXT NOT NULL                                   |
| updated_at: TEXT NOT NULL                                   |
+-------------------------------------------------------------+
                              | 1
                              |
                              | *
+-------------------------------------------------------------+
|                          bins                               |
|-------------------------------------------------------------|
| id: TEXT PRIMARY KEY                                        |
| location_id: TEXT NOT NULL (FK -> locations)                |
| name: TEXT NOT NULL                                         |
| code: TEXT NOT NULL                                         |
| is_active: INTEGER NOT NULL DEFAULT 1                       |
| created_at: TEXT NOT NULL                                   |
| updated_at: TEXT NOT NULL                                   |
+-------------------------------------------------------------+
```

### Prohibited Model
The system explicitly rejects modeling bins as rows in `locations` with `location_type = 'bin'` (Single-Table Model).

### Rationale
1. **Structural and Semantic Clarity:** Locations represent spatial areas that support tree nesting; bins represent terminal physical pick/put slots that belong to a single zone.
2. **Foreign Key Precision:** In future milestones (`F2.11+`), stock allocations, transactions, or pick lists can reference a specific `bin_id` with absolute referential integrity (`REFERENCES bins(id)`), without needing conditional triggers or polymorphic check constraints.
3. **Index Efficiency:** Clean compound unique constraints (`UNIQUE(branch_id, code)` on locations; `UNIQUE(location_id, code)` on bins) avoid partial index complexity or nullable column compromises.

---

## 6. Hierarchy Semantics

The `locations` table supports an optional recursive parent-child hierarchy via composite foreign key `FOREIGN KEY (parent_id, branch_id) REFERENCES locations(id, branch_id) ON DELETE RESTRICT`.

1. **Root Locations:** A location with `parent_id IS NULL` is a root-level storage zone within that branch (e.g., `Main Warehouse`, `Store Floor`).
2. **Child Locations:** A location with a non-null `parent_id` is a sub-zone of the specified parent (e.g., `Aisle 1` child of `Main Warehouse`).
3. **Same-Branch Invariant:** A child location MUST belong to the exact same `branch_id` as its parent. This invariant is enforced both at the database layer via composite foreign key `(parent_id, branch_id) REFERENCES locations(id, branch_id)` (supported by unique index `idx_locations_id_branch_id`), and at the domain service layer.
4. **No Arbitrary Business Depth:** The business domain imposes NO artificial depth limit (such as a maximum depth of 5). Organizations may structure their physical spaces as shallow or as deep as their facility requires.
5. **Terminal Bins:** Bins do NOT support recursive hierarchy. All bins are immediate children of a location.

---

## 7. Cycle Prevention

To guarantee that the location tree forms a valid Directed Acyclic Graph (DAG), the domain engine enforces cycle prevention during all creation and parenting operations:

1. **Immediate Self-Parenting Rejection:**
   A location cannot be its own parent:
   $$\text{target\_parent\_id} \neq \text{location\_id}$$
   Attempts to assign `parent_id = id` must be rejected immediately with a `SelfParenting` domain error.

2. **Transitive Cycle Detection (Ancestor Walk):**
   When assigning or updating a location's `parent_id`, the system must traverse the ancestor chain starting from `target_parent_id`:
   - If `location_id` is encountered at any point in the ancestor chain, the operation must be rejected with a `CycleDetected` domain error:
     $$\text{"Location 'X' cannot be parented under its own descendant 'Y'"}$$
   - If the root is reached (`parent_id IS NULL`), the hierarchy is valid and acyclic.

3. **Implementation Traversal Safety Mechanism:**
   To guard against infinite loops caused by corrupted data or unforeseen database anomalies, the traversal algorithm must implement a defensive safety bound (`MAX_DEFENSIVE_STEPS: usize = 50`), matching the established pattern in `category/mod.rs`. If the safety bound is reached, the operation fails closed with a traversal safety error. This bound is an implementation safeguard, not a business depth rule.

---

## 8. Branch Isolation

Tenancy in POS Global is strictly partitioned by Organization and Branch:

1. **Absolute Branch Isolation:** Locations are physical facility entities tied to a specific branch (`locations.branch_id`).
2. **Forbidden Cross-Branch Parenting:** A location in Branch A cannot have a parent in Branch B. This invariant must be verified prior to inserting or updating `parent_id`.
3. **Indirect Bin Isolation:** Bins inherit branch isolation directly through their mandatory foreign key to `locations`. A bin cannot belong to a location in another branch.
4. **Tenant Scoping & Anti-Leakage:** All queries, lookups, and mutations must validate session branch/organization boundaries. If an authenticated user attempts to access or mutate a location or bin belonging to another branch, the system must fail closed with a scope mismatch or not-found error without leaking existence.

---

## 9. Code and Name Validation

The system enforces strict, evidence-backed input validation across all location and bin attributes:

1. **Sanitization and Normalization:**
   - All string inputs (`name`, `code`, `location_type`) must be trimmed of leading and trailing whitespace.
2. **Non-Empty Invariants:**
   - `name` cannot be empty or whitespace-only.
   - `code` cannot be empty or whitespace-only.
3. **Case-Insensitive Uniqueness:**
   - **Locations:** `code` must be unique per branch, evaluated case-insensitively:
     $$\text{UNIQUE}(\text{branch\_id}, \text{code COLLATE NOCASE})$$
   - **Bins:** `code` must be unique per parent location, evaluated case-insensitively:
     $$\text{UNIQUE}(\text{location\_id}, \text{code COLLATE NOCASE})$$
4. **No Arbitrary Whitelist Restriction:**
   - The system shall NOT enforce an arbitrary character whitelist regex (such as `^[a-zA-Z0-9._/-]+$`).
   - Physical warehouse labels often use spaces, accented letters, non-Latin scripts, dots, slashes, or hashes (e.g., `Bin #12`, `Zone A/B`, `رف 1`). Any valid Unicode string that is trimmed and non-empty is permitted.

---

## 10. Lifecycle and Deactivation

Locations and bins follow the platform's standardized soft-lifecycle architecture:

1. **Active Flag:**
   - `locations.is_active INTEGER NOT NULL DEFAULT 1`
   - `bins.is_active INTEGER NOT NULL DEFAULT 1`
2. **Deactivation Semantics:**
   - Deactivating a location or bin preserves all historical associations while preventing future stock allocation or operational assignment.
   - **Active Children Guard:** A location cannot be deactivated if it has active child locations or active bins. Callers must deactivate child elements first.
3. **Hard Deletion Restrictions:**
   - Foreign key constraints use `ON DELETE RESTRICT`. A location or bin cannot be deleted from the database if related records (child locations, bins, or future stock ledger records) reference it.
   - Administrative workflows must favor soft deactivation over physical database deletion.

---

## 11. Authorization (D3 — Option 3B)

### Approved Decision
The system adopts **Preferred Option 3B: Master-Data Authorization via `Permission::SettingsManage`**.

### Forensic Evidence & Permission Verification
1. **Catalog Integrity:** `Permission::SettingsManage` (`"settings.manage"`) is seeded in the core system catalog (`004_exact_money_and_identity.sql`, `src-tauri/src/permission/mod.rs:101-104`).
2. **Role Mapping:**
   - `Role::Admin`: Granted `Permission::ALL`, which includes `SettingsManage`.
   - `Role::Manager`: Explicitly granted `SettingsManage` (`src-tauri/src/permission/mod.rs:276`).
   - `Role::Cashier`: Denied `SettingsManage` (`src-tauri/src/permission/mod.rs:278-282`).
3. **Semantic Alignment:** Setting up store layout, aisles, racks, and bins is an administrative facility configuration task, perfectly aligned with `SettingsManage`.
4. **Rejection of Premature Permissions:** No new, unseeded permission (such as `locations.manage` or `warehouse.manage`) shall be invented.
5. **Rejection of OR Condition:** The system shall NOT use `InventoryAdjust OR SettingsManage`. `InventoryAdjust` is specifically reserved for stock quantity adjustments (shrinkage, physical count variances, damage write-offs) and must not grant authority to modify physical facility topography.

### Session Enforcement
All Tauri IPC commands for `F2.10` must validate the caller's active session, resolve the authenticated `branch_id`, and verify `Permission::SettingsManage` via `require_permission(conn, session_id, Permission::SettingsManage)`.

---

## 12. Inventory and Ledger Boundary

Milestone `F2.10` is strictly restricted to **master data and physical topography**:

1. **Prohibited Quantity Modifications:**
   - `F2.10` MUST NOT modify `inventory.quantity_milli`.
   - `F2.10` MUST NOT insert or modify records in `stock_movements`.
2. **Prohibited Stock Balances:**
   - `F2.10` does NOT calculate, store, or track on-hand balances at the location or bin level.
3. **Prohibited Transactional Workflows:**
   - Transfers between locations (deferred to `F2.12`).
   - Quantity adjustments (deferred to `F2.13`).
   - Physical inventory counts and reconciliation (deferred to `F2.14`).
4. **Ownership:**
   - Spatial inventory tracking, bin-level stock ledgers, and location-attributed movements belong strictly to `F2.11` (Stock Ledger) and subsequent Phase 2 milestones.

---

## 13. Migration 019 Implications (D2 — Option 2A)

### Approved Scope of Migration 019
Migration 019 shall create only the tables, indexes, and constraints necessary for location and bin master data:
1. `CREATE TABLE locations (...)` with foreign key to `branches(id)` and composite foreign key `(parent_id, branch_id) REFERENCES locations(id, branch_id) ON DELETE RESTRICT`.
2. `CREATE TABLE bins (...)` with foreign key to `locations(id)`.
3. Unique index `idx_locations_id_branch_id` on `locations(id, branch_id)` supporting composite foreign key referential integrity.
4. Case-insensitive unique indexes on `locations(branch_id, code COLLATE NOCASE)` and `bins(location_id, code COLLATE NOCASE)`.
5. Foreign key lookup indexes on `locations(parent_id, branch_id)` and `bins(location_id)`.

### Strict Deferral of Serial and Batch Linkage (Decision D2)
- **Migration 019 MUST NOT alter `serial_numbers` or `product_batches`.**
- Columns such as `location_id` or `bin_id` must NOT be added to `serial_numbers` or `product_batches` in Migration 019.
- **Rationale:** The authoritative requirements define `F2.10` as master data only. Adding unpopulated foreign key columns to transactional tracking tables before the spatial stock ledger (`F2.11`) is designed creates dead schema state and violates the incremental architecture contract.

---

## 14. Compatibility with F2.01–F2.09

`F2.10` introduces zero breaking changes to existing capabilities:
- **Product Catalog (F2.01–F2.05):** Products, categories, brands, manufacturers, units, conversions, barcodes, and matrix variants remain completely unaffected.
- **Weighted Products (F2.06):** Scale integration and weight milligram precision operate independently of physical bin master data.
- **Batches & Expiry (F2.07):** `product_batches` maintains its Migration 016 schema without alteration.
- **Serial, IMEI & Assets (F2.08):** `serial_numbers` maintains its Migration 017 schema without alteration.
- **Warranties (F2.09):** Warranty terms and instance expiration calculations operate independently of storage locations.

---

## 15. Boundaries with Future Milestones (F2.11–F2.15)

The inventory subsystem responsibilities are strictly partitioned:

| Milestone | Scope & Responsibilities | Boundary with F2.10 |
| :--- | :--- | :--- |
| **F2.10 Locations / Bins** | Physical topography master data (zones, aisles, bins, hierarchy, validation, codes). | Master-data only; zero quantity tracking. |
| **F2.11 Stock Ledger** | Spatial stock ledger, bin-level on-hand balances, location/bin linkage for batches and serials. | Consumes `locations` and `bins` as FK targets; owns spatial stock movements. |
| **F2.12 Transfers** | Inter-branch and intra-branch inventory transfers, transit status, transfer receipts. | Moves stock between locations/bins; manages virtual transit states. |
| **F2.13 Adjustments** | Stock adjustments, write-offs, damages, shrinkage reasons. | Adjusts stock balances within specific locations/bins. |
| **F2.14 Stock Count / Reconciliation** | Physical inventory stock counts, barcode scanning per bin, reconciliation diffs. | Audits physical counts against bin balances; triggers adjustment entries. |
| **F2.15 Inventory Tests** | Comprehensive end-to-end integration test suite across all Phase 2 inventory modules. | Validates complete lifecycle from bin definition to stock reconciliation. |

---

## 16. Protected Future Scope

The following features are explicitly deferred to future milestones or phases:
1. **Default / Putaway Bins:** Assigning default bins to products or variants is deferred. No putaway fields shall be added to `products` or `product_variants` in `F2.10`.
2. **Pick Path / Routing Optimization:** Sequence numbers, 3D coordinates ($X, Y, Z$), and routing algorithms for warehouse pickers are deferred to advanced warehouse extensions.
3. **Environmental / Hazard Attributes:** Temperature controls (cold storage), humidity controls, and hazardous materials classifications are deferred.
4. **Virtual / In-Transit Locations:** Dynamic virtual locations (such as goods-in-transit or vendor dropship) are deferred to `F2.12 Transfers`.

---

## 17. Consequences

### Positive Consequences
- **Clean Relational Design:** A discrete two-entity model (`locations` and `bins`) models warehouse and store reality accurately without polymorphism.
- **Zero Disruption to Existing Inventory:** Completely decouples physical layout definition from active stock quantities, ensuring existing sales and stock queries continue uninterrupted.
- **Strict Tenancy & DAG Safety:** Guaranteed branch isolation and cycle-free location hierarchies prevent data corruption and cross-branch data leakage.
- **Appropriate Master-Data Security:** Aligns authorization with the existing `Permission::SettingsManage` pattern without inventing unseeded permissions.

### Negative & Neutral Trade-Offs
- **Two-Step Integration:** Bins and locations will exist as empty master-data containers until `F2.11` connects them to the inventory ledger and tracking tables.
- **Future Migration for Serials/Batches:** A subsequent migration (`020+`) in `F2.11` will be required to introduce `location_id`/`bin_id` to `serial_numbers`, `product_batches`, and the stock ledger.

---

## 18. Revisit Triggers

This architecture shall be revisited only if one of the following authoritative triggers occurs:
1. **Virtual Location Requirement:** If `F2.12 Transfers` conclusively demonstrates that in-transit stock cannot be tracked via transfer documents and requires virtual location rows in the database.
2. **Multi-Level Bin Nesting:** If an authoritative business requirement requires bins to contain sub-bins (e.g. modular drawer dividers).
3. **Cross-Branch Shared Warehouses:** If multi-tenant or multi-branch logistics requirements authorize sharing a single physical warehouse across multiple branches.
4. **Product-Level Putaway Automation:** If commercial catalog requirements prioritize default putaway bin configurations directly on product master data prior to Phase 3.