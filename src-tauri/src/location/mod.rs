// Location and Bin domain models, validation rules, hierarchy invariants, and database operations.
// F2.10 — Locations and Bins Master Data Architecture
// ADR-0012: Discrete Two-Entity Model, Permission::SettingsManage, Same-Branch Invariant, Cycle Prevention.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub const MAX_DEFENSIVE_STEPS: usize = 50;

// =========================================================================
// ERROR TYPES
// =========================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum LocationError {
    Validation(String),
    NotFound(String),
    DuplicateCode(String),
    SelfParenting(String),
    CycleDetected(String),
    CrossBranchParent(String),
    InactiveParent(String),
    DeactivationBlocked(String),
    TraversalSafetyError(String),
    Database(String),
}

impl std::fmt::Display for LocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocationError::Validation(msg) => write!(f, "Validation error: {msg}"),
            LocationError::NotFound(msg) => write!(f, "Not found: {msg}"),
            LocationError::DuplicateCode(msg) => write!(f, "Duplicate code: {msg}"),
            LocationError::SelfParenting(msg) => write!(f, "Self-parenting error: {msg}"),
            LocationError::CycleDetected(msg) => write!(f, "Hierarchy cycle detected: {msg}"),
            LocationError::CrossBranchParent(msg) => write!(f, "Cross-branch error: {msg}"),
            LocationError::InactiveParent(msg) => write!(f, "Inactive parent error: {msg}"),
            LocationError::DeactivationBlocked(msg) => write!(f, "Deactivation blocked: {msg}"),
            LocationError::TraversalSafetyError(msg) => write!(f, "Traversal safety error: {msg}"),
            LocationError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for LocationError {}

impl From<rusqlite::Error> for LocationError {
    fn from(err: rusqlite::Error) -> Self {
        match &err {
            rusqlite::Error::SqliteFailure(sqlite_err, Some(msg)) => {
                if sqlite_err.extended_code == 2067 || msg.contains("UNIQUE constraint failed") {
                    if msg.contains("locations.branch_id") || msg.contains("locations.code") {
                        LocationError::DuplicateCode(
                            "A location with this code already exists in this branch".into(),
                        )
                    } else if msg.contains("bins.location_id") || msg.contains("bins.code") {
                        LocationError::DuplicateCode(
                            "A bin with this code already exists in this location".into(),
                        )
                    } else {
                        LocationError::DuplicateCode("Unique constraint violation".into())
                    }
                } else {
                    LocationError::Database(err.to_string())
                }
            }
            _ => LocationError::Database(err.to_string()),
        }
    }
}

// =========================================================================
// DOMAIN MODELS & DTOs
// =========================================================================

/// Physical storage area or zone within a branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub id: String,
    pub branch_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub code: String,
    pub location_type: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Addressable physical pick/put storage slot belonging to a location.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bin {
    pub id: String,
    pub location_id: String,
    pub name: String,
    pub code: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Hierarchical tree representation of locations and their terminal bins.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocationTreeNode {
    pub location: Location,
    pub children: Vec<LocationTreeNode>,
    pub bins: Vec<Bin>,
}

/// Input payload for creating a location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocationInput {
    pub branch_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub code: String,
    pub location_type: Option<String>,
}

/// Input payload for updating a location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLocationInput {
    pub id: String,
    pub name: Option<String>,
    pub code: Option<String>,
    /// `None` leaves parent untouched. `Some(None)` unsets parent (becomes root). `Some(Some(id))` sets new parent.
    pub parent_id: Option<Option<String>>,
    pub location_type: Option<Option<String>>,
    pub is_active: Option<bool>,
}

/// Filter parameters for listing locations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocationFilter {
    pub branch_id: String,
    /// `None` = all locations in branch.
    /// `Some("root")` or `Some("")` = root locations only (`parent_id IS NULL`).
    /// `Some(id)` = immediate children of `id`.
    pub parent_id: Option<String>,
    pub is_active: Option<bool>,
    pub query: Option<String>,
}

/// Input payload for creating a bin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBinInput {
    pub location_id: String,
    pub name: String,
    pub code: String,
}

