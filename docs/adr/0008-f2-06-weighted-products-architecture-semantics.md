# ADR-0008 — F2.06 Weighted Products Architecture & Calculation Semantics

Status: Accepted
Date: 2026-09-03

## Context

Phase 2 milestone `F2.06 — Weighted products` establishes the domain model, validation rules, exact mathematical calculation, and persistence for products sold by weight/mass. This includes tare weight handling, exact integer milli-unit representations, scale-unit compatibility, and financial calculation without floating-point error.

During the pre-implementation audit, key decisions around schema design, variable-weight barcode scope, canonical unit relationships, pricing semantics across units, exact cross-unit normalization, and rounding arithmetic were analyzed, hardened, and resolved. This ADR records the approved architectural decisions governing F2.06 execution.

---

## Authoritative Existing Facts vs Architectural Decisions Made Now

### Authoritative Existing Facts

1. **Thousandths Quantity Precision Foundation**:
   - `006_quantity_precision_hardening.sql` established `quantity_milli INTEGER NOT NULL DEFAULT 0` as the explicit source of truth across `inventory`, `sale_items`, and `stock_movements`.
   - 1 base unit corresponds to 1,000 milli-units (for kilograms, 1.000 kg = 1,000 milli-units / grams; for grams, 1.000 g = 1,000 milli-units / milligrams).
2. **Seeded Capabilities and Mass Units**:
   - `003_global_commerce_foundation.sql` seeded capability `('WEIGHT', 'Weight', 'product', 'Quantity by mass')`.
   - `003_global_commerce_foundation.sql` seeded canonical units `kg` (Kilogram, mass, precision 3) and `g` (Gram, mass, precision 3).
3. **Existing Product Type Validation & Unit Code Storage**:
   - `products.unit_type TEXT` (`001_initial.sql:60`) stores the unit code reference (e.g. `'kg'`, `'g'`), referencing `units.code COLLATE NOCASE`.
   - `src-tauri/src/product/mod.rs:224-234` already validates `product_type`, explicitly accepting `'weighted'` in addition to `'simple'` and `'variable'`.
   - `products.base_price_minor: i64` stores the authoritative price per 1 whole unit of `products.unit_type` in minor currency units (e.g. cents).
4. **Existing Capability Schema**:
   - `003_global_commerce_foundation.sql` established `product_capabilities (product_id, capability_id, enabled)`.
5. **Unit Dimension Architecture**:
   - `src-tauri/src/unit/mod.rs` defines `UnitDimension::Mass` alongside `Count`, `Volume`, `Length`, `Area`, and `Custom`.
   - Units table has unique case-insensitive code index `idx_units_code_nocase` (`013_units_conversions_hardening.sql`).
6. **Database Rules & Exact Money**:
   - `DATABASE_RULES.md` mandates integer minor units for monetary amounts and prohibits `REAL`/`FLOAT` for authoritative financial state.

---

### Approved Architectural Decisions

1. **Decision 1 — Dedicated Table (`product_weight_configs`) via Migration 015**:
   - Create sequential migration `015_weighted_products.sql` introducing table `product_weight_configs`.
   - Primary key is `product_id TEXT PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE`.
   - No redundant index on `product_id` is created, as SQLite automatically maintains a unique B-tree index for the primary key.
   - All weights are stored exclusively as integer milli-units (`INTEGER NOT NULL DEFAULT 0`).
   - JSON `custom_attributes` must **NOT** be used for core weighted-product state.
   - Premature hardware columns (e.g. `is_scale_integrated`) are excluded; F2.06 stores domain weight attributes only.
2. **Decision 2 — Defer Variable-Weight Barcode Parsing**:
   - F2.06 strictly does **NOT** implement GS1/EAN-13 variable-weight barcode parsing (prefixes `20`–`29` with embedded weight/price) or grocery scale-label printing workflows.
   - Variable-measure barcode parsing and label workflows remain deferred to `F2.19 (Advanced Barcode / Label Printing)` and `F7.03 (Grocery Module)`.
3. **Decision 3 — Exact Integer Half-Up Currency Rounding & Pricing Semantics**:
   - Pricing unit is the product's configured unit (`product.unit_type`).
   - `unit_price_minor` represents the exact price for 1.000 whole unit of `product.unit_type` (e.g. price per 1 kg if unit is `kg`, or price per 1 g if unit is `g`).
   - `net_weight_milli` represents thousandths (milli-units) of the product's pricing unit.
   - Price calculation formula:
     $$\text{price\_minor} = \left\lfloor \frac{\text{net\_weight\_milli} \times \text{unit\_price\_minor} + 500}{1000} \right\rfloor$$
   - Must use checked integer arithmetic (`checked_mul`, `checked_add`, `checked_sub`) and reject potential overflow with domain error.
   - `f64` / `f32` floating-point arithmetic is strictly prohibited for authoritative monetary amounts.
   - Cross-unit mass conversion (e.g. grams to kilograms) must use exact integer rational scaling factors without floating-point evaluation.
