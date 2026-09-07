// F2.11 — Stock Movement Ledger & Spatial Inventory IPC Command Handlers
// ADR-0013: Scoped authorization via Permission::InventoryAdjust, fail-closed security.

use crate::auth::middleware::{require_scoped_permission, require_session, AuthorizeRequest};
use crate::db::DbState;
use crate::permission::Permission;
use crate::stock::{
    BatchSummaryRecord, LocationInventoryRecord, MovementReason, PostMovementRequest,
    StockLedgerService, StockMovementResult, StockSummaryRecord,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

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
    pub notes: Option<String>,
}

pub fn post_stock_movement_impl(
    conn: &mut Connection,
    session_id: &str,
    input: PostMovementInput,
) -> Result<StockMovementResult, String> {
    require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryAdjust,
        None,
        Some(&input.branch_id),
    )
    .map_err(|e| e.to_string())?;

    let session = require_session(conn, session_id).map_err(|e| e.to_string())?;
    let parsed_reason = MovementReason::from_str(&input.reason).map_err(|e| e.to_string())?;

    let req = PostMovementRequest {
        idempotency_key: input.idempotency_key,
        branch_id: input.branch_id,
        product_id: input.product_id,
        variant_id: input.variant_id,
        location_id: input.location_id,
        bin_id: input.bin_id,
        batch_id: input.batch_id,
        serial_id: input.serial_id,
        quantity_delta_milli: input.quantity_delta_milli,
        reason: parsed_reason,
        user_id: Some(session.user_id),
        notes: input.notes,
    };

    StockLedgerService::post_movement(conn, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn post_stock_movement(
    db: State<'_, DbState>,
    session_id: String,
    input: PostMovementInput,
) -> Result<StockMovementResult, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    post_stock_movement_impl(&mut conn, &session_id, input)
}

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

#[tauri::command]
pub fn get_stock_summary(
    db: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    product_id: String,
    variant_id: Option<String>,
) -> Result<StockSummaryRecord, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_stock_summary_impl(&conn, &session_id, &branch_id, &product_id, variant_id.as_deref())
}

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

#[tauri::command]
pub fn get_product_spatial_balances(
    db: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    product_id: String,
) -> Result<Vec<LocationInventoryRecord>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_product_spatial_balances_impl(&conn, &session_id, &branch_id, &product_id)
}

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

    StockLedgerService::get_batch_summary(conn, branch_id, batch_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_batch_summary(
    db: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    batch_id: String,
) -> Result<BatchSummaryRecord, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_batch_summary_impl(&conn, &session_id, &branch_id, &batch_id)
}
