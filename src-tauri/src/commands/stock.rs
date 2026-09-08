// F2.11 — Stock Movement Ledger & Spatial Balances IPC Commands
// ADR-0013: Scoped authorization, branch isolation, and fail-closed security.

use crate::auth::middleware::{require_scoped_permission, AuthorizeRequest};
use crate::db::DbState;
use crate::permission::Permission;
use crate::stock::{
    BatchSummaryRecord, LocationInventoryRecord, MovementReason, PostMovementRequest,
    StockLedgerService, StockMovementResult, StockSummaryRecord,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

// =========================================================================
// INPUT DTOs
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMovementInput {
    pub idempotency_key: String,
    pub branch_id: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub location_id: String,
    pub bin_id: Option<String>,
    pub batch_id: Option<String>,
    pub serial_id: Option<String>,
    pub quantity_delta_milli: i64,
    pub reason: String,
}

// =========================================================================
// DIRECTLY TESTABLE COMMAND IMPLEMENTATIONS
// =========================================================================

/// Posts a stock movement with branch-scoped inventory adjustment permission.
pub fn post_stock_movement_impl(
    conn: &mut Connection,
    session_id: &str,
    input: &PostMovementInput,
) -> Result<StockMovementResult, String> {
    let session = require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryAdjust,
        None,
        Some(&input.branch_id),
    )
    .map_err(|e| e.to_string())?;

    let reason = MovementReason::from_str(&input.reason).map_err(|e| e.to_string())?;

    let req = PostMovementRequest {
        idempotency_key: input.idempotency_key.clone(),
        branch_id: input.branch_id.clone(),
        product_id: input.product_id.clone(),
        variant_id: input.variant_id.clone(),
        location_id: input.location_id.clone(),
        bin_id: input.bin_id.clone(),
        batch_id: input.batch_id.clone(),
        serial_id: input.serial_id.clone(),
        quantity_delta_milli: input.quantity_delta_milli,
        reason,
        user_id: Some(session.user_id),
    };

    StockLedgerService::post_movement(conn, &req).map_err(|e| e.to_string())
}

/// Retrieves aggregate, spatial allocated, and unallocated stock summary with branch isolation.
pub fn get_stock_summary_impl(
    conn: &Connection,
    session_id: &str,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
) -> Result<StockSummaryRecord, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    StockLedgerService::get_stock_summary(conn, branch_id, product_id, variant_id)
        .map_err(|e| e.to_string())
}

/// Retrieves all spatial slot inventory rows for a product within a branch.
pub fn get_product_spatial_balances_impl(
    conn: &Connection,
    session_id: &str,
    branch_id: &str,
    product_id: &str,
) -> Result<Vec<LocationInventoryRecord>, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    StockLedgerService::get_product_spatial_balances(conn, branch_id, product_id)
        .map_err(|e| e.to_string())
}

/// Retrieves batch summary (total, spatial allocated, unallocated) with branch isolation.
pub fn get_batch_summary_impl(
    conn: &Connection,
    session_id: &str,
    branch_id: &str,
    batch_id: &str,
) -> Result<BatchSummaryRecord, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    StockLedgerService::get_batch_summary(conn, branch_id, batch_id).map_err(|e| e.to_string())
}

// =========================================================================
// TAURI IPC COMMAND WRAPPERS
// =========================================================================

/// Tauri IPC command: Posts a stock movement.
#[tauri::command]
pub async fn post_stock_movement(
    state: State<'_, DbState>,
    session_id: String,
    input: PostMovementInput,
) -> Result<StockMovementResult, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    post_stock_movement_impl(&mut conn, &session_id, &input)
}

/// Tauri IPC command: Retrieves stock summary for a product.
#[tauri::command]
pub async fn get_stock_summary(
    state: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    product_id: String,
    variant_id: Option<String>,
) -> Result<StockSummaryRecord, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_stock_summary_impl(
        &conn,
        &session_id,
        &branch_id,
        &product_id,
        variant_id.as_deref(),
    )
}

/// Tauri IPC command: Retrieves spatial inventory slot balances for a product.
#[tauri::command]
pub async fn get_product_spatial_balances(
    state: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    product_id: String,
) -> Result<Vec<LocationInventoryRecord>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_product_spatial_balances_impl(&conn, &session_id, &branch_id, &product_id)
}

/// Tauri IPC command: Retrieves batch total and spatial breakdown.
#[tauri::command]
pub async fn get_batch_summary(
    state: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    batch_id: String,
) -> Result<BatchSummaryRecord, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_batch_summary_impl(&conn, &session_id, &branch_id, &batch_id)
}
