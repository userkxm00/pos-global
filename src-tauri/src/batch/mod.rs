// F2.07 — Batches, Expiry Dates & FEFO Domain Engine
// ADR-0009: Orthogonal capabilities, nullable expiry dates, exact integer milli quantities, and read-only FEFO planning.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Persisted lifecycle status for product batches.
///
/// Allowed statuses: `active`, `quarantined`, `recalled`, `depleted`.
/// Expiration is a derived runtime state evaluated at query time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Batch is available for normal inventory operations.
    Active,
    /// Batch is quarantined and excluded from allocation or sale.
    Quarantined,
    /// Batch is permanently recalled (terminal state).
    Recalled,
    /// Batch balance is depleted (terminal state in F2.07).
    Depleted,
}

impl BatchStatus {
    /// Returns the static lowercase string representation of the batch status.
    pub fn as_str(&self) -> &'static str {
        match self {
            BatchStatus::Active => "active",
            BatchStatus::Quarantined => "quarantined",
            BatchStatus::Recalled => "recalled",
            BatchStatus::Depleted => "depleted",
        }
    }

    /// Parses a status string into a typed `BatchStatus` enum value.
    pub fn from_str(s: &str) -> Result<Self, BatchError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "active" => Ok(BatchStatus::Active),
            "quarantined" => Ok(BatchStatus::Quarantined),
            "recalled" => Ok(BatchStatus::Recalled),
            "depleted" => Ok(BatchStatus::Depleted),
            other => Err(BatchError::Validation(format!(
                "Invalid batch status '{other}'. Allowed: active, quarantined, recalled, depleted"
            ))),
        }
    }
}

/// Authoritative domain representation of a product batch or lot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductBatch {
    /// Unique identifier for the batch record.
    pub id: String,
    /// Product ID this batch belongs to.
    pub product_id: String,
    /// Branch ID where this batch is physically held.
    pub branch_id: String,
    /// Optional variant ID if this batch is for a specific product variant.
    pub variant_id: Option<String>,
    /// Batch or lot number. Nullable for legacy database rows; strictly required on new creation.
    pub batch_number: Option<String>,
    /// Available quantity in thousandths of the product's base unit.
    pub quantity_milli: i64,
    /// Optional acquisition unit cost in minor currency units (cents).
    pub cost_price_minor: Option<i64>,
    /// Operational lifecycle status.
    pub status: BatchStatus,
    /// Optional manufacturing date in YYYY-MM-DD format.
    pub manufactured_date: Option<String>,
    /// Optional expiration date in YYYY-MM-DD format. Mandatory for perishable products.
    pub expiry_date: Option<String>,
    /// Timestamp when the batch was received into the branch.
    pub received_at: String,
    /// Timestamp when the record was created.
    pub created_at: String,
    /// Timestamp when the record was last updated.
    pub updated_at: String,
}

/// Input payload for creating a new product batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBatchInput {
    /// Product ID to associate with the batch.
    pub product_id: String,
    /// Branch ID where the batch is received.
    pub branch_id: String,
    /// Optional variant ID for variant-specific lots.
    pub variant_id: Option<String>,
    /// Lot or batch identifier (cannot be empty or whitespace).
    pub batch_number: String,
    /// Initial received quantity in thousandths of the canonical unit.
    pub quantity_milli: i64,
    /// Optional unit cost price in minor currency units.
    pub cost_price_minor: Option<i64>,
    /// Optional production/manufacturing calendar date (YYYY-MM-DD).
    pub manufactured_date: Option<String>,
    /// Expiration calendar date (YYYY-MM-DD). Required if product requires expiry.
    pub expiry_date: Option<String>,
}

/// Input payload for transitioning a batch's lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBatchStatusInput {
    /// Unique ID of the batch to update.
    pub batch_id: String,
    /// New target lifecycle status.
    pub status: BatchStatus,
}

