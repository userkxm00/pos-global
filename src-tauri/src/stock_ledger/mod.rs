// F2.11 — Stock Ledger & Spatial Balances Domain Service
// ADR-0013: Sole authority for inventory quantity mutations, spatial stock balances,
// immutable audit ledger, exact unit serial transitions, batch lifecycle, and idempotency.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

// =========================================================================
// DOMAIN ENUMS & VALUE OBJECTS
// =========================================================================

/// Authoritative movement reasons supported in F2.11.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockMovementReason {
    /// Initial stock establishment (inbound only, positive delta).
    OpeningBalance,
    /// Inventory correction primitive (inbound or outbound, non-zero delta).
    Adjustment,
    /// Damaged or spoiled goods written off (outbound only, negative delta).
    Damage,
    /// Missing or stolen inventory written off (outbound only, negative delta).
    Loss,
    /// Historical/system reason: Point-of-sale deduction.
    Sale,
    /// Historical/system reason: Customer return / refund.
    Refund,
    /// Historical/system reason: Goods receipt note / purchasing intake.
    PurchaseReceipt,
    /// Historical/system reason: Stock transfer movement.
    Transfer,
    /// Fallback for arbitrary or legacy persisted text to prevent read query conversion aborts.
    #[serde(untagged)]
    Other(String),
}

impl StockMovementReason {
    pub fn as_str(&self) -> &str {
        match self {
            StockMovementReason::OpeningBalance => "opening_balance",
            StockMovementReason::Adjustment => "adjustment",
            StockMovementReason::Damage => "damage",
            StockMovementReason::Loss => "loss",
            StockMovementReason::Sale => "sale",
            StockMovementReason::Refund => "refund",
            StockMovementReason::PurchaseReceipt => "purchase_receipt",
            StockMovementReason::Transfer => "transfer",
            StockMovementReason::Other(s) => s.as_str(),
        }
    }

    /// Returns true if this reason is an authorized F2.11 write mutation reason.
    pub fn is_mutation_reason(&self) -> bool {
        matches!(
            self,
            StockMovementReason::OpeningBalance
                | StockMovementReason::Adjustment
                | StockMovementReason::Damage
                | StockMovementReason::Loss
        )
    }

    /// Enforces that mutations only accept authorized F2.11 reasons.
    pub fn validate_mutation(&self) -> Result<(), StockLedgerError> {
        if !self.is_mutation_reason() {
            return Err(StockLedgerError::InvalidReason(format!(
                "Invalid movement reason '{}'. Allowed: opening_balance, adjustment, damage, loss",
                self.as_str()
            )));
        }
        Ok(())
    }

