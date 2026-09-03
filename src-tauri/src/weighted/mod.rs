// Weighted Products domain model, validation invariants, tare math, and database repository.
// F2.06 — Weighted Products (ADR-0008)

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Canonical configuration entity for a weighted product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductWeightConfig {
    pub product_id: String,
    pub default_tare_milli: i64,
    pub min_weight_milli: Option<i64>,
    pub max_weight_milli: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input payload for creating or updating a weighted product configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertWeightConfigInput {
    pub product_id: String,
    pub default_tare_milli: Option<i64>,
    pub min_weight_milli: Option<i64>,
    pub max_weight_milli: Option<i64>,
}

/// Result of evaluating a weighted item calculation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedCalculationResult {
    pub gross_weight_milli: i64,
    pub tare_weight_milli: i64,
    pub net_weight_milli: i64,
    pub unit_price_minor: i64,
    pub total_price_minor: i64,
}

/// Domain errors for Weighted Products.
#[derive(Debug, PartialEq, Eq)]
pub enum WeightedError {
    Validation(String),
    NotFound(String),
    InvalidUnitDimension {
        unit_code: String,
        dimension: String,
    },
    MissingUnit(String),
    NegativeWeight(String),
    WeightOutOfBounds {
        net_weight_milli: i64,
        min_weight_milli: Option<i64>,
        max_weight_milli: Option<i64>,
    },
    Overflow(String),
    Database(String),
}

impl std::fmt::Display for WeightedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeightedError::Validation(msg) => write!(f, "Validation error: {msg}"),
            WeightedError::NotFound(msg) => write!(f, "Not found: {msg}"),
            WeightedError::InvalidUnitDimension {
                unit_code,
                dimension,
            } => write!(
                f,
                "Invalid unit dimension for weighted product: unit '{unit_code}' has dimension '{dimension}', expected 'mass'"
            ),
            WeightedError::MissingUnit(msg) => write!(f, "Missing unit: {msg}"),
            WeightedError::NegativeWeight(msg) => write!(f, "Negative weight error: {msg}"),
            WeightedError::WeightOutOfBounds {
                net_weight_milli,
                min_weight_milli,
                max_weight_milli,
            } => write!(
                f,
                "Net weight {net_weight_milli} milli-units is outside allowed bounds [{min:?}..={max:?}]",
                min = min_weight_milli,
                max = max_weight_milli
            ),
            WeightedError::Overflow(msg) => write!(f, "Arithmetic overflow: {msg}"),
            WeightedError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for WeightedError {}

impl From<rusqlite::Error> for WeightedError {
    fn from(e: rusqlite::Error) -> Self {
        WeightedError::Database(e.to_string())
    }
}

// =========================================================================
// PURE ARITHMETIC & INVARIANT ENGINE
// =========================================================================

/// Calculates net weight by deducting tare weight from gross weight.
///
/// Invariant: gross_weight_milli >= tare_weight_milli >= 0.
/// Rejects negative gross weight, negative tare weight, and gross < tare.
pub fn deduct_tare(gross_weight_milli: i64, tare_weight_milli: i64) -> Result<i64, WeightedError> {
    if gross_weight_milli < 0 {
        return Err(WeightedError::Validation(
            "Gross weight cannot be negative".into(),
        ));
    }
    if tare_weight_milli < 0 {
        return Err(WeightedError::Validation(
            "Tare weight cannot be negative".into(),
        ));
    }
    if gross_weight_milli < tare_weight_milli {
        return Err(WeightedError::NegativeWeight(format!(
            "Gross weight ({gross_weight_milli} milli) cannot be less than tare weight ({tare_weight_milli} milli)"
        )));
    }

    gross_weight_milli
        .checked_sub(tare_weight_milli)
        .ok_or_else(|| WeightedError::Overflow("Subtraction overflow during tare deduction".into()))
}

