// F2.11 — Stock Ledger & Spatial Balances IPC Command Handlers
// ADR-0013: Scoped authorization via Permission::InventoryAdjust, branch isolation, and fail-closed security.

use crate::auth::middleware::{require_scoped_permission, require_session, AuthorizeRequest};
use crate::db::DbState;
use crate::permission::Permission;
use crate::stock_ledger::{
    LocationInventory, LocationInventoryFilter, PostMovementInput, StockBalanceSummary,
    StockLedgerError, StockLedgerService, StockMovement, StockMovementFilter, StockMovementReason,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;

// =========================================================================
// REQUEST / RESPONSE DTOs
// =========================================================================

/// IPC Request DTO for posting an atomic stock ledger movement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMovementRequest {
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub location_id: String,
    pub bin_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub quantity_delta_milli: i64,
    pub reason: String,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub idempotency_key: Option<String>,
}

/// IPC Request DTO for filtering location inventory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocationInventoryFilterRequest {
    pub branch_id: String,
    pub location_id: Option<String>,
    pub bin_id: Option<String>,
    pub product_id: Option<String>,
    pub variant_id: Option<String>,
    pub batch_id: Option<String>,
}

/// IPC Request DTO for filtering stock movements.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockMovementFilterRequest {
    pub branch_id: String,
    pub product_id: Option<String>,
    pub variant_id: Option<String>,
    pub location_id: Option<String>,
    pub bin_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub reason: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// =========================================================================
// ERROR MAPPING
// =========================================================================

/// Maps domain StockLedgerError to user-facing command error strings, preserving
/// typed diagnostic distinctions without leaking raw database internals.
pub fn map_stock_ledger_error(err: StockLedgerError) -> String {
    match err {
        StockLedgerError::Validation(msg) => format!("Validation error: {msg}"),
        StockLedgerError::NotFound(msg) => format!("Entity not found: {msg}"),
        StockLedgerError::InsufficientStock {
            requested_milli,
            available_milli,
        } => format!(
            "Insufficient stock: requested {requested_milli} milli, available {available_milli} milli"
        ),
        StockLedgerError::IdempotencyConflict(msg) => format!("Idempotency conflict: {msg}"),
        StockLedgerError::InvalidReason(msg) => format!("Invalid reason: {msg}"),
        StockLedgerError::InvalidQuantity(msg) => format!("Invalid quantity: {msg}"),
        StockLedgerError::InvalidLocation(msg) => format!("Invalid location: {msg}"),
        StockLedgerError::InvalidBin(msg) => format!("Invalid bin: {msg}"),
        StockLedgerError::InvalidBatch(msg) => format!("Invalid batch: {msg}"),
        StockLedgerError::InvalidSerial(msg) => format!("Invalid serial: {msg}"),
        StockLedgerError::BranchMismatch(msg) => format!("Branch mismatch: {msg}"),
        StockLedgerError::VariantMismatch(msg) => format!("Variant mismatch: {msg}"),
        StockLedgerError::Database(msg) => format!("Database error: {msg}"),
    }
}

// =========================================================================
// DIRECTLY TESTABLE COMMAND IMPLEMENTATIONS
// =========================================================================

/// Posts an atomic stock movement with InventoryAdjust authorization and branch scope validation.
pub fn post_stock_movement_impl(
    conn: &mut Connection,
    session_id: &str,
    request: PostMovementRequest,
) -> Result<StockMovement, String> {
    let session = require_session(conn, session_id).map_err(|e| e.to_string())?;

    // 1. Authorize: Write requires Permission::InventoryAdjust scoped to the branch
    require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryAdjust,
        None,
        Some(&request.branch_id),
    )
    .map_err(|e| e.to_string())?;

    // 2. Validate branch scope tenancy boundary: user session must be authorized for requested branch
    AuthorizeRequest::new(session_id)
        .with_branch_scope(&request.branch_id)
        .execute(conn)
        .map_err(|e| format!("Branch scope unauthorized: {e}"))?;

    // 3. Parse and validate movement reason string into domain enum
    let reason = StockMovementReason::from_str(&request.reason).map_err(map_stock_ledger_error)?;

    // 4. Construct domain input DTO with user_id bound to the authenticated session
    let input = PostMovementInput {
        branch_id: request.branch_id,
        product_id: request.product_id,
        variant_id: request.variant_id,
        location_id: request.location_id,
        bin_id: request.bin_id,
        batch_id: request.batch_id,
        serial_id: request.serial_id,
        quantity_delta_milli: request.quantity_delta_milli,
        reason,
        source_type: request.source_type,
        source_id: request.source_id,
        user_id: Some(session.user_id),
        idempotency_key: request.idempotency_key,
    };

    // 5. Delegate mutation to StockLedgerService
    StockLedgerService::post_movement(conn, &input).map_err(map_stock_ledger_error)
}

