// F2.11 — Stock Movement Ledger & Spatial Inventory Architecture
// ADR-0013: Spatial balance tracking, immutable movements, single write authority, fail-closed negative stock prevention.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// =========================================================================
// ERROR TYPES
// =========================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum StockLedgerError {
    Validation(String),
    ZeroQuantityDelta,
    MissingLocation,
    LocationNotFound(String),
    LocationInactive(String),
    LocationBranchMismatch(String),
    BinNotFound(String),
    BinInactive(String),
    BinLocationMismatch(String),
    ProductNotFound(String),
    VariantMismatch(String),
    BatchMismatch(String),
    BatchDepletedOrInactive(String),
    SerialMismatch(String),
    SerialInvalidStatus(String),
    SerialInvalidQuantity(String),
    NegativeStockBlocked(String),
    IdempotencyConflict(String),
    Database(String),
}

impl std::fmt::Display for StockLedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StockLedgerError::Validation(msg) => write!(f, "Validation error: {msg}"),
            StockLedgerError::ZeroQuantityDelta => {
                write!(f, "Stock movement quantity delta cannot be zero")
            }
            StockLedgerError::MissingLocation => {
                write!(f, "Spatial stock movement requires a valid location_id")
            }
            StockLedgerError::LocationNotFound(msg) => write!(f, "Location not found: {msg}"),
            StockLedgerError::LocationInactive(msg) => write!(f, "Location is inactive: {msg}"),
            StockLedgerError::LocationBranchMismatch(msg) => {
                write!(f, "Location branch mismatch: {msg}")
            }
            StockLedgerError::BinNotFound(msg) => write!(f, "Bin not found: {msg}"),
            StockLedgerError::BinInactive(msg) => write!(f, "Bin is inactive: {msg}"),
            StockLedgerError::BinLocationMismatch(msg) => write!(f, "Bin location mismatch: {msg}"),
            StockLedgerError::ProductNotFound(msg) => write!(f, "Product not found: {msg}"),
            StockLedgerError::VariantMismatch(msg) => write!(f, "Variant mismatch: {msg}"),
            StockLedgerError::BatchMismatch(msg) => write!(f, "Batch mismatch: {msg}"),
            StockLedgerError::BatchDepletedOrInactive(msg) => {
                write!(f, "Batch depleted or inactive: {msg}")
            }
            StockLedgerError::SerialMismatch(msg) => write!(f, "Serial mismatch: {msg}"),
            StockLedgerError::SerialInvalidStatus(msg) => {
                write!(f, "Serial invalid status: {msg}")
            }
            StockLedgerError::SerialInvalidQuantity(msg) => {
                write!(f, "Serial invalid quantity: {msg}")
            }
            StockLedgerError::NegativeStockBlocked(msg) => {
                write!(f, "Negative stock blocked: {msg}")
            }
            StockLedgerError::IdempotencyConflict(msg) => {
                write!(f, "Idempotency conflict: {msg}")
            }
            StockLedgerError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for StockLedgerError {}

impl From<rusqlite::Error> for StockLedgerError {
    fn from(err: rusqlite::Error) -> Self {
        match &err {
            rusqlite::Error::SqliteFailure(sqlite_err, Some(msg)) => {
                if msg.contains("CHECK constraint failed: quantity_milli >= 0") {
                    StockLedgerError::NegativeStockBlocked(
                        "Negative balance rejected by database check constraint".to_string(),
                    )
                } else if msg.contains("Historical stock_movements rows are immutable") {
                    StockLedgerError::Database(msg.clone())
                } else if msg.contains("Stock movement quantity delta cannot be zero") {
                    StockLedgerError::ZeroQuantityDelta
                } else if msg.contains("Movement location branch does not match movement branch") {
                    StockLedgerError::LocationBranchMismatch(msg.clone())
                } else if msg.contains("Movement bin does not belong to movement location") {
                    StockLedgerError::BinLocationMismatch(msg.clone())
                } else if msg.contains("Movement bin cannot be specified without a location") {
                    StockLedgerError::MissingLocation
                } else if msg.contains("Movement batch does not match product or branch") {
                    StockLedgerError::BatchMismatch(msg.clone())
                } else if msg.contains("Movement serial does not match product or branch") {
                    StockLedgerError::SerialMismatch(msg.clone())
                } else if msg.contains("UNIQUE constraint failed: location_inventory") {
                    StockLedgerError::Database(
                        "Duplicate location_inventory slot collision".to_string(),
                    )
                } else {
                    StockLedgerError::Database(format!("{}: {}", sqlite_err.extended_code, msg))
                }
            }
            _ => StockLedgerError::Database(err.to_string()),
        }
    }
}

