// F2.07 — Batches, Expiry Dates & FEFO Domain Engine
// ADR-0009: Orthogonal capabilities, nullable expiry dates, exact integer milli quantities, and read-only FEFO planning.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Persisted lifecycle status for product batches.
/// Derived 'expired' state is calculated dynamically at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Active,
    Quarantined,
    Recalled,
    Depleted,
}

impl BatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BatchStatus::Active => "active",
            BatchStatus::Quarantined => "quarantined",
            BatchStatus::Recalled => "recalled",
            BatchStatus::Depleted => "depleted",
        }
    }

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

/// Authoritative domain representation of a product batch / lot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductBatch {
    pub id: String,
    pub product_id: String,
    pub branch_id: String,
    pub variant_id: Option<String>,
    pub batch_number: String,
    pub quantity_milli: i64,
    pub cost_price_minor: Option<i64>,
    pub status: BatchStatus,
    pub manufactured_date: Option<String>,
    pub expiry_date: Option<String>,
    pub received_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input payload for creating a new product batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBatchInput {
    pub product_id: String,
    pub branch_id: String,
    pub variant_id: Option<String>,
    pub batch_number: String,
    pub quantity_milli: i64,
    pub cost_price_minor: Option<i64>,
    pub manufactured_date: Option<String>,
    pub expiry_date: Option<String>,
}

/// Input payload for transitioning batch lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBatchStatusInput {
    pub batch_id: String,
    pub status: BatchStatus,
}

/// Single allocation item in a deterministic FEFO plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FefoAllocationLine {
    pub batch_id: String,
    pub batch_number: String,
    pub expiry_date: String,
    pub allocated_quantity_milli: i64,
    pub remaining_batch_quantity_milli: i64,
}

/// Complete FEFO allocation preview plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FefoAllocationPlan {
    pub product_id: String,
    pub branch_id: String,
    pub variant_id: Option<String>,
    pub requested_quantity_milli: i64,
    pub allocated_quantity_milli: i64,
    pub shortfall_quantity_milli: i64,
    pub allocations: Vec<FefoAllocationLine>,
}

/// Typed domain and repository errors for batch operations.
#[derive(Debug, PartialEq, Eq)]
pub enum BatchError {
    Validation(String),
    IneligibleProduct(String),
    NotFound(String),
    DuplicateBatchNumber(String),
    InvalidStatusTransition(String),
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

/// Determines whether a product is eligible to track batches/lots.
/// Enabled if product has BATCH capability, or requires_expiry = 1, or EXPIRY/FEFO capability.
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
/// Required if products.requires_expiry = 1, or has active EXPIRY or FEFO capability.
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
/// Strictly controlled by active FEFO capability in product_capabilities.
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

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
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
// BATCH DOMAIN CRUD OPERATIONS
// =========================================================================

/// Creates a new product batch after validating capability eligibility,
/// date formats, variant ownership, and branch constraints.
pub fn create_batch(
    conn: &Connection,
    input: &CreateBatchInput,
) -> Result<ProductBatch, BatchError> {
    let product_id = input.product_id.trim();
    let branch_id = input.branch_id.trim();
    let batch_number = input.batch_number.trim();

    if batch_number.is_empty() {
        return Err(BatchError::Validation(
            "Batch number cannot be empty or whitespace".into(),
        ));
    }
    if batch_number.len() > 100 {
        return Err(BatchError::Validation(
            "Batch number exceeds maximum length of 100 characters".into(),
        ));
    }

    if input.quantity_milli < 0 {
        return Err(BatchError::Validation(
            "Batch quantity cannot be negative".into(),
        ));
    }

    if let Some(cost) = input.cost_price_minor {
        if cost < 0 {
            return Err(BatchError::Validation(
                "Batch cost price cannot be negative".into(),
            ));
        }
    }

    // 1. Verify branch exists and is active
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

    // 2. Check product batch eligibility
    if !is_batch_tracked(conn, product_id)? {
        return Err(BatchError::IneligibleProduct(format!(
            "Product '{product_id}' is not configured for batch or lot tracking"
        )));
    }

    // 3. Check expiry requirement
    let expiry_required = is_expiry_required(conn, product_id)?;
    let normalized_expiry = match &input.expiry_date {
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

    // 4. Validate manufactured date if present
    let normalized_mfg = match &input.manufactured_date {
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

    // 5. Validate variant if present
    let normalized_variant_id = match &input.variant_id {
        Some(vid) => {
            let vid = vid.trim();
            if vid.is_empty() {
                None
            } else {
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
                Some(vid.to_string())
            }
        }
        None => None,
    };

    // 6. Check unique batch number within (branch_id, product_id, variant_id)
    let duplicate_exists: bool = if let Some(ref vid) = normalized_variant_id {
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
             WHERE branch_id = ?1 AND product_id = ?2 AND variant_id IS NULL AND batch_number = ?4 COLLATE NOCASE",
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

    let initial_status = if input.quantity_milli == 0 {
        BatchStatus::Depleted
    } else {
        BatchStatus::Active
    };

    conn.execute(
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
    )?;

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

    // State machine transitions:
    // active -> quarantined, recalled, depleted
    // quarantined -> active, recalled
    // recalled -> terminal
    // depleted -> terminal
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

    // 1. Check if FEFO is explicitly enabled for this product
    if !is_fefo_enabled(conn, product_id)? {
        return Err(BatchError::Validation(format!(
            "Product '{product_id}' does not have the FEFO capability enabled"
        )));
    }

    // 2. Query active, unexpired, non-depleted candidate batches in deterministic FEFO order:
    // expiry_date ASC, received_at ASC, id ASC
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
            let batch_number: String = row.get(1)?;
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
    let batch_number: String = row.get(4)?;
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
