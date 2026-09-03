// Product variants domain model, validation rules, Cartesian matrix engine, and SQLite repository operations.
// F2.05 — Variants / Matrix

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Maximum safe integer minor units that can be converted to and from IEEE 754 f64 without precision loss.
pub const MAX_SAFE_MINOR_UNITS: i64 = 90_071_992_547_409;

/// Maximum allowed Cartesian combinations per generation request (ADR-0007 Decision 3 / D).
pub const MAX_CARTESIAN_COMBINATIONS: usize = 5_000;

/// Bounded safety limit for sequential candidate SKU collision retries against product_variants.
pub const MAX_SKU_COLLISION_RETRIES: usize = 20;

/// Maximum number of search results returned by variant queries to prevent unbounded scans.
pub const DEFAULT_SEARCH_LIMIT: usize = 100;

/// Canonical Attribute Definition entity (e.g., "Size", "Color", "Material").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributeDefinition {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub created_at: String,
}

/// Canonical Attribute Value entity (e.g., "Small", "Medium", "Red", "Blue").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributeValue {
    pub id: String,
    pub attribute_definition_id: String,
    pub value: String,
    pub sort_order: i64,
    pub created_at: String,
}

/// Canonical Product Variant entity in `product_variants`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductVariant {
    pub id: String,
    pub product_id: String,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_override_minor: Option<i64>,
    pub cost_price_minor: Option<i64>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// Variant with its associated resolved attribute values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariantWithAttributes {
    pub variant: ProductVariant,
    pub attribute_values: Vec<AttributeValue>,
}

/// Input payload for creating a new attribute definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttributeDefinitionInput {
    pub name: String,
    pub sort_order: Option<i64>,
}

/// Input payload for creating a new attribute value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttributeValueInput {
    pub attribute_definition_id: String,
    pub value: String,
    pub sort_order: Option<i64>,
}

/// Input payload for creating a single product variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVariantInput {
    pub product_id: String,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_override_minor: Option<i64>,
    pub cost_price_minor: Option<i64>,
    pub attribute_value_ids: Vec<String>,
}

/// Input payload for updating an existing product variant.
/// Semantics:
/// - `sku: None` => preserve current SKU
/// - `sku: Some("")` (empty string) => explicitly clear SKU to NULL
/// - `sku: Some("VALUE")` => validate and update to new SKU
/// - `barcode: None` => preserve current barcode
/// - `barcode: Some("")` (empty string) => explicitly clear barcode to NULL
/// - `barcode: Some("VALUE")` => validate and update to new barcode
/// - `price_override_minor: None` => clear price override to NULL (variant inherits parent product base price)
/// - `price_override_minor: Some(val)` => validate and set price override in minor units
/// - `cost_price_minor: None` => clear cost price to NULL
/// - `cost_price_minor: Some(val)` => validate and set cost price in minor units
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVariantInput {
    pub id: String,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_override_minor: Option<i64>,
    pub cost_price_minor: Option<i64>,
    pub is_active: bool,
}

/// Input payload for specifying an attribute dimension in matrix generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixDimensionInput {
    pub attribute_definition_id: String,
    pub attribute_value_ids: Vec<String>,
}

/// Input payload for generating a full variant matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateMatrixInput {
    pub product_id: String,
    pub dimensions: Vec<MatrixDimensionInput>,
    pub default_price_override_minor: Option<i64>,
    pub default_cost_price_minor: Option<i64>,
    pub sku_prefix: Option<String>,
}

/// Input payload for previewing a variant matrix without side effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewMatrixInput {
    pub product_id: String,
    pub dimensions: Vec<MatrixDimensionInput>,
}

/// Single combination in a matrix preview.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixCombinationPreview {
    pub attribute_values: Vec<AttributeValue>,
    pub existing_variant_id: Option<String>,
    pub is_new: bool,
}

/// Result of a side-effect free matrix preview.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixPreviewResult {
    pub total_combinations: usize,
    pub new_combinations_count: usize,
    pub existing_combinations_count: usize,
    pub combinations: Vec<MatrixCombinationPreview>,
}

/// Result of variant matrix generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixGenerationResult {
    pub total_combinations: usize,
    pub created_count: usize,
    pub existing_count: usize,
    pub created_variants: Vec<VariantWithAttributes>,
    pub existing_variants: Vec<VariantWithAttributes>,
}

/// Input payload for bulk updating variant active status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkUpdateVariantStatusInput {
    pub variant_ids: Vec<String>,
    pub is_active: bool,
}

/// Input payload for bulk updating variant prices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkUpdateVariantPricesInput {
    pub variant_ids: Vec<String>,
    pub price_override_minor: Option<i64>,
    pub cost_price_minor: Option<i64>,
}

/// Result of an atomic bulk operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BulkOperationResult {
    pub updated_count: usize,
    pub affected_variant_ids: Vec<String>,
}

/// Typed domain and repository errors for variant operations.
#[derive(Debug, PartialEq, Eq)]
pub enum VariantError {
    Validation(String),
    NotFound(String),
    DuplicateName(String),
    DuplicateValue(String),
    DuplicateSku(String),
    DuplicateBarcode(String),
    DuplicateCombination(String),
    Database(String),
}

impl std::fmt::Display for VariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariantError::Validation(msg) => write!(f, "Validation error: {msg}"),
            VariantError::NotFound(msg) => write!(f, "Not found: {msg}"),
            VariantError::DuplicateName(msg) => write!(f, "Duplicate attribute name: {msg}"),
            VariantError::DuplicateValue(msg) => write!(f, "Duplicate attribute value: {msg}"),
            VariantError::DuplicateSku(msg) => write!(f, "Duplicate variant SKU: {msg}"),
            VariantError::DuplicateBarcode(msg) => write!(f, "Duplicate variant barcode: {msg}"),
            VariantError::DuplicateCombination(msg) => {
                write!(f, "Duplicate variant combination: {msg}")
            }
            VariantError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for VariantError {}

