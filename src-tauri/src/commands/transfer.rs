// F2.12 — Stock Transfers IPC Command Handlers
// ADR-0014: Scoped authorization via Permission::InventoryTransfer, branch isolation, and fail-closed security.

use crate::auth::middleware::require_scoped_permission;
use crate::db::DbState;
use crate::permission::Permission;
use crate::transfer::{
    CancelTransferInput, CreateTransferInput, CreateTransferItemInput, DispatchTransferInput,
    InstantIntraBranchTransferInput, ReceiveTransferInput, StockTransfer, TransferError,
    TransferFilter, TransferService, TransferStatus, TransferType,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;

// =========================================================================
// REQUEST DTOs
// =========================================================================

/// IPC Request DTO for creating a new stock transfer document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockTransferRequest {
    pub transfer_type: String,
    pub source_branch_id: String,
    pub destination_branch_id: String,
    pub source_location_id: String,
    pub destination_location_id: String,
    pub source_bin_id: Option<String>,
    pub destination_bin_id: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateTransferItemInput>,
    pub idempotency_key: Option<String>,
}

/// IPC Request DTO for dispatching an inter-branch transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchStockTransferRequest {
    pub transfer_id: String,
    pub idempotency_key: Option<String>,
}

/// IPC Request DTO for receiving an in-transit inter-branch transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiveStockTransferRequest {
    pub transfer_id: String,
    pub destination_location_id: Option<String>,
    pub destination_bin_id: Option<String>,
    pub idempotency_key: Option<String>,
}

/// IPC Request DTO for cancelling a draft transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelStockTransferRequest {
    pub transfer_id: String,
}

/// IPC Request DTO for instant intra-branch relocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantIntraBranchTransferRequest {
    pub branch_id: String,
    pub source_location_id: String,
    pub destination_location_id: String,
    pub source_bin_id: Option<String>,
    pub destination_bin_id: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateTransferItemInput>,
    pub idempotency_key: Option<String>,
}

/// IPC Request DTO for filtering stock transfer documents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListStockTransfersRequest {
    pub branch_id: Option<String>,
    pub source_branch_id: Option<String>,
    pub destination_branch_id: Option<String>,
    pub transfer_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// =========================================================================
// ERROR MAPPING
// =========================================================================

/// Maps domain TransferError into user-facing command error strings, preserving
/// typed diagnostic distinctions without leaking raw database internals.
pub fn map_transfer_error(err: TransferError) -> String {
    match err {
        TransferError::Validation(msg) => format!("Validation error: {msg}"),
        TransferError::NotFound(msg) => format!("Entity not found: {msg}"),
        TransferError::TopologyMismatch(msg) => format!("Topology mismatch: {msg}"),
        TransferError::NoOpRelocation(msg) => format!("No-op relocation error: {msg}"),
        TransferError::InvalidLocation(msg) => format!("Invalid location: {msg}"),
        TransferError::InvalidBin(msg) => format!("Invalid bin: {msg}"),
        TransferError::BranchMismatch(msg) => format!("Branch mismatch: {msg}"),
        TransferError::VariantMismatch(msg) => format!("Variant mismatch: {msg}"),
        TransferError::InsufficientStock {
            product_id,
            requested_milli,
            available_milli,
        } => format!(
            "Insufficient stock for product '{product_id}': requested {requested_milli} milli, available {available_milli} milli"
        ),
        TransferError::InvalidStatusTransition {
            current,
            attempted,
            reason,
        } => format!(
            "Invalid status transition from '{current}' to '{attempted}': {reason}"
        ),
        TransferError::InvalidBatch(msg) => format!("Invalid batch: {msg}"),
        TransferError::InvalidBatchStatus(msg) => format!("Invalid batch status: {msg}"),
        TransferError::InvalidSerial(msg) => format!("Invalid serial: {msg}"),
        TransferError::InvalidSerialStatus(msg) => format!("Invalid serial status: {msg}"),
        TransferError::IdempotencyConflict(msg) => format!("Idempotency conflict: {msg}"),
        TransferError::Unauthorized(msg) => format!("Unauthorized: {msg}"),
        TransferError::Database(msg) => format!("Database error: {msg}"),
    }
}

// =========================================================================
// DIRECTLY TESTABLE COMMAND IMPLEMENTATIONS
// =========================================================================