    /// Validates directional delta invariants according to ADR-0013.
    pub fn validate_delta(&self, delta: i64) -> Result<(), StockLedgerError> {
        self.validate_mutation()?;

        if delta == 0 {
            return Err(StockLedgerError::InvalidQuantity(
                "Quantity delta cannot be zero".to_string(),
            ));
        }

        match self {
            StockMovementReason::OpeningBalance => {
                if delta <= 0 {
                    Err(StockLedgerError::InvalidQuantity(
                        "opening_balance reason requires positive quantity_delta_milli".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            StockMovementReason::Adjustment => Ok(()),
            StockMovementReason::Damage => {
                if delta >= 0 {
                    Err(StockLedgerError::InvalidQuantity(
                        "damage reason requires negative quantity_delta_milli".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            StockMovementReason::Loss => {
                if delta >= 0 {
                    Err(StockLedgerError::InvalidQuantity(
                        "loss reason requires negative quantity_delta_milli".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            _ => unreachable!(),
        }
    }

    /// Reads and parses persisted movement reasons from database rows.
    /// Supports the four F2.11 mutation reasons, known historical/system reasons (e.g. sale),
    /// and safely falls back to Other(String) for arbitrary/corrupt text without crashing readers.
    pub fn from_persisted_str(s: &str) -> Self {
        let trimmed = s.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "opening_balance" => StockMovementReason::OpeningBalance,
            "adjustment" => StockMovementReason::Adjustment,
            "damage" => StockMovementReason::Damage,
            "loss" => StockMovementReason::Loss,
            "sale" => StockMovementReason::Sale,
            "refund" => StockMovementReason::Refund,
            "purchase" | "purchase_receipt" => StockMovementReason::PurchaseReceipt,
            "transfer" => StockMovementReason::Transfer,
            _ => StockMovementReason::Other(trimmed.to_string()),
        }
    }
}

impl std::fmt::Display for StockMovementReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for StockMovementReason {
    type Err = StockLedgerError;

    /// Parses and validates authorized F2.11 mutation reasons.
    /// Rejects historical or arbitrary reasons so write mutations cannot use them.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "opening_balance" => Ok(StockMovementReason::OpeningBalance),
            "adjustment" => Ok(StockMovementReason::Adjustment),
            "damage" => Ok(StockMovementReason::Damage),
            "loss" => Ok(StockMovementReason::Loss),
            other => Err(StockLedgerError::InvalidReason(format!(
                "Invalid movement reason '{other}'. Allowed: opening_balance, adjustment, damage, loss"
            ))),
        }
    }
}

// =========================================================================
// DOMAIN MODELS
// =========================================================================

/// Record of an immutable historical stock movement event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: String,
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub quantity_delta_milli: i64,
    pub quantity_before_milli: Option<i64>,
    pub quantity_after_milli: Option<i64>,
    pub reason: StockMovementReason,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub location_id: Option<String>,
    pub bin_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
}

/// Spatially attributed current physical stock balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationInventory {
    pub id: String,
    pub branch_id: String,
    pub location_id: String,
    pub product_id: String,
    pub bin_id: Option<String>,
    pub variant_id: Option<String>,
    pub batch_id: Option<String>,
    pub quantity_milli: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Comprehensive balance summary showing aggregate, allocated spatial, and unallocated stock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockBalanceSummary {
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub aggregate_quantity_milli: i64,
    pub allocated_spatial_milli: i64,
    pub unallocated_milli: i64,
}

/// Input payload for posting an atomic stock ledger movement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostMovementInput {
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub location_id: String,
    pub bin_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub quantity_delta_milli: i64,
    pub reason: StockMovementReason,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub user_id: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Filter criteria for querying stock movement history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovementFilter {
    pub branch_id: String,
    pub product_id: Option<String>,
    pub variant_id: Option<String>,
    pub location_id: Option<String>,
    pub bin_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub reason: Option<StockMovementReason>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Filter criteria for querying spatial stock balances.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationInventoryFilter {
    pub branch_id: String,
    pub location_id: Option<String>,
    pub bin_id: Option<String>,
    pub product_id: Option<String>,
    pub variant_id: Option<String>,
    pub batch_id: Option<String>,
}

// =========================================================================
// DOMAIN ERRORS
// =========================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum StockLedgerError {
    Validation(String),
    NotFound(String),
    InsufficientStock {
        requested_milli: i64,
        available_milli: i64,
    },
    IdempotencyConflict(String),
    InvalidReason(String),
    InvalidQuantity(String),
    InvalidLocation(String),
    InvalidBin(String),
    InvalidBatch(String),
    InvalidSerial(String),
    BranchMismatch(String),
    VariantMismatch(String),
    Database(String),
}

impl std::fmt::Display for StockLedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StockLedgerError::Validation(msg) => write!(f, "Validation error: {msg}"),
            StockLedgerError::NotFound(msg) => write!(f, "Entity not found: {msg}"),
            StockLedgerError::InsufficientStock {
                requested_milli,
                available_milli,
            } => write!(
                f,
                "Insufficient stock: requested {requested_milli} milli, available {available_milli} milli"
            ),
            StockLedgerError::IdempotencyConflict(msg) => write!(f, "Idempotency conflict: {msg}"),
            StockLedgerError::InvalidReason(msg) => write!(f, "Invalid reason: {msg}"),
            StockLedgerError::InvalidQuantity(msg) => write!(f, "Invalid quantity: {msg}"),
            StockLedgerError::InvalidLocation(msg) => write!(f, "Invalid location: {msg}"),
            StockLedgerError::InvalidBin(msg) => write!(f, "Invalid bin: {msg}"),
            StockLedgerError::InvalidBatch(msg) => write!(f, "Invalid batch: {msg}"),
            StockLedgerError::InvalidSerial(msg) => write!(f, "Invalid serial: {msg}"),
            StockLedgerError::BranchMismatch(msg) => write!(f, "Branch mismatch: {msg}"),
            StockLedgerError::VariantMismatch(msg) => write!(f, "Variant mismatch: {msg}"),
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

impl From<serde_json::Error> for StockLedgerError {
    fn from(err: serde_json::Error) -> Self {
        StockLedgerError::Database(format!("JSON serialization error: {err}"))
    }
}

// =========================================================================
// CANONICAL REQUEST HASHING
// =========================================================================

/// Computes a canonical SHA-256 hash across all request fields that materially
/// determine the persisted stock movement outcome.
pub fn compute_request_hash(input: &PostMovementInput) -> String {
    let canonical = format!(
        "branch_id={}|product_id={}|variant_id={}|location_id={}|bin_id={}|batch_id={}|serial_id={}|quantity_delta_milli={}|reason={}|source_type={}|source_id={}|user_id={}",
        input.branch_id.trim(),
        input.product_id.trim(),
        input.variant_id.as_deref().unwrap_or("").trim(),
        input.location_id.trim(),
        input.bin_id.as_deref().unwrap_or("").trim(),
        input.batch_id.as_deref().unwrap_or("").trim(),
        input.serial_id.as_deref().unwrap_or("").trim(),
        input.quantity_delta_milli,
        input.reason.as_str(),
        input.source_type.as_deref().unwrap_or("").trim(),
        input.source_id.as_deref().unwrap_or("").trim(),
        input.user_id.as_deref().unwrap_or("").trim(),
    );
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

// =========================================================================
// STOCK LEDGER SERVICE (AUTHORITATIVE DOMAIN API)
// =========================================================================

pub struct StockLedgerService;

impl StockLedgerService {
    /// Posts a stock movement atomically through the authoritative ledger.
    pub fn post_movement(
        conn: &mut Connection,
        input: &PostMovementInput,
    ) -> Result<StockMovement, StockLedgerError> {
        post_stock_movement(conn, input)
    }

    /// Computes the comprehensive stock balance (aggregate, spatial, unallocated).
    pub fn get_balance(
        conn: &Connection,
        branch_id: &str,
        product_id: &str,
        variant_id: Option<&str>,
    ) -> Result<StockBalanceSummary, StockLedgerError> {
        get_stock_balance(conn, branch_id, product_id, variant_id)
    }

    /// Queries spatial location inventory balances.
    pub fn list_location_inventory(
        conn: &Connection,
        filter: &LocationInventoryFilter,
    ) -> Result<Vec<LocationInventory>, StockLedgerError> {
        list_location_inventory(conn, filter)
    }

    /// Queries immutable stock movement ledger records.
    pub fn list_movements(
        conn: &Connection,
        filter: &StockMovementFilter,
    ) -> Result<Vec<StockMovement>, StockLedgerError> {
        list_stock_movements(conn, filter)
    }

    /// Retrieves a single movement record by ID scoped to branch.
    pub fn get_movement(
        conn: &Connection,
        branch_id: &str,
        id: &str,
    ) -> Result<Option<StockMovement>, StockLedgerError> {
        get_stock_movement_by_id(conn, branch_id, id)
    }
}

// =========================================================================
// ATOMIC MOVEMENT MUTATION ENGINE
// =========================================================================

/// Primary atomic stock movement mutation engine.
/// Executes within a single transaction boundary:
/// Idempotency -> Validation -> Aggregate -> Spatial -> Batch/Serial -> Movement -> Commit.
pub fn post_stock_movement(
    conn: &mut Connection,
    input: &PostMovementInput,
) -> Result<StockMovement, StockLedgerError> {
    let branch_id = input.branch_id.trim();
    let product_id = input.product_id.trim();
    let location_id = input.location_id.trim();
    let norm_variant_id = input
        .variant_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let norm_bin_id = input
        .bin_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let norm_batch_id = input
        .batch_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let norm_serial_id = input
        .serial_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    // Basic non-empty string validations
    if branch_id.is_empty() {
        return Err(StockLedgerError::Validation(
            "branch_id cannot be empty".to_string(),
        ));
    }
    if product_id.is_empty() {
        return Err(StockLedgerError::Validation(
            "product_id cannot be empty".to_string(),
        ));
    }
    if location_id.is_empty() {
        return Err(StockLedgerError::Validation(
            "location_id cannot be empty".to_string(),
        ));
    }

    // Directional and non-zero delta validation
    input.reason.validate_delta(input.quantity_delta_milli)?;

    let request_hash = compute_request_hash(input);

    let tx = conn.transaction()?;

    // 1. Idempotency check inside the transaction boundary
    if let Some(ref key) = input.idempotency_key {
        let clean_key = key.trim();
        if clean_key.is_empty() {
            return Err(StockLedgerError::Validation(
                "idempotency_key cannot be empty or whitespace".to_string(),
            ));
        }

        let existing: Option<(Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT request_hash, result_json FROM idempotency_keys WHERE key = ?1",
                params![clean_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        if let Some((stored_hash, result_json)) = existing {
            if stored_hash.as_deref() == Some(&request_hash) {
                if let Some(json) = result_json {
                    let cached_movement: StockMovement = serde_json::from_str(&json)?;
                    return Ok(cached_movement);
                }
            } else {
                return Err(StockLedgerError::IdempotencyConflict(format!(
                    "Idempotency key '{clean_key}' already used with different request parameters"
                )));
            }
        }
    }

    // 2. Validate Branch
    let branch_active: bool = tx
        .query_row(
            "SELECT is_active FROM branches WHERE id = ?1",
            params![branch_id],
            |row| {
                let active: i64 = row.get(0)?;
                Ok(active != 0)
            },
        )
        .optional()?
        .ok_or_else(|| StockLedgerError::NotFound(format!("Branch '{branch_id}' not found")))?;

    if !branch_active {
        return Err(StockLedgerError::Validation(format!(
            "Branch '{branch_id}' is inactive"
        )));
    }

    // 3. Validate Product
    let product_active: bool = tx
        .query_row(
            "SELECT is_active FROM products WHERE id = ?1",
            params![product_id],
            |row| {
                let active: i64 = row.get(0)?;
                Ok(active != 0)
            },
        )
        .optional()?
        .ok_or_else(|| StockLedgerError::NotFound(format!("Product '{product_id}' not found")))?;

    if !product_active {
        return Err(StockLedgerError::Validation(format!(
            "Product '{product_id}' is inactive"
        )));
    }

    // 4. Validate Variant
    if let Some(vid) = norm_variant_id {
        let (v_product_id, v_active, v_deleted): (String, i64, Option<String>) = tx
            .query_row(
                "SELECT product_id, is_active, deleted_at FROM product_variants WHERE id = ?1",
                params![vid],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| StockLedgerError::NotFound(format!("Variant '{vid}' not found")))?;

        if v_product_id != product_id {
            return Err(StockLedgerError::VariantMismatch(format!(
                "Variant '{vid}' belongs to product '{v_product_id}', not '{product_id}'"
            )));
        }
        if v_active == 0 || v_deleted.is_some() {
            return Err(StockLedgerError::Validation(format!(
                "Variant '{vid}' is inactive or soft-deleted"
            )));
        }
    }

    // 5. Validate Location and Bin
    let (loc_branch, loc_active): (String, i64) = tx
        .query_row(
            "SELECT branch_id, is_active FROM locations WHERE id = ?1",
            params![location_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| StockLedgerError::NotFound(format!("Location '{location_id}' not found")))?;

    if loc_branch != branch_id {
        return Err(StockLedgerError::BranchMismatch(format!(
            "Location '{location_id}' belongs to branch '{loc_branch}', not '{branch_id}'"
        )));
    }
    if loc_active == 0 {
        return Err(StockLedgerError::InvalidLocation(format!(
            "Location '{location_id}' is inactive"
        )));
    }

    if let Some(bin_id) = norm_bin_id {
        let (bin_location, bin_active): (String, i64) = tx
            .query_row(
                "SELECT location_id, is_active FROM bins WHERE id = ?1",
                params![bin_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| StockLedgerError::NotFound(format!("Bin '{bin_id}' not found")))?;

        if bin_location != location_id {
            return Err(StockLedgerError::InvalidBin(format!(
                "Bin '{bin_id}' belongs to location '{bin_location}', not '{location_id}'"
            )));
        }
        if bin_active == 0 {
            return Err(StockLedgerError::InvalidBin(format!(
                "Bin '{bin_id}' is inactive"
            )));
        }
    }

    // 6. Validate Batch (if specified)
    let mut batch_new_qty_and_status: Option<(i64, &'static str)> = None;
    if let Some(batch_id) = norm_batch_id {
        let (b_prod, b_branch, b_var, b_qty, b_status): (
            String,
            String,
            Option<String>,
            i64,
            String,
        ) = tx
            .query_row(
                "SELECT product_id, branch_id, variant_id, quantity_milli, status FROM product_batches WHERE id = ?1",
                params![batch_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()?
            .ok_or_else(|| StockLedgerError::NotFound(format!("Batch '{batch_id}' not found")))?;

        if b_prod != product_id {
            return Err(StockLedgerError::InvalidBatch(format!(
                "Batch '{batch_id}' belongs to product '{b_prod}', not '{product_id}'"
            )));
        }
        if b_branch != branch_id {
            return Err(StockLedgerError::BranchMismatch(format!(
                "Batch '{batch_id}' belongs to branch '{b_branch}', not '{branch_id}'"
            )));
        }
        if b_var.as_deref() != norm_variant_id {
            return Err(StockLedgerError::VariantMismatch(format!(
                "Batch variant '{:?}' does not match requested variant '{:?}'",
                b_var, norm_variant_id
            )));
        }

        let delta = input.quantity_delta_milli;

        // Batch lifecycle and terminal checks
        match b_status.as_str() {
            "depleted" => {
                return Err(StockLedgerError::InvalidBatch(
                    "Depleted batch is terminal and cannot accept stock intake or movements"
                        .to_string(),
                ));
            }
            "recalled" => {
                if delta > 0 {
                    return Err(StockLedgerError::InvalidBatch(
                        "Recalled batch cannot accept positive stock intake".to_string(),
                    ));
                }
            }
            "quarantined" => {
                if delta > 0 {
                    return Err(StockLedgerError::InvalidBatch(
                        "Quarantined batch cannot accept positive stock intake".to_string(),
                    ));
                }
            }
            "active" => {}
            other => {
                return Err(StockLedgerError::InvalidBatch(format!(
                    "Unrecognized batch status '{other}'"
                )));
            }
        }

        let new_qty = b_qty.checked_add(delta).ok_or_else(|| {
            StockLedgerError::Validation("Batch quantity arithmetic overflow".to_string())
        })?;

        if new_qty < 0 {
            return Err(StockLedgerError::InsufficientStock {
                requested_milli: -delta,
                available_milli: b_qty,
            });
        }

        let new_status = if b_status == "recalled" {
            "recalled"
        } else if new_qty == 0 {
            "depleted"
        } else if b_status == "quarantined" {
            "quarantined"
        } else {
            "active"
        };
        batch_new_qty_and_status = Some((new_qty, new_status));
    }

    // 7. Validate Serial (if specified)
    let mut serial_target_status: Option<&'static str> = None;
    if let Some(serial_id) = norm_serial_id {
        let delta = input.quantity_delta_milli;
        if delta > 0 && delta != 1000 {
            return Err(StockLedgerError::InvalidQuantity(
                "Serialized positive stock movement must be exactly +1000 milli (1 unit)"
                    .to_string(),
            ));
        }
        if delta < 0 && delta != -1000 {
            return Err(StockLedgerError::InvalidQuantity(
                "Serialized negative stock movement must be exactly -1000 milli (1 unit)"
                    .to_string(),
            ));
        }

        let (s_prod, s_branch, s_var, s_status, s_loc, s_bin): (
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
        ) = tx
            .query_row(
                "SELECT product_id, branch_id, variant_id, status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
                params![serial_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .optional()?
            .ok_or_else(|| StockLedgerError::NotFound(format!("Serial '{serial_id}' not found")))?;

        if s_prod != product_id {
            return Err(StockLedgerError::InvalidSerial(format!(
                "Serial '{serial_id}' belongs to product '{s_prod}', not '{product_id}'"
            )));
        }
        if s_branch != branch_id {
            return Err(StockLedgerError::BranchMismatch(format!(
                "Serial '{serial_id}' belongs to branch '{s_branch}', not '{branch_id}'"
            )));
        }
        if s_var.as_deref() != norm_variant_id {
            return Err(StockLedgerError::VariantMismatch(format!(
                "Serial variant '{:?}' does not match requested variant '{:?}'",
                s_var, norm_variant_id
            )));
        }

        if delta == 1000 {
            // Inbound serialized movement: permitted sources are reserved and defective
            if s_status != "reserved" && s_status != "defective" {
                return Err(StockLedgerError::InvalidSerial(format!(
                    "Cannot intake serial in status '{s_status}'. Permitted: reserved, defective"
                )));
            }
            serial_target_status = Some("in_stock");
        } else {
            // Outbound serialized movement: item must currently be in_stock
            if s_status != "in_stock" {
                return Err(StockLedgerError::InvalidSerial(format!(
                    "Cannot deduct serial in status '{s_status}'. Expected 'in_stock'"
                )));
            }
            // Coordinates must match
            if s_loc.as_deref() != Some(location_id) {
                return Err(StockLedgerError::InvalidSerial(format!(
                    "Serial current location '{:?}' does not match requested location '{location_id}'",
                    s_loc
                )));
            }
            if norm_bin_id.is_some() && s_bin.as_deref() != norm_bin_id {
                return Err(StockLedgerError::InvalidSerial(format!(
                    "Serial current bin '{:?}' does not match requested bin '{:?}'",
                    s_bin, norm_bin_id
                )));
            }

            match input.reason {
                StockMovementReason::Damage => serial_target_status = Some("defective"),
                StockMovementReason::Loss => serial_target_status = Some("disposed"),
                StockMovementReason::Adjustment => serial_target_status = Some("reserved"),
                StockMovementReason::OpeningBalance => {
                    return Err(StockLedgerError::InvalidReason(
                        "opening_balance cannot be outbound".to_string(),
                    ))
                }
                _ => {
                    return Err(StockLedgerError::InvalidReason(format!(
                        "Reason '{}' is not an authorized outbound mutation reason",
                        input.reason
                    )))
                }
            }
        }
    }

    let delta = input.quantity_delta_milli;

    // 8. Mutate Aggregate Inventory (`inventory` table)
    let agg_row: Option<(String, i64)> = tx
        .query_row(
            "SELECT id, quantity_milli FROM inventory
             WHERE branch_id = ?1 AND product_id = ?2
               AND (variant_id = ?3 OR (?3 IS NULL AND variant_id IS NULL))",
            params![branch_id, product_id, norm_variant_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let (agg_before, agg_after) = match agg_row {
        Some((inv_id, current_qty)) => {
            let after = current_qty.checked_add(delta).ok_or_else(|| {
                StockLedgerError::Validation("Aggregate inventory quantity overflow".to_string())
            })?;
            if after < 0 {
                return Err(StockLedgerError::InsufficientStock {
                    requested_milli: -delta,
                    available_milli: current_qty,
                });
            }
            tx.execute(
                "UPDATE inventory
                 SET quantity_milli = ?1, quantity = ?1 / 1000.0, updated_at = datetime('now')
                 WHERE id = ?2",
                params![after, inv_id],
            )?;
            (current_qty, after)
        }
        None => {
            if delta < 0 {
                return Err(StockLedgerError::InsufficientStock {
                    requested_milli: -delta,
                    available_milli: 0,
                });
            }
            tx.execute(
                "INSERT INTO inventory (id, branch_id, product_id, variant_id, quantity, quantity_milli, updated_at)
                 VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4 / 1000.0, ?4, datetime('now'))",
                params![branch_id, product_id, norm_variant_id, delta],
            )?;
            (0, delta)
        }
    };

    // 9. Mutate Spatial Inventory (`location_inventory` table)
    let spatial_row: Option<(String, i64)> = tx
        .query_row(
            "SELECT id, quantity_milli FROM location_inventory
             WHERE branch_id = ?1
               AND location_id = ?2
               AND product_id = ?3
               AND (bin_id = ?4 OR (?4 IS NULL AND bin_id IS NULL))
               AND (variant_id = ?5 OR (?5 IS NULL AND variant_id IS NULL))
               AND (batch_id = ?6 OR (?6 IS NULL AND batch_id IS NULL))",
            params![
                branch_id,
                location_id,
                product_id,
                norm_bin_id,
                norm_variant_id,
                norm_batch_id
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match spatial_row {
        Some((loc_inv_id, current_spatial)) => {
            let spatial_after = current_spatial.checked_add(delta).ok_or_else(|| {
                StockLedgerError::Validation("Spatial inventory quantity overflow".to_string())
            })?;
            if spatial_after < 0 {
                return Err(StockLedgerError::InsufficientStock {
                    requested_milli: -delta,
                    available_milli: current_spatial,
                });
            }
            tx.execute(
                "UPDATE location_inventory
                 SET quantity_milli = ?1, updated_at = datetime('now')
                 WHERE id = ?2",
                params![spatial_after, loc_inv_id],
            )?;
        }
        None => {
            if delta < 0 {
                return Err(StockLedgerError::InsufficientStock {
                    requested_milli: -delta,
                    available_milli: 0,
                });
            }
            tx.execute(
                "INSERT INTO location_inventory (
                    id, branch_id, location_id, product_id, bin_id, variant_id, batch_id, quantity_milli, created_at, updated_at
                 ) VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))",
                params![
                    branch_id,
                    location_id,
                    product_id,
                    norm_bin_id,
                    norm_variant_id,
                    norm_batch_id,
                    delta
                ],
            )?;
        }
    };

    // 10. Spatial Unallocated Invariant Verification:
    // allocated_spatial = SUM(location_inventory)
    // unallocated = aggregate - allocated_spatial >= 0
    let allocated_spatial: i64 = tx.query_row(
        "SELECT COALESCE(SUM(quantity_milli), 0) FROM location_inventory
         WHERE branch_id = ?1 AND product_id = ?2
           AND (variant_id = ?3 OR (?3 IS NULL AND variant_id IS NULL))",
        params![branch_id, product_id, norm_variant_id],
        |row| row.get(0),
    )?;

    if agg_after < allocated_spatial {
        return Err(StockLedgerError::Validation(format!(
            "Spatial allocation ({allocated_spatial} milli) exceeds aggregate balance ({agg_after} milli)"
        )));
    }

    // 11. Mutate Batch State
    if let (Some(batch_id), Some((new_b_qty, new_b_status))) =
        (norm_batch_id, batch_new_qty_and_status)
    {
        tx.execute(
            "UPDATE product_batches
             SET quantity_milli = ?1, status = ?2, updated_at = datetime('now')
             WHERE id = ?3",
            params![new_b_qty, new_b_status, batch_id],
        )?;
    }

    // 12. Mutate Serial State
    if let (Some(serial_id), Some(target_status)) = (norm_serial_id, serial_target_status) {
        if target_status == "in_stock" {
            tx.execute(
                "UPDATE serial_numbers
                 SET status = 'in_stock', location_id = ?1, bin_id = ?2, updated_at = datetime('now')
                 WHERE id = ?3",
                params![location_id, norm_bin_id, serial_id],
            )?;
        } else {
            tx.execute(
                "UPDATE serial_numbers
                 SET status = ?1, location_id = NULL, bin_id = NULL, updated_at = datetime('now')
                 WHERE id = ?2",
                params![target_status, serial_id],
            )?;
        }
    }

    // 13. Append Immutable Stock Movement Ledger Entry
    let movement_id: String =
        tx.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;

    tx.execute(
        "INSERT INTO stock_movements (
            id, branch_id, product_id, variant_id,
            quantity_delta, quantity_delta_milli,
            quantity_before, quantity_before_milli,
            quantity_after, quantity_after_milli,
            reason, source_type, source_id,
            location_id, bin_id, batch_id, serial_id, user_id,
            created_at
        ) VALUES (
            ?1, ?2, ?3, ?4,
            ?5 / 1000.0, ?5,
            ?6 / 1000.0, ?6,
            ?7 / 1000.0, ?7,
            ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15,
            datetime('now')
        )",
        params![
            movement_id,
            branch_id,
            product_id,
            norm_variant_id,
            delta,
            agg_before,
            agg_after,
            input.reason.as_str(),
            input.source_type,
            input.source_id,
            location_id,
            norm_bin_id,
            norm_batch_id,
            norm_serial_id,
            input.user_id,
        ],
    )?;

    let created_at: String = tx.query_row(
        "SELECT created_at FROM stock_movements WHERE id = ?1",
        params![movement_id],
        |r| r.get(0),
    )?;

    let movement = StockMovement {
        id: movement_id,
        branch_id: branch_id.to_string(),
        product_id: product_id.to_string(),
        variant_id: norm_variant_id.map(|s| s.to_string()),
        quantity_delta_milli: delta,
        quantity_before_milli: Some(agg_before),
        quantity_after_milli: Some(agg_after),
        reason: input.reason.clone(),
        source_type: input.source_type.clone(),
        source_id: input.source_id.clone(),
        location_id: Some(location_id.to_string()),
        bin_id: norm_bin_id.map(|s| s.to_string()),
        batch_id: norm_batch_id.map(|s| s.to_string()),
        serial_id: norm_serial_id.map(|s| s.to_string()),
        user_id: input.user_id.clone(),
        created_at,
    };

    // 14. Persist Idempotency Record
    if let Some(ref key) = input.idempotency_key {
        let clean_key = key.trim();
        let result_json = serde_json::to_string(&movement)?;
        tx.execute(
            "INSERT INTO idempotency_keys (key, operation, request_hash, result_json, created_at)
             VALUES (?1, 'post_stock_movement', ?2, ?3, datetime('now'))",
            params![clean_key, request_hash, result_json],
        )?;
    }

    tx.commit()?;

    Ok(movement)
}

// =========================================================================
// READ DOMAIN QUERIES
// =========================================================================

/// Returns aggregate, allocated spatial, and unallocated stock for a product or variant.
pub fn get_stock_balance(
    conn: &Connection,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
) -> Result<StockBalanceSummary, StockLedgerError> {
    let clean_branch = branch_id.trim();
    let clean_prod = product_id.trim();
    let norm_var = variant_id.map(|s| s.trim()).filter(|s| !s.is_empty());

    let agg_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM inventory
             WHERE branch_id = ?1 AND product_id = ?2
               AND (variant_id = ?3 OR (?3 IS NULL AND variant_id IS NULL))",
            params![clean_branch, clean_prod, norm_var],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);

    let allocated_spatial: i64 = conn.query_row(
        "SELECT COALESCE(SUM(quantity_milli), 0) FROM location_inventory
         WHERE branch_id = ?1 AND product_id = ?2
           AND (variant_id = ?3 OR (?3 IS NULL AND variant_id IS NULL))",
        params![clean_branch, clean_prod, norm_var],
        |row| row.get(0),
    )?;

    let unallocated = agg_qty - allocated_spatial;

    Ok(StockBalanceSummary {
        branch_id: clean_branch.to_string(),
        product_id: clean_prod.to_string(),
        variant_id: norm_var.map(|s| s.to_string()),
        aggregate_quantity_milli: agg_qty,
        allocated_spatial_milli: allocated_spatial,
        unallocated_milli: unallocated,
    })
}

/// Lists spatial location inventory records matching the filter.
pub fn list_location_inventory(
    conn: &Connection,
    filter: &LocationInventoryFilter,
) -> Result<Vec<LocationInventory>, StockLedgerError> {
    let mut query = "SELECT id, branch_id, location_id, product_id, bin_id, variant_id, batch_id, quantity_milli, created_at, updated_at
                     FROM location_inventory WHERE branch_id = ?1".to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(filter.branch_id.clone())];

    if let Some(ref loc) = filter.location_id {
        params_vec.push(Box::new(loc.clone()));
        query.push_str(&format!(" AND location_id = ?{}", params_vec.len()));
    }
    if let Some(ref bin) = filter.bin_id {
        params_vec.push(Box::new(bin.clone()));
        query.push_str(&format!(" AND bin_id = ?{}", params_vec.len()));
    }
    if let Some(ref prod) = filter.product_id {
        params_vec.push(Box::new(prod.clone()));
        query.push_str(&format!(" AND product_id = ?{}", params_vec.len()));
    }
    if let Some(ref var) = filter.variant_id {
        params_vec.push(Box::new(var.clone()));
        query.push_str(&format!(" AND variant_id = ?{}", params_vec.len()));
    }
    if let Some(ref batch) = filter.batch_id {
        params_vec.push(Box::new(batch.clone()));
        query.push_str(&format!(" AND batch_id = ?{}", params_vec.len()));
    }

    query.push_str(" ORDER BY location_id ASC, product_id ASC, bin_id ASC");

    let mut stmt = conn.prepare(&query)?;
    let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();

    let rows = stmt.query_map(rusqlite_params.as_slice(), |row| {
        Ok(LocationInventory {
            id: row.get(0)?,
            branch_id: row.get(1)?,
            location_id: row.get(2)?,
            product_id: row.get(3)?,
            bin_id: row.get(4)?,
            variant_id: row.get(5)?,
            batch_id: row.get(6)?,
            quantity_milli: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

/// Lists stock movements matching the filter.
pub fn list_stock_movements(
    conn: &Connection,
    filter: &StockMovementFilter,
) -> Result<Vec<StockMovement>, StockLedgerError> {
    let mut query = "SELECT id, branch_id, product_id, variant_id, quantity_delta_milli,
                            quantity_before_milli, quantity_after_milli, reason,
                            source_type, source_id, location_id, bin_id, batch_id, serial_id,
                            user_id, created_at
                     FROM stock_movements WHERE branch_id = ?1"
        .to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(filter.branch_id.clone())];

    if let Some(ref prod) = filter.product_id {
        params_vec.push(Box::new(prod.clone()));
        query.push_str(&format!(" AND product_id = ?{}", params_vec.len()));
    }
    if let Some(ref var) = filter.variant_id {
        params_vec.push(Box::new(var.clone()));
        query.push_str(&format!(" AND variant_id = ?{}", params_vec.len()));
    }
    if let Some(ref loc) = filter.location_id {
        params_vec.push(Box::new(loc.clone()));
        query.push_str(&format!(" AND location_id = ?{}", params_vec.len()));
    }
    if let Some(ref bin) = filter.bin_id {
        params_vec.push(Box::new(bin.clone()));
        query.push_str(&format!(" AND bin_id = ?{}", params_vec.len()));
    }
    if let Some(ref batch) = filter.batch_id {
        params_vec.push(Box::new(batch.clone()));
        query.push_str(&format!(" AND batch_id = ?{}", params_vec.len()));
    }
    if let Some(ref ser) = filter.serial_id {
        params_vec.push(Box::new(ser.clone()));
        query.push_str(&format!(" AND serial_id = ?{}", params_vec.len()));
    }
    if let Some(ref reason) = filter.reason {
        params_vec.push(Box::new(reason.as_str().to_string()));
        query.push_str(&format!(" AND reason = ?{}", params_vec.len()));
    }

    query.push_str(" ORDER BY created_at DESC, id DESC");

    let limit = filter.limit.unwrap_or(100).clamp(1, 1000);
    params_vec.push(Box::new(limit));
    query.push_str(&format!(" LIMIT ?{}", params_vec.len()));

    if let Some(offset) = filter.offset {
        params_vec.push(Box::new(offset.max(0)));
        query.push_str(&format!(" OFFSET ?{}", params_vec.len()));
    }

    let mut stmt = conn.prepare(&query)?;
    let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();

    let rows = stmt.query_map(rusqlite_params.as_slice(), |row| {
        let reason_str: String = row.get(7)?;
        let reason = StockMovementReason::from_persisted_str(&reason_str);

        Ok(StockMovement {
            id: row.get(0)?,
            branch_id: row.get(1)?,
            product_id: row.get(2)?,
            variant_id: row.get(3)?,
            quantity_delta_milli: row.get(4)?,
            quantity_before_milli: row.get(5)?,
            quantity_after_milli: row.get(6)?,
            reason,
            source_type: row.get(8)?,
            source_id: row.get(9)?,
            location_id: row.get(10)?,
            bin_id: row.get(11)?,
            batch_id: row.get(12)?,
            serial_id: row.get(13)?,
            user_id: row.get(14)?,
            created_at: row.get(15)?,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

/// Retrieves a single movement by ID scoped to branch.
pub fn get_stock_movement_by_id(
    conn: &Connection,
    branch_id: &str,
    id: &str,
) -> Result<Option<StockMovement>, StockLedgerError> {
    let sql = "SELECT id, branch_id, product_id, variant_id, quantity_delta_milli,
                      quantity_before_milli, quantity_after_milli, reason,
                      source_type, source_id, location_id, bin_id, batch_id, serial_id,
                      user_id, created_at
               FROM stock_movements
               WHERE branch_id = ?1 AND id = ?2";

    let mut stmt = conn.prepare_cached(sql)?;
    let result = stmt
        .query_row(params![branch_id.trim(), id.trim()], |row| {
            let reason_str: String = row.get(7)?;
            let reason = StockMovementReason::from_persisted_str(&reason_str);

            Ok(StockMovement {
                id: row.get(0)?,
                branch_id: row.get(1)?,
                product_id: row.get(2)?,
                variant_id: row.get(3)?,
                quantity_delta_milli: row.get(4)?,
                quantity_before_milli: row.get(5)?,
                quantity_after_milli: row.get(6)?,
                reason,
                source_type: row.get(8)?,
                source_id: row.get(9)?,
                location_id: row.get(10)?,
                bin_id: row.get(11)?,
                batch_id: row.get(12)?,
                serial_id: row.get(13)?,
                user_id: row.get(14)?,
                created_at: row.get(15)?,
            })
        })
        .optional()?;

    Ok(result)
}
