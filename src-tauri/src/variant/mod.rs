// Module for product attributes and variant matrix domain.
// F2.05 — Variants / Matrix

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Maximum safe integer minor units that can be converted to and from IEEE 754 f64 without precision loss.
pub const MAX_SAFE_MINOR_UNITS: i64 = 90_071_992_547_409;

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

/// Canonical Product Variant entity.
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

/// Variant with its associated attribute values.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVariantInput {
    pub id: String,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_override_minor: Option<i64>,
    pub cost_price_minor: Option<i64>,
    pub is_active: bool,
}

/// Maximum allowed Cartesian combinations per generation request (ADR-0007 Decision 3).
pub const MAX_CARTESIAN_COMBINATIONS: usize = 5_000;

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
            "SKU is already assigned to another active variant".into(),
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
            crate::barcode::BarcodeError::Database(msg) => VariantError::Database(msg),
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

    // Verify attribute definition exists
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

pub fn create_variant(
    conn: &Connection,
    input: CreateVariantInput,
) -> Result<VariantWithAttributes, VariantError> {
    let price_override_minor = validate_price_minor(input.price_override_minor)?;
    let cost_price_minor = validate_price_minor(input.cost_price_minor)?;

    // Verify parent product exists
    let product_exists: Option<String> = conn
        .query_row(
            "SELECT id FROM products WHERE id = ?1",
            params![input.product_id],
            |row| row.get(0),
        )
        .optional()?;

    if product_exists.is_none() {
        return Err(VariantError::NotFound(format!(
            "Parent product '{}' not found",
            input.product_id
        )));
    }

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

    // Verify combination uniqueness for active variants of this product inside immediate write transaction
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
            input.sku,
            input.barcode,
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

pub fn list_variants_by_product(
    conn: &Connection,
    product_id: &str,
    active_only: Option<bool>,
) -> Result<Vec<ProductVariant>, VariantError> {
    let sql = match active_only {
        Some(true) => {
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants
             WHERE product_id = ?1 AND is_active = 1
             ORDER BY created_at ASC"
        }
        Some(false) => {
            "SELECT id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at, deleted_at
             FROM product_variants
             WHERE product_id = ?1 AND is_active = 0
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

pub fn update_variant(
    conn: &Connection,
    input: UpdateVariantInput,
) -> Result<ProductVariant, VariantError> {
    let price_override_minor = validate_price_minor(input.price_override_minor)?;
    let cost_price_minor = validate_price_minor(input.cost_price_minor)?;

    let existing = get_variant(conn, &input.id)?
        .ok_or_else(|| VariantError::NotFound(format!("Variant '{}' not found", input.id)))?;

    conn.execute(
        "UPDATE product_variants
         SET sku = ?1, barcode = ?2, price_override_minor = ?3, cost_price_minor = ?4,
             is_active = ?5, updated_at = datetime('now')
         WHERE id = ?6",
        params![
            input.sku,
            input.barcode,
            price_override_minor,
            cost_price_minor,
            if input.is_active { 1 } else { 0 },
            existing.id
        ],
    )?;

    get_variant(conn, &input.id)?
        .ok_or_else(|| VariantError::Database("Failed to load updated variant".into()))
}

pub fn soft_delete_variant(conn: &Connection, id: &str) -> Result<(), VariantError> {
    let existing = get_variant(conn, id)?
        .ok_or_else(|| VariantError::NotFound(format!("Variant '{id}' not found")))?;

    if !existing.is_active {
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
    let mut result: Vec<Vec<String>> = vec![Vec::new()];
    for dim in dimensions {
        if dim.is_empty() {
            return Vec::new();
        }
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

fn validate_matrix_dimensions(
    conn: &Connection,
    product_id: &str,
    dimensions: &[MatrixDimensionInput],
) -> Result<Vec<Vec<AttributeValue>>, VariantError> {
    let product = crate::product::get_product(conn, product_id)?
        .ok_or_else(|| VariantError::NotFound(format!("Product '{product_id}' not found")))?;

    if !product.is_active {
        return Err(VariantError::Validation(format!(
            "Product '{product_id}' is inactive/deleted"
        )));
    }

    if product.product_type.as_deref() != Some("variable") {
        return Err(VariantError::Validation(format!(
            "Product '{product_id}' has product_type '{:?}', but matrix generation requires 'variable'",
            product.product_type
        )));
    }

    if dimensions.is_empty() {
        return Err(VariantError::Validation(
            "At least one attribute dimension is required for matrix generation".into(),
        ));
    }

    let mut seen_defs = std::collections::HashSet::new();
    for dim in dimensions {
        if !seen_defs.insert(&dim.attribute_definition_id) {
            return Err(VariantError::Validation(format!(
                "Duplicate attribute definition '{}' in matrix dimensions",
                dim.attribute_definition_id
            )));
        }
    }

    let mut resolved_dimensions = Vec::with_capacity(dimensions.len());
    let mut total_combinations: usize = 1;

    for dim in dimensions {
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

        total_combinations = total_combinations
            .checked_mul(resolved_vals.len())
            .ok_or_else(|| {
                VariantError::Validation("Cartesian product size calculation overflowed".into())
            })?;

        if total_combinations > MAX_CARTESIAN_COMBINATIONS {
            return Err(VariantError::Validation(format!(
                "Projected matrix size of {total_combinations} combinations exceeds the maximum safety limit of {MAX_CARTESIAN_COMBINATIONS}"
            )));
        }

        resolved_dimensions.push(resolved_vals);
    }

    Ok(resolved_dimensions)
}

/// Side-effect free preview of a variant matrix (ADR-0007 Decision 4).
pub fn preview_variant_matrix(
    conn: &Connection,
    input: PreviewMatrixInput,
) -> Result<MatrixPreviewResult, VariantError> {
    let resolved_dimensions =
        validate_matrix_dimensions(conn, &input.product_id, &input.dimensions)?;

    let dimension_value_ids: Vec<Vec<String>> = resolved_dimensions
        .iter()
        .map(|dim| dim.iter().map(|v| v.id.clone()).collect())
        .collect();

    let id_combinations = compute_cartesian_product(&dimension_value_ids);

    let mut val_map: std::collections::HashMap<String, AttributeValue> =
        std::collections::HashMap::new();
    for dim in &resolved_dimensions {
        for val in dim {
            val_map.insert(val.id.clone(), val.clone());
        }
    }

    let active_variants = list_variants_by_product(conn, &input.product_id, Some(true))?;
    let mut existing_combos: std::collections::HashMap<Vec<String>, String> =
        std::collections::HashMap::new();
    for variant in active_variants {
        let mut attr_vals = get_variant_attribute_values(conn, &variant.id)?;
        attr_vals.sort_by(|a, b| a.id.cmp(&b.id));
        let key: Vec<String> = attr_vals.into_iter().map(|v| v.id).collect();
        existing_combos.insert(key, variant.id);
    }

    let mut previews = Vec::with_capacity(id_combinations.len());
    let mut new_count = 0;
    let mut existing_count = 0;

    for combo_ids in id_combinations {
        let mut sorted_ids = combo_ids.clone();
        sorted_ids.sort();

        let existing_id = existing_combos.get(&sorted_ids).cloned();
        let is_new = existing_id.is_none();
        if is_new {
            new_count += 1;
        } else {
            existing_count += 1;
        }

        let attribute_values = combo_ids
            .iter()
            .filter_map(|id| val_map.get(id).cloned())
            .collect();

        previews.push(MatrixCombinationPreview {
            attribute_values,
            existing_variant_id: existing_id,
            is_new,
        });
    }

    Ok(MatrixPreviewResult {
        total_combinations: previews.len(),
        new_combinations_count: new_count,
        existing_combinations_count: existing_count,
        combinations: previews,
    })
}

/// Atomically generates missing matrix combinations while preserving existing active variants (ADR-0007 Decision 5).
pub fn generate_variant_matrix(
    conn: &Connection,
    input: GenerateMatrixInput,
) -> Result<MatrixGenerationResult, VariantError> {
    let resolved_dimensions =
        validate_matrix_dimensions(conn, &input.product_id, &input.dimensions)?;
    let default_price_override = validate_price_minor(input.default_price_override_minor)?;
    let default_cost_price = validate_price_minor(input.default_cost_price_minor)?;

    let dimension_value_ids: Vec<Vec<String>> = resolved_dimensions
        .iter()
        .map(|dim| dim.iter().map(|v| v.id.clone()).collect())
        .collect();

    let id_combinations = compute_cartesian_product(&dimension_value_ids);

    let mut val_map: std::collections::HashMap<String, AttributeValue> =
        std::collections::HashMap::new();
    for dim in &resolved_dimensions {
        for val in dim {
            val_map.insert(val.id.clone(), val.clone());
        }
    }

    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate);

    let active_variants = list_variants_by_product(&tx, &input.product_id, Some(true))?;
    let mut existing_combos: std::collections::HashMap<Vec<String>, VariantWithAttributes> =
        std::collections::HashMap::new();
    for variant in active_variants {
        let attr_vals = get_variant_attribute_values(&tx, &variant.id)?;
        let mut sorted_attr_ids: Vec<String> = attr_vals.iter().map(|v| v.id.clone()).collect();
        sorted_attr_ids.sort();
        existing_combos.insert(
            sorted_attr_ids,
            VariantWithAttributes {
                variant,
                attribute_values: attr_vals,
            },
        );
    }

    let mut created_variants = Vec::new();
    let mut existing_variants = Vec::new();

    for combo_ids in id_combinations {
        let mut sorted_ids = combo_ids.clone();
        sorted_ids.sort();

        if let Some(existing) = existing_combos.get(&sorted_ids) {
            existing_variants.push(existing.clone());
            continue;
        }

        let sku = if let Some(ref prefix) = input.sku_prefix {
            Some(crate::barcode::generate_next_sku(&tx, Some(prefix))?)
        } else {
            None
        };

        let variant_id = uuid::Uuid::new_v4().to_string();

        tx.execute(
            "INSERT INTO product_variants (id, product_id, sku, barcode, price_override_minor, cost_price_minor, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, 1, datetime('now'), datetime('now'))",
            params![
                variant_id,
                input.product_id,
                sku,
                default_price_override,
                default_cost_price,
            ],
        )?;

        for val_id in &combo_ids {
            tx.execute(
                "INSERT INTO variant_attribute_values (id, variant_id, attribute_value_id, created_at)
                 VALUES (?1, ?2, ?3, datetime('now'))",
                params![uuid::Uuid::new_v4().to_string(), variant_id, val_id],
            )?;
        }

        let new_variant = get_variant(&tx, &variant_id)?.ok_or_else(|| {
            VariantError::Database("Failed to load newly created matrix variant".into())
        })?;

        let attribute_values = combo_ids
            .iter()
            .filter_map(|id| val_map.get(id).cloned())
            .collect();

        let created_item = VariantWithAttributes {
            variant: new_variant,
            attribute_values,
        };

        existing_combos.insert(sorted_ids, created_item.clone());
        created_variants.push(created_item);
    }

    tx.commit()?;

    let total = created_variants.len() + existing_variants.len();

    Ok(MatrixGenerationResult {
        total_combinations: total,
        created_count: created_variants.len(),
        existing_count: existing_variants.len(),
        created_variants,
        existing_variants,
    })
}

/// Atomically updates active status for a list of variants.
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

    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate);

    for id in &input.variant_ids {
        let existing = get_variant(&tx, id)?
            .ok_or_else(|| VariantError::NotFound(format!("Variant '{id}' not found")))?;

        if existing.is_active != input.is_active {
            tx.execute(
                "UPDATE product_variants
                 SET is_active = ?1,
                     deleted_at = CASE WHEN ?1 = 0 THEN datetime('now') ELSE NULL END,
                     updated_at = datetime('now')
                 WHERE id = ?2",
                params![if input.is_active { 1 } else { 0 }, id],
            )?;
        }
    }

    tx.commit()?;

    Ok(BulkOperationResult {
        updated_count: input.variant_ids.len(),
        affected_variant_ids: input.variant_ids,
    })
}

/// Atomically updates prices for a list of variants.
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

    let tx = rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate);

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
             WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1",
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
             WHERE sku = ?1 COLLATE NOCASE AND is_active = 1",
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

/// Searches variants by SKU, barcode, or attribute value for a given product or across catalog.
pub fn search_variants(
    conn: &Connection,
    product_id: Option<&str>,
    query: &str,
) -> Result<Vec<VariantWithAttributes>, VariantError> {
    let trimmed = query.trim();
    let pattern = format!("%{trimmed}%");

    let sql = match product_id {
        Some(_) => {
            "SELECT DISTINCT pv.id, pv.product_id, pv.sku, pv.barcode, pv.price_override_minor,
                    pv.cost_price_minor, pv.is_active, pv.created_at, pv.updated_at, pv.deleted_at
             FROM product_variants pv
             LEFT JOIN variant_attribute_values vav ON pv.id = vav.variant_id
             LEFT JOIN attribute_values av ON vav.attribute_value_id = av.id
             WHERE pv.product_id = ?1 AND pv.is_active = 1
               AND (pv.sku LIKE ?2 OR pv.barcode LIKE ?2 OR av.value LIKE ?2)
             ORDER BY pv.created_at ASC"
        }
        None => {
            "SELECT DISTINCT pv.id, pv.product_id, pv.sku, pv.barcode, pv.price_override_minor,
                    pv.cost_price_minor, pv.is_active, pv.created_at, pv.updated_at, pv.deleted_at
             FROM product_variants pv
             LEFT JOIN variant_attribute_values vav ON pv.id = vav.variant_id
             LEFT JOIN attribute_values av ON vav.attribute_value_id = av.id
             WHERE pv.is_active = 1
               AND (pv.sku LIKE ?1 OR pv.barcode LIKE ?1 OR av.value LIKE ?1)
             ORDER BY pv.created_at ASC"
        }
    };

    let mut stmt = conn.prepare(sql)?;
    let variants: Vec<ProductVariant> = match product_id {
        Some(pid) => stmt
            .query_map(params![pid, pattern], map_product_variant_row)?
            .filter_map(Result::ok)
            .collect(),
        None => stmt
            .query_map(params![pattern], map_product_variant_row)?
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