/// A single allocation segment within a deterministic FEFO allocation plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FefoAllocationLine {
    /// ID of the allocated batch.
    pub batch_id: String,
    /// Batch/lot number of the allocated batch.
    pub batch_number: Option<String>,
    /// Expiration date of the allocated batch (YYYY-MM-DD).
    pub expiry_date: String,
    /// Allocated quantity in thousandths of a unit.
    pub allocated_quantity_milli: i64,
    /// Projected remaining balance in the batch after this allocation.
    pub remaining_batch_quantity_milli: i64,
}

/// Complete preview plan for FEFO order allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FefoAllocationPlan {
    /// Product ID requested.
    pub product_id: String,
    /// Branch ID requested.
    pub branch_id: String,
    /// Variant ID requested, if any.
    pub variant_id: Option<String>,
    /// Total requested quantity in milli-units.
    pub requested_quantity_milli: i64,
    /// Total quantity successfully planned across available active batches.
    pub allocated_quantity_milli: i64,
    /// Quantity demand that could not be satisfied (shortfall).
    pub shortfall_quantity_milli: i64,
    /// Deterministic list of batch allocations in FEFO order.
    pub allocations: Vec<FefoAllocationLine>,
}

/// Typed domain and repository errors for batch operations.
#[derive(Debug, PartialEq, Eq)]
pub enum BatchError {
    /// Validation failure on input parameters or domain invariants.
    Validation(String),
    /// Attempted batch operation on a product not configured for batch tracking.
    IneligibleProduct(String),
    /// Resource not found or inaccessible.
    NotFound(String),
    /// Duplicate batch number within the branch, product, and variant scope.
    DuplicateBatchNumber(String),
    /// Illegal lifecycle transition attempted.
    InvalidStatusTransition(String),
    /// Underlying persistence or database error.
    Database(String),
}

impl std::fmt::Display for BatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchError::Validation(msg) => write!(f, "Validation error: {msg}"),
            BatchError::IneligibleProduct(msg) => write!(f, "Ineligible product error: {msg}"),
            BatchError::NotFound(msg) => write!(f, "Not found: {msg}"),
            BatchError::DuplicateBatchNumber(msg) => write!(f, "Duplicate batch number: {msg}"),
            BatchError::InvalidStatusTransition(msg) => {
                write!(f, "Invalid status transition: {msg}")
            }
            BatchError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for BatchError {}

impl From<rusqlite::Error> for BatchError {
    fn from(err: rusqlite::Error) -> Self {
        BatchError::Database(err.to_string())
    }
}

// =========================================================================
// CAPABILITY VERIFICATION HELPERS (ORTHOGONAL CAPABILITY MODEL)
// =========================================================================

