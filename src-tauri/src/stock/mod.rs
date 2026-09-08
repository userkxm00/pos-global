// F2.11 — Stock Movement Ledger & Spatial Inventory
// ADR-0013: Spatial balance tracking, immutable ledger, single write authority.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use uuid::Uuid;

// =========================================================================
// DATA STRUCTURES
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementReason {
    #[serde(rename = "opening_balance")]
    OpeningBalance,
    #[serde(rename = "adjustment")]
    Adjustment,
    #[serde(rename = "damage")]
    Damage,
    #[serde(rename = "loss")]
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
        match s {
            "opening_balance" => Ok(MovementReason::OpeningBalance),
            "adjustment" => Ok(MovementReason::Adjustment),
            "damage" => Ok(MovementReason::Damage),
            "loss" => Ok(MovementReason::Loss),
            other => Err(StockLedgerError::Database(format!(
                "Invalid movement reason '{other}'. Permitted: opening_balance, adjustment, damage, loss"
            ))),
        }
    }
}

impl fmt::Display for MovementReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
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
}

impl PostMovementRequest {
    pub fn normalized_variant_id(&self) -> Result<Option<&str>, StockLedgerError> {
        let v = self
            .variant_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        Ok(v)
    }

    pub fn normalized_bin_id(&self) -> Result<Option<&str>, StockLedgerError> {
        let b = self
            .bin_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        Ok(b)
    }

    pub fn normalized_batch_id(&self) -> Result<Option<&str>, StockLedgerError> {
        let b = self
            .batch_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        Ok(b)
    }

    pub fn normalized_serial_id(&self) -> Result<Option<&str>, StockLedgerError> {
        let s = self
            .serial_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        Ok(s)
    }

    pub fn normalized_user_id(&self) -> Result<Option<&str>, StockLedgerError> {
        let u = self
            .user_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        Ok(u)
    }

