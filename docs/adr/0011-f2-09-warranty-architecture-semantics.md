# ADR-0011 — F2.09 Warranty Architecture & Semantics

Status: Accepted
Date: 2026-09-03

---

## 1. Context

Phase 2 milestone `F2.09 — Warranty` establishes the backend domain model, calculation semantics, validation invariants, and Tauri IPC interfaces for warranty tracking in POS Global.

In retail and commercial systems, warranty coverage bridges product catalog definitions and physical instance lifecycles:
- At the **product level**, a product defines commercial warranty terms (e.g. 12 months or 24 months standard warranty).
- At the **instance level**, a physical tracked/serialized instance identified by one or more of `serial_number`, `imei`, or `asset_tag` carries an exact expiration date computed from its activation or purchase date.
- At the **sales level**, non-serialized items carry warranty terms linked to proof-of-purchase transactions.

Following completed milestones `F2.01` through `F2.08` (where `017_serial_imei_assets.sql` preserved the historical `warranty_expires_at` column), milestone `F2.09` operationalizes warranty tracking within Phase 2. This ADR documents the authoritative architectural decisions governing `F2.09`.

---

## 2. Authoritative Existing Facts vs Explicit Decisions Approved Now

To maintain engineering continuity and prevent architectural drift, this document explicitly separates preexisting repository facts from the decisions authorized for `F2.09`.

### A. Authoritative Existing Facts

1. **Legacy Schema Definitions (`001_initial.sql:63, 123`):**
   - In `products`: `warranty_months INTEGER` defines the standard product warranty duration in months.
   - In `serial_numbers`: `warranty_expires_at TEXT` stores the instance-level warranty expiration date.
2. **Table Rebuild Retention (`017_serial_imei_assets.sql:62, 71`):**
   - Migration 017 rebuilt `serial_numbers` to support the flexible triple-identifier model (ADR-0010) and explicitly retained `warranty_expires_at TEXT` for future `F2.09` compatibility.
3. **Core Warranty Capability Seeded (`003_global_commerce_foundation.sql:127`):**
   - The capability `('WARRANTY', 'Warranty', 'product', 'Warranty tracking')` was seeded as an independent primitive in `capabilities`.
4. **Capability Composability (`DOMAIN_CONTRACTS.md:11`, `docs/FOUNDATION_GATE.md:22`):**
   - *"Variant/matrix, weighted, batch, expiry, serial, IMEI and warranty behavior are capabilities, not mutually exclusive global product categories."*
   - Warranty capability composes cleanly with `SERIAL`, `IMEI`, `BATCH`, `WEIGHT`, and `MATRIX` across all industry presets (`CAPABILITY_MATRIX.md:15, 33`).
5. **Product Domain Models (`src-tauri/src/product/mod.rs`):**
   - `Product`, `CreateProductInput`, and `UpdateProductInput` structs include `pub warranty_months: Option<i32>`.
6. **Serial Domain Model (`src-tauri/src/serial/mod.rs`):**
   - `SerializedInstance` includes `pub warranty_expires_at: Option<String>`.
7. **Append-Only Migration Rule (`DATABASE_RULES.md:9-10`, `V2_RULES.md:60`):**
   - Applied migrations 001–017 are immutable. Any database modifications for `F2.09` must be delivered strictly via Migration 018.

---

### B. Explicit Architectural Decisions Approved for F2.09

#### Decision D.1 — Lightweight Core Architecture (Option A)
- **Approved Decision:** `F2.09` builds directly upon the existing schema foundation:
  - `products.warranty_months` (catalog warranty terms)
  - `serial_numbers.warranty_expires_at` (instance expiration date)
  - `WARRANTY` capability code
- **Prohibited Extensions:** The system shall **NOT** introduce complex supplementary tables such as `warranty_registrations` or `warranty_policies`.
- **Prohibited Premature Data Models:** `F2.09` shall **NOT** introduce customer identifiers (`customer_id`), proof-of-purchase document models, activation-source audit entities, or formal claim/RMA lifecycle tables.
- **Rationale:** The repository's authoritative requirements specify core warranty tracking and expiration calculation for Phase 2. Creating separate registration or claim tables at this stage would constitute unrequested scope expansion. Instance registration in `F2.09` updates `serial_numbers.warranty_expires_at` directly.

