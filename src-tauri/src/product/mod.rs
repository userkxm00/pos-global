// Product domain model, validation rules, and SQLite database operations.
// F2.01 — Product CRUD
//
// Barcode Invariant (F2.01 vs F2.03 Boundary):
// Per SQLite migrations 001_initial.sql and 009_remove_redundant_product_barcode_index.sql,
// `products.barcode` enforces a table-wide `UNIQUE` constraint across all rows. Soft-deleted
// products retain their stored barcode to protect historical sales, inventory, and audit ledger integrity.
// Advanced barcode lifecycle management, symbologies, and barcode reassignment are scoped to F2.03.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Maximum safe integer minor units that can be converted to and from IEEE 754 f64 without precision loss.
/// For decimal currency scaled by 100, the bound is floor((2^53 - 1) / 100) = 90_071_992_547_409 minor units
/// (over 900 billion units with 2 decimal places). For any integer x <= 90_071_992_547_409, the maximum
/// absolute floating-point roundoff error in (x / 100.0) * 100.0 is strictly < 0.041 (over 12x below the
/// 0.5 rounding boundary), guaranteeing exact lossless persistence through legacy SQLite REAL columns.
pub const MAX_SAFE_MINOR_UNITS: i64 = 90_071_992_547_409;

/// Canonical Product entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Product {
    pub id: String,
    pub category_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub barcode: Option<String>,
    pub product_type: String,
    pub base_price_minor: i64,
    pub cost_price_minor: Option<i64>,
    pub unit_type: Option<String>,
    pub requires_expiry: bool,
    pub requires_serial: bool,
    pub warranty_months: Option<i32>,
    pub custom_attributes: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input payload for creating a new product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductInput {
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    pub product_type: Option<String>,
    pub base_price_minor: i64,
    pub cost_price_minor: Option<i64>,
    pub unit_type: Option<String>,
    pub requires_expiry: Option<bool>,
    pub requires_serial: Option<bool>,
    pub warranty_months: Option<i32>,
    pub custom_attributes: Option<String>,
}

/// Input payload for updating an existing product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    pub product_type: String,
    pub base_price_minor: i64,
    pub cost_price_minor: Option<i64>,
    pub unit_type: Option<String>,
    pub requires_expiry: bool,
    pub requires_serial: bool,
    pub warranty_months: Option<i32>,
    pub custom_attributes: Option<String>,
    pub is_active: bool,
}

/// Filter criteria for querying/listing products.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductFilter {
    pub query: Option<String>,
    pub is_active: Option<bool>,
    pub category_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Typed domain and repository errors for product operations.
#[derive(Debug, PartialEq, Eq)]
pub enum ProductError {
    Validation(String),
    NotFound(String),
    DuplicateBarcode(String),
    Database(String),
}

impl std::fmt::Display for ProductError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductError::Validation(msg) => write!(f, "Validation error: {msg}"),
            ProductError::NotFound(msg) => write!(f, "Product not found: {msg}"),
            ProductError::DuplicateBarcode(msg) => write!(f, "Duplicate barcode error: {msg}"),
            ProductError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for ProductError {}

impl From<rusqlite::Error> for ProductError {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::SqliteFailure(ref f, Some(ref msg))
                if f.code == rusqlite::ffi::ErrorCode::ConstraintViolation
                    && (msg.contains("products.barcode")
                        || msg.contains("UNIQUE constraint failed: products.barcode")) =>
            {
                ProductError::DuplicateBarcode("Barcode already assigned to another product".into())
            }
            _ => ProductError::Database(e.to_string()),
        }
    }
}

/// Resolves the organization ownership of the local product catalog from business_settings or branches.
pub fn get_catalog_organization_id(conn: &Connection) -> Result<Option<String>, rusqlite::Error> {
    // 1. Check business_settings if configured
    let org_from_settings: Option<String> = conn
        .query_row(
            "SELECT organization_id FROM business_settings WHERE organization_id IS NOT NULL LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if org_from_settings.is_some() {
        return Ok(org_from_settings);
    }

    // 2. Fallback to unique organization from local branches
    let mut stmt = conn.prepare(
        "SELECT DISTINCT organization_id FROM branches WHERE organization_id IS NOT NULL",
    )?;
    let orgs: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(Result::ok)
        .collect();
    if orgs.len() == 1 {
        return Ok(Some(orgs[0].clone()));
    }

    Ok(None)
}