/// Retrieves the comprehensive stock balance (aggregate, spatial, unallocated) for a product or variant.
pub fn get_stock_balance_impl(
    conn: &Connection,
    session_id: &str,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
) -> Result<StockBalanceSummary, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    // Enforce branch tenancy boundary (reads do NOT require InventoryAdjust)
    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| format!("Branch scope unauthorized: {e}"))?;

    StockLedgerService::get_balance(conn, branch_id, product_id, variant_id)
        .map_err(map_stock_ledger_error)
}

/// Lists spatial location inventory balances with branch-scoped access.
pub fn list_location_inventory_impl(
    conn: &Connection,
    session_id: &str,
    request: LocationInventoryFilterRequest,
) -> Result<Vec<LocationInventory>, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    AuthorizeRequest::new(session_id)
        .with_branch_scope(&request.branch_id)
        .execute(conn)
        .map_err(|e| format!("Branch scope unauthorized: {e}"))?;

    let filter = LocationInventoryFilter {
        branch_id: request.branch_id,
        location_id: request.location_id,
        bin_id: request.bin_id,
        product_id: request.product_id,
        variant_id: request.variant_id,
        batch_id: request.batch_id,
    };

    StockLedgerService::list_location_inventory(conn, &filter).map_err(map_stock_ledger_error)
}

/// Lists historical stock movement ledger records with branch-scoped access.
pub fn list_stock_movements_impl(
    conn: &Connection,
    session_id: &str,
    request: StockMovementFilterRequest,
) -> Result<Vec<StockMovement>, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    AuthorizeRequest::new(session_id)
        .with_branch_scope(&request.branch_id)
        .execute(conn)
        .map_err(|e| format!("Branch scope unauthorized: {e}"))?;

    let reason = request.reason.as_deref().map(StockMovementReason::from_persisted_str);

    let filter = StockMovementFilter {
        branch_id: request.branch_id,
        product_id: request.product_id,
        variant_id: request.variant_id,
        location_id: request.location_id,
        bin_id: request.bin_id,
        batch_id: request.batch_id,
        serial_id: request.serial_id,
        reason,
        limit: request.limit,
        offset: request.offset,
    };

    StockLedgerService::list_movements(conn, &filter).map_err(map_stock_ledger_error)
}

/// Retrieves a single stock movement by ID scoped to branch.
pub fn get_stock_movement_impl(
    conn: &Connection,
    session_id: &str,
    branch_id: &str,
    id: &str,
) -> Result<Option<StockMovement>, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| format!("Branch scope unauthorized: {e}"))?;

    StockLedgerService::get_movement(conn, branch_id, id).map_err(map_stock_ledger_error)
}

// =========================================================================
// TAURI IPC COMMAND WRAPPERS
// =========================================================================

/// Tauri IPC command: Posts an atomic stock movement.
#[tauri::command]
pub async fn post_stock_movement(
    state: State<'_, DbState>,
    session_id: String,
    request: PostMovementRequest,
) -> Result<StockMovement, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    post_stock_movement_impl(&mut conn, &session_id, request)
}

/// Tauri IPC command: Retrieves comprehensive stock balance (aggregate, spatial, unallocated).
#[tauri::command]
pub async fn get_stock_balance(
    state: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    product_id: String,
    variant_id: Option<String>,
) -> Result<StockBalanceSummary, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_stock_balance_impl(
        &conn,
        &session_id,
        &branch_id,
        &product_id,
        variant_id.as_deref(),
    )
}

/// Tauri IPC command: Lists spatial location inventory balances.
#[tauri::command]
pub async fn list_location_inventory(
    state: State<'_, DbState>,
    session_id: String,
    request: LocationInventoryFilterRequest,
) -> Result<Vec<LocationInventory>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    list_location_inventory_impl(&conn, &session_id, request)
}

/// Tauri IPC command: Lists historical stock movement ledger records.
#[tauri::command]
pub async fn list_stock_movements(
    state: State<'_, DbState>,
    session_id: String,
    request: StockMovementFilterRequest,
) -> Result<Vec<StockMovement>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    list_stock_movements_impl(&conn, &session_id, request)
}

/// Tauri IPC command: Retrieves a single stock movement by ID.
#[tauri::command]
pub async fn get_stock_movement(
    state: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    id: String,
) -> Result<Option<StockMovement>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_stock_movement_impl(&conn, &session_id, &branch_id, &id)
}