/// Calculates exact item price for a weighted product using standard integer half-up rounding.
///
/// Formula: floor((net_weight_milli * unit_price_minor + 500) / 1000)
///
/// - `net_weight_milli`: Net quantity in thousandths of the product's pricing unit.
/// - `unit_price_minor`: Exact price for 1.000 whole pricing unit in minor currency units.
///
/// Uses checked integer arithmetic to prevent overflow. Floating-point arithmetic is forbidden.
pub fn calculate_weighted_price(
    net_weight_milli: i64,
    unit_price_minor: i64,
) -> Result<i64, WeightedError> {
    if net_weight_milli < 0 {
        return Err(WeightedError::NegativeWeight(
            "Net weight cannot be negative".into(),
        ));
    }
    if unit_price_minor < 0 {
        return Err(WeightedError::Validation(
            "Unit price cannot be negative".into(),
        ));
    }

    let product = net_weight_milli
        .checked_mul(unit_price_minor)
        .ok_or_else(|| {
            WeightedError::Overflow("Multiplication overflow in price calculation".into())
        })?;

    let with_rounding = product.checked_add(500).ok_or_else(|| {
        WeightedError::Overflow("Addition overflow in rounding calculation".into())
    })?;

    Ok(with_rounding / 1000)
}

/// Exact integer scaling between canonical metric mass units (kg and g).
/// Converts a quantity expressed in thousandths (milli-units) of `from_unit_code`
/// to thousandths (milli-units) of `pricing_unit_code`.
pub fn normalize_metric_mass_quantity_milli(
    measured_quantity_milli: i64,
    from_unit_code: &str,
    pricing_unit_code: &str,
) -> Result<i64, WeightedError> {
    if measured_quantity_milli < 0 {
        return Err(WeightedError::NegativeWeight(
            "Measured quantity cannot be negative".into(),
        ));
    }
    let from_clean = from_unit_code.trim().to_lowercase();
    let to_clean = pricing_unit_code.trim().to_lowercase();

    match (from_clean.as_str(), to_clean.as_str()) {
        ("kg", "kg") | ("g", "g") => Ok(measured_quantity_milli),
        // 1 milli-kg = 1 gram = 1,000 milli-g
        ("kg", "g") => measured_quantity_milli
            .checked_mul(1000)
            .ok_or_else(|| WeightedError::Overflow("Overflow converting kg to g milli-units".into())),
        // 1,000 milli-g = 1 gram = 1 milli-kg
        // Sub-gram remainder is rejected to prevent precision loss in integer representation
        ("g", "kg") => {
            if measured_quantity_milli % 1000 != 0 {
                return Err(WeightedError::Validation(format!(
                    "Quantity {measured_quantity_milli} milli-grams has sub-gram remainder ({rem} milli-g) which cannot be losslessly converted to whole milli-kilograms",
                    rem = measured_quantity_milli % 1000
                )));
            }
            Ok(measured_quantity_milli / 1000)
        }
        (f, t) => Err(WeightedError::Validation(format!(
            "Unsupported mass conversion from '{f}' to '{t}'. Only 'kg' and 'g' exact integer conversions are supported in F2.06"
        ))),
    }
}

/// Validates that an input weight falls within configured optional [min, max] bounds.
pub fn validate_weight_bounds(
    net_weight_milli: i64,
    min_weight_milli: Option<i64>,
    max_weight_milli: Option<i64>,
) -> Result<(), WeightedError> {
    if let Some(min) = min_weight_milli {
        if net_weight_milli < min {
            return Err(WeightedError::WeightOutOfBounds {
                net_weight_milli,
                min_weight_milli,
                max_weight_milli,
            });
        }
    }
    if let Some(max) = max_weight_milli {
        if net_weight_milli > max {
            return Err(WeightedError::WeightOutOfBounds {
                net_weight_milli,
                min_weight_milli,
                max_weight_milli,
            });
        }
    }
    Ok(())
}

// =========================================================================
// DOMAIN REPOSITORY & DATABASE OPERATIONS
// =========================================================================

