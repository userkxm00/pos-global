// F2.09 — Warranty IPC Command Handlers
// Implements ADR-0011: Scoped authorization, branch isolation, and fail-closed security.

use crate::auth::middleware::{require_scoped_permission, require_session, AuthorizeRequest};
use crate::db::DbState;
use crate::permission::Permission;
use crate::warranty::{
    calculate_warranty_expiration as domain_calc_exp,
    evaluate_warranty_coverage as domain_eval_cov, get_instance_warranty as domain_get_inst,
    normalize_to_canonical_date, register_instance_warranty as domain_reg_inst,
    InstanceWarrantyRecord, RegisterWarrantyInput, WarrantyCoverageStatus,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

// =========================================================================
// REQUEST TYPES
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterInstanceWarrantyRequest {
    pub branch_id: String,
    pub serial_number_id: String,
    pub start_date: Option<String>,
    pub duration_months: Option<u32>,
    pub warranty_expires_at: Option<String>,
}

// =========================================================================
// DIRECTLY TESTABLE COMMAND IMPLEMENTATIONS
// =========================================================================

/// Calculates warranty expiration date from start date and duration.
pub fn calculate_warranty_expiration_impl(
    conn: &Connection,
    session_id: &str,
    start_date: &str,
    duration_months: u32,
) -> Result<String, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    let canonical_start = normalize_to_canonical_date(start_date).map_err(|e| e.to_string())?;
    domain_calc_exp(&canonical_start, duration_months).map_err(|e| e.to_string())
}

/// Evaluates warranty coverage status for a given expiry date and reference date.
pub fn evaluate_warranty_coverage_impl(
    conn: &Connection,
    session_id: &str,
    expiry_date: Option<&str>,
    as_of_date: Option<&str>,
    is_tracked: bool,
) -> Result<WarrantyCoverageStatus, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    domain_eval_cov(expiry_date, as_of_date, is_tracked).map_err(|e| e.to_string())
}

/// Registers or updates warranty on a serialized inventory instance with scoped permission.
pub fn register_instance_warranty_impl(
    conn: &Connection,
    session_id: &str,
    request: &RegisterInstanceWarrantyRequest,
) -> Result<InstanceWarrantyRecord, String> {
    require_scoped_permission(
        conn,
        session_id,
        Permission::InventoryAdjust,
        None,
        Some(&request.branch_id),
    )
    .map_err(|e| e.to_string())?;

    // Anti-existence leakage: verify branch ownership before performing registration
    let mut stmt = conn
        .prepare_cached("SELECT branch_id FROM serial_numbers WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    let owning_branch: String = stmt
        .query_row([&request.serial_number_id], |row| row.get(0))
        .map_err(|_| {
            format!(
                "Serial instance '{}' not found or inaccessible for this session",
                request.serial_number_id
            )
        })?;

    if owning_branch != request.branch_id {
        return Err(format!(
            "Serial instance '{}' not found or inaccessible for this session",
            request.serial_number_id
        ));
    }

    let input = RegisterWarrantyInput {
        serial_number_id: request.serial_number_id.clone(),
        start_date: request.start_date.clone(),
        duration_months: request.duration_months,
        warranty_expires_at: request.warranty_expires_at.clone(),
    };

    domain_reg_inst(conn, &input).map_err(|e| e.to_string())
}

/// Retrieves warranty status and evaluated coverage for a serialized instance.
pub fn get_instance_warranty_impl(
    conn: &Connection,
    session_id: &str,
    serial_number_id: &str,
    as_of_date: Option<&str>,
) -> Result<InstanceWarrantyRecord, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    // Fetch instance branch for anti-existence leakage check
    let mut stmt = conn
        .prepare_cached("SELECT branch_id FROM serial_numbers WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    let branch_id: String = stmt
        .query_row([serial_number_id], |row| row.get(0))
        .map_err(|_| {
            format!(
                "Serial instance '{}' not found or inaccessible for this session",
                serial_number_id
            )
        })?;

    AuthorizeRequest::new(session_id)
        .with_branch_scope(&branch_id)
        .execute(conn)
        .map_err(|_| {
            format!(
                "Serial instance '{}' not found or inaccessible for this session",
                serial_number_id
            )
        })?;

    domain_get_inst(conn, serial_number_id, as_of_date).map_err(|e| e.to_string())
}

// =========================================================================
// TAURI IPC INVOCATION ENDPOINTS
// =========================================================================

#[tauri::command]
pub fn calculate_warranty_expiration(
    state: State<DbState>,
    session_id: String,
    start_date: String,
    duration_months: u32,
) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    calculate_warranty_expiration_impl(&conn, &session_id, &start_date, duration_months)
}

#[tauri::command]
pub fn evaluate_warranty_coverage(
    state: State<DbState>,
    session_id: String,
    expiry_date: Option<String>,
    as_of_date: Option<String>,
    is_tracked: bool,
) -> Result<WarrantyCoverageStatus, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    evaluate_warranty_coverage_impl(
        &conn,
        &session_id,
        expiry_date.as_deref(),
        as_of_date.as_deref(),
        is_tracked,
    )
}

#[tauri::command]
pub fn register_instance_warranty(
    state: State<DbState>,
    session_id: String,
    request: RegisterInstanceWarrantyRequest,
) -> Result<InstanceWarrantyRecord, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    register_instance_warranty_impl(&conn, &session_id, &request)
}

#[tauri::command]
pub fn get_instance_warranty(
    state: State<DbState>,
    session_id: String,
    serial_number_id: String,
    as_of_date: Option<String>,
) -> Result<InstanceWarrantyRecord, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_instance_warranty_impl(&conn, &session_id, &serial_number_id, as_of_date.as_deref())
}