#### Decision D.2 — Serialized Instance Focus (Option A)
- **Approved Decision:** Instance-level warranty registration and active expiration tracking in `F2.09` are scoped strictly to individual serialized/IMEI units (`serial_numbers`).
- **Product-Level Coverage:** Non-serialized products maintain their catalog-level warranty term template (`products.warranty_months`), but instance registration is not performed for non-serialized inventory in Phase 2.
- **Phase 3 Deferral:** Tracking warranty activations for non-serialized products fundamentally requires transaction receipt linkage (`sale_items`), which belongs to Phase 3 Sales checkout.
- **Rationale:** Instance-level warranty activation for non-serialized inventory is outside F2.09 and deferred to Phase 3 transaction linkage.

#### Decision D.3 — Unified Commercial Warranty Period (Option A)
- **Approved Decision:** The platform maintains a single, unified warranty duration attribute (`warranty_months`).
- **Prohibited Dual-Provider Split:** `F2.09` shall **NOT** split warranty into `manufacturer_warranty_months` and `seller_warranty_months`.
- **Rationale:** Neither `001_initial.sql`, `CAPABILITY_MATRIX.md`, nor `PRODUCT_SPEC.md` requires multi-provider warranty tracking. A single commercial warranty duration attribute (`products.warranty_months`) constitutes the canonical F2.09 warranty-duration model.

---

## 3. Detailed Semantics & Domain Invariants

### 3.1 Product Warranty Capability & Eligibility

A product is considered to have active warranty tracking if:
$$\text{is\_warranty\_tracked}(P) \iff (\text{products.warranty\_months} > 0) \lor \text{has\_capability}(P, \text{'WARRANTY'})$$

- Domain validation strictly enforces `warranty_months >= 0`. A value of `0` or `NULL` indicates no standard warranty is defined.
- Enabling the `WARRANTY` capability on a product without setting `warranty_months` permits custom instance-level warranty durations during serialized instance creation or activation.

### 3.2 Canonical Date Model & Expiration Calculation

1. **Canonical Domain Representation:**
   The domain engine strictly enforces a single canonical date representation:
   - `start_date` = `YYYY-MM-DD`
   - `warranty_expires_at` = `YYYY-MM-DD`
   - `as_of_date` = `YYYY-MM-DD`
   - **No Mixed Date/Timestamp Comparisons:** The domain engine strictly forbids comparing date strings against timestamp strings.
   - **Timestamp Normalization:** If a caller supplies an ISO 8601 timestamp (e.g. `2026-09-03T12:00:00Z`), the input boundary must normalize it to its canonical UTC calendar date (`2026-09-03`) before entering domain calculations.

2. **Calculation Rules:**
   - Expiration date is calculated by advancing the calendar month of `start_date` by `duration_months` ($\ge 1$).
   - **Month-End Clamping:** If the target month has fewer days than the start day, the expiration day is clamped to the last valid day of the target month (e.g. `2026-01-31` + 1 month $\to$ `2026-02-28`, or `2026-02-29` in a leap year).
   - Date calculations are performed in UTC to prevent timezone skew.
   - Output format is strictly the canonical date string `YYYY-MM-DD`.

### 3.3 Instance Registration Semantics

- **Operation:** Registering or updating instance warranty sets `serial_numbers.warranty_expires_at` for the specified `serial_number_id`.
- **Duration Resolution:**
  - If an explicit `duration_months` is provided in the input, the expiration date is calculated from `start_date + duration_months`.
  - If no duration is provided in the input, the domain engine retrieves `products.warranty_months` from the parent product. If `products.warranty_months` is `NULL` or `0`, a validation error is returned.
- **Direct Expiration Input:**
  - To maintain compatibility with existing/historical database state, direct specification of `warranty_expires_at` is supported.
  - This does **not** introduce a batch import subsystem. Normal registration must satisfy domain date invariants (strictly canonical `YYYY-MM-DD` format, valid calendar day).

### 3.4 Warranty Coverage Status Evaluation

Given an instance's canonical `warranty_expires_at` date and a canonical `as_of_date` (defaulting to the current UTC calendar date `YYYY-MM-DD`):

1. **Exact Calendar Semantics:**
   $$\text{days\_remaining} = \max(0, \text{expiry\_date} - \text{as\_of\_date})$$
   $$\text{days\_elapsed} = \max(0, \text{as\_of\_date} - \text{expiry\_date})$$

2. **Status Determination & Expiration-Day Behavior:**
   - **`Active`**: `as_of_date <= expiry_date`.
     - When `as_of_date < expiry_date`: `status = Active`, `days_remaining > 0`.
     - **Expiration-Day Invariant:** When `as_of_date == expiry_date`: `status = Active`, `days_remaining = 0` (coverage remains active through the end of the expiration calendar day).
   - **`Expired`**: `as_of_date > expiry_date`.
     - `status = Expired`, `days_elapsed > 0`.
   - **`NotRegistered`**: The product has warranty capability or duration, but `serial_numbers.warranty_expires_at` is `NULL`.
   - **`NotCovered`**: The product has no warranty capability and `warranty_expires_at` is `NULL`.