/// Converts a floating-point database price into integer minor units (cents).
/// Exact conversion holds for integer minor units within `[-MAX_SAFE_MINOR_UNITS, MAX_SAFE_MINOR_UNITS]`.
pub fn real_to_minor(real: f64) -> i64 {
    (real * 100.0).round() as i64
}

/// Converts integer minor units into floating-point database representation.
/// Exact conversion holds for integer minor units within `[-MAX_SAFE_MINOR_UNITS, MAX_SAFE_MINOR_UNITS]`.
pub fn minor_to_real(minor: i64) -> f64 {
    minor as f64 / 100.0
}

/// Validates product name. Must be non-empty and <= 255 Unicode characters.
pub fn validate_name(name: &str) -> Result<String, ProductError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ProductError::Validation(
            "Product name cannot be empty".into(),
        ));
    }
    if trimmed.chars().count() > 255 {
        return Err(ProductError::Validation(
            "Product name exceeds maximum length of 255 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates base price in minor units. Must be non-negative and within safe precision range.
pub fn validate_base_price_minor(price: i64) -> Result<i64, ProductError> {
    if price < 0 {
        return Err(ProductError::Validation(
            "Base price cannot be negative".into(),
        ));
    }
    if price > MAX_SAFE_MINOR_UNITS {
        return Err(ProductError::Validation(
            "Base price exceeds maximum supported precision".into(),
        ));
    }
    Ok(price)
}

/// Validates cost price in minor units if provided. Must be non-negative and within safe precision range.
pub fn validate_cost_price_minor(cost: Option<i64>) -> Result<Option<i64>, ProductError> {
    if let Some(c) = cost {
        if c < 0 {
            return Err(ProductError::Validation(
                "Cost price cannot be negative".into(),
            ));
        }
        if c > MAX_SAFE_MINOR_UNITS {
            return Err(ProductError::Validation(
                "Cost price exceeds maximum supported precision".into(),
            ));
        }
    }
    Ok(cost)
}

/// Validates barcode. Trims whitespace and normalizes empty string to None.
pub fn validate_barcode(barcode: Option<&str>) -> Option<String> {
    barcode
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// Validates product type. Allowed types: 'simple', 'variable', 'weighted'.
pub fn validate_product_type(ptype: Option<&str>) -> Result<String, ProductError> {
    match ptype.map(str::trim).filter(|s| !s.is_empty()) {
        None | Some("simple") => Ok("simple".to_string()),
        Some("variable") => Ok("variable".to_string()),
        Some("weighted") => Ok("weighted".to_string()),
        Some(invalid) => Err(ProductError::Validation(format!(
            "Invalid product_type '{invalid}'. Allowed: 'simple', 'variable', 'weighted'"
        ))),
    }
}

/// Maps a rusqlite row to the domain `Product` struct.
fn map_product_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Product> {
    let base_price_real: f64 = row.get("base_price")?;
    let cost_price_real: Option<f64> = row.get("cost_price")?;
    let is_active_int: i64 = row.get("is_active")?;
    let req_exp_int: i64 = row.get("requires_expiry")?;
    let req_ser_int: i64 = row.get("requires_serial")?;

    Ok(Product {
        id: row.get("id")?,
        category_id: row.get("category_id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        barcode: row.get("barcode")?,
        product_type: row.get("product_type")?,
        base_price_minor: real_to_minor(base_price_real),
        cost_price_minor: cost_price_real.map(real_to_minor),
        unit_type: row.get("unit_type")?,
        requires_expiry: req_exp_int != 0,
        requires_serial: req_ser_int != 0,
        warranty_months: row.get("warranty_months")?,
        custom_attributes: row.get("custom_attributes")?,
        is_active: is_active_int != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

const PRODUCT_COLUMNS: &str = "id, category_id, name, description, barcode, product_type, base_price, cost_price, unit_type, requires_expiry, requires_serial, warranty_months, custom_attributes, is_active, created_at, updated_at";

const GET_PRODUCT_SQL: &str = "SELECT id, category_id, name, description, barcode, product_type, base_price, cost_price, unit_type, requires_expiry, requires_serial, warranty_months, custom_attributes, is_active, created_at, updated_at FROM products WHERE id = ?1";

const GET_PRODUCT_BY_BARCODE_SQL: &str = "SELECT id, category_id, name, description, barcode, product_type, base_price, cost_price, unit_type, requires_expiry, requires_serial, warranty_months, custom_attributes, is_active, created_at, updated_at FROM products WHERE barcode = ?1";

/// Creates a new product in the local SQLite database.
pub fn create_product(
    conn: &Connection,
    input: CreateProductInput,
) -> Result<Product, ProductError> {
    let name = validate_name(&input.name)?;
    let base_price_minor = validate_base_price_minor(input.base_price_minor)?;
    let cost_price_minor = validate_cost_price_minor(input.cost_price_minor)?;
    let barcode = validate_barcode(input.barcode.as_deref());
    let product_type = validate_product_type(input.product_type.as_deref())?;

    // Check barcode conflict proactively if barcode is present
    if let Some(ref bc) = barcode {
        let existing: Option<String> = conn
            .query_row("SELECT id FROM products WHERE barcode = ?1", [bc], |row| {
                row.get(0)
            })
            .optional()?;
        if let Some(existing_id) = existing {
            return Err(ProductError::DuplicateBarcode(format!(
                "Barcode '{bc}' is already assigned to product '{existing_id}'"
            )));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let base_price_real = minor_to_real(base_price_minor);
    let cost_price_real = cost_price_minor.map(minor_to_real);
    let description = input
        .description
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let category_id = input
        .category_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let unit_type = input
        .unit_type
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let requires_expiry = if input.requires_expiry.unwrap_or(false) {
        1
    } else {
        0
    };
    let requires_serial = if input.requires_serial.unwrap_or(false) {
        1
    } else {
        0
    };

    conn.execute(
        "INSERT INTO products (
            id, category_id, name, description, barcode, product_type,
            base_price, cost_price, unit_type, requires_expiry, requires_serial,
            warranty_months, custom_attributes, is_active, created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, 1, datetime('now'), datetime('now')
        )",
        params![
            id,
            category_id,
            name,
            description,
            barcode,
            product_type,
            base_price_real,
            cost_price_real,
            unit_type,
            requires_expiry,
            requires_serial,
            input.warranty_months,
            input.custom_attributes,
        ],
    )?;

    get_product(conn, &id)?
        .ok_or_else(|| ProductError::Database("Failed to retrieve created product".into()))
}

/// Retrieves a product by its unique ID.
pub fn get_product(conn: &Connection, id: &str) -> Result<Option<Product>, ProductError> {
    let result = conn
        .query_row(GET_PRODUCT_SQL, [id], map_product_row)
        .optional()?;
    Ok(result)
}

/// Retrieves a product by exact barcode match.
pub fn get_product_by_barcode(
    conn: &Connection,
    barcode: &str,
) -> Result<Option<Product>, ProductError> {
    let trimmed = barcode.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let result = conn
        .query_row(GET_PRODUCT_BY_BARCODE_SQL, [trimmed], map_product_row)
        .optional()?;
    Ok(result)
}

/// Updates an existing product.
pub fn update_product(
    conn: &Connection,
    input: UpdateProductInput,
) -> Result<Product, ProductError> {
    let name = validate_name(&input.name)?;
    let base_price_minor = validate_base_price_minor(input.base_price_minor)?;
    let cost_price_minor = validate_cost_price_minor(input.cost_price_minor)?;
    let barcode = validate_barcode(input.barcode.as_deref());
    let product_type = validate_product_type(Some(&input.product_type))?;

    // Check that product exists
    let existing = get_product(conn, &input.id)?;
    if existing.is_none() {
        return Err(ProductError::NotFound(format!(
            "Product with ID '{}' not found",
            input.id
        )));
    }

    // Check barcode conflict if barcode is changing to another product's barcode
    if let Some(ref bc) = barcode {
        let conflict_id: Option<String> = conn
            .query_row(
                "SELECT id FROM products WHERE barcode = ?1 AND id != ?2",
                params![bc, input.id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(conflict) = conflict_id {
            return Err(ProductError::DuplicateBarcode(format!(
                "Barcode '{bc}' is already assigned to product '{conflict}'"
            )));
        }
    }

    let base_price_real = minor_to_real(base_price_minor);
    let cost_price_real = cost_price_minor.map(minor_to_real);
    let description = input
        .description
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let category_id = input
        .category_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let unit_type = input
        .unit_type
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let requires_expiry = if input.requires_expiry { 1 } else { 0 };
    let requires_serial = if input.requires_serial { 1 } else { 0 };
    let is_active = if input.is_active { 1 } else { 0 };

    let affected = conn.execute(
        "UPDATE products SET
            category_id = ?1,
            name = ?2,
            description = ?3,
            barcode = ?4,
            product_type = ?5,
            base_price = ?6,
            cost_price = ?7,
            unit_type = ?8,
            requires_expiry = ?9,
            requires_serial = ?10,
            warranty_months = ?11,
            custom_attributes = ?12,
            is_active = ?13,
            updated_at = datetime('now')
        WHERE id = ?14",
        params![
            category_id,
            name,
            description,
            barcode,
            product_type,
            base_price_real,
            cost_price_real,
            unit_type,
            requires_expiry,
            requires_serial,
            input.warranty_months,
            input.custom_attributes,
            is_active,
            input.id,
        ],
    )?;

    if affected == 0 {
        return Err(ProductError::NotFound(format!(
            "Product with ID '{}' not found",
            input.id
        )));
    }

    get_product(conn, &input.id)?
        .ok_or_else(|| ProductError::Database("Failed to retrieve updated product".into()))
}

/// Soft-deletes / archives a product by setting `is_active = 0`.
/// Preserves historical foreign key relationships. Never issues a hard DELETE.
pub fn delete_product(conn: &Connection, id: &str) -> Result<(), ProductError> {
    let affected = conn.execute(
        "UPDATE products SET
            is_active = 0,
            updated_at = datetime('now')
        WHERE id = ?1 AND is_active = 1",
        [id],
    )?;

    if affected == 0 {
        // Check if it exists but was already inactive
        let exists: Option<i64> = conn
            .query_row("SELECT is_active FROM products WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        match exists {
            Some(_) => Ok(()), // Already archived / idempotent
            None => Err(ProductError::NotFound(format!(
                "Product with ID '{id}' not found"
            ))),
        }
    } else {
        Ok(())
    }
}

/// Lists products with optional search query, category, and active status filters.
pub fn list_products(
    conn: &Connection,
    filter: &ProductFilter,
) -> Result<Vec<Product>, ProductError> {
    let mut sql = format!("SELECT {PRODUCT_COLUMNS} FROM products WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(active) = filter.is_active {
        sql.push_str(" AND is_active = ?");
        params_vec.push(Box::new(if active { 1 } else { 0 }));
    }

    if let Some(ref cat_id) = filter.category_id {
        let trimmed_cat = cat_id.trim();
        if !trimmed_cat.is_empty() {
            sql.push_str(" AND category_id = ?");
            params_vec.push(Box::new(trimmed_cat.to_string()));
        }
    }

    if let Some(ref q) = filter.query {
        let trimmed_q = q.trim();
        if !trimmed_q.is_empty() {
            sql.push_str(" AND (name LIKE ? ESCAPE '\\' OR barcode = ?)");
            let pattern = format!("%{}%", crate::db::escape_like_pattern(trimmed_q));
            params_vec.push(Box::new(pattern));
            params_vec.push(Box::new(trimmed_q.to_string()));
        }
    }

    sql.push_str(" ORDER BY name COLLATE NOCASE ASC");

    match (filter.limit, filter.offset) {
        (Some(limit), Some(offset)) => {
            sql.push_str(" LIMIT ? OFFSET ?");
            params_vec.push(Box::new(limit));
            params_vec.push(Box::new(offset));
        }
        (Some(limit), None) => {
            sql.push_str(" LIMIT ?");
            params_vec.push(Box::new(limit));
        }
        (None, Some(offset)) => {
            sql.push_str(" LIMIT -1 OFFSET ?");
            params_vec.push(Box::new(offset));
        }
        (None, None) => {}
    }

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();
    let mut stmt = conn.prepare(&sql)?;
    let product_iter = stmt.query_map(params_slice.as_slice(), map_product_row)?;

    let mut products = Vec::new();
    for p in product_iter {
        products.push(p?);
    }
    Ok(products)
}
