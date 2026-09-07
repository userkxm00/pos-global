// All Tauri commands exposed to the React UI live under this boundary.
// The frontend must not access SQLite/files directly.

pub mod auth;
pub mod barcode;
pub mod batch;
pub mod branch;
pub mod brand;
pub mod category;
pub mod inventory;
pub mod licence;
pub mod location;
pub mod manufacturer;
pub mod organization;
pub mod product;
pub mod register;
pub mod sales;
pub mod serial;
pub mod unit;
pub mod variant;
pub mod warranty;
pub mod weighted;

use crate::auth::middleware::{require_scoped_permission, AuthorizeRequest};
use crate::permission::Permission;
use crate::product::get_catalog_organization_id;

pub fn authorize_catalog_mutation(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<(), String> {
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
    Ok(())
}

pub fn authorize_catalog_read(conn: &rusqlite::Connection, session_id: &str) -> Result<(), String> {
    let catalog_org = get_catalog_organization_id(conn)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no catalog organization configured".to_string())?;
    AuthorizeRequest::new(session_id)
        .with_organization_scope(&catalog_org)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}