4. **Decision 4 — Strict Mass Dimension Enforcement & Canonical Unit Relationship**:
   - Canonical relationship is strictly: `products.unit_type -> units.code COLLATE NOCASE`.
   - If `product_type = 'weighted'` OR the product is associated with capability `'WEIGHT'` (via `product_capabilities` where `enabled = 1`), then the resolved unit in `units` MUST have `dimension = 'mass'` (`UnitDimension::Mass`).
   - Non-mass units (e.g. `piece`, `meter`, `liter`) are strictly rejected with `WeightedError::InvalidUnitDimension`.
5. **Decision 5 — Tare Weight & Net Weight Invariants**:
   - Net weight is derived as:
     $$\text{net\_weight\_milli} = \text{gross\_weight\_milli} - \text{tare\_weight\_milli}$$
   - Invariant: $\text{gross\_weight\_milli} \ge \text{tare\_weight\_milli} \ge 0$.
   - Negative net weight is strictly rejected with a validation error.
   - A product may define an optional `default_tare_milli` representing packaging/container weight in milli-units of the product's pricing unit.
6. **Decision 6 — Hardware Scale Driver Isolation**:
   - Hardware communication with physical scales (protocols: OPOS, CAS, Toledo, RS-232, USB HID) is strictly deferred to Phase 10 (Hardware Integrations).
   - F2.06 is domain core, math, validation, persistence, and Tauri IPC commands.

---

## Detailed Technical Specifications

### 1. Dedicated Table Schema (`src-tauri/src/db/migrations/015_weighted_products.sql`)

```sql
-- 015_weighted_products.sql
-- F2.06 — Weighted Products domain configuration and tare management.
-- Never edit an applied migration; create a new migration instead.

CREATE TABLE product_weight_configs (
    product_id TEXT PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    default_tare_milli INTEGER NOT NULL DEFAULT 0,
    min_weight_milli INTEGER,
    max_weight_milli INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (default_tare_milli >= 0),
    CHECK (min_weight_milli IS NULL OR min_weight_milli >= 0),
    CHECK (max_weight_milli IS NULL OR max_weight_milli >= 0),
    CHECK (min_weight_milli IS NULL OR max_weight_milli IS NULL OR min_weight_milli <= max_weight_milli)
);
```

*(Note: No redundant index on `product_id` is created because `PRIMARY KEY` in SQLite automatically creates a unique index).*

---

### 2. Weight Price Semantics & Exact Mathematical Proof

#### A. Definitions
- **Pricing Unit ($U_p$):** The unit code specified in `products.unit_type` (e.g. `kg`, `g`), referencing `units.code COLLATE NOCASE`.
- **Base Unit Price ($P_{minor}$):** `products.base_price_minor`, representing the exact price for 1.000 whole pricing unit in minor currency units (e.g. cents).
- **Milli Quantity ($Q_{milli}$):** The net quantity expressed in thousandths (milli-units) of $U_p$, consistent with `006_quantity_precision_hardening.sql`.

#### B. Proof for Supported Mass Units
1. **Product Unit is Kilogram (`kg`):**
   - $U_p = \text{kg}$.
   - $P_{minor}$ = price per 1.000 kg.
   - $1\text{ kg} = 1,000\text{ milli-units}$ (where 1 milli-unit = 1 gram).
   - A measured weight of 1.450 kg has $Q_{milli} = 1450$.
   - $\text{Exact Price} = \frac{Q_{milli}}{1000} \times P_{minor} = \frac{1450 \times P_{minor}}{1000}$.
2. **Product Unit is Gram (`g`):**
   - $U_p = \text{g}$.
   - $P_{minor}$ = price per 1.000 g (e.g. saffron or high-value bulk herbs).
   - $1\text{ g} = 1,000\text{ milli-units}$ (where 1 milli-unit = 1 milligram).
   - A measured weight of 2.500 g has $Q_{milli} = 2500$.
   - $\text{Exact Price} = \frac{Q_{milli}}{1000} \times P_{minor} = \frac{2500 \times P_{minor}}{1000}$.

Because $Q_{milli}$ is *always* defined as thousandths of the product's pricing unit $U_p$, the division by $1000$ scales $Q_{milli}$ back to whole units of $U_p$ in every case.

