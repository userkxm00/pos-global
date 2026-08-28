// Tauri command boundary for Category operations.
// F2.02 — Categories, Brands, Manufacturers

use crate::auth::middleware::{require_scoped_permission, AuthorizeRequest};
use crate::category::{
    self, Category, CategoryFilter, CategoryTreeNode, CreateCategoryInput, UpdateCategoryInput,
};
use crate::db::DbState;
use crate::permission::Permission;
use crate::product::get_catalog_organization_id;

pub fn authorize_category_mutation(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<String, String> {
    let catalog_org = get_catalog_organization_id(conn)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no catalog organization configured".to_string())?;
    require_scoped_permission(
        conn,
        session_id,
        Permission::ProductsManage,
        Some(&catalog_org),
        None,
    )
    .map_err(|e| e.to_string())?;
    Ok(catalog_org)
}

pub fn authorize_category_read(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<String, String> {
    let catalog_org = get_catalog_organization_id(conn)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no catalog organization configured".to_string())?;
    AuthorizeRequest::new(session_id)
        .with_organization_scope(&catalog_org)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    Ok(catalog_org)
}

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
    category::create_category(&conn, request).map_err(|e| e.to_string())
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
    category::get_category(&conn, &id).map_err(|e| e.to_string())
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
    category::update_category(&conn, request).map_err(|e| e.to_string())
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
    category::delete_category(&conn, &id).map_err(|e| e.to_string())
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
    category::list_categories(&conn, &f).map_err(|e| e.to_string())
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
    category::get_category_tree(&conn, include_inactive.unwrap_or(false)).map_err(|e| e.to_string())
}
