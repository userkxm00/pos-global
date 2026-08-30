// Tauri IPC command handlers for Unit of Measure and Unit Conversion operations.
// F2.04 — Units & Conversions

use crate::db::DbState;
use crate::unit::{
    ConversionResult, ConvertQuantityInput, CreateUnitConversionInput, CreateUnitInput, Unit,
    UnitConversion, UnitConversionView, UnitFilter, UpdateUnitInput,
};

use super::{
    authorize_catalog_mutation as authorize_unit_mutation,
    authorize_catalog_read as authorize_unit_read,
};

#[tauri::command]
pub fn create_unit(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateUnitInput,
) -> Result<Unit, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_mutation(&conn, &session_id)?;
    crate::unit::create_unit(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_unit(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<Option<Unit>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_read(&conn, &session_id)?;
    crate::unit::get_unit(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_unit_by_code(
    state: tauri::State<DbState>,
    session_id: String,
    code: String,
) -> Result<Option<Unit>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_read(&conn, &session_id)?;
    crate::unit::get_unit_by_code(&conn, &code).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_units(
    state: tauri::State<DbState>,
    session_id: String,
    filter: Option<UnitFilter>,
) -> Result<Vec<Unit>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_read(&conn, &session_id)?;
    crate::unit::list_units(&conn, filter.unwrap_or_default()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_unit(
    state: tauri::State<DbState>,
    session_id: String,
    request: UpdateUnitInput,
) -> Result<Unit, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_mutation(&conn, &session_id)?;
    crate::unit::update_unit(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_unit(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_mutation(&conn, &session_id)?;
    crate::unit::delete_unit(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_unit_conversion(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateUnitConversionInput,
) -> Result<UnitConversion, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_mutation(&conn, &session_id)?;
    crate::unit::create_unit_conversion(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_unit_conversion(
    state: tauri::State<DbState>,
    session_id: String,
    from_unit_id: String,
    to_unit_id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_mutation(&conn, &session_id)?;
    crate::unit::delete_unit_conversion(&conn, &from_unit_id, &to_unit_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_unit_conversions(
    state: tauri::State<DbState>,
    session_id: String,
    unit_id: Option<String>,
) -> Result<Vec<UnitConversionView>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_read(&conn, &session_id)?;
    crate::unit::list_unit_conversions(&conn, unit_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn convert_quantity(
    state: tauri::State<DbState>,
    session_id: String,
    request: ConvertQuantityInput,
) -> Result<ConversionResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_unit_read(&conn, &session_id)?;
    crate::unit::convert_quantity(&conn, request).map_err(|e| e.to_string())
}