// =========================================================================
// MOVEMENT REASON & DOMAIN MODELS
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementReason {
    OpeningBalance,
    Adjustment,
    Damage,
    Loss,
}

impl MovementReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            MovementReason::OpeningBalance => "opening_balance",
            MovementReason::Adjustment => "adjustment",
            MovementReason::Damage => "damage",
            MovementReason::Loss => "loss",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, StockLedgerError> {
        match s.trim().to_lowercase().as_str() {
            "opening_balance" => Ok(MovementReason::OpeningBalance),
            "adjustment" => Ok(MovementReason::Adjustment),
            "damage" => Ok(MovementReason::Damage),
            "loss" => Ok(MovementReason::Loss),
            _ => Err(StockLedgerError::Validation(format!(
                "Invalid movement reason '{s}'. Authorized F2.11 reasons: opening_balance, adjustment, damage, loss"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMovementRequest {
    pub idempotency_key: String,
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub location_id: String,
    pub bin_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub quantity_delta_milli: i64,
    pub reason: MovementReason,
    pub user_id: Option<String>,
    pub notes: Option<String>,
}

impl PostMovementRequest {
    pub fn canonical_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let payload = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.branch_id.trim(),
            self.product_id.trim(),
            self.variant_id.as_deref().unwrap_or("").trim(),
            self.location_id.trim(),
            self.bin_id.as_deref().unwrap_or("").trim(),
            self.batch_id.as_deref().unwrap_or("").trim(),
            self.serial_id.as_deref().unwrap_or("").trim(),
            self.quantity_delta_milli,
            self.reason.as_str()
        );
        hasher.update(payload.as_bytes());
        let hash_bytes = hasher.finalize();
        format!("{:x}", hash_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovementResult {
    pub movement_id: String,
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub location_id: String,
    pub bin_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub quantity_delta_milli: i64,
    pub quantity_before_milli: i64,
    pub quantity_after_milli: i64,
    pub slot_before_milli: i64,
    pub slot_after_milli: i64,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationInventoryRecord {
    pub id: String,
    pub branch_id: String,
    pub location_id: String,
    pub bin_id: Option<String>,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub batch_id: Option<String>,
    pub quantity_milli: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockSummaryRecord {
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub total_quantity_milli: i64,
    pub spatial_quantity_milli: i64,
    pub unallocated_quantity_milli: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchSummaryRecord {
    pub batch_id: String,
    pub branch_id: String,
    pub product_id: String,
    pub total_quantity_milli: i64,
    pub spatial_quantity_milli: i64,
    pub unallocated_quantity_milli: i64,
}

// =========================================================================
// DECOMPOSED VALIDATION & MUTATION HELPERS (COGNITIVE COMPLEXITY <= 15)
// =========================================================================

fn validate_required_strings_and_delta(req: &PostMovementRequest) -> Result<(), StockLedgerError> {
    if req.idempotency_key.trim().is_empty() {
        return Err(StockLedgerError::Validation(
            "idempotency_key cannot be empty".into(),
        ));
    }
    if req.branch_id.trim().is_empty() {
        return Err(StockLedgerError::Validation(
            "branch_id cannot be empty".into(),
        ));
    }
    if req.product_id.trim().is_empty() {
        return Err(StockLedgerError::Validation(
            "product_id cannot be empty".into(),
        ));
    }
    if req.location_id.trim().is_empty() {
        return Err(StockLedgerError::MissingLocation);
    }
    if req.quantity_delta_milli == 0 {
        return Err(StockLedgerError::ZeroQuantityDelta);
    }
    Ok(())
}

fn validate_movement_reason_and_serial(req: &PostMovementRequest) -> Result<(), StockLedgerError> {
    match req.reason {
        MovementReason::OpeningBalance => {
            if req.quantity_delta_milli <= 0 {
                return Err(StockLedgerError::Validation(
                    "opening_balance movement requires strictly positive quantity_delta_milli"
                        .into(),
                ));
            }
        }
        MovementReason::Damage => {
            if req.quantity_delta_milli >= 0 {
                return Err(StockLedgerError::Validation(
                    "damage movement requires strictly negative quantity_delta_milli".into(),
                ));
            }
        }
        MovementReason::Loss => {
            if req.quantity_delta_milli >= 0 {
                return Err(StockLedgerError::Validation(
                    "loss movement requires strictly negative quantity_delta_milli".into(),
                ));
            }
        }
        MovementReason::Adjustment => {
            if req.quantity_delta_milli == 0 {
                return Err(StockLedgerError::ZeroQuantityDelta);
            }
        }
    }

    if req.serial_id.is_some() && req.quantity_delta_milli.abs() != 1000 {
        return Err(StockLedgerError::SerialInvalidQuantity(
            "Serialized stock movements must have quantity_delta_milli equal to +1000 or -1000"
                .into(),
        ));
    }
    Ok(())
}

fn validate_request_basic(req: &PostMovementRequest) -> Result<(), StockLedgerError> {
    validate_required_strings_and_delta(req)?;
    validate_movement_reason_and_serial(req)?;
    Ok(())
}

fn check_idempotency(
    conn: &Connection,
    key: &str,
    canonical_hash: &str,
) -> Result<Option<StockMovementResult>, StockLedgerError> {
    let existing: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT result_json, request_hash FROM idempotency_keys WHERE key = ?1",
            params![key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    if let Some((result_json, stored_hash)) = existing {
        if stored_hash.as_deref() == Some(canonical_hash) {
            if let Some(json_str) = result_json {
                let cached: StockMovementResult = serde_json::from_str(&json_str).map_err(|e| {
                    StockLedgerError::Database(format!("Corrupted cached result JSON: {e}"))
                })?;
                return Ok(Some(cached));
            }
        } else {
            return Err(StockLedgerError::IdempotencyConflict(format!(
                "Idempotency key collision for key '{key}' with mismatched payload"
            )));
        }
    }
    Ok(None)
}

fn validate_branch_and_location(
    conn: &Connection,
    branch_id: &str,
    location_id: &str,
    bin_id: Option<&str>,
) -> Result<(), StockLedgerError> {
    let branch_active: Option<i64> = conn
        .query_row(
            "SELECT is_active FROM branches WHERE id = ?1",
            params![branch_id],
            |row| row.get(0),
        )
        .optional()?;

    match branch_active {
        None => {
            return Err(StockLedgerError::Validation(format!(
                "Branch '{branch_id}' not found"
            )))
        }
        Some(0) => {
            return Err(StockLedgerError::Validation(format!(
                "Branch '{branch_id}' is inactive"
            )))
        }
        Some(_) => {}
    }

    let loc_info: Option<(String, i64)> = conn
        .query_row(
            "SELECT branch_id, is_active FROM locations WHERE id = ?1",
            params![location_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match loc_info {
        None => {
            return Err(StockLedgerError::LocationNotFound(format!(
                "Location '{location_id}' not found"
            )))
        }
        Some((loc_branch, _)) if loc_branch != branch_id => {
            return Err(StockLedgerError::LocationBranchMismatch(format!(
                "Location '{location_id}' belongs to branch '{loc_branch}', not '{branch_id}'"
            )))
        }
        Some((_, 0)) => {
            return Err(StockLedgerError::LocationInactive(format!(
                "Location '{location_id}' is inactive"
            )))
        }
        Some(_) => {}
    }

    if let Some(bin) = bin_id {
        let bin_info: Option<(String, i64)> = conn
            .query_row(
                "SELECT location_id, is_active FROM bins WHERE id = ?1",
                params![bin],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        match bin_info {
            None => {
                return Err(StockLedgerError::BinNotFound(format!(
                    "Bin '{bin}' not found"
                )))
            }
            Some((bin_loc, _)) if bin_loc != location_id => {
                return Err(StockLedgerError::BinLocationMismatch(format!(
                    "Bin '{bin}' belongs to location '{bin_loc}', not '{location_id}'"
                )))
            }
            Some((_, 0)) => {
                return Err(StockLedgerError::BinInactive(format!(
                    "Bin '{bin}' is inactive"
                )))
            }
            Some(_) => {}
        }
    }

    Ok(())
}

fn validate_product_and_variant(
    conn: &Connection,
    product_id: &str,
    variant_id: Option<&str>,
) -> Result<(), StockLedgerError> {
    let prod_active: Option<i64> = conn
        .query_row(
            "SELECT is_active FROM products WHERE id = ?1",
            params![product_id],
            |row| row.get(0),
        )
        .optional()?;

    match prod_active {
        None => {
            return Err(StockLedgerError::ProductNotFound(format!(
                "Product '{product_id}' not found"
            )))
        }
        Some(0) => {
            return Err(StockLedgerError::Validation(format!(
                "Product '{product_id}' is inactive"
            )))
        }
        Some(_) => {}
    }

    if let Some(var_id) = variant_id {
        let var_prod_id: Option<String> = conn
            .query_row(
                "SELECT product_id FROM product_variants WHERE id = ?1",
                params![var_id],
                |row| row.get(0),
            )
            .optional()?;

        match var_prod_id {
            None => {
                return Err(StockLedgerError::VariantMismatch(format!(
                    "Variant '{var_id}' not found"
                )))
            }
            Some(var_prod) if var_prod != product_id => {
                return Err(StockLedgerError::VariantMismatch(format!(
                    "Variant '{var_id}' belongs to product '{var_prod}', not '{product_id}'"
                )))
            }
            Some(_) => {}
        }
    }

    Ok(())
}

fn validate_batch_lot(
    conn: &Connection,
    batch_id: Option<&str>,
    product_id: &str,
    branch_id: &str,
    variant_id: Option<&str>,
    delta: i64,
) -> Result<Option<(i64, String)>, StockLedgerError> {
    let Some(b_id) = batch_id else {
        return Ok(None);
    };

    let batch_info: Option<(String, String, Option<String>, String, i64)> = conn
        .query_row(
            "SELECT product_id, branch_id, variant_id, status, quantity_milli FROM product_batches WHERE id = ?1",
            params![b_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()?;

    match batch_info {
        None => Err(StockLedgerError::BatchMismatch(format!(
            "Batch '{b_id}' not found"
        ))),
        Some((batch_prod, batch_branch, batch_variant, status, qty_milli)) => {
            if batch_prod != product_id {
                return Err(StockLedgerError::BatchMismatch(format!(
                    "Batch '{b_id}' belongs to product '{batch_prod}', not '{product_id}'"
                )));
            }
            if batch_branch != branch_id {
                return Err(StockLedgerError::BatchMismatch(format!(
                    "Batch '{b_id}' belongs to branch '{batch_branch}', not '{branch_id}'"
                )));
            }
            if batch_variant.as_deref() != variant_id {
                return Err(StockLedgerError::BatchMismatch(format!(
                    "Batch '{b_id}' belongs to variant '{batch_variant:?}', not '{variant_id:?}'"
                )));
            }
            if (status == "recalled" || status == "quarantined") && delta > 0 {
                return Err(StockLedgerError::BatchDepletedOrInactive(format!(
                    "Batch '{b_id}' has non-active status '{status}'"
                )));
            }
            if delta < 0 && (qty_milli + delta) < 0 {
                return Err(StockLedgerError::NegativeStockBlocked(format!(
                    "Insufficient batch quantity: current {qty_milli} milli, delta {delta} milli"
                )));
            }
            Ok(Some((qty_milli, status)))
        }
    }
}

struct SerialRecord {
    id: String,
    product_id: String,
    branch_id: String,
    variant_id: Option<String>,
    status: String,
    location_id: Option<String>,
    bin_id: Option<String>,
}

fn fetch_serial_record(conn: &Connection, s_id: &str) -> Result<SerialRecord, StockLedgerError> {
    conn.query_row(
        "SELECT id, product_id, branch_id, variant_id, status, location_id, bin_id
         FROM serial_numbers WHERE id = ?1",
        params![s_id],
        |row| {
            Ok(SerialRecord {
                id: row.get(0)?,
                product_id: row.get(1)?,
                branch_id: row.get(2)?,
                variant_id: row.get(3)?,
                status: row.get(4)?,
                location_id: row.get(5)?,
                bin_id: row.get(6)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| StockLedgerError::SerialMismatch(format!("Serial '{s_id}' not found")))
}

fn validate_serial_identity(
    record: &SerialRecord,
    product_id: &str,
    branch_id: &str,
    variant_id: Option<&str>,
) -> Result<(), StockLedgerError> {
    if record.product_id != product_id {
        return Err(StockLedgerError::SerialMismatch(format!(
            "Serial '{}' belongs to product '{}', not '{}'",
            record.id, record.product_id, product_id
        )));
    }
    if record.branch_id != branch_id {
        return Err(StockLedgerError::SerialMismatch(format!(
            "Serial '{}' belongs to branch '{}', not '{}'",
            record.id, record.branch_id, branch_id
        )));
    }
    if record.variant_id.as_deref() != variant_id {
        return Err(StockLedgerError::SerialMismatch(format!(
            "Serial '{}' belongs to variant '{:?}', not '{:?}'",
            record.id, record.variant_id, variant_id
        )));
    }
    Ok(())
}

fn validate_serial_status_and_coordinates(
    record: &SerialRecord,
    location_id: &str,
    bin_id: Option<&str>,
    delta: i64,
) -> Result<(), StockLedgerError> {
    let s_id = &record.id;
    if delta > 0 {
        if record.status == "in_stock" {
            return Err(StockLedgerError::SerialInvalidStatus(format!(
                "Serial '{s_id}' is already in_stock"
            )));
        }
        if record.status == "disposed"
            || record.status == "recalled"
            || record.status == "sold"
            || record.status == "transferred"
        {
            return Err(StockLedgerError::SerialInvalidStatus(format!(
                "Serial '{s_id}' has terminal or non-revivable status '{}'",
                record.status
            )));
        }
    } else {
        if record.status != "in_stock" {
            return Err(StockLedgerError::SerialInvalidStatus(format!(
                "Serial '{s_id}' cannot be deducted; status is '{}', expected 'in_stock'",
                record.status
            )));
        }
        match record.location_id.as_deref() {
            Some(loc) if loc != location_id => {
                return Err(StockLedgerError::LocationBranchMismatch(format!(
                    "Serial '{s_id}' is physically located at location '{loc}', not '{location_id}'"
                )));
            }
            None => {
                return Err(StockLedgerError::LocationBranchMismatch(format!(
                    "Serial '{s_id}' has no physical location assigned"
                )));
            }
            _ => {}
        }
        if record.bin_id.as_deref() != bin_id {
            return Err(StockLedgerError::LocationBranchMismatch(format!(
                "Serial '{s_id}' is physically located at bin '{:?}', not '{:?}'",
                record.bin_id, bin_id
            )));
        }
    }
    Ok(())
}

fn validate_serial_asset(
    conn: &Connection,
    serial_id: Option<&str>,
    product_id: &str,
    branch_id: &str,
    variant_id: Option<&str>,
    location_id: &str,
    bin_id: Option<&str>,
    delta: i64,
) -> Result<(), StockLedgerError> {
    let Some(s_id) = serial_id else {
        return Ok(());
    };
    let record = fetch_serial_record(conn, s_id)?;
    validate_serial_identity(&record, product_id, branch_id, variant_id)?;
    validate_serial_status_and_coordinates(&record, location_id, bin_id, delta)?;
    Ok(())
}

fn mutate_aggregate_inventory(
    conn: &Connection,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
    delta: i64,
) -> Result<(i64, i64), StockLedgerError> {
    let agg_info: Option<(String, i64)> = conn
        .query_row(
            "SELECT id, quantity_milli FROM inventory WHERE branch_id = ?1 AND product_id = ?2 AND variant_id IS ?3",
            params![branch_id, product_id, variant_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let (agg_id, agg_before) = match agg_info {
        Some((id, qty)) => (Some(id), qty),
        None => (None, 0),
    };

    let agg_after = agg_before.checked_add(delta).ok_or_else(|| {
        StockLedgerError::Validation("Arithmetic overflow calculating aggregate stock".into())
    })?;

    if agg_after < 0 {
        return Err(StockLedgerError::NegativeStockBlocked(format!(
            "Insufficient aggregate stock: on-hand {agg_before} milli, delta {delta} milli, resulting in negative {agg_after}"
        )));
    }

    if let Some(ref existing_agg_id) = agg_id {
        conn.execute(
            "UPDATE inventory SET quantity_milli = ?1, quantity = ?1 / 1000.0, updated_at = datetime('now') WHERE id = ?2",
            params![agg_after, existing_agg_id],
        )?;
    } else {
        let new_agg_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO inventory (id, branch_id, product_id, variant_id, quantity, quantity_milli, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5 / 1000.0, ?5, datetime('now'))",
            params![new_agg_id, branch_id, product_id, variant_id, agg_after],
        )?;
    }

    Ok((agg_before, agg_after))
}

fn mutate_spatial_inventory(
    conn: &Connection,
    req: &PostMovementRequest,
) -> Result<(i64, i64), StockLedgerError> {
    let branch_id = req.branch_id.trim();
    let location_id = req.location_id.trim();
    let bin_id = req
        .bin_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let product_id = req.product_id.trim();
    let variant_id = req
        .variant_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let batch_id = req
        .batch_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let delta = req.quantity_delta_milli;

    let slot_info: Option<(String, i64)> = conn
        .query_row(
            "SELECT id, quantity_milli FROM location_inventory
             WHERE branch_id = ?1 AND location_id = ?2 AND bin_id IS ?3
               AND product_id = ?4 AND variant_id IS ?5 AND batch_id IS ?6",
            params![
                branch_id,
                location_id,
                bin_id,
                product_id,
                variant_id,
                batch_id
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let (slot_id, slot_before) = match slot_info {
        Some((id, qty)) => (Some(id), qty),
        None => (None, 0),
    };

    let slot_after = slot_before.checked_add(delta).ok_or_else(|| {
        StockLedgerError::Validation("Arithmetic overflow calculating slot stock".into())
    })?;

    if slot_after < 0 {
        return Err(StockLedgerError::NegativeStockBlocked(format!(
            "Insufficient spatial slot stock: on-hand {slot_before} milli, delta {delta} milli, resulting in negative {slot_after}"
        )));
    }

    if let Some(ref existing_slot_id) = slot_id {
        conn.execute(
            "UPDATE location_inventory SET quantity_milli = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![slot_after, existing_slot_id],
        )?;
    } else {
        let new_slot_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO location_inventory (
                id, branch_id, location_id, bin_id, product_id, variant_id, batch_id,
                quantity_milli, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'), datetime('now'))",
            params![
                new_slot_id,
                branch_id,
                location_id,
                bin_id,
                product_id,
                variant_id,
                batch_id,
                slot_after
            ],
        )?;
    }

    Ok((slot_before, slot_after))
}

fn mutate_batch_inventory(
    conn: &Connection,
    batch_id: Option<&str>,
    batch_info: Option<(i64, String)>,
    delta: i64,
) -> Result<(), StockLedgerError> {
    if let (Some(b_id), Some((b_qty, current_status))) = (batch_id, batch_info) {
        let new_b_qty = b_qty
            .checked_add(delta)
            .ok_or_else(|| StockLedgerError::Validation("Batch quantity overflow".to_string()))?;
        if new_b_qty < 0 {
            return Err(StockLedgerError::NegativeStockBlocked(format!(
                "Batch '{b_id}' quantity would drop below zero: {new_b_qty}"
            )));
        }
        let new_status = if new_b_qty == 0 {
            "depleted"
        } else if current_status == "depleted" {
            "active"
        } else {
            &current_status
        };
        conn.execute(
            "UPDATE product_batches SET quantity_milli = ?1, status = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![new_b_qty, new_status, b_id],
        )?;
    }
    Ok(())
}

fn mutate_serial_inventory(
    conn: &Connection,
    serial_id: Option<&str>,
    location_id: &str,
    bin_id: Option<&str>,
    delta: i64,
    reason: MovementReason,
) -> Result<(), StockLedgerError> {
    if let Some(s_id) = serial_id {
        if delta > 0 {
            conn.execute(
                "UPDATE serial_numbers SET status = 'in_stock', location_id = ?1, bin_id = ?2 WHERE id = ?3",
                params![location_id, bin_id, s_id],
            )?;
        } else {
            let new_status = match reason {
                MovementReason::Damage => "defective",
                _ => "disposed",
            };
            conn.execute(
                "UPDATE serial_numbers SET status = ?1, location_id = NULL, bin_id = NULL WHERE id = ?2",
                params![new_status, s_id],
            )?;
        }
    }
    Ok(())
}

fn append_movement_record(
    conn: &Connection,
    req: &PostMovementRequest,
    agg_before: i64,
    agg_after: i64,
) -> Result<(String, String), StockLedgerError> {
    let movement_id = Uuid::new_v4().to_string();
    let reason_str = req.reason.as_str();
    let user_id = req
        .user_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let bin_id = req
        .bin_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let var_id = req
        .variant_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let batch_id = req
        .batch_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let serial_id = req
        .serial_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    conn.execute(
        "INSERT INTO stock_movements (
            id, branch_id, product_id, variant_id,
            quantity_delta, quantity_delta_milli,
            quantity_before, quantity_before_milli,
            quantity_after, quantity_after_milli,
            reason, source_type, source_id, user_id,
            location_id, bin_id, batch_id, serial_id,
            created_at
         ) VALUES (
            ?1, ?2, ?3, ?4,
            ?5 / 1000.0, ?5,
            ?6 / 1000.0, ?6,
            ?7 / 1000.0, ?7,
            ?8, 'manual', NULL, ?9,
            ?10, ?11, ?12, ?13,
            datetime('now')
         )",
        params![
            movement_id,
            req.branch_id.trim(),
            req.product_id.trim(),
            var_id,
            req.quantity_delta_milli,
            agg_before,
            agg_after,
            reason_str,
            user_id,
            req.location_id.trim(),
            bin_id,
            batch_id,
            serial_id,
        ],
    )?;

    let created_at: String = conn.query_row(
        "SELECT created_at FROM stock_movements WHERE id = ?1",
        params![movement_id],
        |row| row.get(0),
    )?;

    Ok((movement_id, created_at))
}

fn persist_idempotency(
    conn: &Connection,
    key: &str,
    result: &StockMovementResult,
    canonical_hash: &str,
) -> Result<(), StockLedgerError> {
    let result_json = serde_json::to_string(result)
        .map_err(|e| StockLedgerError::Database(format!("Failed to serialize result JSON: {e}")))?;

    conn.execute(
        "INSERT INTO idempotency_keys (key, operation, result_json, request_hash, created_at)
         VALUES (?1, 'stock_movement', ?2, ?3, datetime('now'))",
        params![key, result_json, canonical_hash],
    )?;

    Ok(())
}

// =========================================================================
// SERVICE IMPLEMENTATION (SINGLE WRITE AUTHORITY)
// =========================================================================

pub struct StockLedgerService;

impl StockLedgerService {
    /// Posts a stock movement atomically with single write authority over
    /// aggregate, spatial, batch, and serial balances.
    pub fn post_movement(
        conn: &mut Connection,
        req: &PostMovementRequest,
    ) -> Result<StockMovementResult, StockLedgerError> {
        // 1. Basic request validation
        validate_request_basic(req)?;

        let canonical_hash = req.canonical_hash();
        let trimmed_key = req.idempotency_key.trim();

        // 2. Idempotency pre-check
        if let Some(cached) = check_idempotency(conn, trimmed_key, &canonical_hash)? {
            return Ok(cached);
        }

        // 3. Begin atomic transaction
        let tx = conn.transaction()?;

        let branch_id = req.branch_id.trim();
        let product_id = req.product_id.trim();
        let location_id = req.location_id.trim();
        let variant_id = req
            .variant_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let bin_id = req
            .bin_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let batch_id = req
            .batch_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let serial_id = req
            .serial_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        // 4. Validate relational references
        validate_branch_and_location(&tx, branch_id, location_id, bin_id)?;
        validate_product_and_variant(&tx, product_id, variant_id)?;
        let current_batch_qty = validate_batch_lot(
            &tx,
            batch_id,
            product_id,
            branch_id,
            variant_id,
            req.quantity_delta_milli,
        )?;
        validate_serial_asset(
            &tx,
            serial_id,
            product_id,
            branch_id,
            variant_id,
            location_id,
            bin_id,
            req.quantity_delta_milli,
        )?;

        // 5. Mutate balances atomically
        let (agg_before, agg_after) = mutate_aggregate_inventory(
            &tx,
            branch_id,
            product_id,
            variant_id,
            req.quantity_delta_milli,
        )?;
        let (slot_before, slot_after) = mutate_spatial_inventory(&tx, req)?;
        mutate_batch_inventory(&tx, batch_id, current_batch_qty, req.quantity_delta_milli)?;
        mutate_serial_inventory(
            &tx,
            serial_id,
            location_id,
            bin_id,
            req.quantity_delta_milli,
            req.reason,
        )?;

        // 6. Append immutable stock movement
        let (movement_id, created_at) = append_movement_record(&tx, req, agg_before, agg_after)?;

        let result = StockMovementResult {
            movement_id,
            branch_id: branch_id.to_string(),
            product_id: product_id.to_string(),
            variant_id: variant_id.map(String::from),
            location_id: location_id.to_string(),
            bin_id: bin_id.map(String::from),
            batch_id: batch_id.map(String::from),
            serial_id: serial_id.map(String::from),
            quantity_delta_milli: req.quantity_delta_milli,
            quantity_before_milli: agg_before,
            quantity_after_milli: agg_after,
            slot_before_milli: slot_before,
            slot_after_milli: slot_after,
            reason: req.reason.as_str().to_string(),
            created_at,
        };

        // 7. Persist idempotency record
        persist_idempotency(&tx, trimmed_key, &result, &canonical_hash)?;

        // 8. Commit atomic transaction
        tx.commit()?;

        Ok(result)
    }

    /// Computes summary of aggregate, spatial, and unallocated inventory.
    pub fn get_stock_summary(
        conn: &Connection,
        branch_id: &str,
        product_id: &str,
        variant_id: Option<&str>,
    ) -> Result<StockSummaryRecord, StockLedgerError> {
        let branch_id = branch_id.trim();
        let product_id = product_id.trim();
        let trimmed_variant_id = variant_id.map(str::trim).filter(|s| !s.is_empty());

        let total_milli: i64 = conn
            .query_row(
                "SELECT COALESCE(quantity_milli, 0) FROM inventory WHERE branch_id = ?1 AND product_id = ?2 AND variant_id IS ?3",
                params![branch_id, product_id, trimmed_variant_id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);

        let spatial_milli: i64 = conn.query_row(
            "SELECT COALESCE(SUM(quantity_milli), 0) FROM location_inventory
             WHERE branch_id = ?1 AND product_id = ?2 AND variant_id IS ?3",
            params![branch_id, product_id, trimmed_variant_id],
            |row| row.get(0),
        )?;

        let unallocated_milli = total_milli.saturating_sub(spatial_milli);

        Ok(StockSummaryRecord {
            branch_id: branch_id.to_string(),
            product_id: product_id.to_string(),
            variant_id: trimmed_variant_id.map(String::from),
            total_quantity_milli: total_milli,
            spatial_quantity_milli: spatial_milli,
            unallocated_quantity_milli: unallocated_milli,
        })
    }

    /// Computes batch inventory summary.
    pub fn get_batch_summary(
        conn: &Connection,
        branch_id: &str,
        batch_id: &str,
    ) -> Result<BatchSummaryRecord, StockLedgerError> {
        let branch_id = branch_id.trim();
        let batch_id = batch_id.trim();

        let batch_info: Option<(String, i64)> = conn
            .query_row(
                "SELECT product_id, quantity_milli FROM product_batches WHERE id = ?1 AND branch_id = ?2",
                params![batch_id, branch_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let (product_id, total_milli) = match batch_info {
            Some(info) => info,
            None => {
                return Err(StockLedgerError::BatchMismatch(format!(
                    "Batch '{batch_id}' not found for branch '{branch_id}'"
                )))
            }
        };

        let spatial_milli: i64 = conn.query_row(
            "SELECT COALESCE(SUM(quantity_milli), 0) FROM location_inventory
             WHERE branch_id = ?1 AND batch_id = ?2",
            params![branch_id, batch_id],
            |row| row.get(0),
        )?;

        let unallocated_milli = total_milli.saturating_sub(spatial_milli);

        Ok(BatchSummaryRecord {
            batch_id: batch_id.to_string(),
            branch_id: branch_id.to_string(),
            product_id,
            total_quantity_milli: total_milli,
            spatial_quantity_milli: spatial_milli,
            unallocated_quantity_milli: unallocated_milli,
        })
    }

    /// Retrieves spatial slot balance.
    #[cfg(test)]
    pub fn get_location_balance(
        conn: &Connection,
        branch_id: &str,
        location_id: &str,
        bin_id: Option<&str>,
        product_id: &str,
        variant_id: Option<&str>,
        batch_id: Option<&str>,
    ) -> Result<i64, StockLedgerError> {
        let branch_id = branch_id.trim();
        let location_id = location_id.trim();
        let bin_id = bin_id.map(str::trim).filter(|s| !s.is_empty());
        let product_id = product_id.trim();
        let variant_id = variant_id.map(str::trim).filter(|s| !s.is_empty());
        let batch_id = batch_id.map(str::trim).filter(|s| !s.is_empty());

        let qty: i64 = conn
            .query_row(
                "SELECT quantity_milli FROM location_inventory
                 WHERE branch_id = ?1 AND location_id = ?2 AND bin_id IS ?3
                   AND product_id = ?4 AND variant_id IS ?5 AND batch_id IS ?6",
                params![
                    branch_id,
                    location_id,
                    bin_id,
                    product_id,
                    variant_id,
                    batch_id
                ],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);

        Ok(qty)
    }

    /// Lists all spatial inventory records for a product across a branch.
    pub fn get_product_spatial_balances(
        conn: &Connection,
        branch_id: &str,
        product_id: &str,
    ) -> Result<Vec<LocationInventoryRecord>, StockLedgerError> {
        let branch_id = branch_id.trim();
        let product_id = product_id.trim();

        let mut stmt = conn.prepare(
            "SELECT id, branch_id, location_id, bin_id, product_id, variant_id, batch_id, quantity_milli, updated_at
             FROM location_inventory
             WHERE branch_id = ?1 AND product_id = ?2
             ORDER BY location_id, bin_id",
        )?;

        let rows = stmt.query_map(params![branch_id, product_id], |row| {
            Ok(LocationInventoryRecord {
                id: row.get(0)?,
                branch_id: row.get(1)?,
                location_id: row.get(2)?,
                bin_id: row.get(3)?,
                product_id: row.get(4)?,
                variant_id: row.get(5)?,
                batch_id: row.get(6)?,
                quantity_milli: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }
}
