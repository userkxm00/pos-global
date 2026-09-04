// F2.10 — Location and Bin IPC Command Handlers
// ADR-0012: Scoped authorization via Permission::SettingsManage, branch isolation, and fail-closed security.

use crate::auth::middleware::{
    require_permission, require_scoped_permission, require_session, AuthorizeRequest,
};
use crate::db::DbState;
use crate::location::{
    get_bin_branch_id, Bin, BinFilter, CreateBinInput, CreateLocationInput, Location,
    LocationFilter, LocationTreeNode, UpdateBinInput, UpdateLocationInput,
};
use crate::permission::Permission;
use rusqlite::Connection;
use tauri::State;

// =========================================================================
// LOCATION COMMAND IMPLEMENTATIONS (DIRECTLY TESTABLE)
// =========================================================================

/// Creates a new physical storage location with SettingsManage scoped permission check.
pub fn create_location_impl(
    conn: &Connection,
    session_id: &str,
    request: CreateLocationInput,
) -> Result<Location, String> {
    require_scoped_permission(
        conn,
        session_id,
        Permission::SettingsManage,
        None,
        Some(&request.branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::location::create_location(conn, request).map_err(|e| e.to_string())
}

/// Retrieves a single location by ID with branch scope authorization and anti-existence leakage.
pub fn get_location_impl(
    conn: &Connection,
    session_id: &str,
    id: &str,
) -> Result<Option<Location>, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    let location = crate::location::get_location(conn, id).map_err(|e| e.to_string())?;

    if let Some(ref loc) = location {
        if AuthorizeRequest::new(session_id)
            .with_branch_scope(&loc.branch_id)
            .execute(conn)
            .is_err()
        {
            return Ok(None);
        }
    }

    Ok(location)
}

/// Lists locations matching the given filter with branch scope validation.
pub fn list_locations_impl(
    conn: &Connection,
    session_id: &str,
    filter: LocationFilter,
) -> Result<Vec<Location>, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(&filter.branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    crate::location::list_locations(conn, &filter).map_err(|e| e.to_string())
}

/// Updates an existing location with SettingsManage scoped permission check.
pub fn update_location_impl(
    conn: &Connection,
    session_id: &str,
    request: UpdateLocationInput,
) -> Result<Location, String> {
    require_permission(conn, session_id, Permission::SettingsManage).map_err(|e| e.to_string())?;

    let existing = crate::location::get_location(conn, &request.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Location '{}' not found", request.id))?;

    require_scoped_permission(
        conn,
        session_id,
        Permission::SettingsManage,
        None,
        Some(&existing.branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::location::update_location(conn, request).map_err(|e| e.to_string())
}

/// Deactivates a location with SettingsManage scoped permission check.
pub fn deactivate_location_impl(
    conn: &Connection,
    session_id: &str,
    id: &str,
) -> Result<Location, String> {
    require_permission(conn, session_id, Permission::SettingsManage).map_err(|e| e.to_string())?;

    let existing = crate::location::get_location(conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Location '{id}' not found"))?;

    require_scoped_permission(
        conn,
        session_id,
        Permission::SettingsManage,
        None,
        Some(&existing.branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::location::deactivate_location(conn, id).map_err(|e| e.to_string())
}

/// Reactivates an inactive location with SettingsManage scoped permission check.
pub fn reactivate_location_impl(
    conn: &Connection,
    session_id: &str,
    id: &str,
) -> Result<Location, String> {
    require_permission(conn, session_id, Permission::SettingsManage).map_err(|e| e.to_string())?;

    let existing = crate::location::get_location(conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Location '{id}' not found"))?;

    require_scoped_permission(
        conn,
        session_id,
        Permission::SettingsManage,
        None,
        Some(&existing.branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::location::reactivate_location(conn, id).map_err(|e| e.to_string())
}

/// Retrieves the complete location and bin hierarchy tree for a branch.
pub fn get_location_tree_impl(
    conn: &Connection,
    session_id: &str,
    branch_id: &str,
    include_inactive: Option<bool>,
) -> Result<Vec<LocationTreeNode>, String> {
    AuthorizeRequest::new(session_id)
        .with_branch_scope(branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;

    crate::location::get_location_tree(conn, branch_id, include_inactive.unwrap_or(false))
        .map_err(|e| e.to_string())
}

// =========================================================================
// BIN COMMAND IMPLEMENTATIONS (DIRECTLY TESTABLE)
// =========================================================================

/// Creates a new bin within a location with SettingsManage scoped permission check.
pub fn create_bin_impl(
    conn: &Connection,
    session_id: &str,
    request: CreateBinInput,
) -> Result<Bin, String> {
    require_permission(conn, session_id, Permission::SettingsManage).map_err(|e| e.to_string())?;

    let parent_loc = crate::location::get_location(conn, &request.location_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Location '{}' not found", request.location_id))?;

    require_scoped_permission(
        conn,
        session_id,
        Permission::SettingsManage,
        None,
        Some(&parent_loc.branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::location::create_bin(conn, request).map_err(|e| e.to_string())
}

/// Retrieves a single bin by ID with branch scope authorization and anti-existence leakage.
pub fn get_bin_impl(conn: &Connection, session_id: &str, id: &str) -> Result<Option<Bin>, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    let bin = crate::location::get_bin(conn, id).map_err(|e| e.to_string())?;

    if bin.is_some() {
        let branch_id = match get_bin_branch_id(conn, id).map_err(|e| e.to_string())? {
            Some(b) => b,
            None => return Ok(None),
        };

        if AuthorizeRequest::new(session_id)
            .with_branch_scope(&branch_id)
            .execute(conn)
            .is_err()
        {
            return Ok(None);
        }
    }

    Ok(bin)
}

/// Lists bins matching the given filter with branch scope validation.
pub fn list_bins_impl(
    conn: &Connection,
    session_id: &str,
    filter: BinFilter,
) -> Result<Vec<Bin>, String> {
    require_session(conn, session_id).map_err(|e| e.to_string())?;

    if let Some(ref loc_id) = filter.location_id {
        let parent = crate::location::get_location(conn, loc_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                format!("Location '{loc_id}' not found or inaccessible for this session")
            })?;

        AuthorizeRequest::new(session_id)
            .with_branch_scope(&parent.branch_id)
            .execute(conn)
            .map_err(|_| {
                format!("Location '{loc_id}' not found or inaccessible for this session")
            })?;

        if let Some(ref branch_id) = filter.branch_id {
            if branch_id != &parent.branch_id {
                return Err(format!(
                    "Filter branch_id '{branch_id}' does not match location branch_id '{}'",
                    parent.branch_id
                ));
            }
        }
    } else if let Some(ref branch_id) = filter.branch_id {
        AuthorizeRequest::new(session_id)
            .with_branch_scope(branch_id)
            .execute(conn)
            .map_err(|e| e.to_string())?;
    } else {
        return Err("A branch_id or location_id filter is required".to_string());
    }

    crate::location::list_bins(conn, &filter).map_err(|e| e.to_string())
}

/// Updates an existing bin with SettingsManage scoped permission check.
pub fn update_bin_impl(
    conn: &Connection,
    session_id: &str,
    request: UpdateBinInput,
) -> Result<Bin, String> {
    require_permission(conn, session_id, Permission::SettingsManage).map_err(|e| e.to_string())?;

    let branch_id = get_bin_branch_id(conn, &request.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Bin '{}' not found", request.id))?;

    require_scoped_permission(
        conn,
        session_id,
        Permission::SettingsManage,
        None,
        Some(&branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::location::update_bin(conn, request).map_err(|e| e.to_string())
}

/// Deactivates a bin with SettingsManage scoped permission check.
pub fn deactivate_bin_impl(conn: &Connection, session_id: &str, id: &str) -> Result<Bin, String> {
    require_permission(conn, session_id, Permission::SettingsManage).map_err(|e| e.to_string())?;

    let branch_id = get_bin_branch_id(conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Bin '{id}' not found"))?;

    require_scoped_permission(
        conn,
        session_id,
        Permission::SettingsManage,
        None,
        Some(&branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::location::deactivate_bin(conn, id).map_err(|e| e.to_string())
}

/// Reactivates an inactive bin with SettingsManage scoped permission check.
pub fn reactivate_bin_impl(conn: &Connection, session_id: &str, id: &str) -> Result<Bin, String> {
    require_permission(conn, session_id, Permission::SettingsManage).map_err(|e| e.to_string())?;

    let branch_id = get_bin_branch_id(conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Bin '{id}' not found"))?;

    require_scoped_permission(
        conn,
        session_id,
        Permission::SettingsManage,
        None,
        Some(&branch_id),
    )
    .map_err(|e| e.to_string())?;

    crate::location::reactivate_bin(conn, id).map_err(|e| e.to_string())
}

// =========================================================================
// TAURI COMMAND WRAPPERS
// =========================================================================

#[tauri::command]
pub fn create_location(
    state: State<DbState>,
    session_id: String,
    request: CreateLocationInput,
) -> Result<Location, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    create_location_impl(&conn, &session_id, request)
}

#[tauri::command]
pub fn get_location(
    state: State<DbState>,
    session_id: String,
    id: String,
) -> Result<Option<Location>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    get_location_impl(&conn, &session_id, &id)
}

#[tauri::command]
pub fn list_locations(
    state: State<DbState>,
    session_id: String,
    filter: LocationFilter,
) -> Result<Vec<Location>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    list_locations_impl(&conn, &session_id, filter)
}

#[tauri::command]
pub fn update_location(
    state: State<DbState>,
    session_id: String,
    request: UpdateLocationInput,
) -> Result<Location, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    update_location_impl(&conn, &session_id, request)
}

#[tauri::command]
pub fn deactivate_location(
    state: State<DbState>,
    session_id: String,
    id: String,
) -> Result<Location, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    deactivate_location_impl(&conn, &session_id, &id)
}

#[tauri::command]
pub fn reactivate_location(
    state: State<DbState>,
    session_id: String,
    id: String,
) -> Result<Location, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    reactivate_location_impl(&conn, &session_id, &id)
}

#[tauri::command]
pub fn get_location_tree(
    state: State<DbState>,
    session_id: String,
    branch_id: String,
    include_inactive: Option<bool>,
) -> Result<Vec<LocationTreeNode>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    get_location_tree_impl(&conn, &session_id, &branch_id, include_inactive)
}

#[tauri::command]
pub fn create_bin(
    state: State<DbState>,
    session_id: String,
    request: CreateBinInput,
) -> Result<Bin, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    create_bin_impl(&conn, &session_id, request)
}

#[tauri::command]
pub fn get_bin(
    state: State<DbState>,
    session_id: String,
    id: String,
) -> Result<Option<Bin>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    get_bin_impl(&conn, &session_id, &id)
}

#[tauri::command]
pub fn list_bins(
    state: State<DbState>,
    session_id: String,
    filter: BinFilter,
) -> Result<Vec<Bin>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    list_bins_impl(&conn, &session_id, filter)
}

#[tauri::command]
pub fn update_bin(
    state: State<DbState>,
    session_id: String,
    request: UpdateBinInput,
) -> Result<Bin, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    update_bin_impl(&conn, &session_id, request)
}

#[tauri::command]
pub fn deactivate_bin(
    state: State<DbState>,
    session_id: String,
    id: String,
) -> Result<Bin, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    deactivate_bin_impl(&conn, &session_id, &id)
}

#[tauri::command]
pub fn reactivate_bin(
    state: State<DbState>,
    session_id: String,
    id: String,
) -> Result<Bin, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    reactivate_bin_impl(&conn, &session_id, &id)
}