/// Input payload for updating a bin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBinInput {
    pub id: String,
    pub name: Option<String>,
    pub code: Option<String>,
    pub is_active: Option<bool>,
}

/// Filter parameters for listing bins.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinFilter {
    pub location_id: Option<String>,
    pub branch_id: Option<String>,
    pub is_active: Option<bool>,
    pub query: Option<String>,
}

// =========================================================================
// VALIDATION HELPERS
// =========================================================================

/// Trims whitespace, preserves valid Unicode, rejects empty/whitespace-only values.
pub fn validate_and_trim_str(val: &str, field_name: &str) -> Result<String, LocationError> {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return Err(LocationError::Validation(format!(
            "{field_name} cannot be empty or whitespace-only"
        )));
    }
    Ok(trimmed.to_string())
}

/// Trims optional text, converting empty string to `None`.
pub fn normalize_optional_str(val: Option<&str>) -> Option<String> {
    val.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

// =========================================================================
// HIERARCHY & CYCLE VALIDATION
// =========================================================================

/// Validates parent location existence, same-branch invariant, active status,
/// and verifies that assigning this parent does not introduce a self-parenting or transitive cycle.
pub fn validate_parent_hierarchy(
    conn: &Connection,
    child_branch_id: &str,
    location_id: Option<&str>,
    target_parent_id: &str,
) -> Result<(), LocationError> {
    // 1. Parent must exist
    let parent_row: Option<(String, bool)> = conn
        .query_row(
            "SELECT branch_id, is_active FROM locations WHERE id = ?1",
            [target_parent_id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? == 1)),
        )
        .optional()
        .map_err(|e| LocationError::Database(e.to_string()))?;

    let (parent_branch_id, parent_is_active) = match parent_row {
        Some(row) => row,
        None => {
            return Err(LocationError::NotFound(format!(
                "Parent location '{target_parent_id}' not found"
            )));
        }
    };

    // 2. Same-branch invariant (domain/transactional guarantee)
    if parent_branch_id != child_branch_id {
        return Err(LocationError::CrossBranchParent(format!(
            "Parent location '{target_parent_id}' belongs to branch '{parent_branch_id}', but child belongs to '{child_branch_id}'"
        )));
    }

    // 3. Inactive parent check
    if !parent_is_active {
        return Err(LocationError::InactiveParent(format!(
            "Cannot set parent to inactive location '{target_parent_id}'"
        )));
    }

    // 4. Cycle prevention (when updating an existing location)
    if let Some(id) = location_id {
        if id == target_parent_id {
            return Err(LocationError::SelfParenting(
                "Location cannot be its own parent".into(),
            ));
        }

        let mut current = target_parent_id.to_string();
        let mut steps = 0;

        while steps < MAX_DEFENSIVE_STEPS {
            let parent_result: Result<Option<String>, _> = conn.query_row(
                "SELECT parent_id FROM locations WHERE id = ?1",
                [&current],
                |row| row.get(0),
            );

            match parent_result {
                Ok(Some(ancestor_id)) => {
                    if ancestor_id == id {
                        return Err(LocationError::CycleDetected(format!(
                            "Location '{id}' cannot be parented under its own descendant '{target_parent_id}'"
                        )));
                    }
                    current = ancestor_id;
                    steps += 1;
                }
                Ok(None) => {
                    // Reached root; no cycle
                    return Ok(());
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // Ancestor doesn't exist
                    break;
                }
                Err(e) => return Err(LocationError::Database(e.to_string())),
            }
        }

        if steps >= MAX_DEFENSIVE_STEPS {
            return Err(LocationError::TraversalSafetyError(format!(
                "Defensive traversal limit of {MAX_DEFENSIVE_STEPS} steps exceeded during cycle check"
            )));
        }
    }

    Ok(())
}

