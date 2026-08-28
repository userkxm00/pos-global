// Tauri command boundary for Category operations.
// F2.02 — Categories, Brands, Manufacturers

use crate::category::{
    Category, CategoryFilter, CategoryTreeNode, CreateCategoryInput, UpdateCategoryInput,
};
use crate::db::DbState;

use super::{
    authorize_catalog_mutation as authorize_category_mutation,
    authorize_catalog_read as authorize_category_read,
};

#[tauri::command]
pub fn create_category(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateCategoryInput,
) -> Result<Category, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_category_mutation(&conn, &session_id)?;
    crate::category::create_category(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_category(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<Option<Category>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_category_read(&conn, &session_id)?;
    crate::category::get_category(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_category(
    state: tauri::State<DbState>,
    session_id: String,
    request: UpdateCategoryInput,
) -> Result<Category, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_category_mutation(&conn, &session_id)?;
    crate::category::update_category(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_category_mutation(&conn, &session_id)?;
    crate::category::delete_category(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_categories(
    state: tauri::State<DbState>,
    session_id: String,
    filter: Option<CategoryFilter>,
) -> Result<Vec<Category>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_category_read(&conn, &session_id)?;
    let f = filter.unwrap_or_default();
    crate::category::list_categories(&conn, &f).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_category_tree(
    state: tauri::State<DbState>,
    session_id: String,
    include_inactive: Option<bool>,
) -> Result<Vec<CategoryTreeNode>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_category_read(&conn, &session_id)?;
    crate::category::get_category_tree(&conn, include_inactive.unwrap_or_default())
        .map_err(|e| e.to_string())
}
