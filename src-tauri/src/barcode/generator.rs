// Concurrency-safe atomic SKU generator and internal in-store EAN-13 generator.
// F2.03 — SKU / Barcode

use super::check_digit::calculate_gs1_check_digit;
use super::BarcodeError;
use rusqlite::{params, Connection, OptionalExtension};

/// Validates an SKU string.
/// Format requirements: 3 to 64 ASCII characters, allowed set: A-Z, 0-9, -, _, .
pub fn validate_sku(sku: &str) -> Result<String, BarcodeError> {
    let trimmed = sku.trim();
    if trimmed.is_empty() {
        return Err(BarcodeError::Validation("SKU cannot be empty".into()));
    }
    if trimmed.len() < 3 || trimmed.len() > 64 {
        return Err(BarcodeError::Validation(format!(
            "SKU length must be between 3 and 64 characters, got {}",
            trimmed.len()
        )));
    }

    let is_valid_sku_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');

    if !trimmed.chars().all(is_valid_sku_char) {
        return Err(BarcodeError::Validation(
            "SKU contains invalid characters. Allowed: A-Z, 0-9, hyphens (-), underscores (_), dots (.)".into(),
        ));
    }

    Ok(trimmed.to_uppercase())
}

/// Sanitizes an SKU prefix string.
pub fn sanitize_sku_prefix(prefix: Option<&str>) -> String {
    let cleaned: String = prefix
        .unwrap_or("SKU")
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(*c, '_' | '-'))
        .take(8)
        .collect();

    let upper = cleaned.to_uppercase();
    if upper.is_empty() {
        "SKU".to_string()
    } else {
        upper
    }
}

/// Atomically allocates the next sequential integer for a given prefix in `sku_sequences`.
fn allocate_next_sequence(conn: &Connection, clean_prefix: &str) -> Result<i64, BarcodeError> {
    let seq: i64 = conn.query_row(
        "INSERT INTO sku_sequences (prefix, last_sequence, updated_at)
         VALUES (?1, 1, datetime('now'))
         ON CONFLICT(prefix) DO UPDATE SET
             last_sequence = last_sequence + 1,
             updated_at = datetime('now')
         RETURNING last_sequence",
        params![clean_prefix],
        |row| row.get(0),
    )?;

    Ok(seq)
}

/// Concurrency-safe atomic SKU generator backed by the `sku_sequences` table.
/// Output format: `{PREFIX}-{SEQUENCE:06}` (e.g. `SKU-000001`, `ELEC-000042`).
pub fn generate_next_sku(conn: &Connection, prefix: Option<&str>) -> Result<String, BarcodeError> {
    let clean_prefix = sanitize_sku_prefix(prefix);

    // Loop with collision-skipping retry in case a manual entry already took the sequence
    for _ in 0..10 {
        let seq = allocate_next_sequence(conn, &clean_prefix)?;
        let candidate = format!("{clean_prefix}-{seq:06}");

        // Verify candidate is not occupied by an active product
        let occupied: bool = conn
            .query_row(
                "SELECT 1 FROM products WHERE sku = ?1 COLLATE NOCASE AND is_active = 1",
                params![candidate],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !occupied {
            return Ok(candidate);
        }
    }

    Err(BarcodeError::Validation(
        "Failed to allocate unique SKU after 10 sequence increments".into(),
    ))
}

/// Sanitizes internal EAN-13 prefix (must be a 3-digit number in restricted store range 200..=299).
pub fn sanitize_ean13_prefix(prefix: Option<&str>) -> Result<String, BarcodeError> {
    let p = prefix.unwrap_or("200").trim();
    if p.len() != 3 || !p.chars().all(|c| c.is_ascii_digit()) {
        return Err(BarcodeError::Validation(
            "Internal EAN-13 prefix must be a 3-digit numeric string (200-299)".into(),
        ));
    }
    let num: u32 = p.parse().unwrap_or(0);
    if !(200..=299).contains(&num) {
        return Err(BarcodeError::Validation(format!(
            "Internal EAN-13 prefix '{p}' is outside restricted store range 200-299"
        )));
    }
    Ok(p.to_string())
}

/// Generates an internal in-store EAN-13 barcode with prefix `200..=299` and calculated Modulo-10 check digit.
/// Checks uniqueness against `product_barcodes` and `products` tables.
pub fn generate_internal_ean13(
    conn: &Connection,
    prefix: Option<&str>,
) -> Result<String, BarcodeError> {
    let clean_prefix = sanitize_ean13_prefix(prefix)?;
    let seq_prefix = format!("EAN13_{clean_prefix}");

    for _ in 0..10 {
        let seq = allocate_next_sequence(conn, &seq_prefix)?;
        let body = format!("{clean_prefix}{:09}", seq % 1_000_000_000);
        let check_digit = calculate_gs1_check_digit(&body)?;
        let candidate = format!("{body}{check_digit}");

        // Verify uniqueness across both canonical registry and products table
        let occupied_registry: bool = conn
            .query_row(
                "SELECT 1 FROM product_barcodes WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1",
                params![candidate],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        let occupied_products: bool = conn
            .query_row(
                "SELECT 1 FROM products WHERE barcode = ?1 COLLATE NOCASE AND is_active = 1",
                params![candidate],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !occupied_registry && !occupied_products {
            return Ok(candidate);
        }
    }

    Err(BarcodeError::Validation(
        "Failed to generate unique internal EAN-13 after 10 attempts".into(),
    ))
}