/// Checks that a location has no active child locations and no active bins before deactivation.
pub fn validate_can_deactivate_location(
    conn: &Connection,
    location_id: &str,
) -> Result<(), LocationError> {
    let active_children: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM locations WHERE parent_id = ?1 AND is_active = 1",
            [location_id],
            |row| row.get(0),
        )
        .map_err(|e| LocationError::Database(e.to_string()))?;

    if active_children > 0 {
        return Err(LocationError::DeactivationBlocked(format!(
            "Cannot deactivate location '{location_id}': it contains {active_children} active child location(s)"
        )));
    }

    let active_bins: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM bins WHERE location_id = ?1 AND is_active = 1",
            [location_id],
            |row| row.get(0),
        )
        .map_err(|e| LocationError::Database(e.to_string()))?;

    if active_bins > 0 {
        return Err(LocationError::DeactivationBlocked(format!(
            "Cannot deactivate location '{location_id}': it contains {active_bins} active bin(s)"
        )));
    }

    Ok(())
}

// =========================================================================
// LOCATION OPERATIONS
// =========================================================================

/// Creates a new physical storage location.
pub fn create_location(
    conn: &Connection,
    input: CreateLocationInput,
) -> Result<Location, LocationError> {
    let branch_id = validate_and_trim_str(&input.branch_id, "branch_id")?;
    let name = validate_and_trim_str(&input.name, "name")?;
    let code = validate_and_trim_str(&input.code, "code")?;
    let location_type = normalize_optional_str(input.location_type.as_deref());

    // Verify branch exists and is active
    let branch_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM branches WHERE id = ?1",
            [&branch_id],
            |row| row.get(0),
        )
        .map_err(|e| LocationError::Database(e.to_string()))?;

    if !branch_exists {
        return Err(LocationError::NotFound(format!(
            "Branch '{branch_id}' not found"
        )));
    }

    // Validate parent if specified
    let parent_id = match input.parent_id {
        Some(ref pid) if !pid.trim().is_empty() => {
            let pid_trimmed = pid.trim();
            validate_parent_hierarchy(conn, &branch_id, None, pid_trimmed)?;
            Some(pid_trimmed.to_string())
        }
        _ => None,
    };

    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO locations (id, branch_id, parent_id, name, code, location_type, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, datetime('now'), datetime('now'))",
        params![id, branch_id, parent_id, name, code, location_type],
    )?;

    get_location(conn, &id)?
        .ok_or_else(|| LocationError::Database("Failed to retrieve created location".into()))
}

/// Retrieves a location by primary key.
pub fn get_location(conn: &Connection, id: &str) -> Result<Option<Location>, LocationError> {
    conn.query_row(
        "SELECT id, branch_id, parent_id, name, code, location_type, is_active, created_at, updated_at
         FROM locations WHERE id = ?1",
        [id],
        row_to_location,
    )
    .optional()
    .map_err(|e| LocationError::Database(e.to_string()))
}