    pub fn canonical_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.branch_id.trim().as_bytes());
        hasher.update(b"|");
        hasher.update(self.product_id.trim().as_bytes());
        hasher.update(b"|");
        hasher.update(
            self.normalized_variant_id()
                .ok()
                .flatten()
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update(b"|");
        hasher.update(self.location_id.trim().as_bytes());
        hasher.update(b"|");
        hasher.update(
            self.normalized_bin_id()
                .ok()
                .flatten()
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update(b"|");
        hasher.update(
            self.normalized_batch_id()
                .ok()
                .flatten()
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update(b"|");
        hasher.update(
            self.normalized_serial_id()
                .ok()
                .flatten()
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update(b"|");
        hasher.update(self.quantity_delta_milli.to_string().as_bytes());
        hasher.update(b"|");
        hasher.update(self.reason.as_str().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementResult {
    pub movement_id: String,
    pub idempotency_key: String,
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
    pub reason: MovementReason,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSummaryRecord {
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub aggregate_quantity_milli: i64,
    pub allocated_spatial_milli: i64,
    pub unallocated_quantity_milli: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInventoryRecord {
    pub id: String,
    pub branch_id: String,
    pub location_id: String,
    pub bin_id: Option<String>,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub batch_id: Option<String>,
    pub quantity_milli: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummaryRecord {
    pub batch_id: String,
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub batch_number: Option<String>,
    pub status: String,
    pub total_quantity_milli: i64,
    pub spatial_quantity_milli: i64,
    pub unallocated_quantity_milli: i64,
}

// =========================================================================
// ERROR TYPES
// =========================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum StockLedgerError {
    MissingLocation,
    ZeroQuantityDelta,
    NegativeStock(String),
    LocationNotFound(String),
    LocationBranchMismatch(String),
    BinNotFound(String),
    BinLocationMismatch(String),
    ProductNotFound(String),
    VariantNotFound(String),
    VariantProductMismatch(String),
    VariantInactive(String),
    BatchNotFound(String),
    BatchProductMismatch(String),
    BatchBranchMismatch(String),
    BatchVariantMismatch(String),
    BatchInsufficientStock(String),
    BatchArithmeticOverflow(String),
    SerialNotFound(String),
    SerialProductMismatch(String),
    SerialBranchMismatch(String),
    SerialVariantMismatch(String),
    SerialInvalidQuantity(String),
    SerialInvalidStatus(String),
    SerialCoordinateMismatch(String),
    IdempotencyConflict(String),
    Database(String),
}

impl fmt::Display for StockLedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StockLedgerError::MissingLocation => write!(f, "location_id is mandatory"),
            StockLedgerError::ZeroQuantityDelta => {
                write!(f, "quantity_delta_milli must be non-zero")
            }
            StockLedgerError::NegativeStock(msg) => write!(f, "Negative stock violation: {msg}"),
            StockLedgerError::LocationNotFound(msg) => write!(f, "Location not found: {msg}"),
            StockLedgerError::LocationBranchMismatch(msg) => {
                write!(f, "Location branch mismatch: {msg}")
            }
            StockLedgerError::BinNotFound(msg) => write!(f, "Bin not found: {msg}"),
            StockLedgerError::BinLocationMismatch(msg) => write!(f, "Bin location mismatch: {msg}"),
            StockLedgerError::ProductNotFound(msg) => write!(f, "Product not found: {msg}"),
            StockLedgerError::VariantNotFound(msg) => write!(f, "Variant not found: {msg}"),
            StockLedgerError::VariantProductMismatch(msg) => {
                write!(f, "Variant product mismatch: {msg}")
            }
            StockLedgerError::VariantInactive(msg) => write!(f, "Variant inactive: {msg}"),
            StockLedgerError::BatchNotFound(msg) => write!(f, "Batch not found: {msg}"),
            StockLedgerError::BatchProductMismatch(msg) => {
                write!(f, "Batch product mismatch: {msg}")
            }
            StockLedgerError::BatchBranchMismatch(msg) => write!(f, "Batch branch mismatch: {msg}"),
            StockLedgerError::BatchVariantMismatch(msg) => {
                write!(f, "Batch variant mismatch: {msg}")
            }
            StockLedgerError::BatchInsufficientStock(msg) => {
                write!(f, "Batch insufficient stock: {msg}")
            }
            StockLedgerError::BatchArithmeticOverflow(msg) => {
                write!(f, "Batch arithmetic overflow: {msg}")
            }
            StockLedgerError::SerialNotFound(msg) => write!(f, "Serial not found: {msg}"),
            StockLedgerError::SerialProductMismatch(msg) => {
                write!(f, "Serial product mismatch: {msg}")
            }
            StockLedgerError::SerialBranchMismatch(msg) => {
                write!(f, "Serial branch mismatch: {msg}")
            }
            StockLedgerError::SerialVariantMismatch(msg) => {
                write!(f, "Serial variant mismatch: {msg}")
            }
            StockLedgerError::SerialInvalidQuantity(msg) => {
                write!(f, "Serial invalid quantity: {msg}")
            }
            StockLedgerError::SerialInvalidStatus(msg) => write!(f, "Serial invalid status: {msg}"),
            StockLedgerError::SerialCoordinateMismatch(msg) => {
                write!(f, "Serial coordinate mismatch: {msg}")
            }
            StockLedgerError::IdempotencyConflict(msg) => write!(f, "Idempotency conflict: {msg}"),
            StockLedgerError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for StockLedgerError {}

impl From<rusqlite::Error> for StockLedgerError {
    fn from(err: rusqlite::Error) -> Self {
        StockLedgerError::Database(err.to_string())
    }
}

// =========================================================================
// VALIDATION & HELPERS
// =========================================================================

fn validate_request_basic(req: &PostMovementRequest) -> Result<(), StockLedgerError> {
    if req.quantity_delta_milli == 0 {
        return Err(StockLedgerError::ZeroQuantityDelta);
    }
    if req.location_id.trim().is_empty() {
        return Err(StockLedgerError::MissingLocation);
    }
    if req.reason == MovementReason::OpeningBalance && req.quantity_delta_milli < 0 {
        return Err(StockLedgerError::NegativeStock(
            "opening_balance cannot have negative delta".to_string(),
        ));
    }
    if (req.reason == MovementReason::Damage || req.reason == MovementReason::Loss)
        && req.quantity_delta_milli > 0
    {
        return Err(StockLedgerError::Database(
            "damage and loss movements must have negative deltas".to_string(),
        ));
    }
    Ok(())
}

fn check_idempotency(
    conn: &Connection,
    trimmed_key: &str,
    canonical_hash: &str,
) -> Result<Option<StockMovementResult>, StockLedgerError> {
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT result_json, request_hash FROM idempotency_keys WHERE key = ?1",
            params![trimmed_key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;

    if let Some((result_json, stored_hash)) = row {
        let hash_matches = match stored_hash {
            Some(ref h) => h == canonical_hash,
            None => false,
        };

        if hash_matches {
            let res: StockMovementResult = serde_json::from_str(&result_json).map_err(|e| {
                StockLedgerError::Database(format!("Failed to deserialize cached result JSON: {e}"))
            })?;
            return Ok(Some(res));
        } else {
            return Err(StockLedgerError::IdempotencyConflict(format!(
                "Idempotency key '{trimmed_key}' already used with different request parameters"
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
    let loc_info: Option<(String, i64)> = conn
        .query_row(
            "SELECT branch_id, is_active FROM locations WHERE id = ?1",
            params![location_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let Some((loc_branch, loc_active)) = loc_info else {
        return Err(StockLedgerError::LocationNotFound(location_id.to_string()));
    };

    if loc_branch != branch_id {
        return Err(StockLedgerError::LocationBranchMismatch(format!(
            "Location '{location_id}' belongs to branch '{loc_branch}', not '{branch_id}'"
        )));
    }

    if loc_active != 1 {
        return Err(StockLedgerError::LocationNotFound(format!(
            "Location '{location_id}' is inactive"
        )));
    }

    if let Some(b_id) = bin_id {
        let bin_info: Option<(String, i64)> = conn
            .query_row(
                "SELECT location_id, is_active FROM bins WHERE id = ?1",
                params![b_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let Some((bin_loc, bin_active)) = bin_info else {
            return Err(StockLedgerError::BinNotFound(b_id.to_string()));
        };

        if bin_loc != location_id {
            return Err(StockLedgerError::BinLocationMismatch(format!(
                "Bin '{b_id}' belongs to location '{bin_loc}', not '{location_id}'"
            )));
        }

        if bin_active != 1 {
            return Err(StockLedgerError::BinNotFound(format!(
                "Bin '{b_id}' is inactive"
            )));
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

    let Some(active) = prod_active else {
        return Err(StockLedgerError::ProductNotFound(product_id.to_string()));
    };

    if active != 1 {
        return Err(StockLedgerError::ProductNotFound(format!(
            "Product '{product_id}' is inactive"
        )));
    }

    if let Some(v_id) = variant_id {
        let var_info: Option<(String, i64, Option<String>)> = conn
            .query_row(
                "SELECT product_id, is_active, deleted_at FROM product_variants WHERE id = ?1",
                params![v_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let Some((parent_prod, var_active, deleted_at)) = var_info else {
            return Err(StockLedgerError::VariantNotFound(v_id.to_string()));
        };

        if parent_prod != product_id {
            return Err(StockLedgerError::VariantProductMismatch(format!(
                "Variant '{v_id}' belongs to product '{parent_prod}', not '{product_id}'"
            )));
        }

        if var_active != 1 || deleted_at.is_some() {
            return Err(StockLedgerError::VariantInactive(format!(
                "Variant '{v_id}' is inactive or deleted"
            )));
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
) -> Result<Option<i64>, StockLedgerError> {
    let Some(b_id) = batch_id else {
        return Ok(None);
    };

    let batch_info: Option<(String, String, Option<String>, i64)> = conn
        .query_row(
            "SELECT product_id, branch_id, variant_id, quantity_milli FROM product_batches WHERE id = ?1",
            params![b_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;

    let Some((b_prod, b_branch, b_var, b_qty)) = batch_info else {
        return Err(StockLedgerError::BatchNotFound(b_id.to_string()));
    };

    if b_prod != product_id {
        return Err(StockLedgerError::BatchProductMismatch(format!(
            "Batch '{b_id}' belongs to product '{b_prod}', not '{product_id}'"
        )));
    }

    if b_branch != branch_id {
        return Err(StockLedgerError::BatchBranchMismatch(format!(
            "Batch '{b_id}' belongs to branch '{b_branch}', not '{branch_id}'"
        )));
    }

    if b_var.as_deref() != variant_id {
        return Err(StockLedgerError::BatchVariantMismatch(format!(
            "Batch '{b_id}' belongs to variant '{:?}', not '{:?}'",
            b_var, variant_id
        )));
    }

    let resulting_qty = b_qty.checked_add(delta).ok_or_else(|| {
        StockLedgerError::BatchArithmeticOverflow(format!("Batch '{b_id}' overflow"))
    })?;

    if resulting_qty < 0 {
        return Err(StockLedgerError::BatchInsufficientStock(format!(
            "Batch '{b_id}' balance {b_qty} is insufficient for deduction {delta}"
        )));
    }

    Ok(Some(b_qty))
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

fn fetch_serial_record(
    conn: &Connection,
    serial_id: &str,
) -> Result<SerialRecord, StockLedgerError> {
    let rec: Option<SerialRecord> = conn
        .query_row(
            "SELECT id, product_id, branch_id, variant_id, status, location_id, bin_id
             FROM serial_numbers WHERE id = ?1",
            params![serial_id],
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
        .optional()?;

    rec.ok_or_else(|| StockLedgerError::SerialNotFound(serial_id.to_string()))
}

fn validate_serial_identity(
    record: &SerialRecord,
    product_id: &str,
    branch_id: &str,
    variant_id: Option<&str>,
) -> Result<(), StockLedgerError> {
    if record.product_id != product_id {
        return Err(StockLedgerError::SerialProductMismatch(format!(
            "Serial '{}' belongs to product '{}', not '{product_id}'",
            record.id, record.product_id
        )));
    }
    if record.branch_id != branch_id {
        return Err(StockLedgerError::SerialBranchMismatch(format!(
            "Serial '{}' belongs to branch '{}', not '{branch_id}'",
            record.id, record.branch_id
        )));
    }
    if record.variant_id.as_deref() != variant_id {
        return Err(StockLedgerError::SerialVariantMismatch(format!(
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
                return Err(StockLedgerError::SerialCoordinateMismatch(format!(
                    "Serial '{s_id}' is physically located at location '{loc}', not '{location_id}'"
                )));
            }
            None => {
                return Err(StockLedgerError::SerialCoordinateMismatch(
                    "Serial has no physical location assigned".to_string(),
                ));
            }
            _ => {}
        }
        if record.bin_id.as_deref() != bin_id {
            return Err(StockLedgerError::SerialCoordinateMismatch(format!(
                "Serial '{s_id}' is physically located at bin '{:?}', not '{:?}'",
                record.bin_id, bin_id
            )));
        }
    }
    Ok(())
}

fn validate_serial_asset(
    conn: &Connection,
    req: &PostMovementRequest,
) -> Result<(), StockLedgerError> {
    let Some(s_id) = req.normalized_serial_id()? else {
        return Ok(());
    };
    if req.quantity_delta_milli != 1000 && req.quantity_delta_milli != -1000 {
        return Err(StockLedgerError::SerialInvalidQuantity(format!(
            "Serialized movement must have quantity_delta_milli of 1000 or -1000, got {}",
            req.quantity_delta_milli
        )));
    }
    let record = fetch_serial_record(conn, s_id)?;
    let var_id = req.normalized_variant_id()?;
    validate_serial_identity(&record, req.product_id.trim(), req.branch_id.trim(), var_id)?;
    let bin_id = req.normalized_bin_id()?;
    validate_serial_status_and_coordinates(
        &record,
        req.location_id.trim(),
        bin_id,
        req.quantity_delta_milli,
    )?;
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

    let agg_after = agg_before + delta;
    if agg_after < 0 {
        return Err(StockLedgerError::NegativeStock(format!(
            "Aggregate inventory would become {agg_after} (before: {agg_before}, delta: {delta})"
        )));
    }

    if let Some(id) = agg_id {
        conn.execute(
            "UPDATE inventory SET quantity_milli = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![agg_after, id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO inventory (branch_id, product_id, variant_id, quantity_milli, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![branch_id, product_id, variant_id, agg_after],
        )?;
    }

    Ok((agg_before, agg_after))
}

fn mutate_spatial_inventory(
    conn: &Connection,
    req: &PostMovementRequest,
) -> Result<(), StockLedgerError> {
    let branch_id = req.branch_id.trim();
    let location_id = req.location_id.trim();
    let bin_id = req.normalized_bin_id()?;
    let product_id = req.product_id.trim();
    let variant_id = req.normalized_variant_id()?;
    let batch_id = req.normalized_batch_id()?;
    let delta = req.quantity_delta_milli;

    let query = "SELECT id, quantity_milli FROM location_inventory
         WHERE branch_id = ?1
           AND location_id = ?2
           AND bin_id IS ?3
           AND product_id = ?4
           AND variant_id IS ?5
           AND batch_id IS ?6";

    let slot_info: Option<(String, i64)> = conn
        .query_row(
            query,
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

    let slot_after = slot_before + delta;
    if slot_after < 0 {
        return Err(StockLedgerError::NegativeStock(format!(
            "Spatial slot inventory would become {slot_after} (before: {slot_before}, delta: {delta})"
        )));
    }

    if let Some(id) = slot_id {
        conn.execute(
            "UPDATE location_inventory SET quantity_milli = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![slot_after, id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO location_inventory (
                branch_id, location_id, bin_id, product_id, variant_id, batch_id,
                quantity_milli, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))",
            params![
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

    Ok(())
}

fn mutate_batch_inventory(
    conn: &Connection,
    batch_id: Option<&str>,
    delta: i64,
    current_qty: Option<i64>,
) -> Result<(), StockLedgerError> {
    if let (Some(b_id), Some(b_qty)) = (batch_id, current_qty) {
        let new_qty = b_qty + delta;
        let new_status = if new_qty == 0 { "depleted" } else { "active" };

        conn.execute(
            "UPDATE product_batches
             SET quantity_milli = ?1, status = ?2, updated_at = datetime('now')
             WHERE id = ?3",
            params![new_qty, new_status, b_id],
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
                "UPDATE serial_numbers SET status = 'in_stock', location_id = ?1, bin_id = ?2, updated_at = datetime('now') WHERE id = ?3",
                params![location_id, bin_id, s_id],
            )?;
        } else {
            let new_status = match reason {
                MovementReason::Damage => "defective",
                MovementReason::Adjustment => "reserved",
                _ => "disposed",
            };
            conn.execute(
                "UPDATE serial_numbers SET status = ?1, location_id = NULL, bin_id = NULL, updated_at = datetime('now') WHERE id = ?2",
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
    let user_id = req.normalized_user_id()?;
    let bin_id = req.normalized_bin_id()?;
    let var_id = req.normalized_variant_id()?;
    let batch_id = req.normalized_batch_id()?;
    let serial_id = req.normalized_serial_id()?;

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

        // 2. Begin atomic transaction
        let tx = conn.transaction()?;

        // 3. Idempotency check inside atomic transaction
        if let Some(cached) = check_idempotency(&tx, trimmed_key, &canonical_hash)? {
            return Ok(cached);
        }

        let branch_id = req.branch_id.trim();
        let product_id = req.product_id.trim();
        let location_id = req.location_id.trim();
        let variant_id = req.normalized_variant_id()?;
        let bin_id = req.normalized_bin_id()?;
        let batch_id = req.normalized_batch_id()?;
        let serial_id = req.normalized_serial_id()?;

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
        validate_serial_asset(&tx, req)?;

        // 5. Mutate balances atomically
        let (agg_before, agg_after) = mutate_aggregate_inventory(
            &tx,
            branch_id,
            product_id,
            variant_id,
            req.quantity_delta_milli,
        )?;

        mutate_spatial_inventory(&tx, req)?;

        mutate_batch_inventory(&tx, batch_id, req.quantity_delta_milli, current_batch_qty)?;

        mutate_serial_inventory(
            &tx,
            serial_id,
            location_id,
            bin_id,
            req.quantity_delta_milli,
            req.reason,
        )?;

        // 6. Append immutable audit ledger record
        let (movement_id, created_at) = append_movement_record(&tx, req, agg_before, agg_after)?;

        let result = StockMovementResult {
            movement_id,
            idempotency_key: trimmed_key.to_string(),
            branch_id: branch_id.to_string(),
            product_id: product_id.to_string(),
            variant_id: variant_id.map(str::to_string),
            location_id: location_id.to_string(),
            bin_id: bin_id.map(str::to_string),
            batch_id: batch_id.map(str::to_string),
            serial_id: serial_id.map(str::to_string),
            quantity_delta_milli: req.quantity_delta_milli,
            quantity_before_milli: agg_before,
            quantity_after_milli: agg_after,
            reason: req.reason,
            created_at,
        };

        // 7. Persist idempotency result inside the same transaction
        persist_idempotency(&tx, trimmed_key, &result, &canonical_hash)?;

        // 8. Commit atomic transaction
        tx.commit()?;

        Ok(result)
    }

    /// Retrieves aggregate, spatial allocated, and unallocated stock summary.
    pub fn get_stock_summary(
        conn: &Connection,
        branch_id: &str,
        product_id: &str,
        variant_id: Option<&str>,
    ) -> Result<StockSummaryRecord, StockLedgerError> {
        let agg_qty: i64 = conn
            .query_row(
                "SELECT COALESCE(quantity_milli, 0) FROM inventory
                 WHERE branch_id = ?1 AND product_id = ?2 AND variant_id IS ?3",
                params![branch_id, product_id, variant_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let allocated_qty: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(quantity_milli), 0) FROM location_inventory
                 WHERE branch_id = ?1 AND product_id = ?2 AND variant_id IS ?3",
                params![branch_id, product_id, variant_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let unallocated_qty = agg_qty - allocated_qty;

        Ok(StockSummaryRecord {
            branch_id: branch_id.to_string(),
            product_id: product_id.to_string(),
            variant_id: variant_id.map(str::to_string),
            aggregate_quantity_milli: agg_qty,
            allocated_spatial_milli: allocated_qty,
            unallocated_quantity_milli: unallocated_qty,
        })
    }

    /// Retrieves all spatial slot inventory rows for a product within a branch.
    pub fn get_product_spatial_balances(
        conn: &Connection,
        branch_id: &str,
        product_id: &str,
    ) -> Result<Vec<LocationInventoryRecord>, StockLedgerError> {
        let mut stmt = conn.prepare_cached(
            "SELECT id, branch_id, location_id, bin_id, product_id, variant_id, batch_id,
                    quantity_milli, created_at, updated_at
             FROM location_inventory
             WHERE branch_id = ?1 AND product_id = ?2
             ORDER BY location_id ASC, bin_id ASC",
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
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut records = Vec::new();
        for r in rows {
            records.push(r?);
        }

        Ok(records)
    }

    /// Retrieves total batch lot quantity, spatial allocated quantity, and unallocated batch quantity.
    pub fn get_batch_summary(
        conn: &Connection,
        branch_id: &str,
        batch_id: &str,
    ) -> Result<BatchSummaryRecord, StockLedgerError> {
        let batch_row: Option<(String, Option<String>, Option<String>, String, i64)> = conn
            .query_row(
                "SELECT product_id, variant_id, batch_number, status, quantity_milli
                 FROM product_batches
                 WHERE id = ?1 AND branch_id = ?2",
                params![batch_id, branch_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;

        let Some((prod_id, var_id, batch_num, status, total_qty)) = batch_row else {
            return Err(StockLedgerError::BatchNotFound(batch_id.to_string()));
        };

        let spatial_qty: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(quantity_milli), 0) FROM location_inventory
                 WHERE branch_id = ?1 AND batch_id = ?2",
                params![branch_id, batch_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let unallocated = total_qty - spatial_qty;

        Ok(BatchSummaryRecord {
            batch_id: batch_id.to_string(),
            branch_id: branch_id.to_string(),
            product_id: prod_id,
            variant_id: var_id,
            batch_number: batch_num,
            status,
            total_quantity_milli: total_qty,
            spatial_quantity_milli: spatial_qty,
            unallocated_quantity_milli: unallocated,
        })
    }
}