/// Determines whether a product is eligible to track batches or lots.
///
/// True if `products.requires_expiry = 1`, or if the product has an active
/// `'BATCH'`, `'EXPIRY'`, or `'FEFO'` capability in `product_capabilities`.
pub fn is_batch_tracked(conn: &Connection, product_id: &str) -> Result<bool, BatchError> {
    let product_id = product_id.trim();
    let req_exp: Option<bool> = conn
        .query_row(
            "SELECT requires_expiry FROM products WHERE id = ?1 AND is_active = 1",
            params![product_id],
            |row| {
                let val: i32 = row.get(0)?;
                Ok(val != 0)
            },
        )
        .optional()?;

    let Some(req_exp) = req_exp else {
        return Err(BatchError::NotFound(format!(
            "Product '{product_id}' not found or is inactive"
        )));
    };

    if req_exp {
        return Ok(true);
    }

    let has_cap: bool = conn
        .query_row(
            "SELECT 1 FROM product_capabilities pc
             JOIN capabilities c ON pc.capability_id = c.id
             WHERE pc.product_id = ?1
               AND c.code IN ('BATCH', 'EXPIRY', 'FEFO')
               AND pc.enabled = 1
             LIMIT 1",
            params![product_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    Ok(has_cap)
}

/// Determines whether a product requires an expiration date upon batch creation.
///
/// True if `products.requires_expiry = 1` or if an active `'EXPIRY'` or `'FEFO'`
/// capability is configured for the product.
pub fn is_expiry_required(conn: &Connection, product_id: &str) -> Result<bool, BatchError> {
    let product_id = product_id.trim();
    let req_exp: Option<bool> = conn
        .query_row(
            "SELECT requires_expiry FROM products WHERE id = ?1 AND is_active = 1",
            params![product_id],
            |row| {
                let val: i32 = row.get(0)?;
                Ok(val != 0)
            },
        )
        .optional()?;

    let Some(req_exp) = req_exp else {
        return Err(BatchError::NotFound(format!(
            "Product '{product_id}' not found or is inactive"
        )));
    };

    if req_exp {
        return Ok(true);
    }

    let has_cap: bool = conn
        .query_row(
            "SELECT 1 FROM product_capabilities pc
             JOIN capabilities c ON pc.capability_id = c.id
             WHERE pc.product_id = ?1
               AND c.code IN ('EXPIRY', 'FEFO')
               AND pc.enabled = 1
             LIMIT 1",
            params![product_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    Ok(has_cap)
}

/// Determines whether FEFO allocation planning is enabled for a product.
///
/// Strictly controlled by the active `'FEFO'` capability in `product_capabilities`.
pub fn is_fefo_enabled(conn: &Connection, product_id: &str) -> Result<bool, BatchError> {
    let product_id = product_id.trim();
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM products WHERE id = ?1 AND is_active = 1",
            params![product_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    if !exists {
        return Err(BatchError::NotFound(format!(
            "Product '{product_id}' not found or is inactive"
        )));
    }

    let has_fefo: bool = conn
        .query_row(
            "SELECT 1 FROM product_capabilities pc
             JOIN capabilities c ON pc.capability_id = c.id
             WHERE pc.product_id = ?1
               AND c.code = 'FEFO'
               AND pc.enabled = 1
             LIMIT 1",
            params![product_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    Ok(has_fefo)
}

// =========================================================================
// DATE VALIDATION
// =========================================================================

/// Validates strict ISO-8601 calendar date YYYY-MM-DD.
///
/// Enforces character length, hyphens at positions 5 and 8, valid year (1900-2999),
/// valid month (1-12), and leap-year aware days-per-month limits.
pub fn validate_iso_calendar_date(s: &str) -> Result<(), BatchError> {
    let s = s.trim();
    if s.len() != 10 {
        return Err(BatchError::Validation(format!(
            "Date '{s}' must be exactly 10 characters in YYYY-MM-DD format"
        )));
    }

    let bytes = s.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(BatchError::Validation(format!(
            "Date '{s}' must have hyphens at positions 5 and 8 (YYYY-MM-DD)"
        )));
    }

    let year: u32 = s[0..4]
        .parse()
        .map_err(|_| BatchError::Validation(format!("Invalid year in date '{s}'")))?;
    let month: u32 = s[5..7]
        .parse()
        .map_err(|_| BatchError::Validation(format!("Invalid month in date '{s}'")))?;
    let day: u32 = s[8..10]
        .parse()
        .map_err(|_| BatchError::Validation(format!("Invalid day in date '{s}'")))?;

    if !(1900..=2999).contains(&year) {
        return Err(BatchError::Validation(format!(
            "Year {year} in date '{s}' is outside permitted range [1900, 2999]"
        )));
    }
    if !(1..=12).contains(&month) {
        return Err(BatchError::Validation(format!(
            "Month {month} in date '{s}' must be between 1 and 12"
        )));
    }

    let is_leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let max_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap {
                29
            } else {
                28
            }
        }
        _ => 30,
    };

    if day < 1 || day > max_days {
        return Err(BatchError::Validation(format!(
            "Day {day} is invalid for month {month} in year {year} in date '{s}'"
        )));
    }

    Ok(())
}

// =========================================================================
// PRIVATE VALIDATION HELPERS (COGNITIVE COMPLEXITY REDUCTION)
// =========================================================================

fn validate_batch_number(raw: &str) -> Result<String, BatchError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(BatchError::Validation(
            "Batch number cannot be empty or whitespace".into(),
        ));
    }
    if trimmed.len() > 100 {
        return Err(BatchError::Validation(
            "Batch number exceeds maximum length of 100 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_batch_quantities(
    quantity_milli: i64,
    cost_price_minor: Option<i64>,
) -> Result<(), BatchError> {
    if quantity_milli < 0 {
        return Err(BatchError::Validation(
            "Batch quantity cannot be negative".into(),
        ));
    }
    if let Some(cost) = cost_price_minor {
        if cost < 0 {
            return Err(BatchError::Validation(
                "Batch cost price cannot be negative".into(),
            ));
        }
    }
    Ok(())
}

fn validate_batch_dates(
    conn: &Connection,
    product_id: &str,
    manufactured_date: Option<&str>,
    expiry_date: Option<&str>,
) -> Result<(Option<String>, Option<String>), BatchError> {
    let expiry_required = is_expiry_required(conn, product_id)?;
    let normalized_expiry = match expiry_date {
        Some(exp) => {
            let exp = exp.trim();
            if exp.is_empty() {
                if expiry_required {
                    return Err(BatchError::Validation(
                        "Expiry date is mandatory for this product".into(),
                    ));
                }
                None
            } else {
                validate_iso_calendar_date(exp)?;
                Some(exp.to_string())
            }
        }
        None => {
            if expiry_required {
                return Err(BatchError::Validation(
                    "Expiry date is mandatory for this product".into(),
                ));
            }
            None
        }
    };

    let normalized_mfg = match manufactured_date {
        Some(mfg) => {
            let mfg = mfg.trim();
            if mfg.is_empty() {
                None
            } else {
                validate_iso_calendar_date(mfg)?;
                if let Some(ref exp) = normalized_expiry {
                    if mfg >= exp.as_str() {
                        return Err(BatchError::Validation(format!(
                            "Manufactured date '{mfg}' must be strictly before expiry date '{exp}'"
                        )));
                    }
                }
                Some(mfg.to_string())
            }
        }
        None => None,
    };

    Ok((normalized_mfg, normalized_expiry))
}

fn validate_batch_variant(
    conn: &Connection,
    product_id: &str,
    variant_id: Option<&str>,
) -> Result<Option<String>, BatchError> {
    let Some(vid) = variant_id else {
        return Ok(None);
    };
    let vid = vid.trim();
    if vid.is_empty() {
        return Ok(None);
    }

    let var_prod_id: Option<String> = conn
        .query_row(
            "SELECT product_id FROM product_variants WHERE id = ?1 AND deleted_at IS NULL",
            params![vid],
            |row| row.get(0),
        )
        .optional()?;

    let Some(var_prod_id) = var_prod_id else {
        return Err(BatchError::NotFound(format!(
            "Variant '{vid}' not found or is soft-deleted"
        )));
    };

    if var_prod_id != product_id {
        return Err(BatchError::Validation(format!(
            "Variant '{vid}' belongs to product '{var_prod_id}', not '{product_id}'"
        )));
    }

    Ok(Some(vid.to_string()))
}

fn check_batch_uniqueness(
    conn: &Connection,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
    batch_number: &str,
) -> Result<(), BatchError> {
    let duplicate_exists: bool = if let Some(vid) = variant_id {
        conn.query_row(
            "SELECT 1 FROM product_batches
             WHERE branch_id = ?1 AND product_id = ?2 AND variant_id = ?3 AND batch_number = ?4 COLLATE NOCASE",
            params![branch_id, product_id, vid, batch_number],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false)
    } else {
        conn.query_row(
            "SELECT 1 FROM product_batches
             WHERE branch_id = ?1 AND product_id = ?2 AND variant_id IS NULL AND batch_number = ?3 COLLATE NOCASE",
            params![branch_id, product_id, batch_number],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false)
    };

    if duplicate_exists {
        return Err(BatchError::DuplicateBatchNumber(format!(
            "Batch number '{batch_number}' already exists for this product in branch '{branch_id}'"
        )));
    }

    Ok(())
}

// =========================================================================
// BATCH DOMAIN CRUD OPERATIONS
// =========================================================================

/// Creates a new product batch after validating capability eligibility,
/// date formats, variant ownership, branch scope, and uniqueness invariants.
pub fn create_batch(
    conn: &Connection,
    input: &CreateBatchInput,
) -> Result<ProductBatch, BatchError> {
    let product_id = input.product_id.trim();
    let branch_id = input.branch_id.trim();

    let batch_number = validate_batch_number(&input.batch_number)?;
    validate_batch_quantities(input.quantity_milli, input.cost_price_minor)?;

    let branch_exists: bool = conn
        .query_row(
            "SELECT 1 FROM branches WHERE id = ?1",
            params![branch_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    if !branch_exists {
        return Err(BatchError::NotFound(format!(
            "Branch '{branch_id}' not found"
        )));
    }

    if !is_batch_tracked(conn, product_id)? {
        return Err(BatchError::IneligibleProduct(format!(
            "Product '{product_id}' is not configured for batch or lot tracking"
        )));
    }

    let (normalized_mfg, normalized_expiry) = validate_batch_dates(
        conn,
        product_id,
        input.manufactured_date.as_deref(),
        input.expiry_date.as_deref(),
    )?;

    let normalized_variant_id =
        validate_batch_variant(conn, product_id, input.variant_id.as_deref())?;

    check_batch_uniqueness(
        conn,
        branch_id,
        product_id,
        normalized_variant_id.as_deref(),
        &batch_number,
    )?;

    let initial_status = if input.quantity_milli == 0 {
        BatchStatus::Depleted
    } else {
        BatchStatus::Active
    };

    let insert_res = conn.execute(
        "INSERT INTO product_batches (
            product_id, branch_id, variant_id, batch_number, quantity_milli,
            cost_price_minor, status, manufactured_date, expiry_date,
            created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'), datetime('now')
        )",
        params![
            product_id,
            branch_id,
            normalized_variant_id,
            batch_number,
            input.quantity_milli,
            input.cost_price_minor,
            initial_status.as_str(),
            normalized_mfg,
            normalized_expiry,
        ],
    );

    if let Err(rusqlite::Error::SqliteFailure(err, Some(ref msg))) = &insert_res {
        if err.code == rusqlite::ErrorCode::ConstraintViolation && msg.contains("UNIQUE") {
            return Err(BatchError::DuplicateBatchNumber(format!(
                "Batch number '{batch_number}' already exists for this product in branch '{branch_id}'"
            )));
        }
    }
    insert_res?;

    let rowid = conn.last_insert_rowid();
    let batch = conn.query_row(
        "SELECT id, product_id, branch_id, variant_id, batch_number, quantity_milli,
                cost_price_minor, status, manufactured_date, expiry_date, received_at,
                created_at, updated_at
         FROM product_batches WHERE rowid = ?1",
        params![rowid],
        map_batch_row,
    )?;

    Ok(batch)
}

/// Retrieves a single batch by ID.
pub fn get_batch(conn: &Connection, batch_id: &str) -> Result<Option<ProductBatch>, BatchError> {
    let batch_id = batch_id.trim();
    conn.query_row(
        "SELECT id, product_id, branch_id, variant_id, batch_number, quantity_milli,
                cost_price_minor, status, manufactured_date, expiry_date, received_at,
                created_at, updated_at
         FROM product_batches WHERE id = ?1",
        params![batch_id],
        map_batch_row,
    )
    .optional()
    .map_err(BatchError::from)
}

/// Lists all batches for a specific product and optional variant within a branch.
pub fn list_batches(
    conn: &Connection,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
) -> Result<Vec<ProductBatch>, BatchError> {
    let branch_id = branch_id.trim();
    let product_id = product_id.trim();

    let mut stmt = if let Some(vid) = variant_id {
        let vid = vid.trim();
        let mut s = conn.prepare(
            "SELECT id, product_id, branch_id, variant_id, batch_number, quantity_milli,
                    cost_price_minor, status, manufactured_date, expiry_date, received_at,
                    created_at, updated_at
             FROM product_batches
             WHERE branch_id = ?1 AND product_id = ?2 AND variant_id = ?3
             ORDER BY received_at DESC, id ASC",
        )?;
        let rows = s.query_map(params![branch_id, product_id, vid], map_batch_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        return Ok(list);
    } else {
        conn.prepare(
            "SELECT id, product_id, branch_id, variant_id, batch_number, quantity_milli,
                    cost_price_minor, status, manufactured_date, expiry_date, received_at,
                    created_at, updated_at
             FROM product_batches
             WHERE branch_id = ?1 AND product_id = ?2
             ORDER BY received_at DESC, id ASC",
        )?
    };

    let rows = stmt.query_map(params![branch_id, product_id], map_batch_row)?;
    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

/// Transitions batch lifecycle status according to the approved state machine.
///
/// Transitions:
/// - `active` -> `quarantined`, `recalled`, `depleted`
/// - `quarantined` -> `active`, `recalled`
/// - `recalled` -> terminal
/// - `depleted` -> terminal
pub fn update_batch_status(
    conn: &Connection,
    input: &UpdateBatchStatusInput,
) -> Result<ProductBatch, BatchError> {
    let batch_id = input.batch_id.trim();

    let current = get_batch(conn, batch_id)?;
    let Some(current) = current else {
        return Err(BatchError::NotFound(format!(
            "Batch '{batch_id}' not found"
        )));
    };

    if current.status == input.status {
        return Ok(current);
    }

    match (current.status, input.status) {
        (BatchStatus::Active, BatchStatus::Quarantined)
        | (BatchStatus::Active, BatchStatus::Recalled)
        | (BatchStatus::Active, BatchStatus::Depleted)
        | (BatchStatus::Quarantined, BatchStatus::Active)
        | (BatchStatus::Quarantined, BatchStatus::Recalled) => {
            conn.execute(
                "UPDATE product_batches
                 SET status = ?1, updated_at = datetime('now')
                 WHERE id = ?2",
                params![input.status.as_str(), batch_id],
            )?;
        }
        (BatchStatus::Recalled, _) => {
            return Err(BatchError::InvalidStatusTransition(
                "Recalled batches are terminal and cannot be transitioned to any other status"
                    .into(),
            ));
        }
        (BatchStatus::Depleted, _) => {
            return Err(BatchError::InvalidStatusTransition(
                "Depleted batches are terminal and cannot be arbitrarily reopened in F2.07".into(),
            ));
        }
        (from, to) => {
            return Err(BatchError::InvalidStatusTransition(format!(
                "Transition from '{}' to '{}' is not permitted",
                from.as_str(),
                to.as_str()
            )));
        }
    }

    let updated = get_batch(conn, batch_id)?.ok_or_else(|| {
        BatchError::NotFound(format!("Batch '{batch_id}' not found after update"))
    })?;
    Ok(updated)
}

// =========================================================================
// FEFO ALLOCATION PLANNING (READ-ONLY CALCULATION)
// =========================================================================

/// Calculates a deterministic First-Expire, First-Out (FEFO) allocation plan
/// for a requested product quantity without modifying any database rows.
///
/// Invariant: Does NOT mutate batch balances or insert stock movement ledger rows.
pub fn plan_fefo_allocation(
    conn: &Connection,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
    requested_quantity_milli: i64,
) -> Result<FefoAllocationPlan, BatchError> {
    let branch_id = branch_id.trim();
    let product_id = product_id.trim();

    if requested_quantity_milli <= 0 {
        return Err(BatchError::Validation(
            "Requested allocation quantity must be strictly positive".into(),
        ));
    }

    if !is_fefo_enabled(conn, product_id)? {
        return Err(BatchError::Validation(format!(
            "Product '{product_id}' does not have the FEFO capability enabled"
        )));
    }

    let mut stmt = conn.prepare(
        "SELECT id, batch_number, expiry_date, quantity_milli
         FROM product_batches
         WHERE branch_id = ?1
           AND product_id = ?2
           AND (variant_id = ?3 OR (?3 IS NULL AND variant_id IS NULL))
           AND status = 'active'
           AND quantity_milli > 0
           AND expiry_date IS NOT NULL
           AND expiry_date >= strftime('%Y-%m-%d', 'now')
         ORDER BY expiry_date ASC, received_at ASC, id ASC",
    )?;

    let candidate_rows = stmt.query_map(
        params![branch_id, product_id, variant_id.map(|v| v.trim())],
        |row| {
            let id: String = row.get(0)?;
            let batch_number: Option<String> = row.get(1)?;
            let expiry_date: String = row.get(2)?;
            let quantity_milli: i64 = row.get(3)?;
            Ok((id, batch_number, expiry_date, quantity_milli))
        },
    )?;

    let mut allocations = Vec::new();
    let mut remaining_demand = requested_quantity_milli;

    for candidate in candidate_rows {
        if remaining_demand == 0 {
            break;
        }

        let (b_id, b_num, exp_date, b_qty) = candidate?;
        let allocated_qty = std::cmp::min(remaining_demand, b_qty);
        let remaining_in_batch = b_qty - allocated_qty;

        allocations.push(FefoAllocationLine {
            batch_id: b_id,
            batch_number: b_num,
            expiry_date: exp_date,
            allocated_quantity_milli: allocated_qty,
            remaining_batch_quantity_milli: remaining_in_batch,
        });

        remaining_demand -= allocated_qty;
    }

    let allocated_total = requested_quantity_milli - remaining_demand;

    Ok(FefoAllocationPlan {
        product_id: product_id.to_string(),
        branch_id: branch_id.to_string(),
        variant_id: variant_id.map(|v| v.trim().to_string()),
        requested_quantity_milli,
        allocated_quantity_milli: allocated_total,
        shortfall_quantity_milli: remaining_demand,
        allocations,
    })
}

// =========================================================================
// ROW MAPPING HELPER
// =========================================================================

fn map_batch_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductBatch> {
    let id: String = row.get(0)?;
    let product_id: String = row.get(1)?;
    let branch_id: String = row.get(2)?;
    let variant_id: Option<String> = row.get(3)?;
    let batch_number: Option<String> = row.get(4)?;
    let quantity_milli: i64 = row.get(5)?;
    let cost_price_minor: Option<i64> = row.get(6)?;
    let status_str: String = row.get(7)?;
    let manufactured_date: Option<String> = row.get(8)?;
    let expiry_date: Option<String> = row.get(9)?;
    let received_at: String = row.get(10)?;
    let created_at: String = row.get(11)?;
    let updated_at: String = row.get(12)?;

    let status = BatchStatus::from_str(&status_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Ok(ProductBatch {
        id,
        product_id,
        branch_id,
        variant_id,
        batch_number,
        quantity_milli,
        cost_price_minor,
        status,
        manufactured_date,
        expiry_date,
        received_at,
        created_at,
        updated_at,
    })
}
