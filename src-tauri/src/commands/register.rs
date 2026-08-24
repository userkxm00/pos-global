// Tauri command boundary for Register operations.
// The frontend must access register data strictly through these commands.

use crate::db::DbState;
use crate::register::{CreateRegisterInput, Register, UpdateRegisterInput};

#[tauri::command]
pub fn create_register(
    state: tauri::State<DbState>,
    request: CreateRegisterInput,
) -> Result<Register, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::register::create_register(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_register(state: tauri::State<DbState>, register_id: String) -> Result<Register, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    match crate::register::get_register(&conn, &register_id).map_err(|e| e.to_string())? {
        Some(r) => Ok(r),
        None => Err(format!("Register with ID '{register_id}' not found")),
    }
}

#[tauri::command]
pub fn update_register(
    state: tauri::State<DbState>,
    request: UpdateRegisterInput,
) -> Result<Register, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::register::update_register(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_registers(
    state: tauri::State<DbState>,
    branch_id: String,
) -> Result<Vec<Register>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::register::list_registers(&conn, &branch_id).map_err(|e| e.to_string())
}
