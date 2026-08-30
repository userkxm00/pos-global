# ADR-0007 — F2.05 Cartesian Variant Matrix Generation Semantics

Status: Accepted  
Date: 2026-08-30  

## Context

Phase 2 milestone `F2.05 — Variants / Matrix` requires deterministic n-dimensional Cartesian matrix generation for products with variant matrices. During the contract audit and decision gate, five specific architectural behaviors required formal recording: SKU generation behavior, soft-deleted combination handling, combinatorial safety boundaries, preview side-effect isolation, and transactional batch generation.

## Decisions

### Decision 1 — Matrix SKU Assignment
- If `sku_prefix` is provided in the generation payload, matrix generation allocates unique, sequential SKUs using the existing canonical SKU generator from F2.03 (`crate::barcode::generate_next_sku`).
- If `sku_prefix` is omitted or `None`, newly generated variants are created with `sku = NULL`.
- Matrix generation must **never** construct custom slug-based SKUs from attribute display names or values (e.g. `{sku_prefix}-{val1}-{val2}`).

*Rationale:* F2.05 reuses the canonical SKU capability established in F2.03 and does not invent divergent or ad-hoc SKU generation algorithms.

### Decision 2 — Soft-Deleted Combination Handling
- A soft-deleted/archived historical variant (`is_active = 0` or `deleted_at IS NOT NULL`) must **never** be silently reactivated by matrix generation.
- When generating a matrix, if no **active** variant (`is_active = 1 AND deleted_at IS NULL`) exists with that exact combination of attribute values, matrix generation creates a **new active variant** with a fresh UUIDv4.
- Historical soft-deleted rows and their audit timestamps remain untouched.

*Rationale:* Preserves immutable historical lifecycle identity and auditability; active-combination uniqueness applies strictly to active records.

### Decision 3 — Cartesian Safety Limit
- The maximum allowed Cartesian combination count per generation request is **5,000**.
- The generator must validate the projected combination count before allocating collections or executing database operations.
- The calculation must use overflow-safe checked multiplication (`checked_mul`). If the projected count exceeds 5,000 or overflows, generation is rejected immediately with `VariantError::Validation`.

*Explicit Distinction:* The 5,000 threshold is an **ARCHITECTURAL DECISION MADE NOW** to protect against combinatorial exhaustion / memory exhaustion DOS attacks, not an existing specification claim.

### Decision 4 — Preview Isolation
- Matrix preview (`preview_variant_matrix`) must be strictly side-effect free.
- Preview must **never** allocate SKUs from `sku_sequences`, insert/update variant rows, mutate timestamps, or write any database state.

### Decision 5 — Generation Atomicity
- Actual variant matrix generation (`generate_variant_matrix`) must execute entirely within a single `rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)`.
- If any combination fails (e.g., constraint error, SKU collision, foreign key mismatch), the transaction rolls back automatically via RAII, guaranteeing zero partial or orphaned variants.

## Authoritative Classification & Sources

### Authoritative From Existing Specifications
1. **Milestone & Capability Boundaries**: `BACKLOG.md:118`, `TASK_DEPENDENCY_GRAPH.md:81-87`, `ACCEPTANCE_MATRIX.md:11`, `PRODUCT_STRATEGY.md:10-14`, `DOMAIN_CONTRACTS.md:9-12`.
2. **Canonical SKU Engine**: `src-tauri/src/barcode/generator.rs:67-95` (`generate_next_sku`).
3. **Soft-Deletion & Active Index Rules**: `014_product_variants_hardening.sql:27-36`, `.agents/rules/project_engineering_rules.md:16-25`, `DATABASE_RULES.md:50-53`.
4. **Active-Combination Uniqueness Invariant**: `014_product_variants_hardening.sql`, `src-tauri/src/variant/mod.rs`.

### Architectural Decisions Made Now
1. **Decision 1**: SKU prefix delegation to canonical `generate_next_sku` vs `NULL` default.
2. **Decision 2**: Non-reactivation of archived combinations and fresh UUID allocation for active variants.
3. **Decision 3**: 5,000 Cartesian combination safety upper bound with overflow-safe multiplication.
4. **Decision 4**: Side-effect free preview contract.
5. **Decision 5**: Batch generation transaction boundary using `TransactionBehavior::Immediate`.

## Protected Future Phases

The following downstream milestones remain strictly protected from scope expansion:
- `F2.06`: Weighted Products
- `F2.07`: Batch / Expiry / FEFO
- `F2.08`: Serial / IMEI / Assets
- `F2.09`: Warranty
- `F2.11`: Stock Movement Ledger
- `F2.21`: Matrix / Variant Editor Grid UI
- `F2.27`: Variant Cloud Sync Projections
- `F3.15`: POS Cart Matrix Selector

## Consequences

- Matrix generation has clear, unambiguous, and deterministic contracts for SKU allocation, archived combinations, safety bounds, preview, and atomic writes.
- The domain implementation can proceed without guessing business rules or introducing breaking schema changes.