fn map_sqlite_constraint_error(msg: &str) -> Option<VariantError> {
    if msg.contains("attribute_definitions.name") || msg.contains("idx_attribute_definitions_name")
    {
        return Some(VariantError::DuplicateName(
            "An attribute definition with this name already exists".into(),
        ));
    }
    if msg.contains("attribute_values") || msg.contains("idx_attribute_values_def_val") {
        return Some(VariantError::DuplicateValue(
            "This attribute value already exists for this definition".into(),
        ));
    }
    if msg.contains("product_variants.sku") || msg.contains("idx_product_variants_sku_active") {
        return Some(VariantError::DuplicateSku(
            "SKU is already assigned to another variant".into(),
        ));
    }
    if msg.contains("product_variants.barcode")
        || msg.contains("idx_product_variants_barcode_active")
    {
        return Some(VariantError::DuplicateBarcode(
            "Barcode is already assigned to another active variant or product".into(),
        ));
    }
    None
}

impl From<rusqlite::Error> for VariantError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref f, Some(ref msg)) = e {
            if f.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                if let Some(err) = map_sqlite_constraint_error(msg) {
                    return err;
                }
            }
        }
        VariantError::Database(e.to_string())
    }
}

impl From<crate::barcode::BarcodeError> for VariantError {
    fn from(e: crate::barcode::BarcodeError) -> Self {
        match e {
            crate::barcode::BarcodeError::DuplicateSku(msg) => VariantError::DuplicateSku(msg),
            crate::barcode::BarcodeError::DuplicateBarcode(msg) => {
                VariantError::DuplicateBarcode(msg)
            }
            crate::barcode::BarcodeError::Validation(msg) => VariantError::Validation(msg),
            crate::barcode::BarcodeError::NotFound(msg) => VariantError::NotFound(msg),
            crate::barcode::BarcodeError::InvalidCheckDigit { expected, actual } => {
                VariantError::Validation(format!(
                    "Invalid GS1 check digit: expected {expected}, got {actual}"
                ))
            }
            crate::barcode::BarcodeError::Database(msg) => VariantError::Database(msg),
        }
    }
}

impl From<crate::product::ProductError> for VariantError {
    fn from(e: crate::product::ProductError) -> Self {
        match e {
            crate::product::ProductError::Validation(msg) => VariantError::Validation(msg),
            crate::product::ProductError::NotFound(msg) => VariantError::NotFound(msg),
            crate::product::ProductError::DuplicateBarcode(msg) => {
                VariantError::DuplicateBarcode(msg)
            }
            crate::product::ProductError::DuplicateSku(msg) => VariantError::DuplicateSku(msg),
            crate::product::ProductError::Database(msg) => VariantError::Database(msg),
        }
    }
}

/// Validates attribute definition name. Must be non-empty and <= 100 characters.
pub fn validate_attribute_name(name: &str) -> Result<String, VariantError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(VariantError::Validation(
            "Attribute definition name cannot be empty".into(),
        ));
    }
    if trimmed.chars().count() > 100 {
        return Err(VariantError::Validation(
            "Attribute definition name exceeds maximum length of 100 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates attribute value. Must be non-empty and <= 100 characters.
pub fn validate_attribute_value(value: &str) -> Result<String, VariantError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(VariantError::Validation(
            "Attribute value cannot be empty".into(),
        ));
    }
    if trimmed.chars().count() > 100 {
        return Err(VariantError::Validation(
            "Attribute value exceeds maximum length of 100 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates monetary amount in integer minor units (non-negative and within safe precision range).
pub fn validate_price_minor(price: Option<i64>) -> Result<Option<i64>, VariantError> {
    if let Some(p) = price {
        if p < 0 {
            return Err(VariantError::Validation("Price cannot be negative".into()));
        }
        if p > MAX_SAFE_MINOR_UNITS {
            return Err(VariantError::Validation(
                "Price exceeds maximum supported precision".into(),
            ));
        }
    }
    Ok(price)
}

/// Escapes special SQLite LIKE pattern wildcards (`\`, `%`, `_`) using `\` as the escape character.
pub fn escape_like_pattern(pattern: &str) -> String {
    let mut escaped = String::with_capacity(pattern.len() + 8);
    for c in pattern.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '%' => escaped.push_str("\\%"),
            '_' => escaped.push_str("\\_"),
            other => escaped.push(other),
        }
    }
    escaped
}

// ---------------------------------------------------------------------------
// Attribute Definitions Repository
// ---------------------------------------------------------------------------

fn map_attribute_definition_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AttributeDefinition> {
    Ok(AttributeDefinition {
        id: row.get("id")?,
        name: row.get("name")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
    })
}

pub fn create_attribute_definition(
    conn: &Connection,
    input: CreateAttributeDefinitionInput,
) -> Result<AttributeDefinition, VariantError> {
    let clean_name = validate_attribute_name(&input.name)?;
    let id = uuid::Uuid::new_v4().to_string();
    let sort_order = input.sort_order.unwrap_or(0);

    // Pre-check for duplicate case-insensitive name
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM attribute_definitions WHERE name = ?1 COLLATE NOCASE LIMIT 1",
            params![clean_name],
            |row| row.get(0),
        )
        .optional()?;

    if existing.is_some() {
        return Err(VariantError::DuplicateName(format!(
            "An attribute definition with name '{clean_name}' already exists"
        )));
    }

    conn.execute(
        "INSERT INTO attribute_definitions (id, name, sort_order, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        params![id, clean_name, sort_order],
    )?;

    get_attribute_definition(conn, &id)?.ok_or_else(|| {
        VariantError::Database("Failed to load newly created attribute definition".into())
    })
}

pub fn get_attribute_definition(
    conn: &Connection,
    id: &str,
) -> Result<Option<AttributeDefinition>, VariantError> {
    let def = conn
        .query_row(
            "SELECT id, name, sort_order, created_at FROM attribute_definitions WHERE id = ?1",
            params![id],
            map_attribute_definition_row,
        )
        .optional()?;
    Ok(def)
}

pub fn list_attribute_definitions(
    conn: &Connection,
) -> Result<Vec<AttributeDefinition>, VariantError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, sort_order, created_at FROM attribute_definitions ORDER BY sort_order ASC, name ASC",
    )?;
    let defs = stmt
        .query_map([], map_attribute_definition_row)?
        .filter_map(Result::ok)
        .collect();
    Ok(defs)
}