/// Creates a new stock transfer document with InventoryTransfer authorization scoped to source branch.
pub fn create_stock_transfer_impl(
    conn: &mut Connection,
    session_id: &str,
    request: CreateStockTransferRequest,
) -> Result<StockTransfer, String> {
    let session = require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryTransfer,
        None,
        Some(&request.source_branch_id),
    )
    .map_err(|e| e.to_string())?;

    let transfer_type =
        TransferType::from_str(&request.transfer_type).map_err(map_transfer_error)?;

    let input = CreateTransferInput {
        transfer_type,
        source_branch_id: request.source_branch_id,
        destination_branch_id: request.destination_branch_id,
        source_location_id: request.source_location_id,
        destination_location_id: request.destination_location_id,
        source_bin_id: request.source_bin_id,
        destination_bin_id: request.destination_bin_id,
        notes: request.notes,
        items: request.items,
        user_id: Some(session.user_id),
        idempotency_key: request.idempotency_key,
    };

    TransferService::create_transfer(conn, &input).map_err(map_transfer_error)
}

/// Dispatches an inter-branch transfer with InventoryTransfer authorization scoped to source branch.
pub fn dispatch_stock_transfer_impl(
    conn: &mut Connection,
    session_id: &str,
    request: DispatchStockTransferRequest,
) -> Result<StockTransfer, String> {
    let source_branch_id: String = conn
        .query_row(
            "SELECT source_branch_id FROM stock_transfers WHERE id = ?1",
            params![request.transfer_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Database error: {e}"))?
        .ok_or_else(|| {
            map_transfer_error(TransferError::NotFound(format!(
                "Transfer '{}' not found",
                request.transfer_id
            )))
        })?;

    let session = require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryTransfer,
        None,
        Some(&source_branch_id),
    )
    .map_err(|e| e.to_string())?;

    let input = DispatchTransferInput {
        transfer_id: request.transfer_id,
        user_id: Some(session.user_id),
        idempotency_key: request.idempotency_key,
    };

    TransferService::dispatch_transfer(conn, &input).map_err(map_transfer_error)
}

/// Receives an in-transit inter-branch transfer with InventoryTransfer authorization scoped to destination branch.
pub fn receive_stock_transfer_impl(
    conn: &mut Connection,
    session_id: &str,
    request: ReceiveStockTransferRequest,
) -> Result<StockTransfer, String> {
    let destination_branch_id: String = conn
        .query_row(
            "SELECT destination_branch_id FROM stock_transfers WHERE id = ?1",
            params![request.transfer_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Database error: {e}"))?
        .ok_or_else(|| {
            map_transfer_error(TransferError::NotFound(format!(
                "Transfer '{}' not found",
                request.transfer_id
            )))
        })?;

    let session = require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryTransfer,
        None,
        Some(&destination_branch_id),
    )
    .map_err(|e| e.to_string())?;

    let input = ReceiveTransferInput {
        transfer_id: request.transfer_id,
        destination_location_id: request.destination_location_id,
        destination_bin_id: request.destination_bin_id,
        user_id: Some(session.user_id),
        idempotency_key: request.idempotency_key,
    };

    TransferService::receive_transfer(conn, &input).map_err(map_transfer_error)
}

/// Cancels a draft transfer with InventoryTransfer authorization scoped to source branch.
pub fn cancel_stock_transfer_impl(
    conn: &mut Connection,
    session_id: &str,
    request: CancelStockTransferRequest,
) -> Result<StockTransfer, String> {
    let source_branch_id: String = conn
        .query_row(
            "SELECT source_branch_id FROM stock_transfers WHERE id = ?1",
            params![request.transfer_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Database error: {e}"))?
        .ok_or_else(|| {
            map_transfer_error(TransferError::NotFound(format!(
                "Transfer '{}' not found",
                request.transfer_id
            )))
        })?;

    let session = require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryTransfer,
        None,
        Some(&source_branch_id),
    )
    .map_err(|e| e.to_string())?;

    let input = CancelTransferInput {
        transfer_id: request.transfer_id,
        user_id: Some(session.user_id),
    };

    TransferService::cancel_transfer(conn, &input).map_err(map_transfer_error)
}

/// Performs an immediate, atomic intra-branch relocation with InventoryTransfer authorization scoped to branch.
pub fn instant_intra_branch_transfer_impl(
    conn: &mut Connection,
    session_id: &str,
    request: InstantIntraBranchTransferRequest,
) -> Result<StockTransfer, String> {
    let session = require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryTransfer,
        None,
        Some(&request.branch_id),
    )
    .map_err(|e| e.to_string())?;

    let input = InstantIntraBranchTransferInput {
        branch_id: request.branch_id,
        source_location_id: request.source_location_id,
        destination_location_id: request.destination_location_id,
        source_bin_id: request.source_bin_id,
        destination_bin_id: request.destination_bin_id,
        notes: request.notes,
        items: request.items,
        user_id: Some(session.user_id),
        idempotency_key: request.idempotency_key,
    };

    TransferService::instant_intra_branch_transfer(conn, &input).map_err(map_transfer_error)
}

/// Retrieves a single stock transfer document by ID with tenancy verification.
pub fn get_stock_transfer_impl(
    conn: &Connection,
    session_id: &str,
    id: &str,
) -> Result<Option<StockTransfer>, String> {
    let session =
        require_scoped_permission(conn, session_id, Permission::InventoryTransfer, None, None)
            .map_err(|e| e.to_string())?;

    let transfer = TransferService::get_transfer(conn, id).map_err(map_transfer_error)?;

    if let Some(ref t) = transfer {
        if t.source_branch_id != session.branch_id && t.destination_branch_id != session.branch_id {
            return Err(format!(
                "Scope mismatch: operation requires scope '{}' or '{}', but session has scope '{}'",
                t.source_branch_id, t.destination_branch_id, session.branch_id
            ));
        }
    }

    Ok(transfer)
}

/// Lists stock transfer documents strictly scoped to caller's branch tenancy.
pub fn list_stock_transfers_impl(
    conn: &Connection,
    session_id: &str,
    request: ListStockTransfersRequest,
) -> Result<Vec<StockTransfer>, String> {
    let session =
        require_scoped_permission(conn, session_id, Permission::InventoryTransfer, None, None)
            .map_err(|e| e.to_string())?;

    if let Some(ref b_id) = request.branch_id {
        if b_id != &session.branch_id {
            return Err(format!(
                "Scope mismatch: operation requires scope '{}', but session has scope '{}'",
                b_id, session.branch_id
            ));
        }
    }

    let transfer_type = if let Some(ref tt) = request.transfer_type {
        Some(TransferType::from_str(tt).map_err(map_transfer_error)?)
    } else {
        None
    };

    let status = if let Some(ref st) = request.status {
        Some(TransferStatus::from_str(st).map_err(map_transfer_error)?)
    } else {
        None
    };

    let filter = TransferFilter {
        branch_id: Some(session.branch_id),
        source_branch_id: request.source_branch_id,
        destination_branch_id: request.destination_branch_id,
        transfer_type,
        status,
        limit: request.limit,
        offset: request.offset,
    };

    TransferService::list_transfers(conn, &filter).map_err(map_transfer_error)
}

// =========================================================================
// TAURI IPC COMMAND WRAPPERS
// =========================================================================

/// Tauri IPC command: Creates a new stock transfer document.
#[tauri::command]
pub async fn create_stock_transfer(
    state: State<'_, DbState>,
    session_id: String,
    request: CreateStockTransferRequest,
) -> Result<StockTransfer, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    create_stock_transfer_impl(&mut conn, &session_id, request)
}

/// Tauri IPC command: Dispatches an inter-branch transfer.
#[tauri::command]
pub async fn dispatch_stock_transfer(
    state: State<'_, DbState>,
    session_id: String,
    request: DispatchStockTransferRequest,
) -> Result<StockTransfer, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    dispatch_stock_transfer_impl(&mut conn, &session_id, request)
}

