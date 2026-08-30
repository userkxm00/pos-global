// Product domain model, validation rules, and SQLite database operations.
// F2.01 / F2.03 — Product CRUD & SKU / Barcode Management

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Maximum safe integer minor units that can be converted to and from IEEE 754 f64 without precision loss.
pub const MAX_SAFE_MINOR_UNITS: i64 = 90_071_992_547_409;

/// Canonical Product entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Product {
    pub id: String,
    pub category_id: Option<String>,
    pub sku: Option<String>,
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
    pub sku: Option<String>,
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
    pub sku: Option<String>,
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
    DuplicateSku(String),
    Database(String),
}

impl std::fmt::Display for ProductError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductError::Validation(msg) => write!(f, "Validation error: {msg}"),
            ProductError::NotFound(msg) => write!(f, "Product not found: {msg}"),
            ProductError::DuplicateBarcode(msg) => write!(f, "Duplicate barcode error: {msg}"),
            ProductError::DuplicateSku(msg) => write!(f, "Duplicate SKU error: {msg}"),
            ProductError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for ProductError {}

impl From<rusqlite::Error> for ProductError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref f, Some(ref msg)) = e {
            if f.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                if msg.contains("products.barcode")
                    || msg.contains("UNIQUE constraint failed: products.barcode")
                    || msg.contains("product_barcodes.barcode")
                    || msg.contains("idx_product_barcodes_unique_active")
                {
                    return ProductError::DuplicateBarcode(
                        "Barcode already assigned to another product".into(),
                    );
                }
                if msg.contains("products.sku")
                    || msg.contains("idx_products_sku_active")
                    || msg.contains("UNIQUE constraint failed: products.sku")
                {
                    return ProductError::DuplicateSku(
                        "SKU already assigned to another product".into(),
                    );
                }
            }
        }
        ProductError::Database(e.to_string())
    }
}

