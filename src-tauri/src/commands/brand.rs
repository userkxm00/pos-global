// Tauri command boundary for Brand operations.
// F2.02 — Categories, Brands, Manufacturers

use crate::auth::middleware::{require_scoped_permission, AuthorizeRequest};
use crate::brand::{self, Brand, BrandFilter, CreateBrandInput, UpdateBrandInput};
use crate::db::DbState;
use crate::permission::Permission;
use crate::product::get_catalog_organization_id;

pub fn authorize_brand_mutation(
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

pub fn authorize_brand_read(
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
    brand::create_brand(&conn, request).map_err(|e| e.to_string())
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
    brand::get_brand(&conn, &id).map_err(|e| e.to_string())
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
    brand::update_brand(&conn, request).map_err(|e| e.to_string())
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
    brand::delete_brand(&conn, &id).map_err(|e| e.to_string())
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
    brand::list_brands(&conn, &f).map_err(|e| e.to_string())
}
