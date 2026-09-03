// F2.07 — Batches, Expiry Dates & FEFO IPC Command Handlers
// ADR-0009: Scoped authorization, branch isolation, and fail-closed security.

use crate::auth::middleware::{require_scoped_permission, require_session, AuthorizeRequest};
use crate::batch::{CreateBatchInput, FefoAllocationPlan, ProductBatch, UpdateBatchStatusInput};
use crate::db::DbState;
use crate::permission::Permission;
use rusqlite::Connection;
use tauri::State;

// =========================================================================
// DIRECTLY TESTABLE COMMAND IMPLEMENTATIONS
// =========================================================================

/// Creates a new product batch with scoped authorization check.
pub fn create_product_batch_impl(
    conn: &Connection,
    session_id: &str,
    request: &CreateBatchInput,
) -> Result<ProductBatch, String> {
    require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryAdjust,
        None,
        Some(&request.branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::batch::create_batch(conn, request).map_err(|e| e.to_string())
}

/// Retrieves a single batch by ID with branch scope authorization.
pub fn get_product_batch_impl(
    conn: &Connection,
    session_id: &str,
    batch_id: &str,
) -> Result<Option<ProductBatch>, String> {
    let session = require_session(conn, session_id).map_err(|e| e.to_string())?;

    let batch = crate::batch::get_batch(conn, batch_id).map_err(|e| e.to_string())?;

    if let Some(ref b) = batch {
        // Enforce branch tenancy scope boundary to prevent cross-tenant/cross-branch leakage
        AuthorizeRequest::new(session_id)
            .with_branch_scope(&b.branch_id)
            .execute(conn)
            .map_err(|_| {
                format!("Batch '{batch_id}' not found or inaccessible for this session")
            })?;
    } else {
        let _ = session;
    }

    Ok(batch)
}

/// Lists all batches for a specific product and optional variant within a branch.
pub fn list_product_batches_impl(
    conn: &Connection,
    session_id: &str,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
) -> Result<Vec<ProductBatch>, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    crate::batch::list_batches(conn, branch_id, product_id, variant_id).map_err(|e| e.to_string())
}

/// Transitions a batch lifecycle status with branch-scoped inventory permission.
pub fn update_batch_status_impl(
    conn: &Connection,
    session_id: &str,
    request: &UpdateBatchStatusInput,
) -> Result<ProductBatch, String> {
    let existing = crate::batch::get_batch(conn, &request.batch_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Batch '{}' not found", request.batch_id))?;

    require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryAdjust,
        None,
        Some(&existing.branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::batch::update_batch_status(conn, request).map_err(|e| e.to_string())
}

/// Calculates a deterministic FEFO order allocation plan without mutating database state.
pub fn plan_fefo_allocation_impl(
    conn: &Connection,
    session_id: &str,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
    requested_quantity_milli: i64,
) -> Result<FefoAllocationPlan, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    crate::batch::plan_fefo_allocation(
        conn,
        branch_id,
        product_id,
        variant_id,
        requested_quantity_milli,
    )
    .map_err(|e| e.to_string())
}

// =========================================================================
// TAURI IPC COMMAND WRAPPERS
// =========================================================================

/// Tauri IPC command: Creates a new product batch.
#[tauri::command]
pub async fn create_product_batch(
    state: State<'_, DbState>,
    session_id: String,
    request: CreateBatchInput,
) -> Result<ProductBatch, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    create_product_batch_impl(&conn, &session_id, &request)
}

/// Tauri IPC command: Retrieves a single product batch by ID.
#[tauri::command]
pub async fn get_product_batch(
    state: State<'_, DbState>,
    session_id: String,
    batch_id: String,
) -> Result<Option<ProductBatch>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_product_batch_impl(&conn, &session_id, &batch_id)
}

/// Tauri IPC command: Lists product batches for a product and branch.
#[tauri::command]
pub async fn list_product_batches(
    state: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    product_id: String,
    variant_id: Option<String>,
) -> Result<Vec<ProductBatch>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    list_product_batches_impl(
        &conn,
        &session_id,
        &branch_id,
        &product_id,
        variant_id.as_deref(),
    )
}

/// Tauri IPC command: Updates a batch's lifecycle status.
#[tauri::command]
pub async fn update_batch_status(
    state: State<'_, DbState>,
    session_id: String,
    request: UpdateBatchStatusInput,
) -> Result<ProductBatch, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    update_batch_status_impl(&conn, &session_id, &request)
}

/// Tauri IPC command: Calculates a deterministic read-only FEFO allocation plan.
#[tauri::command]
pub async fn plan_fefo_allocation(
    state: State<'_, DbState>,
    session_id: String,
    branch_id: String,
    product_id: String,
    variant_id: Option<String>,
    requested_quantity_milli: i64,
) -> Result<FefoAllocationPlan, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    plan_fefo_allocation_impl(
        &conn,
        &session_id,
        &branch_id,
        &product_id,
        variant_id.as_deref(),
        requested_quantity_milli,
    )
}
