// Tauri command boundary for Brand operations.
// F2.02 — Categories, Brands, Manufacturers

use crate::brand::{Brand, BrandFilter, CreateBrandInput, UpdateBrandInput};
use crate::db::DbState;

use super::{
    authorize_catalog_mutation as authorize_brand_mutation,
    authorize_catalog_read as authorize_brand_read,
};

#[tauri::command]
pub fn create_brand(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateBrandInput,
) -> Result<Brand, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_brand_mutation(&conn, &session_id)?;
    crate::brand::create_brand(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_brand(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<Option<Brand>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_brand_read(&conn, &session_id)?;
    crate::brand::get_brand(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_brand(
    state: tauri::State<DbState>,
    session_id: String,
    request: UpdateBrandInput,
) -> Result<Brand, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_brand_mutation(&conn, &session_id)?;
    crate::brand::update_brand(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_brand(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_brand_mutation(&conn, &session_id)?;
    crate::brand::delete_brand(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_brands(
    state: tauri::State<DbState>,
    session_id: String,
    filter: Option<BrandFilter>,
) -> Result<Vec<Brand>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_brand_read(&conn, &session_id)?;
    let f = filter.unwrap_or_default();
    crate::brand::list_brands(&conn, &f).map_err(|e| e.to_string())
}