---

## 4. Multi-Tenancy & Authorization Security

To prevent security vulnerabilities and data leakage across organizations and branches:
1. **Mandatory Authentication:** All warranty commands require an active session via `crate::user::session::require_session`.
2. **Scoped Authorization:**
   - Modifying product-level warranty terms requires `Permission::ProductsManage`.
   - Registering or updating instance warranty requires `Permission::InventoryAdjust` scoped to the branch owning the serial instance (`require_scoped_permission`).
   - Reading warranty coverage requires valid session access to the owning organization.
3. **Existence-Leakage Prevention:**
   - Authorization and branch checks are evaluated before entity existence is revealed.
   - If an instance does not exist or belongs to another branch/organization, the system returns a uniform generic error: `"Serial instance '<id>' not found or inaccessible for this session"`.

---

## 5. Migration 018 Specification (Documentation Only)

> **IMPORTANT:** This section documents the schema design for Migration 018. **NO SQL file is created under this ADR.**

### A. Explicit Migration Decision
- **File:** `018_warranty.sql`
- **Scope:** Migration 018 adds **only** the warranty expiration query index to optimize coverage evaluations and does **not** rebuild `products` or alter the existing `products.warranty_months` column:
  ```sql
  CREATE INDEX IF NOT EXISTS idx_serial_numbers_warranty_expires
  ON serial_numbers(warranty_expires_at)
  WHERE warranty_expires_at IS NOT NULL;
  ```
- **Constraint Enforcement:** Non-negative warranty duration (`warranty_months >= 0`) is enforced at the domain validation layer. Database-level table rebuilds for check constraints on `products` are deferred to avoid unnecessary migration risk on existing product data.

### B. Legacy Data Safety
- Existing values in `products.warranty_months` and `serial_numbers.warranty_expires_at` are completely preserved.
- Zero columns are dropped or renamed.

---

## 6. Scope Firewalls (Strictly Protected Boundaries)

The following areas are strictly forbidden within milestone `F2.09`:

| Milestone / Subsystem | Boundary Invariant | Forbidden in F2.09 |
| :--- | :--- | :--- |
| **Phase 3 Sales** | POS checkout cart, tender, receipts | Deducting stock on sale, printing receipt warranty disclaimers, checkout warranty activation |
| **Phase 4 Purchasing** | Purchase Order Intake (GRN) | Receiving supplier shipments, vendor warranty registration |
| **Phase 7.11 Service/RMA** | Repair service tickets, labor | Creating repair tickets, tracking parts/labor on claims, RMA dispatch |
| **F2.10 Locations / Bins** | Warehouse bin/shelf allocation | Assigning warranty instances to specific physical bins |
| **F2.11 Stock Ledger** | Double-entry movement ledger | Inserting rows into `stock_movements` |
| **F2.24 Warranty UI** | Frontend user interface | Creating React components, dialogs, or forms |
| **Phase 10 Hardware** | Device protocols | Any physical hardware integration |

---

## 7. Consequences

### Positive
- Delivers complete, robust warranty tracking without introducing premature, speculative database tables.
- Establishes a strict, single canonical date format (`YYYY-MM-DD`) preventing subtle date/timestamp comparison bugs.
- Provides explicit expiration-day semantics where coverage remains active through the expiration day (`days_remaining = 0`).
- Minimizes database migration risk by adding only an index in Migration 018, avoiding a destructive `products` table rebuild.
- Maintains 100% composability across all capability combinations (`SERIAL`, `IMEI`, `WEIGHT`, `BATCH`, `VARIANT`).

### Negative / Trade-offs
- Non-serialized warranty tracking is deferred to Phase 3 checkout, meaning bulk accessories sold without serial numbers cannot have individual warranty activation dates in Phase 2. (This is logically sound, as bulk items require receipt identifiers to track warranty).
- No historical log of warranty extensions is maintained in Phase 2; updating warranty expiration updates `serial_numbers.warranty_expires_at` in place.

---

## 8. Revisit Triggers

This architecture shall be revisited if:
1. Phase 3 Sales checkout requires multi-tier warranty policies with differing coverage rules per line item.
2. Phase 7 Service / RMA introduces customer repair ticket requirements that mandate a dedicated `warranty_claims` database table.
3. Commercial regulations in target jurisdictions mandate tracking separate manufacturer vs seller statutory warranty durations.
