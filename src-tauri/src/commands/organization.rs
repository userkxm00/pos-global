// Tauri command boundary for Organization operations.
// The frontend must access organization data strictly through these commands.

use crate::db::DbState;
use crate::organization::{
    CreateOrganizationInput, Organization, UpdateOrganizationInput,
};

#[tauri::command]
pub fn create_organization(
    state: tauri::State<DbState>,
    request: CreateOrganizationInput,
) -> Result<Organization, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::organization::create_organization(&conn, request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_organization(
    state: tauri::State<DbState>,
    organization_id: String,
) -> Result<Organization, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    match crate::organization::get_organization(&conn, &organization_id)
        .map_err(|e| e.to_string())?
    {
        Some(org) => Ok(org),
        None => Err(format!(
            "Organization with ID '{organization_id}' not found"
        )),
    }
}

#[tauri::command]
pub fn update_organization(
    state: tauri::State<DbState>,
    request: UpdateOrganizationInput,
) -> Result<Organization, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::organization::update_organization(&conn, request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_organizations(
    state: tauri::State<DbState>,
) -> Result<Vec<Organization>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    crate::organization::list_organizations(&conn).map_err(|e| e.to_string())
}