// ---------------------------------------------------------------------------
// Attribute Values Repository
// ---------------------------------------------------------------------------

fn map_attribute_value_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AttributeValue> {
    Ok(AttributeValue {
        id: row.get("id")?,
        attribute_definition_id: row.get("attribute_definition_id")?,
        value: row.get("value")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
    })
}

pub fn create_attribute_value(
    conn: &Connection,
    input: CreateAttributeValueInput,
) -> Result<AttributeValue, VariantError> {
    let clean_value = validate_attribute_value(&input.value)?;
    let id = uuid::Uuid::new_v4().to_string();
    let sort_order = input.sort_order.unwrap_or(0);

    // Verify parent attribute definition exists
    let def_exists: Option<String> = conn
        .query_row(
            "SELECT id FROM attribute_definitions WHERE id = ?1",
            params![input.attribute_definition_id],
            |row| row.get(0),
        )
        .optional()?;

    if def_exists.is_none() {
        return Err(VariantError::NotFound(format!(
            "Attribute definition '{}' not found",
            input.attribute_definition_id
        )));
    }

    // Pre-check for duplicate case-insensitive value within the same definition
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM attribute_values WHERE attribute_definition_id = ?1 AND value = ?2 COLLATE NOCASE LIMIT 1",
            params![input.attribute_definition_id, clean_value],
            |row| row.get(0),
        )
        .optional()?;

    if existing.is_some() {
        return Err(VariantError::DuplicateValue(format!(
            "Attribute value '{clean_value}' already exists for this definition"
        )));
    }

    conn.execute(
        "INSERT INTO attribute_values (id, attribute_definition_id, value, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![id, input.attribute_definition_id, clean_value, sort_order],
    )?;

    get_attribute_value(conn, &id)?.ok_or_else(|| {
        VariantError::Database("Failed to load newly created attribute value".into())
    })
}

pub fn get_attribute_value(
    conn: &Connection,
    id: &str,
) -> Result<Option<AttributeValue>, VariantError> {
    let val = conn
        .query_row(
            "SELECT id, attribute_definition_id, value, sort_order, created_at FROM attribute_values WHERE id = ?1",
            params![id],
            map_attribute_value_row,
        )
        .optional()?;
    Ok(val)
}

pub fn list_attribute_values_by_definition(
    conn: &Connection,
    attribute_definition_id: &str,
) -> Result<Vec<AttributeValue>, VariantError> {
    let mut stmt = conn.prepare(
        "SELECT id, attribute_definition_id, value, sort_order, created_at
         FROM attribute_values
         WHERE attribute_definition_id = ?1
         ORDER BY sort_order ASC, value ASC",
    )?;
    let vals = stmt
        .query_map(params![attribute_definition_id], map_attribute_value_row)?
        .filter_map(Result::ok)
        .collect();
    Ok(vals)
}

// ---------------------------------------------------------------------------
// Product Variants Repository
// ---------------------------------------------------------------------------

fn map_product_variant_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductVariant> {
    let is_active_int: i64 = row.get("is_active")?;
    Ok(ProductVariant {
        id: row.get("id")?,
        product_id: row.get("product_id")?,
        sku: row.get("sku")?,
        barcode: row.get("barcode")?,
        price_override_minor: row.get("price_override_minor")?,
        cost_price_minor: row.get("cost_price_minor")?,
        is_active: is_active_int != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        deleted_at: row.get("deleted_at")?,
    })
}

