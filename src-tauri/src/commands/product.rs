// Tauri command boundary for Product CRUD operations.
// F2.01 — Product CRUD

use crate::auth::middleware::{require_scoped_permission, AuthorizeRequest};
use crate::db::DbState;
use crate::permission::Permission;
use crate::product::{
    get_catalog_organization_id, CreateProductInput, Product, ProductFilter, UpdateProductInput,
};

pub fn authorize_product_mutation(
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

pub fn authorize_product_read(
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
pub fn create_product(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateProductInput,
) -> Result<Product, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_product_mutation(&conn, &session_id)?;
    crate::product::create_product(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_product(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
) -> Result<Product, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_product_read(&conn, &session_id)?;
    match crate::product::get_product(&conn, &product_id).map_err(|e| e.to_string())? {
        Some(p) => Ok(p),
        None => Err(format!("Product with ID '{product_id}' not found")),
    }
}

#[tauri::command]
pub fn get_product_by_barcode(
    state: tauri::State<DbState>,
    session_id: String,
    barcode: String,
) -> Result<Product, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_product_read(&conn, &session_id)?;
    match crate::product::get_product_by_barcode(&conn, &barcode).map_err(|e| e.to_string())? {
        Some(p) => Ok(p),
        None => Err(format!("Product with barcode '{barcode}' not found")),
    }
}

#[tauri::command]
pub fn update_product(
    state: tauri::State<DbState>,
    session_id: String,
    request: UpdateProductInput,
) -> Result<Product, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_product_mutation(&conn, &session_id)?;
    crate::product::update_product(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_product(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_product_mutation(&conn, &session_id)?;
    crate::product::delete_product(&conn, &product_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_catalog_products(
    state: tauri::State<DbState>,
    session_id: String,
    filter: ProductFilter,
) -> Result<Vec<Product>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_product_read(&conn, &session_id)?;
    crate::product::list_products(&conn, &filter).map_err(|e| e.to_string())
}