/// Lists locations matching the given filter.
pub fn list_locations(
    conn: &Connection,
    filter: &LocationFilter,
) -> Result<Vec<Location>, LocationError> {
    let mut sql = String::from(
        "SELECT id, branch_id, parent_id, name, code, location_type, is_active, created_at, updated_at
         FROM locations WHERE branch_id = ?1",
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(filter.branch_id.clone())];

    if let Some(ref pid) = filter.parent_id {
        if pid == "root" || pid.is_empty() {
            sql.push_str(" AND parent_id IS NULL");
        } else {
            params_vec.push(Box::new(pid.clone()));
            sql.push_str(&format!(" AND parent_id = ?{}", params_vec.len()));
        }
    }

    if let Some(active) = filter.is_active {
        params_vec.push(Box::new(if active { 1 } else { 0 }));
        sql.push_str(&format!(" AND is_active = ?{}", params_vec.len()));
    }

    if let Some(ref q) = filter.query {
        let trimmed_q = q.trim();
        if !trimmed_q.is_empty() {
            let pattern = format!("%{trimmed_q}%");
            params_vec.push(Box::new(pattern));
            let idx = params_vec.len();
            sql.push_str(&format!(" AND (name LIKE ?{idx} OR code LIKE ?{idx})"));
        }
    }

    sql.push_str(" ORDER BY name COLLATE NOCASE ASC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| LocationError::Database(e.to_string()))?;

    let locations = stmt
        .query_map(params_slice.as_slice(), row_to_location)
        .map_err(|e| LocationError::Database(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(locations)
}

/// Updates an existing location. Explicitly updates `updated_at = datetime('now')`.
pub fn update_location(
    conn: &Connection,
    input: UpdateLocationInput,
) -> Result<Location, LocationError> {
    let existing = get_location(conn, &input.id)?
        .ok_or_else(|| LocationError::NotFound(format!("Location '{}' not found", input.id)))?;

    let name = match input.name {
        Some(n) => validate_and_trim_str(&n, "name")?,
        None => existing.name,
    };

    let code = match input.code {
        Some(c) => validate_and_trim_str(&c, "code")?,
        None => existing.code,
    };

    let location_type = match input.location_type {
        Some(opt) => normalize_optional_str(opt.as_deref()),
        None => existing.location_type,
    };

    let parent_id = match input.parent_id {
        Some(Some(ref pid)) if !pid.trim().is_empty() => {
            let pid_trimmed = pid.trim();
            validate_parent_hierarchy(conn, &existing.branch_id, Some(&existing.id), pid_trimmed)?;
            Some(pid_trimmed.to_string())
        }
        Some(Some(_)) | Some(None) => None, // Explicitly unsetting parent -> root
        None => existing.parent_id,
    };

    let is_active = match input.is_active {
        Some(active) => {
            if !active && existing.is_active {
                validate_can_deactivate_location(conn, &existing.id)?;
            }
            active
        }
        None => existing.is_active,
    };

    conn.execute(
        "UPDATE locations
         SET name = ?1, code = ?2, parent_id = ?3, location_type = ?4, is_active = ?5, updated_at = datetime('now')
         WHERE id = ?6",
        params![
            name,
            code,
            parent_id,
            location_type,
            if is_active { 1 } else { 0 },
            existing.id
        ],
    )?;

    get_location(conn, &existing.id)?
        .ok_or_else(|| LocationError::Database("Failed to retrieve updated location".into()))
}

/// Deactivates a location with active children and active bins dependency guards.
pub fn deactivate_location(conn: &Connection, id: &str) -> Result<Location, LocationError> {
    update_location(
        conn,
        UpdateLocationInput {
            id: id.to_string(),
            name: None,
            code: None,
            parent_id: None,
            location_type: None,
            is_active: Some(false),
        },
    )
}

/// Reactivates an inactive location. If it has a parent, verifies the parent is active.
pub fn reactivate_location(conn: &Connection, id: &str) -> Result<Location, LocationError> {
    let existing = get_location(conn, id)?
        .ok_or_else(|| LocationError::NotFound(format!("Location '{id}' not found")))?;

    if let Some(ref pid) = existing.parent_id {
        let parent_active: Option<bool> = conn
            .query_row(
                "SELECT is_active = 1 FROM locations WHERE id = ?1",
                [pid],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| LocationError::Database(e.to_string()))?;

        if let Some(false) = parent_active {
            return Err(LocationError::InactiveParent(format!(
                "Cannot reactivate location '{id}': its parent location '{pid}' is inactive"
            )));
        }
    }

    update_location(
        conn,
        UpdateLocationInput {
            id: id.to_string(),
            name: None,
            code: None,
            parent_id: None,
            location_type: None,
            is_active: Some(true),
        },
    )
}

/// Builds the hierarchical location tree for a branch, embedding child locations and terminal bins.
pub fn get_location_tree(
    conn: &Connection,
    branch_id: &str,
    include_inactive: bool,
) -> Result<Vec<LocationTreeNode>, LocationError> {
    let filter = LocationFilter {
        branch_id: branch_id.to_string(),
        parent_id: None,
        is_active: if include_inactive { None } else { Some(true) },
        query: None,
    };
    let all_locations = list_locations(conn, &filter)?;

    let bin_filter = BinFilter {
        location_id: None,
        branch_id: Some(branch_id.to_string()),
        is_active: if include_inactive { None } else { Some(true) },
        query: None,
    };
    let all_bins = list_bins(conn, &bin_filter)?;

    // Group bins by location_id
    let mut bins_by_loc: std::collections::HashMap<String, Vec<Bin>> =
        std::collections::HashMap::new();
    for bin in all_bins {
        bins_by_loc
            .entry(bin.location_id.clone())
            .or_default()
            .push(bin);
    }

    // Group locations by parent_id
    let mut children_by_parent: std::collections::HashMap<Option<String>, Vec<Location>> =
        std::collections::HashMap::new();
    for loc in all_locations {
        children_by_parent
            .entry(loc.parent_id.clone())
            .or_default()
            .push(loc);
    }

    // Recursively build tree nodes starting from roots (parent_id IS NULL)
    fn build_node(
        loc: Location,
        children_map: &std::collections::HashMap<Option<String>, Vec<Location>>,
        bins_map: &std::collections::HashMap<String, Vec<Bin>>,
    ) -> LocationTreeNode {
        let child_locs = children_map
            .get(&Some(loc.id.clone()))
            .cloned()
            .unwrap_or_default();
        let children = child_locs
            .into_iter()
            .map(|c| build_node(c, children_map, bins_map))
            .collect();
        let bins = bins_map.get(&loc.id).cloned().unwrap_or_default();

        LocationTreeNode {
            location: loc,
            children,
            bins,
        }
    }

    let roots = children_by_parent.remove(&None).unwrap_or_default();
    let tree = roots
        .into_iter()
        .map(|r| build_node(r, &children_by_parent, &bins_by_loc))
        .collect();

    Ok(tree)
}

// =========================================================================
// BIN OPERATIONS
// =========================================================================

/// Creates a new addressable bin within a location.
pub fn create_bin(conn: &Connection, input: CreateBinInput) -> Result<Bin, LocationError> {
    let location_id = validate_and_trim_str(&input.location_id, "location_id")?;
    let name = validate_and_trim_str(&input.name, "name")?;
    let code = validate_and_trim_str(&input.code, "code")?;

    // Parent location must exist and be active
    let parent_active: Option<bool> = conn
        .query_row(
            "SELECT is_active = 1 FROM locations WHERE id = ?1",
            [&location_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| LocationError::Database(e.to_string()))?;

    match parent_active {
        Some(true) => {}
        Some(false) => {
            return Err(LocationError::InactiveParent(format!(
                "Cannot create bin in inactive location '{location_id}'"
            )));
        }
        None => {
            return Err(LocationError::NotFound(format!(
                "Location '{location_id}' not found"
            )));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 1, datetime('now'), datetime('now'))",
        params![id, location_id, name, code],
    )?;

    get_bin(conn, &id)?
        .ok_or_else(|| LocationError::Database("Failed to retrieve created bin".into()))
}

/// Retrieves a bin by primary key.
pub fn get_bin(conn: &Connection, id: &str) -> Result<Option<Bin>, LocationError> {
    conn.query_row(
        "SELECT id, location_id, name, code, is_active, created_at, updated_at
         FROM bins WHERE id = ?1",
        [id],
        row_to_bin,
    )
    .optional()
    .map_err(|e| LocationError::Database(e.to_string()))
}

/// Resolves the branch_id for a given bin via its parent location.
pub fn get_bin_branch_id(conn: &Connection, bin_id: &str) -> Result<Option<String>, LocationError> {
    conn.query_row(
        "SELECT l.branch_id
         FROM bins b
         JOIN locations l ON b.location_id = l.id
         WHERE b.id = ?1",
        [bin_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| LocationError::Database(e.to_string()))
}

/// Lists bins matching the given filter.
pub fn list_bins(conn: &Connection, filter: &BinFilter) -> Result<Vec<Bin>, LocationError> {
    let mut sql = String::from(
        "SELECT b.id, b.location_id, b.name, b.code, b.is_active, b.created_at, b.updated_at
         FROM bins b",
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut clauses: Vec<String> = Vec::new();

    if let Some(ref branch_id) = filter.branch_id {
        sql.push_str(" JOIN locations l ON b.location_id = l.id");
        params_vec.push(Box::new(branch_id.clone()));
        clauses.push(format!("l.branch_id = ?{}", params_vec.len()));
    }

    if let Some(ref loc_id) = filter.location_id {
        params_vec.push(Box::new(loc_id.clone()));
        clauses.push(format!("b.location_id = ?{}", params_vec.len()));
    }

    if let Some(active) = filter.is_active {
        params_vec.push(Box::new(if active { 1 } else { 0 }));
        clauses.push(format!("b.is_active = ?{}", params_vec.len()));
    }

    if let Some(ref q) = filter.query {
        let trimmed_q = q.trim();
        if !trimmed_q.is_empty() {
            let pattern = format!("%{trimmed_q}%");
            params_vec.push(Box::new(pattern));
            let idx = params_vec.len();
            clauses.push(format!("(b.name LIKE ?{idx} OR b.code LIKE ?{idx})"));
        }
    }

    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }

    sql.push_str(" ORDER BY b.code COLLATE NOCASE ASC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| LocationError::Database(e.to_string()))?;

    let bins = stmt
        .query_map(params_slice.as_slice(), row_to_bin)
        .map_err(|e| LocationError::Database(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(bins)
}

/// Updates an existing bin. Explicitly updates `updated_at = datetime('now')`.
pub fn update_bin(conn: &Connection, input: UpdateBinInput) -> Result<Bin, LocationError> {
    let existing = get_bin(conn, &input.id)?
        .ok_or_else(|| LocationError::NotFound(format!("Bin '{}' not found", input.id)))?;

    let name = match input.name {
        Some(n) => validate_and_trim_str(&n, "name")?,
        None => existing.name,
    };

    let code = match input.code {
        Some(c) => validate_and_trim_str(&c, "code")?,
        None => existing.code,
    };

    let is_active = input.is_active.unwrap_or(existing.is_active);

    // If reactivating, verify that the parent location is active
    if is_active && !existing.is_active {
        let parent_active: Option<bool> = conn
            .query_row(
                "SELECT is_active = 1 FROM locations WHERE id = ?1",
                [&existing.location_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| LocationError::Database(e.to_string()))?;

        if let Some(false) = parent_active {
            return Err(LocationError::InactiveParent(format!(
                "Cannot activate bin '{}': its parent location '{}' is inactive",
                existing.id, existing.location_id
            )));
        }
    }

    conn.execute(
        "UPDATE bins
         SET name = ?1, code = ?2, is_active = ?3, updated_at = datetime('now')
         WHERE id = ?4",
        params![name, code, if is_active { 1 } else { 0 }, existing.id],
    )?;

    get_bin(conn, &existing.id)?
        .ok_or_else(|| LocationError::Database("Failed to retrieve updated bin".into()))
}

/// Deactivates a bin.
pub fn deactivate_bin(conn: &Connection, id: &str) -> Result<Bin, LocationError> {
    update_bin(
        conn,
        UpdateBinInput {
            id: id.to_string(),
            name: None,
            code: None,
            is_active: Some(false),
        },
    )
}

/// Reactivates an inactive bin. Verifies parent location is active.
pub fn reactivate_bin(conn: &Connection, id: &str) -> Result<Bin, LocationError> {
    update_bin(
        conn,
        UpdateBinInput {
            id: id.to_string(),
            name: None,
            code: None,
            is_active: Some(true),
        },
    )
}

// =========================================================================
// ROW MAPPERS & UTILITIES
// =========================================================================

fn row_to_location(row: &rusqlite::Row<'_>) -> rusqlite::Result<Location> {
    Ok(Location {
        id: row.get(0)?,
        branch_id: row.get(1)?,
        parent_id: row.get(2)?,
        name: row.get(3)?,
        code: row.get(4)?,
        location_type: row.get(5)?,
        is_active: row.get::<_, i64>(6)? == 1,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn row_to_bin(row: &rusqlite::Row<'_>) -> rusqlite::Result<Bin> {
    Ok(Bin {
        id: row.get(0)?,
        location_id: row.get(1)?,
        name: row.get(2)?,
        code: row.get(3)?,
        is_active: row.get::<_, i64>(4)? == 1,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}
