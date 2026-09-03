// F2.08 — Serial, IMEI & Tracked Assets IPC Command Handlers
// ADR-0010: Scoped authorization, branch isolation, and fail-closed security.

use crate::auth::middleware::{require_scoped_permission, require_session, AuthorizeRequest};
use crate::db::DbState;
use crate::permission::Permission;
use crate::serial::{CreateSerialInput, SerialFilter, SerializedInstance, UpdateSerialStatusInput};
use rusqlite::Connection;
use tauri::State;

// =========================================================================
// DIRECTLY TESTABLE COMMAND IMPLEMENTATIONS
// =========================================================================

/// Creates a new serialized inventory instance with scoped permission check.
pub fn create_serial_instance_impl(
    conn: &Connection,
    session_id: &str,
    request: &CreateSerialInput,
) -> Result<SerializedInstance, String> {
    require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryAdjust,
        None,
        Some(&request.branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::serial::create_serial_instance(conn, request).map_err(|e| e.to_string())
}

/// Retrieves a single serialized instance by ID with branch scope authorization.
pub fn get_serial_instance_impl(
    conn: &Connection,
    session_id: &str,
    id: &str,
) -> Result<Option<SerializedInstance>, String> {
    let session = require_session(conn, session_id).map_err(|e| e.to_string())?;

    let instance = crate::serial::get_serial_instance(conn, id).map_err(|e| e.to_string())?;

    if let Some(ref inst) = instance {
        AuthorizeRequest::new(session_id)
            .with_branch_scope(&inst.branch_id)
            .execute(conn)
            .map_err(|_| {
                format!("Serial instance '{id}' not found or inaccessible for this session")
            })?;
    } else {
        let _ = session;
    }

    Ok(instance)
}

/// Looks up an active serialized instance by identifier within a branch.
pub fn lookup_serial_instance_impl(
    conn: &Connection,
    session_id: &str,
    identifier: &str,
    branch_id: &str,
) -> Result<Option<SerializedInstance>, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    crate::serial::lookup_serial_instance(conn, identifier, branch_id).map_err(|e| e.to_string())
}

/// Lists serialized instances matching the filter criteria within a branch.
pub fn list_serial_instances_impl(
    conn: &Connection,
    session_id: &str,
    filter: &SerialFilter,
) -> Result<Vec<SerializedInstance>, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(&filter.branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    crate::serial::list_serial_instances(conn, filter).map_err(|e| e.to_string())
}

/// Updates the status of a serialized instance with branch-scoped inventory permission.
pub fn update_serial_status_impl(
    conn: &Connection,
    session_id: &str,
    request: &UpdateSerialStatusInput,
) -> Result<SerializedInstance, String> {
    require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryAdjust,
        None,
        Some(&request.branch_id),
    )
    .map_err(|e| e.to_string())?;

    let existing = crate::serial::get_serial_instance(conn, &request.id)
        .map_err(|e| e.to_string())?
        .filter(|inst| inst.branch_id == request.branch_id)
        .ok_or_else(|| {
            format!(
                "Serial instance '{}' not found or inaccessible for this session",
                request.id
            )
        })?;

    let _ = existing;
    crate::serial::update_serial_status(conn, request).map_err(|e| e.to_string())
}

// =========================================================================
// TAURI IPC INVOCATION ENDPOINTS
// =========================================================================

#[tauri::command]
pub fn create_serial_instance(
    state: State<DbState>,
    session_id: String,
    request: CreateSerialInput,
) -> Result<SerializedInstance, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    create_serial_instance_impl(&conn, &session_id, &request)
}

#[tauri::command]
pub fn get_serial_instance(
    state: State<DbState>,
    session_id: String,
    id: String,
) -> Result<Option<SerializedInstance>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_serial_instance_impl(&conn, &session_id, &id)
}

#[tauri::command]
pub fn lookup_serial_instance(
    state: State<DbState>,
    session_id: String,
    identifier: String,
    branch_id: String,
) -> Result<Option<SerializedInstance>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    lookup_serial_instance_impl(&conn, &session_id, &identifier, &branch_id)
}

#[tauri::command]
pub fn list_serial_instances(
    state: State<DbState>,
    session_id: String,
    filter: SerialFilter,
) -> Result<Vec<SerializedInstance>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    list_serial_instances_impl(&conn, &session_id, &filter)
}

#[tauri::command]
pub fn update_serial_status(
    state: State<DbState>,
    session_id: String,
    request: UpdateSerialStatusInput,
) -> Result<SerializedInstance, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    update_serial_status_impl(&conn, &session_id, &request)
}
