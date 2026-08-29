// Barcode domain model, symbology validation, lifecycle management, and repository operations.
// F2.03 — SKU / Barcode

pub mod check_digit;
pub mod generator;
pub mod symbology;

pub use check_digit::{calculate_gs1_check_digit, verify_gs1_check_digit};
pub use generator::{generate_internal_ean13, generate_next_sku, validate_sku};
pub use symbology::{detect_symbology, validate_barcode_symbology, BarcodeSymbology};

use crate::product::Product;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Canonical ProductBarcode entity in `product_barcodes`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductBarcode {
    pub id: String,
    pub product_id: String,
    pub barcode: String,
    pub symbology: BarcodeSymbology,
    pub is_primary: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input request for adding a barcode to a product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBarcodeRequest {
    pub product_id: String,
    pub barcode: String,
    pub symbology: Option<BarcodeSymbology>,
    pub is_primary: Option<bool>,
}

/// Detailed discrepancy record for catalog integrity audit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BarcodeIntegrityMismatch {
    pub product_id: String,
    pub product_name: String,
    pub legacy_mirror: Option<String>,
    pub canonical_primary: Option<String>,
    pub description: String,
}

/// Typed domain and repository errors for barcode operations.
#[derive(Debug, PartialEq, Eq)]
pub enum BarcodeError {
    Validation(String),
    NotFound(String),
    DuplicateBarcode(String),
    DuplicateSku(String),
    InvalidCheckDigit { expected: u8, actual: u8 },
    Database(String),
}

impl std::fmt::Display for BarcodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BarcodeError::Validation(msg) => write!(f, "Validation error: {msg}"),
            BarcodeError::NotFound(msg) => write!(f, "Not found: {msg}"),
            BarcodeError::DuplicateBarcode(msg) => write!(f, "Duplicate barcode error: {msg}"),
            BarcodeError::DuplicateSku(msg) => write!(f, "Duplicate SKU error: {msg}"),
            BarcodeError::InvalidCheckDigit { expected, actual } => {
                write!(
                    f,
                    "Invalid GS1 check digit: expected {expected}, got {actual}"
                )
            }
            BarcodeError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for BarcodeError {}

impl From<rusqlite::Error> for BarcodeError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref f, Some(ref msg)) = e {
            if f.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                if msg.contains("product_barcodes.barcode")
                    || msg.contains("idx_product_barcodes_unique_active")
                    || msg.contains("products.barcode")
                {
                    return BarcodeError::DuplicateBarcode(
                        "Barcode is already actively assigned to another product".into(),
                    );
                }
                if msg.contains("products.sku") || msg.contains("idx_products_sku_active") {
                    return BarcodeError::DuplicateSku(
                        "SKU is already actively assigned to another product".into(),
                    );
                }
                if msg.contains("idx_product_barcodes_one_active_primary") {
                    return BarcodeError::Validation(
                        "Product can have at most one active primary barcode".into(),
                    );
                }
            }
        }
        BarcodeError::Database(e.to_string())
    }
}

