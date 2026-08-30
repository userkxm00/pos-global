// Product Variants & Matrix Domain Model, validation rules, and SQLite operations.
// F2.05-T1 — Schema / Migration 014 + Domain Contract Foundation

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

/// Association between a variant and an attribute value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariantAttributeValue {
    pub variant_id: String,
    pub attribute_value_id: String,
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

impl From<rusqlite::Error> for VariantError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref f, Some(ref msg)) = e {
            if f.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                if msg.contains("attribute_definitions.name")
                    || msg.contains("idx_attribute_definitions_name")
                {
                    return VariantError::DuplicateName(
                        "An attribute definition with this name already exists".into(),
                    );
                }
                if msg.contains("attribute_values") || msg.contains("idx_attribute_values_def_val")
                {
                    return VariantError::DuplicateValue(
                        "This attribute value already exists for this definition".into(),
                    );
                }
                if msg.contains("product_variants.sku")
                    || msg.contains("idx_product_variants_sku_active")
                {
                    return VariantError::DuplicateSku(
                        "SKU is already assigned to another active variant".into(),
                    );
                }
                if msg.contains("product_barcodes.barcode")
                    || msg.contains("idx_product_variants_barcode_active")
                {
                    return VariantError::DuplicateBarcode(
                        "Barcode is already assigned to another active variant or product".into(),
                    );
                }
            }
        }
        VariantError::Database(e.to_string())
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

    // Verify combination uniqueness for active variants of this product
    if !input.attribute_value_ids.is_empty() {
        let existing_variants = list_variants_by_product(conn, &input.product_id, Some(true))?;
        for existing in existing_variants {
            let existing_vals = get_variant_attribute_values(conn, &existing.id)?;
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

    let variant_id = uuid::Uuid::new_v4().to_string();

    conn.execute(
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
        conn.execute(
            "INSERT INTO variant_attribute_values (variant_id, attribute_value_id)
             VALUES (?1, ?2)",
            params![variant_id, val_id],
        )?;
    }

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