/// Resolves the organization ownership of the local product catalog from business_settings or branches.
pub fn get_catalog_organization_id(conn: &Connection) -> Result<Option<String>, rusqlite::Error> {
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
pub fn real_to_minor(real: f64) -> i64 {
    (real * 100.0).round() as i64
}

/// Converts integer minor units into floating-point database representation.
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
        sku: row.get("sku")?,
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

const PRODUCT_COLUMNS: &str = "id, category_id, sku, name, description, barcode, product_type, base_price, cost_price, unit_type, requires_expiry, requires_serial, warranty_months, custom_attributes, is_active, created_at, updated_at";

/// Escapes SQL LIKE wildcards ('%', '_', and '\') using '\' as the escape character.
fn escape_like_pattern(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        if c == '\\' || c == '%' || c == '_' {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

fn sanitize_optional_string(s: Option<&str>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn validate_and_check_sku(
    conn: &Connection,
    raw_sku: Option<&str>,
    exclude_id: Option<&str>,
) -> Result<Option<String>, ProductError> {
    let s = match raw_sku {
        Some(val) if !val.trim().is_empty() => val.trim(),
        _ => return Ok(None),
    };
    let sanitized =
        crate::barcode::validate_sku(s).map_err(|e| ProductError::Validation(e.to_string()))?;

    let conflict: Option<String> = if let Some(ex_id) = exclude_id {
        conn.query_row(
            "SELECT id FROM products WHERE sku = ?1 COLLATE NOCASE AND id != ?2 AND is_active = 1",
            params![sanitized, ex_id],
            |row| row.get(0),
        )
        .optional()?
    } else {
        conn.query_row(
            "SELECT id FROM products WHERE sku = ?1 COLLATE NOCASE AND is_active = 1",
            params![sanitized],
            |row| row.get(0),
        )
        .optional()?
    };

    if let Some(conflict_id) = conflict {
        return Err(ProductError::DuplicateSku(format!(
            "SKU '{sanitized}' is already assigned to product '{conflict_id}'"
        )));
    }
    Ok(Some(sanitized))
}

fn check_barcode_conflict(
    conn: &Connection,
    barcode: Option<&str>,
    exclude_id: Option<&str>,
) -> Result<(), ProductError> {
    let bc = match barcode {
        Some(val) if !val.trim().is_empty() => val.trim(),
        _ => return Ok(()),
    };

    let (prod_conflict, reg_conflict): (Option<String>, Option<String>) = if let Some(ex_id) =
        exclude_id
    {
        let p_c = conn
            .query_row(
                "SELECT id FROM products WHERE barcode = ?1 COLLATE NOCASE AND id != ?2 AND is_active = 1",
                params![bc, ex_id],
                |row| row.get(0),
            )
            .optional()?;
        let r_c = conn
            .query_row(
                "SELECT product_id FROM product_barcodes WHERE barcode = ?1 COLLATE NOCASE AND product_id != ?2 AND is_active = 1",
                params![bc, ex_id],
                |row| row.get(0),
            )
            .optional()?;
        (p_c, r_c)
    } else {
        let p_c = conn
            .query_row(
                "SELECT id FROM products WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1",
                params![bc],
                |row| row.get(0),
            )
            .optional()?;
        let r_c = conn
            .query_row(
                "SELECT product_id FROM product_barcodes WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1",
                params![bc],
                |row| row.get(0),
            )
            .optional()?;
        (p_c, r_c)
    };

    if let Some(id) = prod_conflict.or(reg_conflict) {
        return Err(ProductError::DuplicateBarcode(format!(
            "Barcode '{bc}' is already assigned to product '{id}'"
        )));
    }
    Ok(())
}

fn sync_product_primary_barcode(
    conn: &Connection,
    product_id: &str,
    barcode: Option<&str>,
    is_active: bool,
) -> Result<(), ProductError> {
    if !is_active {
        conn.execute(
            "UPDATE product_barcodes SET is_active = 0, is_primary = 0, updated_at = datetime('now') WHERE product_id = ?1",
            params![product_id],
        )?;
        return Ok(());
    }

    conn.execute(
        "UPDATE product_barcodes SET is_primary = 0, updated_at = datetime('now') WHERE product_id = ?1",
        params![product_id],
    )?;

    if let Some(bc) = barcode {
        let existing_barcode_id: Option<String> = conn
            .query_row(
                "SELECT id FROM product_barcodes WHERE product_id = ?1 AND barcode = ?2 COLLATE NOCASE",
                params![product_id, bc],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(b_id) = existing_barcode_id {
            conn.execute(
                "UPDATE product_barcodes SET is_primary = 1, is_active = 1, updated_at = datetime('now') WHERE id = ?1",
                params![b_id],
            )?;
        } else {
            let symbology = crate::barcode::detect_symbology(bc);
            let b_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO product_barcodes (id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, 1, datetime('now'), datetime('now'))",
                params![b_id, product_id, bc, symbology.as_str()],
            )?;
        }
    }

    Ok(())
}

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
    let sku = validate_and_check_sku(conn, input.sku.as_deref(), None)?;

    check_barcode_conflict(conn, barcode.as_deref(), None)?;

    let id = uuid::Uuid::new_v4().to_string();
    let base_price_real = minor_to_real(base_price_minor);
    let cost_price_real = cost_price_minor.map(minor_to_real);
    let description = sanitize_optional_string(input.description.as_deref());
    let category_id = sanitize_optional_string(input.category_id.as_deref());
    let unit_type = sanitize_optional_string(input.unit_type.as_deref());
    let requires_expiry = i64::from(input.requires_expiry.unwrap_or(false));
    let requires_serial = i64::from(input.requires_serial.unwrap_or(false));

    conn.execute("BEGIN IMMEDIATE;", [])?;
    let tx_res: Result<(), ProductError> = (|| {
        conn.execute(
            "INSERT INTO products (
                id, category_id, sku, name, description, barcode, product_type,
                base_price, cost_price, unit_type, requires_expiry, requires_serial,
                warranty_months, custom_attributes, is_active, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, 1, datetime('now'), datetime('now')
            )",
            params![
                id,
                category_id,
                sku,
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

        sync_product_primary_barcode(conn, &id, barcode.as_deref(), true)?;
        Ok(())
    })();

    match tx_res {
        Ok(()) => {
            conn.execute("COMMIT;", [])?;
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            return Err(e);
        }
    }

    get_product(conn, &id)?
        .ok_or_else(|| ProductError::Database("Failed to retrieve created product".into()))
}

/// Retrieves a product by its unique ID.
pub fn get_product(conn: &Connection, id: &str) -> Result<Option<Product>, ProductError> {
    let sql = format!("SELECT {PRODUCT_COLUMNS} FROM products WHERE id = ?1");
    let result = conn.query_row(&sql, [id], map_product_row).optional()?;
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
    let product_type = validate_product_type(Some(&input.product_type))?;

    if get_product(conn, &input.id)?.is_none() {
        return Err(ProductError::NotFound(format!(
            "Product with ID '{}' not found",
            input.id
        )));
    }

    let sku = if input.is_active {
        validate_and_check_sku(conn, input.sku.as_deref(), Some(&input.id))?
    } else {
        sanitize_optional_string(input.sku.as_deref())
    };

    let effective_barcode = if input.is_active {
        let bc = validate_barcode(input.barcode.as_deref());
        check_barcode_conflict(conn, bc.as_deref(), Some(&input.id))?;
        bc
    } else {
        None
    };

    let base_price_real = minor_to_real(base_price_minor);
    let cost_price_real = cost_price_minor.map(minor_to_real);
    let description = sanitize_optional_string(input.description.as_deref());
    let category_id = sanitize_optional_string(input.category_id.as_deref());
    let unit_type = sanitize_optional_string(input.unit_type.as_deref());
    let requires_expiry = i64::from(input.requires_expiry);
    let requires_serial = i64::from(input.requires_serial);
    let is_active = i64::from(input.is_active);

    conn.execute("BEGIN IMMEDIATE;", [])?;
    let tx_res: Result<(), ProductError> = (|| {
        let affected = conn.execute(
            "UPDATE products SET
                category_id = ?1,
                sku = ?2,
                name = ?3,
                description = ?4,
                barcode = ?5,
                product_type = ?6,
                base_price = ?7,
                cost_price = ?8,
                unit_type = ?9,
                requires_expiry = ?10,
                requires_serial = ?11,
                warranty_months = ?12,
                custom_attributes = ?13,
                is_active = ?14,
                updated_at = datetime('now')
            WHERE id = ?15",
            params![
                category_id,
                sku,
                name,
                description,
                effective_barcode,
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

        sync_product_primary_barcode(
            conn,
            &input.id,
            effective_barcode.as_deref(),
            input.is_active,
        )?;
        Ok(())
    })();

    match tx_res {
        Ok(()) => {
            conn.execute("COMMIT;", [])?;
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            return Err(e);
        }
    }

    get_product(conn, &input.id)?
        .ok_or_else(|| ProductError::Database("Failed to retrieve updated product".into()))
}

/// Soft-deletes / archives a product by setting `is_active = 0`.
/// Preserves historical foreign key relationships. Never issues a hard DELETE.
/// Clears `products.barcode = NULL` and deactivates all associated rows in `product_barcodes`.
pub fn delete_product(conn: &Connection, id: &str) -> Result<(), ProductError> {
    conn.execute("BEGIN IMMEDIATE;", [])?;
    let tx_res: Result<(), ProductError> = (|| {
        let affected = conn.execute(
            "UPDATE products SET
                barcode = NULL,
                is_active = 0,
                updated_at = datetime('now')
            WHERE id = ?1 AND is_active = 1",
            [id],
        )?;

        if affected == 0 {
            let exists: Option<i64> = conn
                .query_row("SELECT is_active FROM products WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            match exists {
                Some(_) => {
                    conn.execute(
                        "UPDATE product_barcodes SET is_active = 0, is_primary = 0, updated_at = datetime('now') WHERE product_id = ?1",
                        [id],
                    )?;
                }
                None => {
                    return Err(ProductError::NotFound(format!(
                        "Product with ID '{id}' not found"
                    )));
                }
            }
        } else {
            conn.execute(
                "UPDATE product_barcodes SET is_active = 0, is_primary = 0, updated_at = datetime('now') WHERE product_id = ?1",
                [id],
            )?;
        }
        Ok(())
    })();

    match tx_res {
        Ok(()) => {
            conn.execute("COMMIT;", [])?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            Err(e)
        }
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
            sql.push_str(" AND (name LIKE ? ESCAPE '\\' OR barcode = ? OR sku = ?)");
            let pattern = format!("%{}%", escape_like_pattern(trimmed_q));
            params_vec.push(Box::new(pattern));
            params_vec.push(Box::new(trimmed_q.to_string()));
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
