// F2.08 — Serial, IMEI & Tracked Assets Domain Engine
// ADR-0010: Flexible triple-identifier model, single IMEI, global NOCASE serial uniqueness, and branch tenancy.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Persisted operational lifecycle status for serialized inventory instances.
///
/// Allowed statuses: `in_stock`, `reserved`, `sold`, `transferred`, `defective`, `recalled`, `disposed`.
/// `recalled` and `disposed` are permanent terminal states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerialStatus {
    /// Instance is physically available at the branch for sale, reservation, or transfer.
    InStock,
    /// Instance is held for a pending customer order, quotation, or reservation.
    Reserved,
    /// Instance has been sold and dispatched to a customer.
    Sold,
    /// Instance is in-transit or transferred to another branch.
    Transferred,
    /// Instance is defective, damaged, or pending repair.
    Defective,
    /// Instance is permanently recalled by manufacturer or health/safety authority (terminal state).
    Recalled,
    /// Instance has been scrapped, written off, or decommissioned (terminal state).
    Disposed,
}

impl SerialStatus {
    /// Returns the static lowercase string representation of the status.
    pub fn as_str(&self) -> &'static str {
        match self {
            SerialStatus::InStock => "in_stock",
            SerialStatus::Reserved => "reserved",
            SerialStatus::Sold => "sold",
            SerialStatus::Transferred => "transferred",
            SerialStatus::Defective => "defective",
            SerialStatus::Recalled => "recalled",
            SerialStatus::Disposed => "disposed",
        }
    }

    /// Returns whether this status represents a permanent terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, SerialStatus::Recalled | SerialStatus::Disposed)
    }
}

impl FromStr for SerialStatus {
    type Err = SerialError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "in_stock" => Ok(SerialStatus::InStock),
            "reserved" => Ok(SerialStatus::Reserved),
            "sold" => Ok(SerialStatus::Sold),
            "transferred" => Ok(SerialStatus::Transferred),
            "defective" => Ok(SerialStatus::Defective),
            "recalled" => Ok(SerialStatus::Recalled),
            "disposed" => Ok(SerialStatus::Disposed),
            other => Err(SerialError::Validation(format!(
                "Invalid serial status '{other}'. Allowed: in_stock, reserved, sold, transferred, defective, recalled, disposed"
            ))),
        }
    }
}

/// Domain representation of a tracked physical unit instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializedInstance {
    /// Unique internal record identifier.
    pub id: String,
    /// Product ID this unit belongs to.
    pub product_id: String,
    /// Branch ID where this unit is physically located.
    pub branch_id: String,
    /// Optional variant ID if this unit belongs to a specific product variant SKU.
    pub variant_id: Option<String>,
    /// Manufacturer or merchant assigned serial number. Globally unique when present.
    pub serial_number: Option<String>,
    /// International Mobile Equipment Identity. 15-digit Luhn-validated string. Globally unique when present.
    pub imei: Option<String>,
    /// Internal organizational asset tag. Branch-scoped unique when present.
    pub asset_tag: Option<String>,
    /// Acquisition unit cost in integer minor currency units (cents).
    pub cost_price_minor: Option<i64>,
    /// Operational lifecycle status.
    pub status: SerialStatus,
    /// Historical sale ID if this unit was sold.
    pub sold_in_sale_id: Option<String>,
    /// Historical warranty expiration date if registered.
    pub warranty_expires_at: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last update timestamp.
    pub updated_at: String,
}

/// Input payload for registering a new serialized instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSerialInput {
    pub product_id: String,
    pub branch_id: String,
    pub variant_id: Option<String>,
    pub serial_number: Option<String>,
    pub imei: Option<String>,
    pub asset_tag: Option<String>,
    pub cost_price_minor: Option<i64>,
}

/// Input payload for updating the status of an existing serialized instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSerialStatusInput {
    pub id: String,
    pub branch_id: String,
    pub status: SerialStatus,
}

/// Filter criteria for listing serialized instances within a branch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialFilter {
    pub branch_id: String,
    pub product_id: Option<String>,
    pub variant_id: Option<String>,
    pub status: Option<SerialStatus>,
}

/// Strongly-typed domain errors for serialized inventory operations.
#[derive(Debug, PartialEq, Eq)]
pub enum SerialError {
    Validation(String),
    DuplicateSerial(String),
    DuplicateImei(String),
    DuplicateAssetTag(String),
    NotFound(String),
    ProductNotSerialized(String),
    InvalidVariant(String),
    InvalidStatusTransition { from: String, to: String },
    TerminalStatus(String),
    Database(String),
}

