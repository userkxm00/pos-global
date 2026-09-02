# ADR-0007 — F2.05 Cartesian Variant Matrix Generation & SKU Architecture Semantics

Status: Accepted
Date: 2026-09-02

## Context

Phase 2 milestone `F2.05 — Variants / Matrix` introduces deterministic n-dimensional Cartesian matrix generation, single-variant management, and attribute definition/value repositories. During the final pre-implementation contract and forensic audit, critical ambiguities around the SKU identity model, canonical generator integration, archived Variant SKU availability, combinatorial safety, preview isolation, and transaction atomicity were audited.

This ADR records the project-level architectural decisions governing F2.05 execution, explicitly distinguishing existing authoritative specification facts from architectural decisions established now.

---

## Authoritative Existing Facts vs Architectural Decisions Made Now

### Authoritative Existing Facts
1. **Separate Table-Local SKU Constraints**:
   - `products.sku` uniqueness is scoped to the `products` table via partial unique index `idx_products_sku_active ON products(sku COLLATE NOCASE) WHERE sku IS NOT NULL AND is_active = 1` (`012_sku_and_multi_barcode.sql:6-8`).
   - `product_variants.sku` uniqueness is scoped to the `product_variants` table via column-level constraint `sku TEXT UNIQUE` (`001_initial.sql:85`).
   - There is no unified cross-table `skus` table and no cross-table index or constraint linking products and variants.
2. **Canonical F2.03 SKU Generator**:
   - `crate::barcode::generate_next_sku(conn, prefix)` atomically increments a sequence in `sku_sequences` and formats candidates as `{PREFIX}-{SEQUENCE:06}` (`src-tauri/src/barcode/generator.rs:51-95`).
3. **Canonical Generator Checks Products Only**:
   - `generate_next_sku` verifies candidate availability via `SELECT 1 FROM products WHERE sku = ?1 COLLATE NOCASE AND is_active = 1` (`src-tauri/src/barcode/generator.rs:78-86`).
   - `generate_next_sku` has zero awareness of `product_variants`.
4. **Variant SKU Constraint is Unconditional in SQLite**:
   - `001_initial.sql:85` established `sku TEXT UNIQUE` on `product_variants` without `WHERE is_active = 1`. In SQLite, this creates an implicit unique index covering all rows, including soft-deleted ones.

### Architectural Decisions Made Now
1. **Decision A — SKU Namespace is Table-Local**:
   - SKU uniqueness remains strictly table-local.
   - `products.sku` and `product_variants.sku` operate in separate namespaces.
   - A Product and a Variant may share the same SKU string without cross-table violation.
   - No cross-table constraint or unified SKU registry is introduced.
2. **Decision B — F2.05 Local Active-Variant Collision Check**:
   - F2.05 matrix generation must reuse the canonical F2.03 generator `crate::barcode::generate_next_sku` when `sku_prefix` is supplied.
   - F2.05 must not modify `src-tauri/src/barcode/generator.rs`.
   - Instead, F2.05 matrix generation takes responsibility for checking candidates against active `product_variants` inside the immediate generation transaction, advancing the sequence if a collision occurs, and failing closed if a bounded safety limit is exceeded.
   - Slug-based SKUs (e.g. `{prefix}-{size}-{color}`) are strictly banned.
3. **Decision C — Soft-Deleted Variant SKUs Remain Reserved**:
   - Archived/soft-deleted Variant SKUs (`is_active = 0` or `deleted_at IS NOT NULL`) remain permanently reserved.
   - A Variant SKU is not released merely because `is_active = 0`.
   - Matrix generation must never attempt to reuse an archived Variant SKU.
   - No table rebuild is performed; migration 001 remains immutable.
4. **Decision D — Combinatorial Safety Bound**:
   - Maximum allowed Cartesian combination count per generation request is 5,000, validated via overflow-safe `checked_mul`.
5. **Decision E — Preview Side-Effect Freedom**:
   - Matrix preview (`preview_variant_matrix`) must be strictly side-effect free (zero database writes, zero sequence mutations).
6. **Decision F — Transactional Generation Atomicity**:
   - Matrix generation executes entirely within a single `rusqlite::TransactionBehavior::Immediate` transaction; any error triggers complete automatic rollback of all variant inserts and SKU sequence increments.

---

## Detailed Decisions

### Decision A — SKU Namespace
- SKU uniqueness remains **TABLE-LOCAL**.
- `products.sku` retains its existing product-table uniqueness semantics (`idx_products_sku_active`).
- `product_variants.sku` retains its existing variant-table uniqueness semantics (`001_initial.sql:85`).
- A Product SKU and a Variant SKU **MAY contain the same string**.
- No global cross-table SKU namespace, unified SKU registry, or cross-table database constraint is introduced.

*Rationale:* The established architecture and schema define separate table-local SKU constraints. Introducing a global cross-table SKU namespace would require cross-table synchronization or table redesign without specification justification.