/// Determines whether a product is classified as weighted, either via `product_type = 'weighted'`
/// or via active capability `'WEIGHT'` in `product_capabilities`.
pub fn is_product_weighted(conn: &Connection, product_id: &str) -> Result<bool, WeightedError> {
    let product_id = product_id.trim();
    let ptype: Option<String> = conn
        .query_row(
            "SELECT product_type FROM products WHERE id = ?1 AND is_active = 1",
            params![product_id],
            |row| row.get(0),
        )
        .optional()?;

    let Some(ptype) = ptype else {
        return Err(WeightedError::NotFound(format!(
            "Product '{product_id}' not found"
        )));
    };

    if ptype.eq_ignore_ascii_case("weighted") {
        return Ok(true);
    }

    let has_weight_cap: bool = conn
        .query_row(
            "SELECT 1 FROM product_capabilities pc
             JOIN capabilities c ON pc.capability_id = c.id
             WHERE pc.product_id = ?1
               AND c.code = 'WEIGHT'
               AND pc.enabled = 1",
            params![product_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    Ok(has_weight_cap)
}

/// Enforces that a weighted product has an assigned unit in `products.unit_type`
/// resolving to `units.code COLLATE NOCASE` with `dimension = 'mass'` and supported
/// metric mass units ('kg' or 'g') for exact integer pricing.
pub fn validate_weighted_product_unit(
    conn: &Connection,
    product_id: &str,
) -> Result<String, WeightedError> {
    let product_id = product_id.trim();
    let unit_code: Option<String> = conn
        .query_row(
            "SELECT unit_type FROM products WHERE id = ?1",
            params![product_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();

    let unit_code = match unit_code {
        Some(code) if !code.trim().is_empty() => code.trim().to_string(),
        _ => {
            return Err(WeightedError::MissingUnit(format!(
                "Weighted product '{product_id}' must have an assigned unit_type"
            )));
        }
    };

    let dimension: Option<String> = conn
        .query_row(
            "SELECT dimension FROM units WHERE code = ?1 COLLATE NOCASE",
            params![unit_code],
            |row| row.get(0),
        )
        .optional()?;

    let Some(dimension_str) = dimension else {
        return Err(WeightedError::NotFound(format!(
            "Unit with code '{unit_code}' not found in canonical units catalog"
        )));
    };

    if !dimension_str.eq_ignore_ascii_case("mass") {
        return Err(WeightedError::InvalidUnitDimension {
            unit_code,
            dimension: dimension_str,
        });
    }

    // F2.06 exact integer pricing supports canonical metric mass units: kg and g
    let unit_lower = unit_code.to_lowercase();
    if unit_lower != "kg" && unit_lower != "g" {
        return Err(WeightedError::Validation(format!(
            "Unsupported mass unit '{unit_code}' for weighted product. F2.06 requires canonical metric mass units ('kg' or 'g') for exact integer pricing."
        )));
    }

    Ok(unit_code)
}

/// Maps a database row from `product_weight_configs` to the `ProductWeightConfig` struct.
fn map_weight_config_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductWeightConfig> {
    Ok(ProductWeightConfig {
        product_id: row.get("product_id")?,
        default_tare_milli: row.get("default_tare_milli")?,
        min_weight_milli: row.get("min_weight_milli")?,
        max_weight_milli: row.get("max_weight_milli")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Upserts a weighted product configuration in `product_weight_configs`.
/// Validates bounds, verifies the product exists, and enforces mass dimension.
pub fn upsert_product_weight_config(
    conn: &Connection,
    input: &UpsertWeightConfigInput,
) -> Result<ProductWeightConfig, WeightedError> {
    let product_id = input.product_id.trim();
    if product_id.is_empty() {
        return Err(WeightedError::Validation(
            "Product ID cannot be empty".into(),
        ));
    }

    let default_tare = input.default_tare_milli.unwrap_or(0);
    if default_tare < 0 {
        return Err(WeightedError::Validation(
            "Default tare weight cannot be negative".into(),
        ));
    }

    if let Some(min) = input.min_weight_milli {
        if min < 0 {
            return Err(WeightedError::Validation(
                "Minimum weight cannot be negative".into(),
            ));
        }
    }

    if let Some(max) = input.max_weight_milli {
        if max < 0 {
            return Err(WeightedError::Validation(
                "Maximum weight cannot be negative".into(),
            ));
        }
    }

    if let (Some(min), Some(max)) = (input.min_weight_milli, input.max_weight_milli) {
        if min > max {
            return Err(WeightedError::Validation(format!(
                "Minimum weight ({min}) cannot exceed maximum weight ({max})"
            )));
        }
    }

    // Verify product exists, is active, and is classified as weighted
    if !is_product_weighted(conn, product_id)? {
        return Err(WeightedError::Validation(format!(
            "Product '{product_id}' is not configured as a weighted product (must have product_type = 'weighted' or active 'WEIGHT' capability)"
        )));
    }

    // Enforce unit dimension is Mass
    validate_weighted_product_unit(conn, product_id)?;

    conn.execute(
        "INSERT INTO product_weight_configs (
            product_id, default_tare_milli, min_weight_milli, max_weight_milli, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
         ON CONFLICT(product_id) DO UPDATE SET
            default_tare_milli = excluded.default_tare_milli,
            min_weight_milli = excluded.min_weight_milli,
            max_weight_milli = excluded.max_weight_milli,
            updated_at = datetime('now')",
        params![
            product_id,
            default_tare,
            input.min_weight_milli,
            input.max_weight_milli,
        ],
    )?;

    let config = get_product_weight_config(conn, product_id)?
        .ok_or_else(|| WeightedError::Database("Failed to load upserted weight config".into()))?;

    Ok(config)
}

/// Retrieves the weight configuration for a product.
pub fn get_product_weight_config(
    conn: &Connection,
    product_id: &str,
) -> Result<Option<ProductWeightConfig>, WeightedError> {
    let product_id = product_id.trim();
    let config = conn
        .query_row(
            "SELECT product_id, default_tare_milli, min_weight_milli, max_weight_milli, created_at, updated_at
             FROM product_weight_configs WHERE product_id = ?1",
            params![product_id],
            map_weight_config_row,
        )
        .optional()?;

    Ok(config)
}

/// Deletes the weight configuration for a product.
pub fn delete_product_weight_config(
    conn: &Connection,
    product_id: &str,
) -> Result<(), WeightedError> {
    let product_id = product_id.trim();
    let affected = conn.execute(
        "DELETE FROM product_weight_configs WHERE product_id = ?1",
        params![product_id],
    )?;

    if affected == 0 {
        return Err(WeightedError::NotFound(format!(
            "No weight config found for product '{product_id}'"
        )));
    }

    Ok(())
}

/// Evaluates a full weighted item calculation for a product.
/// Resolves base price, applies tare, verifies boundaries, and calculates exact minor price.
pub fn calculate_weighted_item(
    conn: &Connection,
    product_id: &str,
    gross_weight_milli: i64,
    custom_tare_milli: Option<i64>,
    source_unit: Option<&str>,
) -> Result<WeightedCalculationResult, WeightedError> {
    let product_id = product_id.trim();

    // 1. Verify product exists and is classified as weighted
    if !is_product_weighted(conn, product_id)? {
        return Err(WeightedError::Validation(format!(
            "Product '{product_id}' is not configured as a weighted product"
        )));
    }

    // 2. Fetch product base price (stored as REAL in products.base_price, converted to exact integer minor)
    let row_opt = conn
        .query_row(
            "SELECT base_price FROM products WHERE id = ?1 AND is_active = 1",
            params![product_id],
            |row| row.get::<_, f64>(0),
        )
        .optional()?;

    let Some(base_price_real) = row_opt else {
        return Err(WeightedError::NotFound(format!(
            "Product '{product_id}' not found or inactive"
        )));
    };

    let unit_price_minor = crate::product::real_to_minor(base_price_real);

    // 3. Validate product unit dimension and metric requirement
    let pricing_unit = validate_weighted_product_unit(conn, product_id)?;

    // 4. Normalize gross weight if measured in a different metric mass unit
    let normalized_gross_milli = match source_unit {
        Some(unit)
            if !unit.trim().is_empty() && !unit.trim().eq_ignore_ascii_case(&pricing_unit) =>
        {
            let unit_trimmed = unit.trim();
            let unit_lower = unit_trimmed.to_lowercase();
            if unit_lower != "kg" && unit_lower != "g" {
                return Err(WeightedError::Validation(format!(
                    "Unsupported scale measurement unit '{unit_trimmed}'. F2.06 requires canonical metric mass units ('kg' or 'g')."
                )));
            }
            normalize_metric_mass_quantity_milli(gross_weight_milli, unit_trimmed, &pricing_unit)?
        }
        _ => gross_weight_milli,
    };

    // 5. Resolve tare weight (custom tare overrides default tare)
    let config = get_product_weight_config(conn, product_id)?;
    let tare_weight_milli = match custom_tare_milli {
        Some(custom) => custom,
        None => config.as_ref().map(|c| c.default_tare_milli).unwrap_or(0),
    };

    // 6. Deduct tare
    let net_weight_milli = deduct_tare(normalized_gross_milli, tare_weight_milli)?;

    // 7. Enforce configured boundaries if config exists
    if let Some(cfg) = &config {
        validate_weight_bounds(net_weight_milli, cfg.min_weight_milli, cfg.max_weight_milli)?;
    }

    // 8. Calculate exact integer half-up price
    let total_price_minor = calculate_weighted_price(net_weight_milli, unit_price_minor)?;

    Ok(WeightedCalculationResult {
        gross_weight_milli,
        tare_weight_milli,
        net_weight_milli,
        unit_price_minor,
        total_price_minor,
    })
}