impl std::fmt::Display for SerialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerialError::Validation(msg) => write!(f, "Validation error: {msg}"),
            SerialError::DuplicateSerial(msg) => write!(f, "Duplicate serial number: {msg}"),
            SerialError::DuplicateImei(msg) => write!(f, "Duplicate IMEI: {msg}"),
            SerialError::DuplicateAssetTag(msg) => write!(f, "Duplicate asset tag: {msg}"),
            SerialError::NotFound(msg) => write!(f, "Serial instance not found: {msg}"),
            SerialError::ProductNotSerialized(msg) => {
                write!(f, "Product '{msg}' is not tracked by serial or IMEI")
            }
            SerialError::InvalidVariant(msg) => write!(f, "Invalid variant: {msg}"),
            SerialError::InvalidStatusTransition { from, to } => {
                write!(f, "Invalid status transition from '{from}' to '{to}'")
            }
            SerialError::TerminalStatus(msg) => {
                write!(f, "Cannot update status of {msg}: terminal state")
            }
            SerialError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for SerialError {}

impl From<rusqlite::Error> for SerialError {
    fn from(err: rusqlite::Error) -> Self {
        SerialError::Database(err.to_string())
    }
}

// =========================================================================
// VALIDATION HELPERS
// =========================================================================

/// Validates that a string satisfies the standard Luhn Mod-10 checksum formula.
pub fn validate_luhn_checksum(digits: &str) -> bool {
    let mut sum = 0;
    for (i, c) in digits.chars().rev().enumerate() {
        let Some(d_val) = c.to_digit(10) else {
            return false;
        };
        let mut d = d_val;
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    sum % 10 == 0
}

/// Normalizes and validates a serial number string.
/// Enforces Unicode character-count limits (`chars().count() <= 100`).
pub fn validate_serial_number(s: &str) -> Result<String, SerialError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(SerialError::Validation(
            "Serial number cannot be empty or whitespace-only".to_string(),
        ));
    }
    if trimmed.chars().count() > 100 {
        return Err(SerialError::Validation(
            "Serial number exceeds maximum length of 100 characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Normalizes and validates a 15-digit decimal IMEI string using the Luhn checksum.
pub fn validate_imei(imei: &str) -> Result<String, SerialError> {
    let trimmed = imei.trim();
    if trimmed.len() != 15 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err(SerialError::Validation(
            "IMEI must be exactly 15 decimal digits".to_string(),
        ));
    }
    if !validate_luhn_checksum(trimmed) {
        return Err(SerialError::Validation(
            "IMEI failed Luhn check digit validation".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Normalizes and validates an internal asset tag string.
/// Enforces Unicode character-count limits (`chars().count() <= 100`).
pub fn validate_asset_tag(tag: &str) -> Result<String, SerialError> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        return Err(SerialError::Validation(
            "Asset tag cannot be empty or whitespace-only".to_string(),
        ));
    }
    if trimmed.chars().count() > 100 {
        return Err(SerialError::Validation(
            "Asset tag exceeds maximum length of 100 characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates the triple-identifier invariant: at least one identifier must be provided,
/// and every provided identifier must be non-empty and well-formed.
pub fn validate_identifiers(
    serial: Option<&str>,
    imei: Option<&str>,
    asset_tag: Option<&str>,
) -> Result<(Option<String>, Option<String>, Option<String>), SerialError> {
    let norm_serial = match serial {
        Some(s) if !s.trim().is_empty() => Some(validate_serial_number(s)?),
        _ => None,
    };
    let norm_imei = match imei {
        Some(i) if !i.trim().is_empty() => Some(validate_imei(i)?),
        _ => None,
    };
    let norm_asset_tag = match asset_tag {
        Some(a) if !a.trim().is_empty() => Some(validate_asset_tag(a)?),
        _ => None,
    };

    if norm_serial.is_none() && norm_imei.is_none() && norm_asset_tag.is_none() {
        return Err(SerialError::Validation(
            "At least one valid identifier (serial_number, imei, or asset_tag) must be provided"
                .to_string(),
        ));
    }

    Ok((norm_serial, norm_imei, norm_asset_tag))
}

/// Validates that an optional unit cost price is non-negative.
fn validate_cost_price(cost: Option<i64>) -> Result<(), SerialError> {
    if let Some(c) = cost {
        if c < 0 {
            return Err(SerialError::Validation(
                "Cost price cannot be negative".to_string(),
            ));
        }
    }
    Ok(())
}

/// Validates operational lifecycle status transitions.
pub fn validate_status_transition(
    current: SerialStatus,
    target: SerialStatus,
) -> Result<(), SerialError> {
    if current.is_terminal() {
        return Err(SerialError::TerminalStatus(current.as_str().to_string()));
    }
    if current == target {
        return Ok(());
    }

    let is_valid = match current {
        SerialStatus::InStock => matches!(
            target,
            SerialStatus::Reserved
                | SerialStatus::Sold
                | SerialStatus::Transferred
                | SerialStatus::Defective
                | SerialStatus::Recalled
                | SerialStatus::Disposed
        ),
        SerialStatus::Reserved => matches!(
            target,
            SerialStatus::InStock
                | SerialStatus::Sold
                | SerialStatus::Defective
                | SerialStatus::Recalled
                | SerialStatus::Disposed
        ),
        SerialStatus::Sold => matches!(
            target,
            SerialStatus::InStock
                | SerialStatus::Defective
                | SerialStatus::Recalled
                | SerialStatus::Disposed
        ),
        SerialStatus::Transferred => matches!(
            target,
            SerialStatus::InStock
                | SerialStatus::Defective
                | SerialStatus::Recalled
                | SerialStatus::Disposed
        ),
        SerialStatus::Defective => matches!(
            target,
            SerialStatus::InStock | SerialStatus::Recalled | SerialStatus::Disposed
        ),
        SerialStatus::Recalled | SerialStatus::Disposed => false,
    };

    if !is_valid {
        return Err(SerialError::InvalidStatusTransition {
            from: current.as_str().to_string(),
            to: target.as_str().to_string(),
        });
    }

    Ok(())
}

// =========================================================================
// CAPABILITY & RELATIONAL CHECKS
// =========================================================================

/// Evaluates whether a product is eligible for serial, IMEI, or asset tracking.
///
/// Authoritative rule:
/// `is_serial_tracked(P) <=> products.requires_serial = 1 OR has_capability(P, 'SERIAL') OR has_capability(P, 'IMEI')`
pub fn is_serial_tracked(conn: &Connection, product_id: &str) -> Result<bool, SerialError> {
    let mut stmt = conn.prepare_cached(
        "SELECT p.requires_serial,
                EXISTS(
                    SELECT 1 FROM product_capabilities pc
                    JOIN capabilities c ON pc.capability_id = c.id
                    WHERE pc.product_id = p.id
                      AND c.code IN ('SERIAL', 'IMEI')
                      AND pc.enabled = 1
                ) as has_cap
         FROM products p
         WHERE p.id = ?1",
    )?;

    let row = stmt.query_row(params![product_id], |row| {
        let req_ser: i64 = row.get(0)?;
        let has_cap: i64 = row.get(1)?;
        Ok((req_ser != 0) || (has_cap != 0))
    });

    match row {
        Ok(tracked) => Ok(tracked),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(SerialError::Validation(format!(
            "Product '{product_id}' not found"
        ))),
        Err(e) => Err(SerialError::Database(e.to_string())),
    }
}

/// Verifies that a variant exists, is active and non-deleted, and belongs to the parent product.
pub fn verify_variant_association(
    conn: &Connection,
    product_id: &str,
    variant_id: &str,
) -> Result<(), SerialError> {
    let mut stmt = conn.prepare_cached(
        "SELECT product_id, is_active, deleted_at FROM product_variants WHERE id = ?1",
    )?;

    let result = stmt.query_row(params![variant_id], |row| {
        let parent_id: String = row.get(0)?;
        let is_active: i64 = row.get(1)?;
        let deleted_at: Option<String> = row.get(2)?;
        Ok((parent_id, is_active != 0, deleted_at))
    });

    match result {
        Ok((parent_id, is_active, deleted_at)) => {
            if parent_id != product_id {
                return Err(SerialError::InvalidVariant(format!(
                    "Variant '{variant_id}' belongs to product '{parent_id}', not '{product_id}'"
                )));
            }
            if !is_active || deleted_at.is_some() {
                return Err(SerialError::InvalidVariant(format!(
                    "Variant '{variant_id}' is inactive or soft-deleted"
                )));
            }
            Ok(())
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(SerialError::InvalidVariant(format!(
            "Variant '{variant_id}' not found"
        ))),
        Err(e) => Err(SerialError::Database(e.to_string())),
    }
}

// =========================================================================
// COLLISION CHECK HELPERS
// =========================================================================

fn check_identifier_collisions(
    conn: &Connection,
    branch_id: &str,
    serial: Option<&str>,
    imei: Option<&str>,
    asset_tag: Option<&str>,
) -> Result<(), SerialError> {
    if let Some(s) = serial {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM serial_numbers WHERE serial_number = ?1 COLLATE NOCASE)",
            params![s],
            |row| row.get(0),
        )?;
        if exists {
            return Err(SerialError::DuplicateSerial(s.to_string()));
        }
    }

    if let Some(i) = imei {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM serial_numbers WHERE imei = ?1)",
            params![i],
            |row| row.get(0),
        )?;
        if exists {
            return Err(SerialError::DuplicateImei(i.to_string()));
        }
    }

    if let Some(a) = asset_tag {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM serial_numbers WHERE branch_id = ?1 AND asset_tag = ?2 COLLATE NOCASE)",
            params![branch_id, a],
            |row| row.get(0),
        )?;
        if exists {
            return Err(SerialError::DuplicateAssetTag(a.to_string()));
        }
    }

    Ok(())
}

fn map_sqlite_collision_error(e: rusqlite::Error) -> SerialError {
    let is_unique_constraint = match &e {
        rusqlite::Error::SqliteFailure(err, _) => {
            err.extended_code == 2067 || err.code == rusqlite::ffi::ErrorCode::ConstraintViolation
        }
        _ => false,
    };

    let msg = e.to_string();
    if is_unique_constraint || msg.contains("UNIQUE constraint failed") {
        if msg.contains("idx_serial_numbers_serial_active")
            || msg.contains("serial_numbers.serial_number")
        {
            return SerialError::DuplicateSerial("Serial number already exists".to_string());
        }
        if msg.contains("idx_serial_numbers_imei_active") || msg.contains("serial_numbers.imei") {
            return SerialError::DuplicateImei("IMEI already exists".to_string());
        }
        if msg.contains("idx_serial_numbers_asset_tag_branch")
            || msg.contains("serial_numbers.branch_id, serial_numbers.asset_tag")
            || msg.contains("serial_numbers.asset_tag")
        {
            return SerialError::DuplicateAssetTag(
                "Asset tag already exists in this branch".to_string(),
            );
        }
    }

    SerialError::Database(msg)
}

// =========================================================================
// CRUD DOMAIN OPERATIONS
// =========================================================================

const SERIAL_COLUMNS: &str = "id, product_id, branch_id, variant_id, serial_number, imei, asset_tag, cost_price_minor, status, sold_in_sale_id, warranty_expires_at, created_at, updated_at";

fn row_to_instance(row: &rusqlite::Row) -> rusqlite::Result<SerializedInstance> {
    let status_str: String = row.get("status")?;
    let status = SerialStatus::from_str(&status_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        )
    })?;

    Ok(SerializedInstance {
        id: row.get("id")?,
        product_id: row.get("product_id")?,
        branch_id: row.get("branch_id")?,
        variant_id: row.get("variant_id")?,
        serial_number: row.get("serial_number")?,
        imei: row.get("imei")?,
        asset_tag: row.get("asset_tag")?,
        cost_price_minor: row.get("cost_price_minor")?,
        status,
        sold_in_sale_id: row.get("sold_in_sale_id")?,
        warranty_expires_at: row.get("warranty_expires_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Registers a new serialized instance.
pub fn create_serial_instance(
    conn: &Connection,
    input: &CreateSerialInput,
) -> Result<SerializedInstance, SerialError> {
    if !is_serial_tracked(conn, &input.product_id)? {
        return Err(SerialError::ProductNotSerialized(input.product_id.clone()));
    }

    if let Some(ref vid) = input.variant_id {
        verify_variant_association(conn, &input.product_id, vid)?;
    }

    let (norm_serial, norm_imei, norm_asset_tag) = validate_identifiers(
        input.serial_number.as_deref(),
        input.imei.as_deref(),
        input.asset_tag.as_deref(),
    )?;

    validate_cost_price(input.cost_price_minor)?;

    check_identifier_collisions(
        conn,
        &input.branch_id,
        norm_serial.as_deref(),
        norm_imei.as_deref(),
        norm_asset_tag.as_deref(),
    )?;

    let id = crate::auth::generate_id();

    let sql = format!(
        "INSERT INTO serial_numbers (
            id, product_id, branch_id, variant_id,
            serial_number, imei, asset_tag, cost_price_minor,
            status, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'in_stock', datetime('now'), datetime('now'))
        RETURNING {SERIAL_COLUMNS}"
    );

    conn.query_row(
        &sql,
        params![
            id,
            input.product_id,
            input.branch_id,
            input.variant_id,
            norm_serial,
            norm_imei,
            norm_asset_tag,
            input.cost_price_minor,
        ],
        row_to_instance,
    )
    .map_err(map_sqlite_collision_error)
}

/// Retrieves a serialized instance by unique record ID.
pub fn get_serial_instance(
    conn: &Connection,
    id: &str,
) -> Result<Option<SerializedInstance>, SerialError> {
    let sql = format!("SELECT {SERIAL_COLUMNS} FROM serial_numbers WHERE id = ?1");
    let mut stmt = conn.prepare_cached(&sql)?;
    let result = stmt.query_row(params![id], row_to_instance).optional()?;
    Ok(result)
}

/// Searches for an active serialized instance by identifier (serial, IMEI, or asset tag) within a branch.
pub fn lookup_serial_instance(
    conn: &Connection,
    identifier: &str,
    branch_id: &str,
) -> Result<Option<SerializedInstance>, SerialError> {
    let clean = identifier.trim();
    if clean.is_empty() {
        return Ok(None);
    }

    let sql = format!(
        "SELECT {SERIAL_COLUMNS} FROM serial_numbers
         WHERE branch_id = ?1
           AND (
               serial_number = ?2 COLLATE NOCASE
               OR imei = ?2
               OR asset_tag = ?2 COLLATE NOCASE
           )
         LIMIT 1"
    );

    let mut stmt = conn.prepare_cached(&sql)?;
    let result = stmt
        .query_row(params![branch_id, clean], row_to_instance)
        .optional()?;
    Ok(result)
}

/// Lists serialized instances matching the filter criteria.
pub fn list_serial_instances(
    conn: &Connection,
    filter: &SerialFilter,
) -> Result<Vec<SerializedInstance>, SerialError> {
    let mut query = format!("SELECT {SERIAL_COLUMNS} FROM serial_numbers WHERE branch_id = ?1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(filter.branch_id.clone())];

    if let Some(ref pid) = filter.product_id {
        params_vec.push(Box::new(pid.clone()));
        query.push_str(&format!(" AND product_id = ?{}", params_vec.len()));
    }

    if let Some(ref vid) = filter.variant_id {
        params_vec.push(Box::new(vid.clone()));
        query.push_str(&format!(" AND variant_id = ?{}", params_vec.len()));
    }

    if let Some(status) = filter.status {
        params_vec.push(Box::new(status.as_str().to_string()));
        query.push_str(&format!(" AND status = ?{}", params_vec.len()));
    }

    query.push_str(" ORDER BY created_at DESC, id ASC");

    let mut stmt = conn.prepare(&query)?;
    let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();

    let rows = stmt.query_map(rusqlite_params.as_slice(), row_to_instance)?;

    let mut instances = Vec::new();
    for row in rows {
        instances.push(row?);
    }

    Ok(instances)
}

/// Updates the operational lifecycle status of a serialized instance.
pub fn update_serial_status(
    conn: &Connection,
    input: &UpdateSerialStatusInput,
) -> Result<SerializedInstance, SerialError> {
    let current = get_serial_instance(conn, &input.id)?
        .ok_or_else(|| SerialError::NotFound(input.id.clone()))?;

    if current.branch_id != input.branch_id {
        return Err(SerialError::NotFound(input.id.clone()));
    }

    validate_status_transition(current.status, input.status)?;

    let sql = format!(
        "UPDATE serial_numbers
         SET status = ?1, updated_at = datetime('now')
         WHERE id = ?2
         RETURNING {SERIAL_COLUMNS}"
    );

    conn.query_row(
        &sql,
        params![input.status.as_str(), input.id],
        row_to_instance,
    )
    .map_err(|e| SerialError::Database(e.to_string()))
}