fn map_barcode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductBarcode> {
    let symbology_str: String = row.get("symbology")?;
    let is_primary_int: i64 = row.get("is_primary")?;
    let is_active_int: i64 = row.get("is_active")?;

    Ok(ProductBarcode {
        id: row.get("id")?,
        product_id: row.get("product_id")?,
        barcode: row.get("barcode")?,
        symbology: BarcodeSymbology::parse(&symbology_str).unwrap_or(BarcodeSymbology::Unknown),
        is_primary: is_primary_int != 0,
        is_active: is_active_int != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Retrieves a single barcode record by its unique ID.
pub fn get_barcode_by_id(
    conn: &Connection,
    id: &str,
) -> Result<Option<ProductBarcode>, BarcodeError> {
    let id_trimmed = id.trim();
    if id_trimmed.is_empty() {
        return Ok(None);
    }

    let barcode = conn
        .query_row(
            "SELECT id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at
             FROM product_barcodes
             WHERE id = ?1",
            params![id_trimmed],
            map_barcode_row,
        )
        .optional()?;

    Ok(barcode)
}

/// Adds a new barcode to a product in the canonical registry and updates the legacy mirror if primary.
pub fn add_product_barcode(
    conn: &Connection,
    request: AddBarcodeRequest,
) -> Result<ProductBarcode, BarcodeError> {
    let product_id = request.product_id.trim();
    if product_id.is_empty() {
        return Err(BarcodeError::Validation(
            "Product ID cannot be empty".into(),
        ));
    }

    // Verify product exists and is active
    let product_exists: Option<i64> = conn
        .query_row(
            "SELECT is_active FROM products WHERE id = ?1",
            params![product_id],
            |row| row.get(0),
        )
        .optional()?;

    let product_is_active = product_exists.ok_or_else(|| {
        BarcodeError::NotFound(format!("Product with ID '{product_id}' not found"))
    })?;

    if product_is_active == 0 {
        return Err(BarcodeError::Validation(
            "Cannot assign barcode to an archived product".into(),
        ));
    }

    let symbology = request
        .symbology
        .unwrap_or_else(|| detect_symbology(&request.barcode));
    let valid_barcode = validate_barcode_symbology(&request.barcode, symbology)?;

    // Check if barcode is already active on another product
    let existing_active_owner: Option<String> = conn
        .query_row(
            "SELECT product_id FROM product_barcodes WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1",
            params![valid_barcode],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(owner) = existing_active_owner {
        return Err(BarcodeError::DuplicateBarcode(format!(
            "Barcode '{valid_barcode}' is already actively assigned to product '{owner}'"
        )));
    }

    // Determine if this should be marked as primary
    let has_active_barcodes: bool = conn
        .query_row(
            "SELECT 1 FROM product_barcodes WHERE product_id = ?1 AND is_active = 1 LIMIT 1",
            params![product_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    let is_primary = request.is_primary.unwrap_or(!has_active_barcodes);
    let barcode_id = uuid::Uuid::new_v4().to_string();

    conn.execute("BEGIN IMMEDIATE;", [])?;

    let res: Result<ProductBarcode, BarcodeError> = (|| {
        if is_primary {
            // Demote existing primary barcodes for this product
            conn.execute(
                "UPDATE product_barcodes SET is_primary = 0, updated_at = datetime('now') WHERE product_id = ?1",
                params![product_id],
            )?;
        }

        conn.execute(
            "INSERT INTO product_barcodes (id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, datetime('now'), datetime('now'))",
            params![
                barcode_id,
                product_id,
                valid_barcode,
                symbology.as_str(),
                if is_primary { 1 } else { 0 }
            ],
        )?;

        if is_primary {
            conn.execute(
                "UPDATE products SET barcode = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![valid_barcode, product_id],
            )?;
        }

        let created = get_barcode_by_id(conn, &barcode_id)?
            .ok_or_else(|| BarcodeError::Database("Failed to load newly created barcode".into()))?;

        Ok(created)
    })();

    match res {
        Ok(created) => {
            conn.execute("COMMIT;", [])?;
            Ok(created)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            Err(e)
        }
    }
}

/// Deactivates / soft-deletes a barcode record.
/// Preserves historical rows and updates the legacy mirror if it was primary.
pub fn remove_product_barcode(conn: &Connection, barcode_id: &str) -> Result<(), BarcodeError> {
    let existing = get_barcode_by_id(conn, barcode_id)?.ok_or_else(|| {
        BarcodeError::NotFound(format!("Barcode with ID '{barcode_id}' not found"))
    })?;

    if !existing.is_active {
        return Ok(());
    }

    conn.execute("BEGIN IMMEDIATE;", [])?;

    let res: Result<(), BarcodeError> = (|| {
        conn.execute(
            "UPDATE product_barcodes SET is_active = 0, is_primary = 0, updated_at = datetime('now') WHERE id = ?1",
            params![barcode_id],
        )?;

        if existing.is_primary {
            // Find another active barcode to promote, or set legacy mirror to NULL
            let next_primary_id: Option<String> = conn
                .query_row(
                    "SELECT id FROM product_barcodes WHERE product_id = ?1 AND is_active = 1 ORDER BY created_at ASC LIMIT 1",
                    params![existing.product_id],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(next_id) = next_primary_id {
                let next_barcode: String = conn.query_row(
                    "SELECT barcode FROM product_barcodes WHERE id = ?1",
                    params![next_id],
                    |row| row.get(0),
                )?;
                conn.execute(
                    "UPDATE product_barcodes SET is_primary = 1, updated_at = datetime('now') WHERE id = ?1",
                    params![next_id],
                )?;
                conn.execute(
                    "UPDATE products SET barcode = ?1, updated_at = datetime('now') WHERE id = ?2",
                    params![next_barcode, existing.product_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE products SET barcode = NULL, updated_at = datetime('now') WHERE id = ?1",
                    params![existing.product_id],
                )?;
            }
        }

        Ok(())
    })();

    match res {
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

/// Atomically sets a barcode as the primary barcode for a product.
pub fn set_primary_barcode(
    conn: &Connection,
    product_id: &str,
    barcode_id: &str,
) -> Result<ProductBarcode, BarcodeError> {
    let p_id = product_id.trim();
    let b_id = barcode_id.trim();

    let target = get_barcode_by_id(conn, b_id)?
        .ok_or_else(|| BarcodeError::NotFound(format!("Barcode with ID '{b_id}' not found")))?;

    if target.product_id != p_id {
        return Err(BarcodeError::Validation(format!(
            "Barcode '{b_id}' does not belong to product '{p_id}'"
        )));
    }

    if !target.is_active {
        return Err(BarcodeError::Validation(
            "Cannot set an inactive barcode as primary".into(),
        ));
    }

    conn.execute("BEGIN IMMEDIATE;", [])?;

    let res: Result<ProductBarcode, BarcodeError> = (|| {
        // Demote all existing primary barcodes for this product
        conn.execute(
            "UPDATE product_barcodes SET is_primary = 0, updated_at = datetime('now') WHERE product_id = ?1",
            params![p_id],
        )?;

        // Promote target barcode
        conn.execute(
            "UPDATE product_barcodes SET is_primary = 1, updated_at = datetime('now') WHERE id = ?1",
            params![b_id],
        )?;

        // Update legacy mirror
        conn.execute(
            "UPDATE products SET barcode = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![target.barcode, p_id],
        )?;

        let updated = get_barcode_by_id(conn, b_id)?
            .ok_or_else(|| BarcodeError::Database("Failed to load updated barcode".into()))?;

        Ok(updated)
    })();

    match res {
        Ok(updated) => {
            conn.execute("COMMIT;", [])?;
            Ok(updated)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            Err(e)
        }
    }
}

/// Reassigns a barcode from one product to another atomically.
pub fn reassign_product_barcode(
    conn: &Connection,
    barcode_id: &str,
    target_product_id: &str,
    as_primary: bool,
) -> Result<ProductBarcode, BarcodeError> {
    let b_id = barcode_id.trim();
    let t_pid = target_product_id.trim();

    let barcode = get_barcode_by_id(conn, b_id)?
        .ok_or_else(|| BarcodeError::NotFound(format!("Barcode with ID '{b_id}' not found")))?;

    // Verify target product exists and is active
    let target_active: Option<i64> = conn
        .query_row(
            "SELECT is_active FROM products WHERE id = ?1",
            params![t_pid],
            |row| row.get(0),
        )
        .optional()?;

    let target_is_active = target_active.ok_or_else(|| {
        BarcodeError::NotFound(format!("Target product with ID '{t_pid}' not found"))
    })?;

    if target_is_active == 0 {
        return Err(BarcodeError::Validation(
            "Cannot reassign barcode to an archived product".into(),
        ));
    }

    conn.execute("BEGIN IMMEDIATE;", [])?;

    let res: Result<ProductBarcode, BarcodeError> = (|| {
        // If it was primary on old product, remove primary mirror on old product
        if barcode.is_primary && barcode.is_active {
            conn.execute(
                "UPDATE products SET barcode = NULL, updated_at = datetime('now') WHERE id = ?1 AND barcode = ?2",
                params![barcode.product_id, barcode.barcode],
            )?;
        }

        if as_primary {
            conn.execute(
                "UPDATE product_barcodes SET is_primary = 0, updated_at = datetime('now') WHERE product_id = ?1",
                params![t_pid],
            )?;
            conn.execute(
                "UPDATE products SET barcode = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![barcode.barcode, t_pid],
            )?;
        }

        conn.execute(
            "UPDATE product_barcodes SET product_id = ?1, is_primary = ?2, is_active = 1, updated_at = datetime('now') WHERE id = ?3",
            params![t_pid, if as_primary { 1 } else { 0 }, b_id],
        )?;

        let updated = get_barcode_by_id(conn, b_id)?
            .ok_or_else(|| BarcodeError::Database("Failed to load reassigned barcode".into()))?;

        Ok(updated)
    })();

    match res {
        Ok(updated) => {
            conn.execute("COMMIT;", [])?;
            Ok(updated)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            Err(e)
        }
    }
}

/// Strictly read-only barcode lookup.
/// Resolves a product and matched barcode metadata from the canonical registry with zero database side effects.
pub fn get_product_by_barcode(
    conn: &Connection,
    barcode: &str,
) -> Result<Option<(Product, Option<ProductBarcode>)>, BarcodeError> {
    let trimmed = barcode.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    // 1. Check canonical registry for active barcode
    let barcode_row = conn
        .query_row(
            "SELECT id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at
             FROM product_barcodes
             WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1
             LIMIT 1",
            params![trimmed],
            map_barcode_row,
        )
        .optional()?;

    if let Some(bc) = barcode_row {
        if let Some(p) = crate::product::get_product(conn, &bc.product_id)
            .map_err(|e| BarcodeError::Database(e.to_string()))?
        {
            return Ok(Some((p, Some(bc))));
        }
    }

    // 2. Backward compatibility fallback: check legacy products.barcode
    if let Some(p) = crate::product::get_product_by_barcode(conn, trimmed)
        .map_err(|e| BarcodeError::Database(e.to_string()))?
    {
        return Ok(Some((p, None)));
    }

    Ok(None)
}

/// Lists all barcodes registered for a given product.
pub fn list_product_barcodes(
    conn: &Connection,
    product_id: &str,
    include_inactive: bool,
) -> Result<Vec<ProductBarcode>, BarcodeError> {
    let p_id = product_id.trim();
    if p_id.is_empty() {
        return Err(BarcodeError::Validation(
            "Product ID cannot be empty".into(),
        ));
    }

    let sql = if include_inactive {
        "SELECT id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at
         FROM product_barcodes
         WHERE product_id = ?1
         ORDER BY is_primary DESC, is_active DESC, created_at ASC"
    } else {
        "SELECT id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at
         FROM product_barcodes
         WHERE product_id = ?1 AND is_active = 1
         ORDER BY is_primary DESC, created_at ASC"
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![p_id], map_barcode_row)?;

    let mut list = Vec::new();
    for row in rows {
        list.push(row?);
    }

    Ok(list)
}

/// Read-only diagnostic verification of catalog barcode integrity.
/// Scans for discrepancies between canonical `product_barcodes` and legacy `products.barcode`.
pub fn verify_catalog_barcode_integrity(
    conn: &Connection,
) -> Result<Vec<BarcodeIntegrityMismatch>, BarcodeError> {
    let mut mismatches = Vec::new();

    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.barcode as legacy_barcode, pb.barcode as canonical_primary
         FROM products p
         LEFT JOIN product_barcodes pb ON p.id = pb.product_id AND pb.is_primary = 1 AND pb.is_active = 1
         WHERE p.is_active = 1",
    )?;

    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let legacy: Option<String> = row.get(2)?;
        let canonical: Option<String> = row.get(3)?;
        Ok((id, name, legacy, canonical))
    })?;

    for row in rows {
        let (id, name, legacy, canonical) = row?;
        if legacy != canonical {
            mismatches.push(BarcodeIntegrityMismatch {
                product_id: id,
                product_name: name,
                legacy_mirror: legacy.clone(),
                canonical_primary: canonical.clone(),
                description: format!(
                    "Legacy products.barcode ({legacy:?}) disagrees with canonical primary in product_barcodes ({canonical:?})"
                ),
            });
        }
    }

    Ok(mismatches)
}

/// Explicit administrative repair command reconciling `products.barcode` to match canonical primary records.
pub fn reconcile_catalog_barcode_mirrors(conn: &Connection) -> Result<usize, BarcodeError> {
    let updated = conn.execute(
        "UPDATE products
         SET barcode = (
             SELECT pb.barcode
             FROM product_barcodes pb
             WHERE pb.product_id = products.id
               AND pb.is_primary = 1
               AND pb.is_active = 1
             LIMIT 1
         ),
         updated_at = datetime('now')
         WHERE is_active = 1
           AND (
               barcode IS NOT (
                   SELECT pb.barcode
                   FROM product_barcodes pb
                   WHERE pb.product_id = products.id
                     AND pb.is_primary = 1
                     AND pb.is_active = 1
                   LIMIT 1
               )
           )",
        [],
    )?;

    Ok(updated)
}