### Decision B — Matrix SKU Collision Handling & Sequence Advancement
- When `sku_prefix` is provided in `GenerateMatrixInput`, F2.05 **MUST reuse the canonical F2.03 generator** `crate::barcode::generate_next_sku`.
- When `sku_prefix` is omitted or `None`, generated variants receive `sku = NULL`.
- Because the F2.03 generator inspects only `products`, F2.05 matrix generation must wrap candidate retrieval with a **local active-Variant collision check**:
  1. Start the F2.05 `Immediate` transaction.
  2. Request a candidate SKU from canonical `generate_next_sku(&tx, Some(prefix))`.
  3. Query `tx` for collision against active variants: `SELECT 1 FROM product_variants WHERE sku = ?1 COLLATE NOCASE AND is_active = 1`.
  4. If occupied, loop to request the next sequence from canonical `generate_next_sku(&tx, Some(prefix))` and check again.
  5. Continue until a candidate is confirmed unoccupied in active variants.
  6. Bound this resolution loop (fail closed with `VariantError::Validation` if unique candidate cannot be allocated within safety limit).
  7. The final Variant insert and SKU sequence mutation remain inside the same transaction.
- **Constraints**:
  - Do NOT modify `src-tauri/src/barcode/generator.rs`.
  - Do NOT create a second independent SKU-generation algorithm.
  - Do NOT fabricate slug-based SKUs from attribute display names or values.

*Rationale:* Adheres to F2.03 immutability while guaranteeing that matrix generation never causes unhandled SQLite `UNIQUE constraint failed: product_variants.sku` crashes.

### Decision C — Archived Variant SKU Reuse Policy
- Archived / soft-deleted Variant SKUs **REMAIN RESERVED**.
- A Variant SKU is **NOT released** merely because `is_active = 0` or `deleted_at IS NOT NULL`.
- Matrix generation **MUST NOT** attempt to reuse an archived Variant SKU.
- Do NOT perform an SQLite table rebuild to remove the legacy `UNIQUE` constraint on `product_variants.sku`.
- Do NOT modify migration `001_initial.sql`.
- Do NOT introduce a new SKU archival or release policy outside this decision.

*Rationale:* `001_initial.sql:85` established an unconditional `UNIQUE` constraint on `product_variants.sku`. Preserving unconditional reservation guarantees zero schema mutation risk, prevents foreign-key or historical audit ambiguities, and respects migration immutability.

### Decision D — Cartesian Combinatorial Safety Limit
- The maximum allowed Cartesian combination count per generation request is **5,000**.
- The generator must compute the projected combination count before allocating collections or executing database operations.
- The computation must use overflow-safe checked multiplication (`checked_mul`). If the projected count exceeds 5,000 or overflows, generation is rejected immediately with `VariantError::Validation`.

*Rationale:* Prevents memory exhaustion and CPU denial-of-service from combinatorial explosions (e.g., 10 dimensions with 10 values each = 10,000,000,000 combinations).

### Decision E — Preview Side-Effect Isolation
- Matrix preview (`preview_variant_matrix`) must be **strictly side-effect free**.
- Preview must **never** allocate SKUs from `sku_sequences`, insert/update variant rows, mutate timestamps, or write any database state.

*Rationale:* Preview is an analytical query intended to allow UI visualization before committing changes.

### Decision F — Generation Atomicity & Rollback
- Actual variant matrix generation (`generate_variant_matrix`) must execute entirely within a single `rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)`.
- If any combination fails (e.g., constraint error, SKU collision, foreign key mismatch, database error), the transaction rolls back automatically via RAII.
- Rollback revokes all variant inserts AND rolls back sequence increments in `sku_sequences`, guaranteeing zero partial or orphaned records.

---

## Required Decision Tests (To Be Implemented Under Feature Authorization)

When feature implementation is explicitly authorized, the test suite (`src-tauri/src/tests/variant_tests.rs`) must explicitly prove:

1. **Table-Local Namespace**: A Product and a Variant may have the exact same SKU (e.g., `"ELEC-000001"`) simultaneously without cross-table collision or error.
2. **Matrix Active Variant Uniqueness**: Matrix-generated Variant SKUs never duplicate any existing active Variant SKU.
3. **Collision Sequence Advancement**: When a pre-existing active variant occupies sequence `N`, matrix generation automatically advances to `N+1` without error.
4. **Archived SKU Reservation**: An archived variant's SKU remains unavailable and cannot be reassigned to a new variant.
5. **Transaction Atomicity & Rollback**: If matrix generation fails mid-batch, both variant inserts and sequence number mutations in `sku_sequences` roll back completely.
6. **Concurrency Safety**: Multi-connection concurrent variant matrix generation preserves sequence integrity and does not produce duplicate SKUs or deadlocks.

---

## Protected Future Phases

The following downstream milestones remain strictly protected from scope expansion:
- `F2.06`: Weighted Products
- `F2.07`: Batch / Expiry / FEFO
- `F2.08`: Serial / IMEI / Assets
- `F2.09`: Warranty
- `F2.10`: Locations / Bins
- `F2.11`: Stock Movement Ledger
- `F2.21`: Matrix / Variant Editor Grid UI
- `F2.27`: Variant Cloud Sync Projections
- `F3.15`: POS Cart Matrix Selector
