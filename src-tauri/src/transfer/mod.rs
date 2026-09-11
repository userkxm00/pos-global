// F2.12 — Transfers Architecture & Domain Service
// ADR-0014: Authoritative transfer document management, dual transfer topologies (intra-branch / inter-branch),
// atomic spatial relocations, two-stage inter-branch lifecycle, tracked unit continuity, and idempotency.

use crate::stock_ledger::{
    PostMovementInput, StockLedgerError, StockLedgerService, StockMovementReason,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

// =========================================================================
// DOMAIN ENUMS
// =========================================================================

/// Topographical classification of the inventory transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferType {
    /// Spatial relocation within the same branch (source_branch == destination_branch).
    IntraBranch,
    /// Logistics transfer across distinct physical branches (source_branch != destination_branch).
    InterBranch,
}

impl TransferType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransferType::IntraBranch => "intra_branch",
            TransferType::InterBranch => "inter_branch",
        }
    }
}

impl FromStr for TransferType {
    type Err = TransferError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "intra_branch" => Ok(TransferType::IntraBranch),
            "inter_branch" => Ok(TransferType::InterBranch),
            other => Err(TransferError::Validation(format!(
                "Invalid transfer type '{other}'. Expected 'intra_branch' or 'inter_branch'"
            ))),
        }
    }
}

/// Operational lifecycle state of the transfer document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    /// Document draft; stock has not been deducted or moved.
    Draft,
    /// Inter-branch stock has departed source branch and is in transit.
    InTransit,
    /// Transfer received or completed; stock credited to destination.
    Completed,
    /// Draft transfer cancelled; no inventory balance effect.
    Cancelled,
}

impl TransferStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransferStatus::Draft => "draft",
            TransferStatus::InTransit => "in_transit",
            TransferStatus::Completed => "completed",
            TransferStatus::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, TransferStatus::Completed | TransferStatus::Cancelled)
    }
}

impl FromStr for TransferStatus {
    type Err = TransferError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "draft" => Ok(TransferStatus::Draft),
            "in_transit" => Ok(TransferStatus::InTransit),
            "completed" => Ok(TransferStatus::Completed),
            "cancelled" => Ok(TransferStatus::Cancelled),
            other => Err(TransferError::Validation(format!(
                "Invalid transfer status '{other}'. Expected 'draft', 'in_transit', 'completed', or 'cancelled'"
            ))),
        }
    }
}

// =========================================================================
// DOMAIN MODELS
// =========================================================================

/// Authoritative stock transfer header document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockTransfer {
    pub id: String,
    pub transfer_number: String,
    pub transfer_type: TransferType,
    pub source_branch_id: String,
    pub destination_branch_id: String,
    pub source_location_id: String,
    pub destination_location_id: String,
    pub source_bin_id: Option<String>,
    pub destination_bin_id: Option<String>,
    pub status: TransferStatus,
    pub dispatched_at: Option<String>,
    pub dispatched_by: Option<String>,
    pub received_at: Option<String>,
    pub received_by: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<StockTransferItem>,
}

/// Child line item belonging to a stock transfer document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockTransferItem {
    pub id: String,
    pub transfer_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub quantity_milli: i64,
    pub received_quantity_milli: Option<i64>,
    pub created_at: String,
}

// =========================================================================
// INPUT DTOs
// =========================================================================

/// Specification for a single transfer line item during creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTransferItemInput {
    pub product_id: String,
    pub variant_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub quantity_milli: i64,
}