/// Checks if a barcode is already assigned to any active variant (excluding `exclude_id` if given)
/// or active product.
fn check_variant_barcode_conflict(
    conn: &Connection,
    barcode: &str,
    exclude_variant_id: Option<&str>,
) -> Result<(), VariantError> {
    let trimmed = barcode.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    // Check active variants
    let variant_conflict: Option<String> = if let Some(ex_id) = exclude_variant_id {
        conn.query_row(
            "SELECT id FROM product_variants WHERE barcode = ?1 COLLATE NOCASE AND id != ?2 AND is_active = 1 AND deleted_at IS NULL",
            params![trimmed, ex_id],
            |row| row.get(0),
        )
        .optional()?
    } else {
        conn.query_row(
            "SELECT id FROM product_variants WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1 AND deleted_at IS NULL",
            params![trimmed],
            |row| row.get(0),
        )
        .optional()?
    };

    if let Some(c_id) = variant_conflict {
        return Err(VariantError::DuplicateBarcode(format!(
            "Barcode '{trimmed}' is already assigned to active variant '{c_id}'"
        )));
    }

    // Check active products
    let product_conflict: Option<String> = conn
        .query_row(
            "SELECT id FROM products WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1",
            params![trimmed],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(p_id) = product_conflict {
        return Err(VariantError::DuplicateBarcode(format!(
            "Barcode '{trimmed}' is already assigned to active product '{p_id}'"
        )));
    }

    // Check product_barcodes table if present
    let barcode_reg_conflict: Option<String> = conn
        .query_row(
            "SELECT product_id FROM product_barcodes WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1",
            params![trimmed],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(pr_id) = barcode_reg_conflict {
        return Err(VariantError::DuplicateBarcode(format!(
            "Barcode '{trimmed}' is already registered to active product '{pr_id}'"
        )));
    }

    Ok(())
}

/// Checks if an SKU is already assigned to another variant.
/// Under Decision C, all rows in product_variants (including soft-deleted) reserve their SKU.
fn check_variant_sku_conflict(
    conn: &Connection,
    sku: &str,
    exclude_variant_id: Option<&str>,
) -> Result<(), VariantError> {
    let sanitized = crate::barcode::validate_sku(sku)?;

    let variant_conflict: Option<String> = if let Some(ex_id) = exclude_variant_id {
        conn.query_row(
            "SELECT id FROM product_variants WHERE sku = ?1 COLLATE NOCASE AND id != ?2",
            params![sanitized, ex_id],
            |row| row.get(0),
        )
        .optional()?
    } else {
        conn.query_row(
            "SELECT id FROM product_variants WHERE sku = ?1 COLLATE NOCASE",
            params![sanitized],
            |row| row.get(0),
        )
        .optional()?
    };

    if let Some(c_id) = variant_conflict {
        return Err(VariantError::DuplicateSku(format!(
            "SKU '{sanitized}' is already assigned to variant '{c_id}'"
        )));
    }

    Ok(())
}

/// Enforces active-combination uniqueness when activating or reactivating a variant.
/// Ensures no other active variant for the same product has the exact same attribute combination.
fn check_variant_combination_conflict_on_activation(
    conn: &Connection,
    product_id: &str,
    target_variant_id: &str,
) -> Result<(), VariantError> {
    let target_vals = get_variant_attribute_values(conn, target_variant_id)?;
    if target_vals.is_empty() {
        return Ok(());
    }
    let target_set: std::collections::HashSet<String> =
        target_vals.into_iter().map(|v| v.id).collect();

    let other_active = list_variants_by_product(conn, product_id, Some(true))?;
    for other in other_active {
        if other.id == target_variant_id {
            continue;
        }
        let other_vals = get_variant_attribute_values(conn, &other.id)?;
        let other_set: std::collections::HashSet<String> =
            other_vals.into_iter().map(|v| v.id).collect();
        if target_set == other_set {
            return Err(VariantError::DuplicateCombination(format!(
                "Cannot activate variant '{target_variant_id}': another active variant ('{}') already has the exact same attribute combination",
                other.id
            )));
        }
    }
    Ok(())
}

pub fn create_variant(
    conn: &Connection,
    input: CreateVariantInput,
) -> Result<VariantWithAttributes, VariantError> {
    let price_override_minor = validate_price_minor(input.price_override_minor)?;
    let cost_price_minor = validate_price_minor(input.cost_price_minor)?;

    // Verify parent product exists and is active
    let parent_product =
        crate::product::get_product(conn, &input.product_id)?.ok_or_else(|| {
            VariantError::NotFound(format!("Parent product '{}' not found", input.product_id))
        })?;

    if !parent_product.is_active {
        return Err(VariantError::Validation(format!(
            "Cannot create variant for inactive product '{}'",
            input.product_id
        )));
    }

    // Validate SKU if provided
    let clean_sku = match input.sku {
        Some(ref s) if !s.trim().is_empty() => {
            let val = crate::barcode::validate_sku(s)?;
            check_variant_sku_conflict(conn, &val, None)?;
            Some(val)
        }
        _ => None,
    };

    // Validate barcode if provided
    let clean_barcode = match input.barcode {
        Some(ref b) if !b.trim().is_empty() => {
            let trimmed = b.trim();
            check_variant_barcode_conflict(conn, trimmed, None)?;
            Some(trimmed.to_string())
        }
        _ => None,
    };

    // Verify all attribute values exist and collect definitions to check for duplicates
    let mut attr_defs_seen = std::collections::HashSet::new();
    let mut resolved_attribute_values = Vec::new();

    for val_id in &input.attribute_value_ids {
        let attr_val = get_attribute_value(conn, val_id)?.ok_or_else(|| {
            VariantError::NotFound(format!("Attribute value '{val_id}' not found"))
        })?;
        if !attr_defs_seen.insert(attr_val.attribute_definition_id.clone()) {
            return Err(VariantError::Validation(format!(
                "Multiple attribute values provided for attribute definition '{}'",
                attr_val.attribute_definition_id
            )));
        }
        resolved_attribute_values.push(attr_val);
    }

    let variant_id = uuid::Uuid::new_v4().to_string();

    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;

    // Verify active combination uniqueness for this product inside immediate write transaction
    if !input.attribute_value_ids.is_empty() {
        let existing_variants = list_variants_by_product(&tx, &input.product_id, Some(true))?;
        for existing in existing_variants {
            let existing_vals = get_variant_attribute_values(&tx, &existing.id)?;
            let existing_set: std::collections::HashSet<String> =
                existing_vals.into_iter().map(|v| v.id).collect();
            let new_set: std::collections::HashSet<String> =
                input.attribute_value_ids.iter().cloned().collect();
            if existing_set == new_set {
                return Err(VariantError::DuplicateCombination(
                    "An active variant with the exact same combination of attribute values already exists for this product"
                        .into(),
                ));
            }
        }
    }

    tx.execute(
        "INSERT INTO product_variants (
            id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, datetime('now'), datetime('now'))",
        params![
            variant_id,
            input.product_id,
            clean_sku,
            clean_barcode,
            price_override_minor,
            cost_price_minor,
        ],
    )?;

    for val_id in &input.attribute_value_ids {
        tx.execute(
            "INSERT INTO variant_attribute_values (variant_id, attribute_value_id)
             VALUES (?1, ?2)",
            params![variant_id, val_id],
        )?;
    }

    tx.commit()?;

    let created_variant = get_variant(conn, &variant_id)?
        .ok_or_else(|| VariantError::Database("Failed to load newly created variant".into()))?;

    Ok(VariantWithAttributes {
        variant: created_variant,
        attribute_values: resolved_attribute_values,
    })
}

pub fn get_variant(conn: &Connection, id: &str) -> Result<Option<ProductVariant>, VariantError> {
    let variant = conn
        .query_row(
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants
             WHERE id = ?1",
            params![id],
            map_product_variant_row,
        )
        .optional()?;
    Ok(variant)
}

pub fn get_variant_attribute_values(
    conn: &Connection,
    variant_id: &str,
) -> Result<Vec<AttributeValue>, VariantError> {
    let mut stmt = conn.prepare(
        "SELECT av.id, av.attribute_definition_id, av.value, av.sort_order, av.created_at
         FROM attribute_values av
         INNER JOIN variant_attribute_values vav ON av.id = vav.attribute_value_id
         WHERE vav.variant_id = ?1
         ORDER BY av.sort_order ASC, av.value ASC",
    )?;
    let vals = stmt
        .query_map(params![variant_id], map_attribute_value_row)?
        .filter_map(Result::ok)
        .collect();
    Ok(vals)
}

/// Lists variants by parent product.
/// When `active_only` is `Some(true)`, uses complete active predicate: `is_active = 1 AND deleted_at IS NULL`.
pub fn list_variants_by_product(
    conn: &Connection,
    product_id: &str,
    active_only: Option<bool>,
) -> Result<Vec<ProductVariant>, VariantError> {
    let sql = match active_only {
        Some(true) => {
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants
             WHERE product_id = ?1 AND is_active = 1 AND deleted_at IS NULL
             ORDER BY created_at ASC"
        }
        Some(false) => {
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants
             WHERE product_id = ?1 AND (is_active = 0 OR deleted_at IS NOT NULL)
             ORDER BY created_at ASC"
        }
        None => {
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants
             WHERE product_id = ?1
             ORDER BY created_at ASC"
        }
    };

    let mut stmt = conn.prepare(sql)?;
    let variants = stmt
        .query_map(params![product_id], map_product_variant_row)?
        .filter_map(Result::ok)
        .collect();
    Ok(variants)
}

/// Updates an existing product variant with partial update semantics.
/// - `input.sku = None` => preserve existing SKU
/// - `input.sku = Some("")` => explicitly clear SKU to NULL
/// - `input.sku = Some("val")` => validate, conflict-check, and update SKU
/// - `input.barcode = None` => preserve existing barcode
/// - `input.barcode = Some("")` => explicitly clear barcode to NULL
/// - `input.barcode = Some("val")` => validate, conflict-check, and update barcode
/// - If activating (`is_active = true`), enforces active-combination uniqueness against other active variants.
pub fn update_variant(
    conn: &Connection,
    input: UpdateVariantInput,
) -> Result<ProductVariant, VariantError> {
    let price_override_minor = validate_price_minor(input.price_override_minor)?;
    let cost_price_minor = validate_price_minor(input.cost_price_minor)?;

    let existing = get_variant(conn, &input.id)?
        .ok_or_else(|| VariantError::NotFound(format!("Variant '{}' not found", input.id)))?;

    // Partial update semantics for SKU:
    // None => preserve; Some("") => clear to NULL; Some("val") => validate & update
    let clean_sku = match input.sku {
        None => existing.sku.clone(),
        Some(ref s) if s.trim().is_empty() => None,
        Some(ref s) => {
            let val = crate::barcode::validate_sku(s)?;
            check_variant_sku_conflict(conn, &val, Some(&input.id))?;
            Some(val)
        }
    };

    // Partial update semantics for Barcode:
    // None => preserve; Some("") => clear to NULL; Some("val") => validate & update
    let clean_barcode = match input.barcode {
        None => {
            if let (true, Some(ref bc)) = (input.is_active, &existing.barcode) {
                check_variant_barcode_conflict(conn, bc, Some(&input.id))?;
            }
            existing.barcode.clone()
        }
        Some(ref b) if b.trim().is_empty() => None,
        Some(ref b) => {
            let trimmed = b.trim();
            if input.is_active {
                check_variant_barcode_conflict(conn, trimmed, Some(&input.id))?;
            }
            Some(trimmed.to_string())
        }
    };

    // If activating (or staying active), enforce combination uniqueness against other active variants
    if input.is_active {
        check_variant_combination_conflict_on_activation(conn, &existing.product_id, &existing.id)?;
    }

    let is_active_int = if input.is_active { 1 } else { 0 };

    conn.execute(
        "UPDATE product_variants
         SET sku = ?1, barcode = ?2, price_override_minor = ?3, cost_price_minor = ?4,
             is_active = ?5,
             deleted_at = CASE WHEN ?5 = 1 THEN NULL ELSE datetime('now') END,
             updated_at = datetime('now')
         WHERE id = ?6",
        params![
            clean_sku,
            clean_barcode,
            price_override_minor,
            cost_price_minor,
            is_active_int,
            existing.id
        ],
    )?;

    get_variant(conn, &input.id)?
        .ok_or_else(|| VariantError::Database("Failed to load updated variant".into()))
}

/// Soft-deletes a product variant by setting `is_active = 0` and `deleted_at = datetime('now')`.
/// Preserves historical records and foreign key relationships. Never issues a hard DELETE.
pub fn soft_delete_variant(conn: &Connection, id: &str) -> Result<(), VariantError> {
    let existing = get_variant(conn, id)?
        .ok_or_else(|| VariantError::NotFound(format!("Variant '{id}' not found")))?;

    if !existing.is_active && existing.deleted_at.is_some() {
        return Ok(());
    }

    conn.execute(
        "UPDATE product_variants
         SET is_active = 0, deleted_at = datetime('now'), updated_at = datetime('now')
         WHERE id = ?1",
        params![id],
    )?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Cartesian Matrix Generation & Bulk Operations
// ---------------------------------------------------------------------------

/// Deterministic Cartesian product calculation over an arbitrary number of dimensions.
pub fn compute_cartesian_product(dimensions: &[Vec<String>]) -> Vec<Vec<String>> {
    if dimensions.is_empty() {
        return Vec::new();
    }
    for dim in dimensions {
        if dim.is_empty() {
            return Vec::new();
        }
    }
    let mut result: Vec<Vec<String>> = vec![Vec::new()];
    for dim in dimensions {
        let mut next = Vec::with_capacity(result.len() * dim.len());
        for existing in &result {
            for val in dim {
                let mut combo = existing.clone();
                combo.push(val.clone());
                next.push(combo);
            }
        }
        result = next;
    }
    result
}

fn validate_parent_product_for_matrix(
    conn: &Connection,
    product_id: &str,
) -> Result<(), VariantError> {
    let product = crate::product::get_product(conn, product_id)?
        .ok_or_else(|| VariantError::NotFound(format!("Product '{product_id}' not found")))?;

    if !product.is_active {
        return Err(VariantError::Validation(format!(
            "Product '{product_id}' is inactive/deleted"
        )));
    }

    if product.product_type.as_str() != "variable" {
        return Err(VariantError::Validation(format!(
            "Product '{product_id}' has product_type '{}', but matrix generation requires 'variable'",
            product.product_type
        )));
    }

    Ok(())
}

fn check_duplicate_attribute_definitions(
    dimensions: &[MatrixDimensionInput],
) -> Result<(), VariantError> {
    let mut seen_defs = std::collections::HashSet::new();
    for dim in dimensions {
        if !seen_defs.insert(&dim.attribute_definition_id) {
            return Err(VariantError::Validation(format!(
                "Duplicate attribute definition '{}' in matrix dimensions",
                dim.attribute_definition_id
            )));
        }
    }
    Ok(())
}

fn resolve_single_dimension_values(
    conn: &Connection,
    dim: &MatrixDimensionInput,
) -> Result<Vec<AttributeValue>, VariantError> {
    if dim.attribute_value_ids.is_empty() {
        return Err(VariantError::Validation(format!(
            "Attribute definition '{}' has no selected values",
            dim.attribute_definition_id
        )));
    }

    get_attribute_definition(conn, &dim.attribute_definition_id)?.ok_or_else(|| {
        VariantError::NotFound(format!(
            "Attribute definition '{}' not found",
            dim.attribute_definition_id
        ))
    })?;

    let mut seen_vals = std::collections::HashSet::new();
    let mut resolved_vals = Vec::with_capacity(dim.attribute_value_ids.len());

    for val_id in &dim.attribute_value_ids {
        if !seen_vals.insert(val_id) {
            return Err(VariantError::Validation(format!(
                "Duplicate attribute value '{val_id}' in dimension '{}'",
                dim.attribute_definition_id
            )));
        }

        let val = get_attribute_value(conn, val_id)?.ok_or_else(|| {
            VariantError::NotFound(format!("Attribute value '{val_id}' not found"))
        })?;

        if val.attribute_definition_id != dim.attribute_definition_id {
            return Err(VariantError::Validation(format!(
                "Attribute value '{}' belongs to definition '{}', not '{}'",
                val.id, val.attribute_definition_id, dim.attribute_definition_id
            )));
        }

        resolved_vals.push(val);
    }

    Ok(resolved_vals)
}

fn check_and_multiply_combinations(
    current_total: usize,
    count: usize,
) -> Result<usize, VariantError> {
    let next_total = current_total.checked_mul(count).ok_or_else(|| {
        VariantError::Validation(format!(
            "Matrix generation overflow: projected combinations exceed {MAX_CARTESIAN_COMBINATIONS}"
        ))
    })?;

    if next_total > MAX_CARTESIAN_COMBINATIONS {
        return Err(VariantError::Validation(format!(
            "Projected combination count ({next_total}) exceeds maximum allowed limit of {MAX_CARTESIAN_COMBINATIONS}"
        )));
    }

    Ok(next_total)
}

fn validate_matrix_dimensions(
    conn: &Connection,
    product_id: &str,
    dimensions: &[MatrixDimensionInput],
) -> Result<Vec<Vec<AttributeValue>>, VariantError> {
    validate_parent_product_for_matrix(conn, product_id)?;

    if dimensions.is_empty() {
        return Err(VariantError::Validation(
            "At least one attribute dimension is required for matrix generation".into(),
        ));
    }

    check_duplicate_attribute_definitions(dimensions)?;

    let mut resolved_dimensions = Vec::with_capacity(dimensions.len());
    let mut total_combinations: usize = 1;

    for dim in dimensions {
        let resolved_vals = resolve_single_dimension_values(conn, dim)?;
        total_combinations =
            check_and_multiply_combinations(total_combinations, resolved_vals.len())?;
        resolved_dimensions.push(resolved_vals);
    }

    Ok(resolved_dimensions)
}

/// Previews matrix generation without any database mutations or sequence increments.
pub fn preview_variant_matrix(
    conn: &Connection,
    input: PreviewMatrixInput,
) -> Result<MatrixPreviewResult, VariantError> {
    let resolved_dimensions =
        validate_matrix_dimensions(conn, &input.product_id, &input.dimensions)?;

    let value_id_matrix: Vec<Vec<String>> = resolved_dimensions
        .iter()
        .map(|dim| dim.iter().map(|v| v.id.clone()).collect())
        .collect();

    let combinations = compute_cartesian_product(&value_id_matrix);

    // Map attribute value objects by ID
    let mut val_map = std::collections::HashMap::new();
    for dim in &resolved_dimensions {
        for v in dim {
            val_map.insert(v.id.clone(), v.clone());
        }
    }

    // Load active variants with their attribute values
    let active_variants = list_variants_by_product(conn, &input.product_id, Some(true))?;
    let mut existing_combos: Vec<(String, std::collections::HashSet<String>)> = Vec::new();

    for v in active_variants {
        let vals = get_variant_attribute_values(conn, &v.id)?;
        let set: std::collections::HashSet<String> = vals.into_iter().map(|val| val.id).collect();
        existing_combos.push((v.id, set));
    }

    let mut preview_combinations = Vec::with_capacity(combinations.len());
    let mut new_count = 0;
    let mut existing_count = 0;

    for combo in combinations {
        let combo_set: std::collections::HashSet<String> = combo.iter().cloned().collect();
        let existing_match = existing_combos
            .iter()
            .find(|(_, set)| *set == combo_set)
            .map(|(id, _)| id.clone());

        let is_new = existing_match.is_none();
        if is_new {
            new_count += 1;
        } else {
            existing_count += 1;
        }

        let combo_vals: Vec<AttributeValue> = combo
            .iter()
            .filter_map(|vid| val_map.get(vid).cloned())
            .collect();

        preview_combinations.push(MatrixCombinationPreview {
            attribute_values: combo_vals,
            existing_variant_id: existing_match,
            is_new,
        });
    }

    Ok(MatrixPreviewResult {
        total_combinations: preview_combinations.len(),
        new_combinations_count: new_count,
        existing_combinations_count: existing_count,
        combinations: preview_combinations,
    })
}

/// Allocates an active, unoccupied Variant SKU using the canonical F2.03 generator with bounded retry.
/// Follows ADR-0007 Decisions A, B, and C:
/// - Reuses canonical `crate::barcode::generate_next_sku`.
/// - Table-local namespace: checks candidates against `product_variants`.
/// - Archived variant SKUs remain reserved (Decision C: query checks all rows in `product_variants`).
fn allocate_variant_sku(
    tx: &rusqlite::Transaction<'_>,
    prefix: &str,
) -> Result<String, VariantError> {
    for _ in 0..MAX_SKU_COLLISION_RETRIES {
        let candidate = crate::barcode::generate_next_sku(tx, Some(prefix))?;

        // Under Decision C and migration 001's unconditional UNIQUE constraint,
        // candidate must not exist anywhere in product_variants (active or archived).
        let occupied: bool = tx
            .query_row(
                "SELECT 1 FROM product_variants WHERE sku = ?1 COLLATE NOCASE",
                params![candidate],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !occupied {
            return Ok(candidate);
        }
    }

    Err(VariantError::Validation(format!(
        "Failed to allocate unique Variant SKU for prefix '{prefix}' after {MAX_SKU_COLLISION_RETRIES} sequence increments"
    )))
}

/// Generates variants for all unrepresented combinations of the matrix inside an atomic transaction.
/// Preserves existing active variants, their IDs, prices, and SKUs.
/// Follows ADR-0007 Decisions A through F.
pub fn generate_variant_matrix(
    conn: &Connection,
    input: GenerateMatrixInput,
) -> Result<MatrixGenerationResult, VariantError> {
    let default_price_minor = validate_price_minor(input.default_price_override_minor)?;
    let default_cost_minor = validate_price_minor(input.default_cost_price_minor)?;

    let resolved_dimensions =
        validate_matrix_dimensions(conn, &input.product_id, &input.dimensions)?;

    let value_id_matrix: Vec<Vec<String>> = resolved_dimensions
        .iter()
        .map(|dim| dim.iter().map(|v| v.id.clone()).collect())
        .collect();

    let combinations = compute_cartesian_product(&value_id_matrix);

    let mut val_map = std::collections::HashMap::new();
    for dim in &resolved_dimensions {
        for v in dim {
            val_map.insert(v.id.clone(), v.clone());
        }
    }

    // Begin immediate write transaction for atomic batch generation
    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;

    let active_variants = list_variants_by_product(&tx, &input.product_id, Some(true))?;
    let mut existing_combos: Vec<(ProductVariant, Vec<AttributeValue>)> = Vec::new();

    for v in active_variants {
        let vals = get_variant_attribute_values(&tx, &v.id)?;
        existing_combos.push((v, vals));
    }

    let mut created_variants = Vec::new();
    let mut existing_results = Vec::new();

    for combo in combinations {
        let combo_set: std::collections::HashSet<String> = combo.iter().cloned().collect();

        // Check if an active variant already exists with this exact combination
        let existing_match = existing_combos.iter().find(|(_, vals)| {
            let set: std::collections::HashSet<String> =
                vals.iter().map(|v| v.id.clone()).collect();
            set == combo_set
        });

        if let Some((existing_var, existing_vals)) = existing_match {
            existing_results.push(VariantWithAttributes {
                variant: existing_var.clone(),
                attribute_values: existing_vals.clone(),
            });
            continue;
        }

        // Allocate SKU if prefix was provided (ADR-0007 Decision 1 & Decision B)
        let sku = if let Some(ref prefix) = input.sku_prefix {
            Some(allocate_variant_sku(&tx, prefix)?)
        } else {
            None
        };

        let variant_id = uuid::Uuid::new_v4().to_string();

        tx.execute(
            "INSERT INTO product_variants (
                id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at
             ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, 1, datetime('now'), datetime('now'))",
            params![
                variant_id,
                input.product_id,
                sku,
                default_price_minor,
                default_cost_minor,
            ],
        )?;

        for val_id in &combo {
            tx.execute(
                "INSERT INTO variant_attribute_values (variant_id, attribute_value_id)
                 VALUES (?1, ?2)",
                params![variant_id, val_id],
            )?;
        }

        let new_var = tx.query_row(
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants WHERE id = ?1",
            params![variant_id],
            map_product_variant_row,
        )?;

        let combo_vals: Vec<AttributeValue> = combo
            .iter()
            .filter_map(|vid| val_map.get(vid).cloned())
            .collect();

        created_variants.push(VariantWithAttributes {
            variant: new_var,
            attribute_values: combo_vals,
        });
    }

    tx.commit()?;

    let total = created_variants.len() + existing_results.len();
    Ok(MatrixGenerationResult {
        total_combinations: total,
        created_count: created_variants.len(),
        existing_count: existing_results.len(),
        created_variants,
        existing_variants: existing_results,
    })
}

/// Atomically bulk updates active status for a list of variant IDs.
/// When activating (`is_active = true`), validates active-combination uniqueness and barcode uniqueness.
pub fn bulk_update_variant_status(
    conn: &Connection,
    input: BulkUpdateVariantStatusInput,
) -> Result<BulkOperationResult, VariantError> {
    if input.variant_ids.is_empty() {
        return Ok(BulkOperationResult {
            updated_count: 0,
            affected_variant_ids: Vec::new(),
        });
    }

    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;

    for id in &input.variant_ids {
        let variant = get_variant(&tx, id)?
            .ok_or_else(|| VariantError::NotFound(format!("Variant '{id}' not found")))?;

        if input.is_active {
            // When reactivating, verify active-combination uniqueness and barcode conflict
            check_variant_combination_conflict_on_activation(&tx, &variant.product_id, id)?;
            if let Some(ref bc) = variant.barcode {
                check_variant_barcode_conflict(&tx, bc, Some(id))?;
            }

            tx.execute(
                "UPDATE product_variants
                 SET is_active = 1, deleted_at = NULL, updated_at = datetime('now')
                 WHERE id = ?1",
                params![id],
            )?;
        } else {
            tx.execute(
                "UPDATE product_variants
                 SET is_active = 0, deleted_at = datetime('now'), updated_at = datetime('now')
                 WHERE id = ?1",
                params![id],
            )?;
        }
    }

    tx.commit()?;

    Ok(BulkOperationResult {
        updated_count: input.variant_ids.len(),
        affected_variant_ids: input.variant_ids,
    })
}

/// Atomically bulk updates price overrides for a list of variant IDs.
pub fn bulk_update_variant_prices(
    conn: &Connection,
    input: BulkUpdateVariantPricesInput,
) -> Result<BulkOperationResult, VariantError> {
    if input.variant_ids.is_empty() {
        return Ok(BulkOperationResult {
            updated_count: 0,
            affected_variant_ids: Vec::new(),
        });
    }

    let price_override_minor = validate_price_minor(input.price_override_minor)?;
    let cost_price_minor = validate_price_minor(input.cost_price_minor)?;

    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;

    for id in &input.variant_ids {
        get_variant(&tx, id)?
            .ok_or_else(|| VariantError::NotFound(format!("Variant '{id}' not found")))?;

        tx.execute(
            "UPDATE product_variants
             SET price_override_minor = ?1,
                 cost_price_minor = ?2,
                 updated_at = datetime('now')
             WHERE id = ?3",
            params![price_override_minor, cost_price_minor, id],
        )?;
    }

    tx.commit()?;

    Ok(BulkOperationResult {
        updated_count: input.variant_ids.len(),
        affected_variant_ids: input.variant_ids,
    })
}

/// Resolves a single active variant and its attribute values by barcode.
pub fn get_variant_by_barcode(
    conn: &Connection,
    barcode: &str,
) -> Result<Option<VariantWithAttributes>, VariantError> {
    let trimmed = barcode.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let variant: Option<ProductVariant> = conn
        .query_row(
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants
             WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1 AND deleted_at IS NULL",
            params![trimmed],
            map_product_variant_row,
        )
        .optional()?;

    match variant {
        Some(v) => {
            let attribute_values = get_variant_attribute_values(conn, &v.id)?;
            Ok(Some(VariantWithAttributes {
                variant: v,
                attribute_values,
            }))
        }
        None => Ok(None),
    }
}

/// Resolves a single active variant and its attribute values by SKU.
pub fn get_variant_by_sku(
    conn: &Connection,
    sku: &str,
) -> Result<Option<VariantWithAttributes>, VariantError> {
    let trimmed = sku.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let variant: Option<ProductVariant> = conn
        .query_row(
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants
             WHERE sku = ?1 COLLATE NOCASE AND is_active = 1 AND deleted_at IS NULL",
            params![trimmed],
            map_product_variant_row,
        )
        .optional()?;

    match variant {
        Some(v) => {
            let attribute_values = get_variant_attribute_values(conn, &v.id)?;
            Ok(Some(VariantWithAttributes {
                variant: v,
                attribute_values,
            }))
        }
        None => Ok(None),
    }
}

/// Searches variants by SKU, barcode, or attribute value for a given product or across the catalog.
/// Escapes LIKE wildcards (`\`, `%`, `_`) and applies an explicit limit (`DEFAULT_SEARCH_LIMIT`).
/// Empty or blank queries return empty results without scanning the catalog.
pub fn search_variants(
    conn: &Connection,
    product_id: Option<&str>,
    query: &str,
) -> Result<Vec<VariantWithAttributes>, VariantError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let escaped = escape_like_pattern(trimmed);
    let pattern = format!("%{escaped}%");
    let limit = DEFAULT_SEARCH_LIMIT as i64;

    let sql = match product_id {
        Some(_) => {
            "SELECT DISTINCT pv.id, pv.product_id, pv.sku, pv.barcode, pv.price_override_minor,
                    pv.cost_price_minor, pv.is_active, pv.created_at, pv.updated_at, pv.deleted_at
             FROM product_variants pv
             LEFT JOIN variant_attribute_values vav ON pv.id = vav.variant_id
             LEFT JOIN attribute_values av ON vav.attribute_value_id = av.id
             WHERE pv.product_id = ?1 AND pv.is_active = 1 AND pv.deleted_at IS NULL
               AND (pv.sku LIKE ?2 ESCAPE '\\' OR pv.barcode LIKE ?2 ESCAPE '\\' OR av.value LIKE ?2 ESCAPE '\\')
             ORDER BY pv.created_at ASC
             LIMIT ?3"
        }
        None => {
            "SELECT DISTINCT pv.id, pv.product_id, pv.sku, pv.barcode, pv.price_override_minor,
                    pv.cost_price_minor, pv.is_active, pv.created_at, pv.updated_at, pv.deleted_at
             FROM product_variants pv
             LEFT JOIN variant_attribute_values vav ON pv.id = vav.variant_id
             LEFT JOIN attribute_values av ON vav.attribute_value_id = av.id
             WHERE pv.is_active = 1 AND pv.deleted_at IS NULL
               AND (pv.sku LIKE ?1 ESCAPE '\\' OR pv.barcode LIKE ?1 ESCAPE '\\' OR av.value LIKE ?1 ESCAPE '\\')
             ORDER BY pv.created_at ASC
             LIMIT ?2"
        }
    };

    let mut stmt = conn.prepare(sql)?;
    let variants: Vec<ProductVariant> = match product_id {
        Some(pid) => stmt
            .query_map(params![pid, pattern, limit], map_product_variant_row)?
            .filter_map(Result::ok)
            .collect(),
        None => stmt
            .query_map(params![pattern, limit], map_product_variant_row)?
            .filter_map(Result::ok)
            .collect(),
    };

    let mut results = Vec::with_capacity(variants.len());
    for v in variants {
        let attribute_values = get_variant_attribute_values(conn, &v.id)?;
        results.push(VariantWithAttributes {
            variant: v,
            attribute_values,
        });
    }

    Ok(results)
}
