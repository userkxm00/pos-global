// Tauri command boundary for Branch operations.
// The frontend must access branch data strictly through these commands.

use crate::branch::{Branch, CreateBranchInput, UpdateBranchInput};
use crate::db::DbState;

#[tauri::command]
pub fn create_branch(
    state: tauri::State<DbState>,
    request: CreateBranchInput,
) -> Result<Branch, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::branch::create_branch(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_branch(state: tauri::State<DbState>, branch_id: String) -> Result<Branch, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    match crate::branch::get_branch(&conn, &branch_id).map_err(|e| e.to_string())? {
        Some(b) => Ok(b),
        None => Err(format!("Branch with ID '{branch_id}' not found")),
    }
}

#[tauri::command]
pub fn update_branch(
    state: tauri::State<DbState>,
    request: UpdateBranchInput,
) -> Result<Branch, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::branch::update_branch(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_branches(
    state: tauri::State<DbState>,
    organization_id: String,
) -> Result<Vec<Branch>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::branch::list_branches(&conn, &organization_id).map_err(|e| e.to_string())
}