/// Specification for creating a stock transfer header and line items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTransferInput {
    pub transfer_type: TransferType,
    pub source_branch_id: String,
    pub destination_branch_id: String,
    pub source_location_id: String,
    pub destination_location_id: String,
    pub source_bin_id: Option<String>,
    pub destination_bin_id: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateTransferItemInput>,
    pub user_id: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Specification for dispatching a draft inter-branch transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchTransferInput {
    pub transfer_id: String,
    pub user_id: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Specification for receiving an in-transit inter-branch transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiveTransferInput {
    pub transfer_id: String,
    pub destination_location_id: Option<String>,
    pub destination_bin_id: Option<String>,
    pub user_id: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Specification for cancelling a draft transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelTransferInput {
    pub transfer_id: String,
    pub user_id: Option<String>,
}

/// Specification for an immediate single-transaction intra-branch relocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstantIntraBranchTransferInput {
    pub branch_id: String,
    pub source_location_id: String,
    pub destination_location_id: String,
    pub source_bin_id: Option<String>,
    pub destination_bin_id: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateTransferItemInput>,
    pub user_id: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Filter criteria for querying transfer documents.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferFilter {
    pub branch_id: Option<String>,
    pub source_branch_id: Option<String>,
    pub destination_branch_id: Option<String>,
    pub transfer_type: Option<TransferType>,
    pub status: Option<TransferStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// =========================================================================
// DOMAIN ERRORS
// =========================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum TransferError {
    Validation(String),
    NotFound(String),
    TopologyMismatch(String),
    NoOpRelocation(String),
    InvalidLocation(String),
    InvalidBin(String),
    BranchMismatch(String),
    VariantMismatch(String),
    InsufficientStock {
        product_id: String,
        requested_milli: i64,
        available_milli: i64,
    },
    InvalidStatusTransition {
        current: String,
        attempted: String,
        reason: String,
    },
    InvalidBatch(String),
    InvalidBatchStatus(String),
    InvalidSerial(String),
    InvalidSerialStatus(String),
    IdempotencyConflict(String),
    Unauthorized(String),
    Database(String),
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferError::Validation(msg) => write!(f, "Validation error: {msg}"),
            TransferError::NotFound(msg) => write!(f, "Not found: {msg}"),
            TransferError::TopologyMismatch(msg) => write!(f, "Topology mismatch: {msg}"),
            TransferError::NoOpRelocation(msg) => write!(f, "No-op relocation error: {msg}"),
            TransferError::InvalidLocation(msg) => write!(f, "Invalid location: {msg}"),
            TransferError::InvalidBin(msg) => write!(f, "Invalid bin: {msg}"),
            TransferError::BranchMismatch(msg) => write!(f, "Branch mismatch: {msg}"),
            TransferError::VariantMismatch(msg) => write!(f, "Variant mismatch: {msg}"),
            TransferError::InsufficientStock {
                product_id,
                requested_milli,
                available_milli,
            } => write!(
                f,
                "Insufficient stock for product '{product_id}': requested {requested_milli} milli, available {available_milli} milli"
            ),
            TransferError::InvalidStatusTransition {
                current,
                attempted,
                reason,
            } => write!(
                f,
                "Cannot transition transfer from '{current}' to '{attempted}': {reason}"
            ),
            TransferError::InvalidBatch(msg) => write!(f, "Invalid batch: {msg}"),
            TransferError::InvalidBatchStatus(msg) => write!(f, "Invalid batch status: {msg}"),
            TransferError::InvalidSerial(msg) => write!(f, "Invalid serial: {msg}"),
            TransferError::InvalidSerialStatus(msg) => write!(f, "Invalid serial status: {msg}"),
            TransferError::IdempotencyConflict(msg) => write!(f, "Idempotency conflict: {msg}"),
            TransferError::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            TransferError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for TransferError {}

impl From<rusqlite::Error> for TransferError {
    fn from(err: rusqlite::Error) -> Self {
        TransferError::Database(err.to_string())
    }
}

impl From<serde_json::Error> for TransferError {
    fn from(err: serde_json::Error) -> Self {
        TransferError::Database(format!("JSON serialization error: {err}"))
    }
}

impl From<StockLedgerError> for TransferError {
    fn from(err: StockLedgerError) -> Self {
        match err {
            StockLedgerError::InsufficientStock {
                requested_milli,
                available_milli,
            } => TransferError::InsufficientStock {
                product_id: String::new(),
                requested_milli,
                available_milli,
            },
            StockLedgerError::BranchMismatch(msg) => TransferError::BranchMismatch(msg),
            StockLedgerError::VariantMismatch(msg) => TransferError::VariantMismatch(msg),
            StockLedgerError::InvalidLocation(msg) => TransferError::InvalidLocation(msg),
            StockLedgerError::InvalidBin(msg) => TransferError::InvalidBin(msg),
            StockLedgerError::InvalidBatch(msg) => TransferError::InvalidBatch(msg),
            StockLedgerError::InvalidSerial(msg) => TransferError::InvalidSerial(msg),
            StockLedgerError::IdempotencyConflict(msg) => TransferError::IdempotencyConflict(msg),
            StockLedgerError::Validation(msg) => TransferError::Validation(msg),
            StockLedgerError::NotFound(msg) => TransferError::NotFound(msg),
            StockLedgerError::Database(msg) => TransferError::Database(msg),
            other => TransferError::Database(other.to_string()),
        }
    }
}

// =========================================================================
// CANONICAL REQUEST HASHING & IDEMPOTENCY
// =========================================================================

pub fn compute_canonical_hash<T: Serialize>(payload: &T) -> String {
    let canonical_bytes =
        serde_json::to_vec(payload).expect("canonical request serialization is infallible");
    let mut hasher = Sha256::new();
    hasher.update(&canonical_bytes);
    format!("{:x}", hasher.finalize())
}

fn check_idempotency(
    tx: &Transaction<'_>,
    operation: &str,
    clean_key: &str,
    request_hash: &str,
) -> Result<Option<StockTransfer>, TransferError> {
    let existing: Option<(Option<String>, Option<String>)> = tx
        .query_row(
            "SELECT request_hash, result_json FROM idempotency_keys WHERE key = ?1 AND operation = ?2",
            params![clean_key, operation],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let Some((stored_hash, result_json)) = existing else {
        return Ok(None);
    };

    if stored_hash.as_deref() != Some(request_hash) {
        return Err(TransferError::IdempotencyConflict(format!(
            "Idempotency key '{clean_key}' already used with different request parameters for operation '{operation}'"
        )));
    }

    if let Some(json) = result_json {
        let cached_transfer: StockTransfer = serde_json::from_str(&json)?;
        Ok(Some(cached_transfer))
    } else {
        Ok(None)
    }
}

fn record_idempotency(
    tx: &Transaction<'_>,
    operation: &str,
    clean_key: &str,
    request_hash: &str,
    transfer: &StockTransfer,
) -> Result<(), TransferError> {
    let result_json = serde_json::to_string(transfer)?;
    tx.execute(
        "INSERT INTO idempotency_keys (key, operation, request_hash, result_json, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![clean_key, operation, request_hash, result_json],
    )?;
    Ok(())
}

// =========================================================================
// VALIDATION HELPERS
// =========================================================================

fn validate_topology(
    transfer_type: TransferType,
    source_branch_id: &str,
    destination_branch_id: &str,
) -> Result<(), TransferError> {
    if source_branch_id.trim().is_empty() {
        return Err(TransferError::Validation(
            "source_branch_id cannot be empty".into(),
        ));
    }
    if destination_branch_id.trim().is_empty() {
        return Err(TransferError::Validation(
            "destination_branch_id cannot be empty".into(),
        ));
    }

    match transfer_type {
        TransferType::IntraBranch => {
            if source_branch_id != destination_branch_id {
                return Err(TransferError::TopologyMismatch(format!(
                    "intra_branch transfer requires source_branch_id == destination_branch_id ('{source_branch_id}' != '{destination_branch_id}')"
                )));
            }
        }
        TransferType::InterBranch => {
            if source_branch_id == destination_branch_id {
                return Err(TransferError::TopologyMismatch(format!(
                    "inter_branch transfer requires source_branch_id != destination_branch_id (both '{source_branch_id}')"
                )));
            }
        }
    }
    Ok(())
}

fn validate_location_and_bin_ownership(
    tx: &Transaction<'_>,
    branch_id: &str,
    location_id: &str,
    bin_id: Option<&str>,
    label: &str,
) -> Result<(), TransferError> {
    let (loc_branch, loc_active): (String, i64) = tx
        .query_row(
            "SELECT branch_id, is_active FROM locations WHERE id = ?1",
            params![location_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| {
            TransferError::NotFound(format!("{label} location '{location_id}' not found"))
        })?;

    if loc_branch != branch_id {
        return Err(TransferError::BranchMismatch(format!(
            "{label} location '{location_id}' belongs to branch '{loc_branch}', not '{branch_id}'"
        )));
    }
    if loc_active == 0 {
        return Err(TransferError::InvalidLocation(format!(
            "{label} location '{location_id}' is inactive"
        )));
    }

    if let Some(b_id) = bin_id {
        let (bin_loc, bin_active): (String, i64) = tx
            .query_row(
                "SELECT location_id, is_active FROM bins WHERE id = ?1",
                params![b_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| TransferError::NotFound(format!("{label} bin '{b_id}' not found")))?;

        if bin_loc != location_id {
            return Err(TransferError::InvalidBin(format!(
                "{label} bin '{b_id}' belongs to location '{bin_loc}', not '{location_id}'"
            )));
        }
        if bin_active == 0 {
            return Err(TransferError::InvalidBin(format!(
                "{label} bin '{b_id}' is inactive"
            )));
        }
    }

    Ok(())
}

fn validate_no_op_relocation(
    transfer_type: TransferType,
    source_location_id: &str,
    destination_location_id: &str,
    source_bin_id: Option<&str>,
    destination_bin_id: Option<&str>,
) -> Result<(), TransferError> {
    if transfer_type == TransferType::IntraBranch && source_location_id == destination_location_id {
        let same_bin = match (source_bin_id, destination_bin_id) {
            (None, None) => true,
            (Some(sb), Some(db)) => sb == db,
            _ => false,
        };
        if same_bin {
            return Err(TransferError::NoOpRelocation(
                "Cannot perform intra-branch relocation to the exact same location and bin".into(),
            ));
        }
    }
    Ok(())
}

fn validate_item_consistency(
    tx: &Transaction<'_>,
    source_branch_id: &str,
    item: &CreateTransferItemInput,
    index: usize,
) -> Result<(), TransferError> {
    if item.quantity_milli <= 0 {
        return Err(TransferError::Validation(format!(
            "Item at index {index} has non-positive quantity_milli ({})",
            item.quantity_milli
        )));
    }

    if item.batch_id.is_some() && item.serial_id.is_some() {
        return Err(TransferError::Validation(format!(
            "Item at index {index} specifies both batch_id and serial_id"
        )));
    }

    if item.serial_id.is_some() && item.quantity_milli != 1000 {
        return Err(TransferError::Validation(format!(
            "Serialized item at index {index} must have quantity_milli exactly 1000 (1 unit), got {}",
            item.quantity_milli
        )));
    }

    // Product check
    let product_active: i64 = tx
        .query_row(
            "SELECT is_active FROM products WHERE id = ?1",
            params![item.product_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| {
            TransferError::NotFound(format!("Product '{}' not found", item.product_id))
        })?;

    if product_active == 0 {
        return Err(TransferError::Validation(format!(
            "Product '{}' is inactive",
            item.product_id
        )));
    }

    // Variant check
    if let Some(ref vid) = item.variant_id {
        let (v_prod, v_active, v_deleted): (String, i64, Option<String>) = tx
            .query_row(
                "SELECT product_id, is_active, deleted_at FROM product_variants WHERE id = ?1",
                params![vid],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| TransferError::NotFound(format!("Variant '{vid}' not found")))?;

        if v_prod != item.product_id {
            return Err(TransferError::VariantMismatch(format!(
                "Variant '{vid}' belongs to product '{v_prod}', not requested '{}'",
                item.product_id
            )));
        }
        if v_active == 0 || v_deleted.is_some() {
            return Err(TransferError::Validation(format!(
                "Variant '{vid}' is inactive or soft-deleted"
            )));
        }
    }

    // Batch check
    if let Some(ref bid) = item.batch_id {
        let (b_prod, b_branch, b_var, b_status): (String, String, Option<String>, String) = tx
            .query_row(
                "SELECT product_id, branch_id, variant_id, status FROM product_batches WHERE id = ?1",
                params![bid],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?
            .ok_or_else(|| TransferError::NotFound(format!("Batch '{bid}' not found")))?;

        if b_prod != item.product_id {
            return Err(TransferError::InvalidBatch(format!(
                "Batch '{bid}' belongs to product '{b_prod}', not requested '{}'",
                item.product_id
            )));
        }
        if b_branch != source_branch_id {
            return Err(TransferError::BranchMismatch(format!(
                "Batch '{bid}' belongs to branch '{b_branch}', not source branch '{source_branch_id}'"
            )));
        }
        if b_var != item.variant_id {
            return Err(TransferError::VariantMismatch(format!(
                "Batch '{bid}' variant '{:?}' does not match item variant '{:?}'",
                b_var, item.variant_id
            )));
        }
        if b_status == "depleted" {
            return Err(TransferError::InvalidBatchStatus(format!(
                "Batch '{bid}' is depleted and cannot be transferred"
            )));
        }
    }

    // Serial check
    if let Some(ref sid) = item.serial_id {
        let (s_prod, s_branch, s_var, s_status): (String, String, Option<String>, String) = tx
            .query_row(
                "SELECT product_id, branch_id, variant_id, status FROM serial_numbers WHERE id = ?1",
                params![sid],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?
            .ok_or_else(|| TransferError::NotFound(format!("Serial '{sid}' not found")))?;

        if s_prod != item.product_id {
            return Err(TransferError::InvalidSerial(format!(
                "Serial '{sid}' belongs to product '{s_prod}', not requested '{}'",
                item.product_id
            )));
        }
        if s_branch != source_branch_id {
            return Err(TransferError::BranchMismatch(format!(
                "Serial '{sid}' belongs to branch '{s_branch}', not source branch '{source_branch_id}'"
            )));
        }
        if s_var != item.variant_id {
            return Err(TransferError::VariantMismatch(format!(
                "Serial '{sid}' variant '{:?}' does not match item variant '{:?}'",
                s_var, item.variant_id
            )));
        }
        if s_status != "in_stock" {
            return Err(TransferError::InvalidSerialStatus(format!(
                "Serial '{sid}' has status '{s_status}'. Only 'in_stock' serials can be transferred"
            )));
        }
    }

    Ok(())
}

fn generate_transfer_number(tx: &Transaction<'_>, prefix: &str) -> Result<String, TransferError> {
    let rand_bytes: String = tx.query_row("SELECT lower(hex(randomblob(4)))", [], |r| r.get(0))?;
    let timestamp: String =
        tx.query_row("SELECT strftime('%Y%m%d%H%M%S', 'now')", [], |r| r.get(0))?;
    Ok(format!(
        "{prefix}-{timestamp}-{}",
        rand_bytes.to_ascii_uppercase()
    ))
}

fn load_transfer_items(
    conn: &Connection,
    transfer_id: &str,
) -> Result<Vec<StockTransferItem>, TransferError> {
    let mut stmt = conn.prepare(
        "SELECT id, transfer_id, product_id, variant_id, batch_id, serial_id,
                quantity_milli, received_quantity_milli, created_at
         FROM stock_transfer_items
         WHERE transfer_id = ?1
         ORDER BY created_at ASC",
    )?;

    let rows = stmt.query_map(params![transfer_id], |row| {
        Ok(StockTransferItem {
            id: row.get(0)?,
            transfer_id: row.get(1)?,
            product_id: row.get(2)?,
            variant_id: row.get(3)?,
            batch_id: row.get(4)?,
            serial_id: row.get(5)?,
            quantity_milli: row.get(6)?,
            received_quantity_milli: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;

    let mut items = Vec::new();
    for r in rows {
        items.push(r?);
    }
    Ok(items)
}

fn map_transfer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(StockTransfer, String)> {
    let id: String = row.get(0)?;
    let type_str: String = row.get(2)?;
    let status_str: String = row.get(9)?;

    let transfer_type = TransferType::from_str(&type_str).unwrap_or(TransferType::IntraBranch);
    let status = TransferStatus::from_str(&status_str).unwrap_or(TransferStatus::Draft);

    let transfer = StockTransfer {
        id: id.clone(),
        transfer_number: row.get(1)?,
        transfer_type,
        source_branch_id: row.get(3)?,
        destination_branch_id: row.get(4)?,
        source_location_id: row.get(5)?,
        destination_location_id: row.get(6)?,
        source_bin_id: row.get(7)?,
        destination_bin_id: row.get(8)?,
        status,
        dispatched_at: row.get(10)?,
        dispatched_by: row.get(11)?,
        received_at: row.get(12)?,
        received_by: row.get(13)?,
        notes: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        items: Vec::new(),
    };

    Ok((transfer, id))
}

// =========================================================================
// BATCH RESOLUTION / CONTINUITY ON RECEIPT
// =========================================================================

fn resolve_destination_batch(
    tx: &Transaction<'_>,
    source_batch_id: &str,
    destination_branch_id: &str,
) -> Result<String, TransferError> {
    let (src_prod, src_var, src_num, src_exp, src_cost, src_mfg): (
        String,
        Option<String>,
        String,
        String,
        Option<i64>,
        Option<String>,
    ) = tx
        .query_row(
            "SELECT product_id, variant_id, batch_number, expiry_date, cost_price_minor, manufactured_date
             FROM product_batches WHERE id = ?1",
            params![source_batch_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .optional()?
        .ok_or_else(|| TransferError::NotFound(format!("Source batch '{source_batch_id}' not found")))?;

    let existing: Option<(String, String, String)> = tx
        .query_row(
            "SELECT id, status, expiry_date FROM product_batches
             WHERE branch_id = ?1 AND product_id = ?2
               AND (variant_id = ?3 OR (?3 IS NULL AND variant_id IS NULL))
               AND batch_number = ?4 COLLATE NOCASE",
            params![destination_branch_id, src_prod, src_var, src_num],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    if let Some((dst_id, dst_status, dst_exp)) = existing {
        if matches!(dst_status.as_str(), "depleted" | "recalled" | "quarantined") {
            return Err(TransferError::InvalidBatchStatus(format!(
                "Destination batch '{dst_id}' is in terminal/quarantine status '{dst_status}' and cannot receive intake"
            )));
        }
        if dst_exp != src_exp {
            return Err(TransferError::InvalidBatch(format!(
                "Destination batch '{dst_id}' expiry date '{dst_exp}' does not match incoming batch expiry '{src_exp}'"
            )));
        }
        Ok(dst_id)
    } else {
        let new_batch_id: String =
            tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;

        tx.execute(
            "INSERT INTO product_batches (
                id, branch_id, product_id, variant_id, batch_number,
                cost_price_minor, manufactured_date, expiry_date,
                quantity_milli, status, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8,
                0, 'active', datetime('now'), datetime('now')
             )",
            params![
                new_batch_id,
                destination_branch_id,
                src_prod,
                src_var,
                src_num,
                src_cost,
                src_mfg,
                src_exp
            ],
        )?;

        Ok(new_batch_id)
    }
}

// =========================================================================
// AUTHORITATIVE DOMAIN SERVICE IMPLEMENTATION
// =========================================================================

pub struct TransferService;

impl TransferService {
    /// Creates a new stock transfer document in 'draft' status.
    pub fn create_transfer(
        conn: &mut Connection,
        input: &CreateTransferInput,
    ) -> Result<StockTransfer, TransferError> {
        validate_topology(
            input.transfer_type,
            &input.source_branch_id,
            &input.destination_branch_id,
        )?;

        if input.items.is_empty() {
            return Err(TransferError::Validation(
                "Transfer must contain at least one line item".into(),
            ));
        }

        validate_no_op_relocation(
            input.transfer_type,
            &input.source_location_id,
            &input.destination_location_id,
            input.source_bin_id.as_deref(),
            input.destination_bin_id.as_deref(),
        )?;

        let request_hash = compute_canonical_hash(input);

        let tx = conn.transaction()?;

        // Idempotency check
        if let Some(ref key) = input.idempotency_key {
            let clean_key = key.trim();
            if !clean_key.is_empty() {
                if let Some(cached) =
                    check_idempotency(&tx, "create_stock_transfer", clean_key, &request_hash)?
                {
                    return Ok(cached);
                }
            }
        }

        // Validate locations and bins
        validate_location_and_bin_ownership(
            &tx,
            &input.source_branch_id,
            &input.source_location_id,
            input.source_bin_id.as_deref(),
            "Source",
        )?;
        validate_location_and_bin_ownership(
            &tx,
            &input.destination_branch_id,
            &input.destination_location_id,
            input.destination_bin_id.as_deref(),
            "Destination",
        )?;

        // Validate each item and protect against duplicate serial_ids in the same payload
        let mut seen_serials = std::collections::HashSet::new();
        for (idx, item) in input.items.iter().enumerate() {
            if let Some(ref sid) = item.serial_id {
                let trimmed_sid = sid.trim();
                if !seen_serials.insert(trimmed_sid.to_string()) {
                    return Err(TransferError::Validation(format!(
                        "Duplicate serial_id '{trimmed_sid}' in transfer items at index {idx}"
                    )));
                }
            }
            validate_item_consistency(&tx, &input.source_branch_id, item, idx)?;
        }

        let transfer_id: String =
            tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
        let transfer_number = generate_transfer_number(&tx, "TRF")?;

        tx.execute(
            "INSERT INTO stock_transfers (
                id, transfer_number, transfer_type,
                source_branch_id, destination_branch_id,
                source_location_id, destination_location_id,
                source_bin_id, destination_bin_id,
                status, notes, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3,
                ?4, ?5,
                ?6, ?7,
                ?8, ?9,
                'draft', ?10, datetime('now'), datetime('now')
             )",
            params![
                transfer_id,
                transfer_number,
                input.transfer_type.as_str(),
                input.source_branch_id,
                input.destination_branch_id,
                input.source_location_id,
                input.destination_location_id,
                input.source_bin_id,
                input.destination_bin_id,
                input.notes,
            ],
        )?;

        let mut persisted_items = Vec::with_capacity(input.items.len());

        for item in &input.items {
            let item_id: String =
                tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;

            tx.execute(
                "INSERT INTO stock_transfer_items (
                    id, transfer_id, product_id, variant_id, batch_id, serial_id,
                    quantity_milli, received_quantity_milli, created_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, NULL, datetime('now')
                 )",
                params![
                    item_id,
                    transfer_id,
                    item.product_id,
                    item.variant_id,
                    item.batch_id,
                    item.serial_id,
                    item.quantity_milli,
                ],
            )?;

            let item_created_at: String = tx.query_row(
                "SELECT created_at FROM stock_transfer_items WHERE id = ?1",
                params![item_id],
                |r| r.get(0),
            )?;

            persisted_items.push(StockTransferItem {
                id: item_id,
                transfer_id: transfer_id.clone(),
                product_id: item.product_id.clone(),
                variant_id: item.variant_id.clone(),
                batch_id: item.batch_id.clone(),
                serial_id: item.serial_id.clone(),
                quantity_milli: item.quantity_milli,
                received_quantity_milli: None,
                created_at: item_created_at,
            });
        }

        let (created_at, updated_at): (String, String) = tx.query_row(
            "SELECT created_at, updated_at FROM stock_transfers WHERE id = ?1",
            params![transfer_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        let transfer = StockTransfer {
            id: transfer_id,
            transfer_number,
            transfer_type: input.transfer_type,
            source_branch_id: input.source_branch_id.clone(),
            destination_branch_id: input.destination_branch_id.clone(),
            source_location_id: input.source_location_id.clone(),
            destination_location_id: input.destination_location_id.clone(),
            source_bin_id: input.source_bin_id.clone(),
            destination_bin_id: input.destination_bin_id.clone(),
            status: TransferStatus::Draft,
            dispatched_at: None,
            dispatched_by: None,
            received_at: None,
            received_by: None,
            notes: input.notes.clone(),
            created_at,
            updated_at,
            items: persisted_items,
        };

        if let Some(ref key) = input.idempotency_key {
            let clean_key = key.trim();
            if !clean_key.is_empty() {
                record_idempotency(
                    &tx,
                    "create_stock_transfer",
                    clean_key,
                    &request_hash,
                    &transfer,
                )?;
            }
        }

        tx.commit()?;

        Ok(transfer)
    }

    /// Dispatches a draft inter-branch transfer, deducting stock from source branch and transitioning to 'in_transit'.
    pub fn dispatch_transfer(
        conn: &mut Connection,
        input: &DispatchTransferInput,
    ) -> Result<StockTransfer, TransferError> {
        let request_hash = compute_canonical_hash(input);

        let tx = conn.transaction()?;

        // Idempotency check
        if let Some(ref key) = input.idempotency_key {
            let clean_key = key.trim();
            if !clean_key.is_empty() {
                if let Some(cached) =
                    check_idempotency(&tx, "dispatch_stock_transfer", clean_key, &request_hash)?
                {
                    return Ok(cached);
                }
            }
        }

        // Lock & fetch transfer
        let transfer_opt = tx
            .query_row(
                "SELECT id, transfer_number, transfer_type, source_branch_id, destination_branch_id,
                        source_location_id, destination_location_id, source_bin_id, destination_bin_id,
                        status, dispatched_at, dispatched_by, received_at, received_by, notes,
                        created_at, updated_at
                 FROM stock_transfers WHERE id = ?1",
                params![input.transfer_id],
                |row| map_transfer_row(row),
            )
            .optional()?;

        let Some((transfer_head, _)) = transfer_opt else {
            return Err(TransferError::NotFound(format!(
                "Transfer '{}' not found",
                input.transfer_id
            )));
        };

        if transfer_head.transfer_type != TransferType::InterBranch {
            return Err(TransferError::InvalidStatusTransition {
                current: transfer_head.status.as_str().into(),
                attempted: "in_transit".into(),
                reason: "Only inter_branch transfers can be dispatched into in_transit status"
                    .into(),
            });
        }

        if transfer_head.status != TransferStatus::Draft {
            return Err(TransferError::InvalidStatusTransition {
                current: transfer_head.status.as_str().into(),
                attempted: "in_transit".into(),
                reason: "Only draft transfers can be dispatched".into(),
            });
        }

        // Fetch items
        let mut item_stmt = tx.prepare(
            "SELECT id, transfer_id, product_id, variant_id, batch_id, serial_id,
                    quantity_milli, received_quantity_milli, created_at
             FROM stock_transfer_items WHERE transfer_id = ?1 ORDER BY created_at ASC",
        )?;
        let item_rows = item_stmt.query_map(params![input.transfer_id], |row| {
            Ok(StockTransferItem {
                id: row.get(0)?,
                transfer_id: row.get(1)?,
                product_id: row.get(2)?,
                variant_id: row.get(3)?,
                batch_id: row.get(4)?,
                serial_id: row.get(5)?,
                quantity_milli: row.get(6)?,
                received_quantity_milli: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut items = Vec::new();
        for r in item_rows {
            items.push(r?);
        }
        drop(item_stmt);

        if items.is_empty() {
            return Err(TransferError::Validation(
                "Cannot dispatch transfer with no line items".into(),
            ));
        }

        // Execute outbound ledger deductions
        for item in &items {
            let movement_input = PostMovementInput {
                branch_id: transfer_head.source_branch_id.clone(),
                product_id: item.product_id.clone(),
                variant_id: item.variant_id.clone(),
                location_id: transfer_head.source_location_id.clone(),
                bin_id: transfer_head.source_bin_id.clone(),
                batch_id: item.batch_id.clone(),
                serial_id: item.serial_id.clone(),
                quantity_delta_milli: -item.quantity_milli,
                reason: StockMovementReason::Transfer,
                source_type: Some("transfer".into()),
                source_id: Some(transfer_head.id.clone()),
                user_id: input.user_id.clone(),
                idempotency_key: None,
            };

            StockLedgerService::post_movement_tx(&tx, &movement_input)?;
        }

        // Update transfer status
        let updated_rows = tx.execute(
            "UPDATE stock_transfers
             SET status = 'in_transit', dispatched_at = datetime('now'), dispatched_by = ?1, updated_at = datetime('now')
             WHERE id = ?2 AND status = 'draft'",
            params![input.user_id, input.transfer_id],
        )?;

        if updated_rows == 0 {
            return Err(TransferError::InvalidStatusTransition {
                current: "concurrently_modified".into(),
                attempted: "in_transit".into(),
                reason: "Transfer status changed concurrently; dispatch aborted".into(),
            });
        }

        let (dispatched_at, updated_at): (Option<String>, String) = tx.query_row(
            "SELECT dispatched_at, updated_at FROM stock_transfers WHERE id = ?1",
            params![input.transfer_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        let mut updated_transfer = transfer_head;
        updated_transfer.status = TransferStatus::InTransit;
        updated_transfer.dispatched_at = dispatched_at;
        updated_transfer.dispatched_by = input.user_id.clone();
        updated_transfer.updated_at = updated_at;
        updated_transfer.items = items;

        if let Some(ref key) = input.idempotency_key {
            let clean_key = key.trim();
            if !clean_key.is_empty() {
                record_idempotency(
                    &tx,
                    "dispatch_stock_transfer",
                    clean_key,
                    &request_hash,
                    &updated_transfer,
                )?;
            }
        }

        tx.commit()?;

        Ok(updated_transfer)
    }

    /// Receives an in-transit inter-branch transfer, crediting stock at destination and transitioning to 'completed'.
    pub fn receive_transfer(
        conn: &mut Connection,
        input: &ReceiveTransferInput,
    ) -> Result<StockTransfer, TransferError> {
        let request_hash = compute_canonical_hash(input);

        let tx = conn.transaction()?;

        // Idempotency check
        if let Some(ref key) = input.idempotency_key {
            let clean_key = key.trim();
            if !clean_key.is_empty() {
                if let Some(cached) =
                    check_idempotency(&tx, "receive_stock_transfer", clean_key, &request_hash)?
                {
                    return Ok(cached);
                }
            }
        }

        // Lock & fetch transfer
        let transfer_opt = tx
            .query_row(
                "SELECT id, transfer_number, transfer_type, source_branch_id, destination_branch_id,
                        source_location_id, destination_location_id, source_bin_id, destination_bin_id,
                        status, dispatched_at, dispatched_by, received_at, received_by, notes,
                        created_at, updated_at
                 FROM stock_transfers WHERE id = ?1",
                params![input.transfer_id],
                |row| map_transfer_row(row),
            )
            .optional()?;

        let Some((transfer_head, _)) = transfer_opt else {
            return Err(TransferError::NotFound(format!(
                "Transfer '{}' not found",
                input.transfer_id
            )));
        };

        if transfer_head.status != TransferStatus::InTransit {
            return Err(TransferError::InvalidStatusTransition {
                current: transfer_head.status.as_str().into(),
                attempted: "completed".into(),
                reason: "Only in_transit transfers can be received".into(),
            });
        }

        let dest_location_id = input
            .destination_location_id
            .as_deref()
            .unwrap_or(&transfer_head.destination_location_id);
        let dest_bin_id = input
            .destination_bin_id
            .as_deref()
            .or(transfer_head.destination_bin_id.as_deref());

        validate_location_and_bin_ownership(
            &tx,
            &transfer_head.destination_branch_id,
            dest_location_id,
            dest_bin_id,
            "Destination",
        )?;

        // If receive overrides destination location/bin, update them on stock_transfers before ledger intake
        if input.destination_location_id.is_some() || input.destination_bin_id.is_some() {
            tx.execute(
                "UPDATE stock_transfers SET destination_location_id = ?1, destination_bin_id = ?2, updated_at = datetime('now') WHERE id = ?3",
                params![dest_location_id, dest_bin_id, input.transfer_id],
            )?;
        }

        // Fetch items
        let mut item_stmt = tx.prepare(
            "SELECT id, transfer_id, product_id, variant_id, batch_id, serial_id,
                    quantity_milli, received_quantity_milli, created_at
             FROM stock_transfer_items WHERE transfer_id = ?1 ORDER BY created_at ASC",
        )?;
        let item_rows = item_stmt.query_map(params![input.transfer_id], |row| {
            Ok(StockTransferItem {
                id: row.get(0)?,
                transfer_id: row.get(1)?,
                product_id: row.get(2)?,
                variant_id: row.get(3)?,
                batch_id: row.get(4)?,
                serial_id: row.get(5)?,
                quantity_milli: row.get(6)?,
                received_quantity_milli: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut items = Vec::new();
        for r in item_rows {
            items.push(r?);
        }
        drop(item_stmt);

        if items.is_empty() {
            return Err(TransferError::Validation(
                "Cannot receive transfer with no line items".into(),
            ));
        }

        // Execute inbound ledger intake
        for item in &items {
            let dst_batch_id = if let Some(ref src_bid) = item.batch_id {
                Some(resolve_destination_batch(
                    &tx,
                    src_bid,
                    &transfer_head.destination_branch_id,
                )?)
            } else {
                None
            };

            let movement_input = PostMovementInput {
                branch_id: transfer_head.destination_branch_id.clone(),
                product_id: item.product_id.clone(),
                variant_id: item.variant_id.clone(),
                location_id: dest_location_id.to_string(),
                bin_id: dest_bin_id.map(ToString::to_string),
                batch_id: dst_batch_id,
                serial_id: item.serial_id.clone(),
                quantity_delta_milli: item.quantity_milli,
                reason: StockMovementReason::Transfer,
                source_type: Some("transfer".into()),
                source_id: Some(transfer_head.id.clone()),
                user_id: input.user_id.clone(),
                idempotency_key: None,
            };

            StockLedgerService::post_movement_tx(&tx, &movement_input)?;

            // Update item received_quantity_milli to exact quantity_milli (all-or-nothing receipt)
            tx.execute(
                "UPDATE stock_transfer_items SET received_quantity_milli = quantity_milli WHERE id = ?1",
                params![item.id],
            )?;
        }

        // Update transfer status
        let updated_rows = tx.execute(
            "UPDATE stock_transfers
             SET status = 'completed', received_at = datetime('now'), received_by = ?1, updated_at = datetime('now')
             WHERE id = ?2 AND status = 'in_transit'",
            params![input.user_id, input.transfer_id],
        )?;

        if updated_rows == 0 {
            return Err(TransferError::InvalidStatusTransition {
                current: "concurrently_modified".into(),
                attempted: "completed".into(),
                reason: "Transfer status changed concurrently; receive aborted".into(),
            });
        }

        let (received_at, updated_at): (Option<String>, String) = tx.query_row(
            "SELECT received_at, updated_at FROM stock_transfers WHERE id = ?1",
            params![input.transfer_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        // Update items in returned struct
        for item in &mut items {
            item.received_quantity_milli = Some(item.quantity_milli);
        }

        let mut updated_transfer = transfer_head;
        updated_transfer.destination_location_id = dest_location_id.to_string();
        updated_transfer.destination_bin_id = dest_bin_id.map(ToString::to_string);
        updated_transfer.status = TransferStatus::Completed;
        updated_transfer.received_at = received_at;
        updated_transfer.received_by = input.user_id.clone();
        updated_transfer.updated_at = updated_at;
        updated_transfer.items = items;

        if let Some(ref key) = input.idempotency_key {
            let clean_key = key.trim();
            if !clean_key.is_empty() {
                record_idempotency(
                    &tx,
                    "receive_stock_transfer",
                    clean_key,
                    &request_hash,
                    &updated_transfer,
                )?;
            }
        }

        tx.commit()?;

        Ok(updated_transfer)
    }

    /// Cancels a draft transfer document.
    pub fn cancel_transfer(
        conn: &mut Connection,
        input: &CancelTransferInput,
    ) -> Result<StockTransfer, TransferError> {
        let tx = conn.transaction()?;

        let transfer_opt = tx
            .query_row(
                "SELECT id, transfer_number, transfer_type, source_branch_id, destination_branch_id,
                        source_location_id, destination_location_id, source_bin_id, destination_bin_id,
                        status, dispatched_at, dispatched_by, received_at, received_by, notes,
                        created_at, updated_at
                 FROM stock_transfers WHERE id = ?1",
                params![input.transfer_id],
                |row| map_transfer_row(row),
            )
            .optional()?;

        let Some((mut transfer_head, _)) = transfer_opt else {
            return Err(TransferError::NotFound(format!(
                "Transfer '{}' not found",
                input.transfer_id
            )));
        };

        if transfer_head.status == TransferStatus::Cancelled {
            drop(tx);
            transfer_head.items = load_transfer_items(conn, &transfer_head.id)?;
            return Ok(transfer_head);
        }

        if transfer_head.status != TransferStatus::Draft {
            return Err(TransferError::InvalidStatusTransition {
                current: transfer_head.status.as_str().into(),
                attempted: "cancelled".into(),
                reason: format!(
                    "Only draft transfers can be cancelled; current status is '{}'",
                    transfer_head.status.as_str()
                ),
            });
        }

        let updated_rows = tx.execute(
            "UPDATE stock_transfers SET status = 'cancelled', updated_at = datetime('now') WHERE id = ?1 AND status = 'draft'",
            params![input.transfer_id],
        )?;

        if updated_rows == 0 {
            return Err(TransferError::InvalidStatusTransition {
                current: "concurrently_modified".into(),
                attempted: "cancelled".into(),
                reason: "Transfer status changed concurrently; cancellation aborted".into(),
            });
        }

        let updated_at: String = tx.query_row(
            "SELECT updated_at FROM stock_transfers WHERE id = ?1",
            params![input.transfer_id],
            |r| r.get(0),
        )?;

        transfer_head.status = TransferStatus::Cancelled;
        transfer_head.updated_at = updated_at;

        tx.commit()?;

        transfer_head.items = load_transfer_items(conn, &transfer_head.id)?;

        Ok(transfer_head)
    }

    /// Performs an immediate, atomic intra-branch relocation in a single transaction.
    /// Net aggregate branch delta = 0.
    pub fn instant_intra_branch_transfer(
        conn: &mut Connection,
        input: &InstantIntraBranchTransferInput,
    ) -> Result<StockTransfer, TransferError> {
        if input.items.is_empty() {
            return Err(TransferError::Validation(
                "Transfer must contain at least one line item".into(),
            ));
        }

        validate_no_op_relocation(
            TransferType::IntraBranch,
            &input.source_location_id,
            &input.destination_location_id,
            input.source_bin_id.as_deref(),
            input.destination_bin_id.as_deref(),
        )?;

        let request_hash = compute_canonical_hash(input);

        let tx = conn.transaction()?;

        // Idempotency check
        if let Some(ref key) = input.idempotency_key {
            let clean_key = key.trim();
            if !clean_key.is_empty() {
                if let Some(cached) = check_idempotency(
                    &tx,
                    "instant_intra_branch_transfer",
                    clean_key,
                    &request_hash,
                )? {
                    return Ok(cached);
                }
            }
        }

        // Validate locations and bins within branch
        validate_location_and_bin_ownership(
            &tx,
            &input.branch_id,
            &input.source_location_id,
            input.source_bin_id.as_deref(),
            "Source",
        )?;
        validate_location_and_bin_ownership(
            &tx,
            &input.branch_id,
            &input.destination_location_id,
            input.destination_bin_id.as_deref(),
            "Destination",
        )?;

        // Validate each item and protect against duplicate serial_ids in the same payload
        let mut seen_serials = std::collections::HashSet::new();
        for (idx, item) in input.items.iter().enumerate() {
            if let Some(ref sid) = item.serial_id {
                let trimmed_sid = sid.trim();
                if !seen_serials.insert(trimmed_sid.to_string()) {
                    return Err(TransferError::Validation(format!(
                        "Duplicate serial_id '{trimmed_sid}' in transfer items at index {idx}"
                    )));
                }
            }
            validate_item_consistency(&tx, &input.branch_id, item, idx)?;
        }

        let transfer_id: String =
            tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
        let transfer_number = generate_transfer_number(&tx, "TRF-INTRA")?;

        // 3. Insert completed transfer header before movements so transfer context validation succeeds
        tx.execute(
            "INSERT INTO stock_transfers (
                id, transfer_number, transfer_type,
                source_branch_id, destination_branch_id,
                source_location_id, destination_location_id,
                source_bin_id, destination_bin_id,
                status, dispatched_at, dispatched_by,
                received_at, received_by,
                notes, created_at, updated_at
             ) VALUES (
                ?1, ?2, 'intra_branch',
                ?3, ?3,
                ?4, ?5,
                ?6, ?7,
                'completed', datetime('now'), ?8,
                datetime('now'), ?8,
                ?9, datetime('now'), datetime('now')
             )",
            params![
                transfer_id,
                transfer_number,
                input.branch_id,
                input.source_location_id,
                input.destination_location_id,
                input.source_bin_id,
                input.destination_bin_id,
                input.user_id,
                input.notes,
            ],
        )?;

        // 4. Post negative movement at source location/bin
        // 5. Post positive movement at destination location/bin
        // 6. Insert completed items
        let mut persisted_items = Vec::with_capacity(input.items.len());

        for item in &input.items {
            let outbound_input = PostMovementInput {
                branch_id: input.branch_id.clone(),
                product_id: item.product_id.clone(),
                variant_id: item.variant_id.clone(),
                location_id: input.source_location_id.clone(),
                bin_id: input.source_bin_id.clone(),
                batch_id: item.batch_id.clone(),
                serial_id: item.serial_id.clone(),
                quantity_delta_milli: -item.quantity_milli,
                reason: StockMovementReason::Transfer,
                source_type: Some("transfer".into()),
                source_id: Some(transfer_id.clone()),
                user_id: input.user_id.clone(),
                idempotency_key: None,
            };
            StockLedgerService::post_movement_tx(&tx, &outbound_input)?;

            let inbound_input = PostMovementInput {
                branch_id: input.branch_id.clone(),
                product_id: item.product_id.clone(),
                variant_id: item.variant_id.clone(),
                location_id: input.destination_location_id.clone(),
                bin_id: input.destination_bin_id.clone(),
                batch_id: item.batch_id.clone(),
                serial_id: item.serial_id.clone(),
                quantity_delta_milli: item.quantity_milli,
                reason: StockMovementReason::Transfer,
                source_type: Some("transfer".into()),
                source_id: Some(transfer_id.clone()),
                user_id: input.user_id.clone(),
                idempotency_key: None,
            };
            StockLedgerService::post_movement_tx(&tx, &inbound_input)?;

            let item_id: String =
                tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;

            tx.execute(
                "INSERT INTO stock_transfer_items (
                    id, transfer_id, product_id, variant_id, batch_id, serial_id,
                    quantity_milli, received_quantity_milli, created_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, ?7, datetime('now')
                 )",
                params![
                    item_id,
                    transfer_id,
                    item.product_id,
                    item.variant_id,
                    item.batch_id,
                    item.serial_id,
                    item.quantity_milli,
                ],
            )?;

            let item_created_at: String = tx.query_row(
                "SELECT created_at FROM stock_transfer_items WHERE id = ?1",
                params![item_id],
                |r| r.get(0),
            )?;

            persisted_items.push(StockTransferItem {
                id: item_id,
                transfer_id: transfer_id.clone(),
                product_id: item.product_id.clone(),
                variant_id: item.variant_id.clone(),
                batch_id: item.batch_id.clone(),
                serial_id: item.serial_id.clone(),
                quantity_milli: item.quantity_milli,
                received_quantity_milli: Some(item.quantity_milli),
                created_at: item_created_at,
            });
        }

        let (dispatched_at, received_at, created_at, updated_at): (
            Option<String>,
            Option<String>,
            String,
            String,
        ) = tx.query_row(
            "SELECT dispatched_at, received_at, created_at, updated_at FROM stock_transfers WHERE id = ?1",
            params![transfer_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;

        let transfer = StockTransfer {
            id: transfer_id,
            transfer_number,
            transfer_type: TransferType::IntraBranch,
            source_branch_id: input.branch_id.clone(),
            destination_branch_id: input.branch_id.clone(),
            source_location_id: input.source_location_id.clone(),
            destination_location_id: input.destination_location_id.clone(),
            source_bin_id: input.source_bin_id.clone(),
            destination_bin_id: input.destination_bin_id.clone(),
            status: TransferStatus::Completed,
            dispatched_at,
            dispatched_by: input.user_id.clone(),
            received_at,
            received_by: input.user_id.clone(),
            notes: input.notes.clone(),
            created_at,
            updated_at,
            items: persisted_items,
        };

        if let Some(ref key) = input.idempotency_key {
            let clean_key = key.trim();
            if !clean_key.is_empty() {
                record_idempotency(
                    &tx,
                    "instant_intra_branch_transfer",
                    clean_key,
                    &request_hash,
                    &transfer,
                )?;
            }
        }

        tx.commit()?;

        Ok(transfer)
    }

    /// Retrieves a single stock transfer document with its line items by ID.
    pub fn get_transfer(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<StockTransfer>, TransferError> {
        let transfer_opt = conn
            .query_row(
                "SELECT id, transfer_number, transfer_type, source_branch_id, destination_branch_id,
                        source_location_id, destination_location_id, source_bin_id, destination_bin_id,
                        status, dispatched_at, dispatched_by, received_at, received_by, notes,
                        created_at, updated_at
                 FROM stock_transfers WHERE id = ?1",
                params![id],
                |row| map_transfer_row(row),
            )
            .optional()?;

        let Some((mut transfer, _)) = transfer_opt else {
            return Ok(None);
        };

        transfer.items = load_transfer_items(conn, &transfer.id)?;
        Ok(Some(transfer))
    }

    /// Queries stock transfer documents matching the provided filter criteria.
    pub fn list_transfers(
        conn: &Connection,
        filter: &TransferFilter,
    ) -> Result<Vec<StockTransfer>, TransferError> {
        let mut query = String::from(
            "SELECT id, transfer_number, transfer_type, source_branch_id, destination_branch_id,
                    source_location_id, destination_location_id, source_bin_id, destination_bin_id,
                    status, dispatched_at, dispatched_by, received_at, received_by, notes,
                    created_at, updated_at
             FROM stock_transfers WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref b_id) = filter.branch_id {
            params_vec.push(Box::new(b_id.clone()));
            let idx = params_vec.len();
            query.push_str(&format!(
                " AND (source_branch_id = ?{idx} OR destination_branch_id = ?{idx})"
            ));
        }

        if let Some(ref sb_id) = filter.source_branch_id {
            params_vec.push(Box::new(sb_id.clone()));
            query.push_str(&format!(" AND source_branch_id = ?{}", params_vec.len()));
        }

        if let Some(ref db_id) = filter.destination_branch_id {
            params_vec.push(Box::new(db_id.clone()));
            query.push_str(&format!(
                " AND destination_branch_id = ?{}",
                params_vec.len()
            ));
        }

        if let Some(tt) = filter.transfer_type {
            params_vec.push(Box::new(tt.as_str().to_string()));
            query.push_str(&format!(" AND transfer_type = ?{}", params_vec.len()));
        }

        if let Some(st) = filter.status {
            params_vec.push(Box::new(st.as_str().to_string()));
            query.push_str(&format!(" AND status = ?{}", params_vec.len()));
        }

        query.push_str(" ORDER BY created_at DESC");

        let limit = filter.limit.unwrap_or(100).clamp(1, 1000);
        params_vec.push(Box::new(limit));
        query.push_str(&format!(" LIMIT ?{}", params_vec.len()));

        if let Some(offset) = filter.offset {
            params_vec.push(Box::new(offset.max(0)));
            query.push_str(&format!(" OFFSET ?{}", params_vec.len()));
        }

        let mut stmt = conn.prepare(&query)?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(AsRef::as_ref).collect();

        let rows = stmt.query_map(rusqlite_params.as_slice(), |row| map_transfer_row(row))?;

        let mut transfers = Vec::new();
        for r in rows {
            let (mut transfer, id) = r?;
            transfer.items = load_transfer_items(conn, &id)?;
            transfers.push(transfer);
        }

        Ok(transfers)
    }
}
