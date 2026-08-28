// Tauri command boundary for Manufacturer operations.
// F2.02 — Categories, Brands, Manufacturers

use crate::db::DbState;
use crate::manufacturer::{
    self, CreateManufacturerInput, Manufacturer, ManufacturerFilter, UpdateManufacturerInput,
};

use super::{
    authorize_catalog_mutation as authorize_manufacturer_mutation,
    authorize_catalog_read as authorize_manufacturer_read,
};

#[tauri::command]
pub fn create_manufacturer(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateManufacturerInput,
) -> Result<Manufacturer, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_manufacturer_mutation(&conn, &session_id)?;
    manufacturer::create_manufacturer(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_manufacturer(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<Option<Manufacturer>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_manufacturer_read(&conn, &session_id)?;
    manufacturer::get_manufacturer(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_manufacturer(
    state: tauri::State<DbState>,
    session_id: String,
    request: UpdateManufacturerInput,
) -> Result<Manufacturer, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_manufacturer_mutation(&conn, &session_id)?;
    manufacturer::update_manufacturer(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_manufacturer(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_manufacturer_mutation(&conn, &session_id)?;
    manufacturer::delete_manufacturer(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_manufacturers(
    state: tauri::State<DbState>,
    session_id: String,
    filter: Option<ManufacturerFilter>,
) -> Result<Vec<Manufacturer>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_manufacturer_read(&conn, &session_id)?;
    let f = filter.unwrap_or_default();
    manufacturer::list_manufacturers(&conn, &f).map_err(|e| e.to_string())
}