#### C. Exact Cross-Unit Weight Normalization (Zero Floating-Point)
F2.06 strictly prohibits using `f64` conversions from `src-tauri/src/unit/mod.rs` for authoritative financial price determination. When mass conversion between supported metric units (`kg` and `g`) is required:
- Metric mass conversion is governed by the exact integer ratio:
  $$1\text{ kg} = 1,000\text{ g} \implies 1\text{ milli-kg} = 1\text{ g} = 1,000\text{ milli-g}$$
- To convert a weight measured in milli-grams ($W_{\text{milli-g}}$) into $Q_{milli}$ of a product priced in kilograms ($U_p = \text{kg}$):
  $$Q_{milli}(\text{kg}) = \frac{W_{\text{milli-g}}}{1000}$$
  Sub-gram fractional remainders ($W_{\text{milli-g}} \pmod{1000} \ne 0$) cannot be losslessly represented in integer milli-kilograms and are rejected fail-closed to prevent precision loss.
- To convert a weight measured in milli-kilograms ($W_{\text{milli-kg}}$) into $Q_{milli}$ of a product priced in grams ($U_p = \text{g}$):
  $$Q_{milli}(\text{g}) = W_{\text{milli-kg}} \times 1000$$
- This integer normalization is exact, lossless, and free from IEEE 754 precision drift.
- General `f64` conversions from F2.04 are restricted to informational UI display and must never feed into financial calculations.

#### D. Integer Half-Up Rounding Arithmetic
$$\text{price\_minor} = \left\lfloor \frac{\text{net\_weight\_milli} \times \text{unit\_price\_minor} + 500}{1000} \right\rfloor$$

```rust
pub fn calculate_weighted_price(
    net_weight_milli: i64,
    unit_price_minor: i64,
) -> Result<i64, WeightedError> {
    if net_weight_milli < 0 {
        return Err(WeightedError::Validation("Net weight cannot be negative".into()));
    }
    if unit_price_minor < 0 {
        return Err(WeightedError::Validation("Unit price cannot be negative".into()));
    }

    // Checked arithmetic: (net_weight_milli * unit_price_minor + 500) / 1000
    let product = net_weight_milli
        .checked_mul(unit_price_minor)
        .ok_or_else(|| WeightedError::Overflow("Multiplication overflow in price calculation".into()))?;

    let with_rounding = product
        .checked_add(500)
        .ok_or_else(|| WeightedError::Overflow("Addition overflow in rounding calculation".into()))?;

    Ok(with_rounding / 1000)
}
```

---

### 3. Canonical Unit Relationship & Capability Invariants

#### A. Relationship Definition
- The unit link is strictly:
  $$\text{products.unit\_type} \longrightarrow \text{units.code COLLATE NOCASE}$$
- `products.unit_type` stores the unit code string (e.g. `'kg'`, `'g'`). It is NOT a UUID foreign key.

#### B. Weight Capability & Product Type Validation Invariant
A product is classified as a weighted product if:
1. `products.product_type = 'weighted'`, OR
2. The product has an active `'WEIGHT'` capability record in SQLite:
   ```sql
   SELECT 1 FROM product_capabilities pc
   JOIN capabilities c ON pc.capability_id = c.id
   WHERE pc.product_id = ?1
     AND c.code = 'WEIGHT'
     AND pc.enabled = 1
   ```

If a product satisfies either condition:
- `products.unit_type` MUST be non-null and non-empty.
- The unit code in `products.unit_type` MUST exist in `units`:
  ```sql
  SELECT dimension FROM units WHERE code = ?1 COLLATE NOCASE
  ```
- The resolved unit MUST have `dimension = 'mass'` (`UnitDimension::Mass`).
- If `unit_type` is missing or resolves to any dimension other than `'mass'` (e.g. `'count'`, `'volume'`, `'length'`), the operation MUST be rejected with `WeightedError::InvalidUnitDimension`.

---

## Consequences

### Positive
- Unified quantity and price math: 100% consistent for both `kg` and `g`.
- No redundant indexes in SQLite; primary key index is reused.
- Zero floating-point drift in financial calculations; metric mass normalization is exact integer math.
- No premature hardware flags or hardware assumptions in Phase 2.
- Clean relational separation via `product_weight_configs` with foreign key cascade.
- Complete reuse of F2.04 unit definitions, dimensions, and capability registry.

### Negative / Trade-offs
- Setting up a weighted product requires an entry in `product_weight_configs` in addition to `products` (managed atomically within transactions).

---

## Revisit Triggers
Revisit this ADR if:
- Legal metrology compliance for a specific target jurisdiction requires certified tare audit trails.
- Scale driver integration in Phase 10 requires device-specific calibration data stored at the product level.
- Non-metric mass units with fractional conversion factors (e.g. imperial pounds/ounces) are added to the active catalog, requiring exact integer rational fraction arithmetic.