/// Tauri IPC command: Receives an in-transit inter-branch transfer.
#[tauri::command]
pub async fn receive_stock_transfer(
    state: State<'_, DbState>,
    session_id: String,
    request: ReceiveStockTransferRequest,
) -> Result<StockTransfer, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    receive_stock_transfer_impl(&mut conn, &session_id, request)
}

/// Tauri IPC command: Cancels a draft transfer.
#[tauri::command]
pub async fn cancel_stock_transfer(
    state: State<'_, DbState>,
    session_id: String,
    request: CancelStockTransferRequest,
) -> Result<StockTransfer, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    cancel_stock_transfer_impl(&mut conn, &session_id, request)
}

/// Tauri IPC command: Performs an immediate, atomic intra-branch relocation.
#[tauri::command]
pub async fn instant_intra_branch_transfer(
    state: State<'_, DbState>,
    session_id: String,
    request: InstantIntraBranchTransferRequest,
) -> Result<StockTransfer, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    instant_intra_branch_transfer_impl(&mut conn, &session_id, request)
}

/// Tauri IPC command: Retrieves a single stock transfer document by ID.
#[tauri::command]
pub async fn get_stock_transfer(
    state: State<'_, DbState>,
    session_id: String,
    id: String,
) -> Result<Option<StockTransfer>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_stock_transfer_impl(&conn, &session_id, &id)
}

/// Tauri IPC command: Lists stock transfer documents strictly scoped to caller's branch.
#[tauri::command]
pub async fn list_stock_transfers(
    state: State<'_, DbState>,
    session_id: String,
    request: ListStockTransfersRequest,
) -> Result<Vec<StockTransfer>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    list_stock_transfers_impl(&conn, &session_id, request)
}
