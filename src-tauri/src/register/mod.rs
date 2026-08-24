// Register domain model, validation rules, and database operations.
// F1.03 — Register / Device Model

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Register {
    pub id: String,
    pub organization_id: String,
    pub branch_id: String,
    pub name: String,
    pub code: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRegisterInput {
    pub organization_id: String,
    pub branch_id: String,
    pub name: String,
    pub code: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRegisterInput {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RegisterError {
    Validation(String),
    NotFound(String),
    InvalidOrganization(String),
    InvalidBranch(String),
    Database(String),
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterError::Validation(msg) => {
                write!(f, "Validation error: {msg}")
            }
            RegisterError::NotFound(msg) => {
                write!(f, "Register not found: {msg}")
            }
            RegisterError::InvalidOrganization(msg) => {
                write!(f, "Invalid organization: {msg}")
            }
            RegisterError::InvalidBranch(msg) => {
                write!(f, "Invalid branch: {msg}")
            }
            RegisterError::Database(msg) => {
                write!(f, "Database error: {msg}")
            }
        }
    }
}

impl std::error::Error for RegisterError {}

impl From<rusqlite::Error> for RegisterError {
    fn from(e: rusqlite::Error) -> Self {
        RegisterError::Database(e.to_string())
    }
}

/// Validates register display name. Must be non-empty and maximum 255 Unicode characters.
pub fn validate_name(name: &str) -> Result<String, RegisterError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(RegisterError::Validation(
            "Register name cannot be empty".into(),
        ));
    }
    if trimmed.chars().count() > 255 {
        return Err(RegisterError::Validation(
            "Register name exceeds maximum length of 255 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates register identifier.
pub fn validate_id(id: &str) -> Result<String, RegisterError> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(RegisterError::Validation(
            "Register ID cannot be empty".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates organization identifier and verifies that the organization exists.
pub fn validate_organization_id(
    conn: &Connection,
    organization_id: &str,
) -> Result<String, RegisterError> {
    let trimmed = organization_id.trim();
    if trimmed.is_empty() {
        return Err(RegisterError::Validation(
            "Organization ID cannot be empty".into(),
        ));
    }

    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM organizations WHERE id = ?1",
            params![trimmed],
            |_| Ok(()),
        )
        .optional()?
        .is_some();

    if !exists {
        return Err(RegisterError::InvalidOrganization(format!(
            "Organization with ID '{trimmed}' does not exist"
        )));
    }

    Ok(trimmed.to_string())
}

/// Validates branch identifier, verifies that the branch exists, and ensures
/// the branch belongs to the specified organization (enforcing tenant hierarchy).
pub fn validate_branch_id(
    conn: &Connection,
    organization_id: &str,
    branch_id: &str,
) -> Result<String, RegisterError> {
    let trimmed_branch = branch_id.trim();
    if trimmed_branch.is_empty() {
        return Err(RegisterError::Validation(
            "Branch ID cannot be empty".into(),
        ));
    }

    let branch_org: Option<Option<String>> = conn
        .query_row(
            "SELECT organization_id FROM branches WHERE id = ?1",
            params![trimmed_branch],
            |row| row.get(0),
        )
        .optional()?;

    match branch_org {
        Some(Some(org)) if org == organization_id => Ok(trimmed_branch.to_string()),
        Some(Some(org)) => Err(RegisterError::InvalidBranch(format!(
            "Branch '{trimmed_branch}' belongs to organization '{org}', not '{organization_id}'"
        ))),
        Some(None) => Err(RegisterError::InvalidBranch(format!(
            "Branch '{trimmed_branch}' has corrupt or NULL organization_id"
        ))),
        None => Err(RegisterError::InvalidBranch(format!(
            "Branch with ID '{trimmed_branch}' does not exist"
        ))),
    }
}

/// Creates a new register attached to an existing branch and organization in SQLite.
pub fn create_register(
    conn: &Connection,
    input: CreateRegisterInput,
) -> Result<Register, RegisterError> {
    let org_id = validate_organization_id(conn, &input.organization_id)?;
    let branch_id = validate_branch_id(conn, &org_id, &input.branch_id)?;
    let name = validate_name(&input.name)?;
    let code = input
        .code
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty());
    let is_active = input.is_active.unwrap_or(true);
    let is_active_int = if is_active { 1 } else { 0 };

    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO registers (id, organization_id, branch_id, name, code, is_active, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
        params![id, org_id, branch_id, name, code, is_active_int],
    )?;

    match get_register(conn, &id)? {
        Some(r) => Ok(r),
        None => Err(RegisterError::Database(
            "Failed to load newly created register".into(),
        )),
    }
}

/// Retrieves a register by ID.
pub fn get_register(conn: &Connection, id: &str) -> Result<Option<Register>, RegisterError> {
    let id = validate_id(id)?;

    let mut stmt = conn.prepare(
        "SELECT id, organization_id, branch_id, name, code, is_active, created_at
         FROM registers
         WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let register_id: String = row.get(0)?;
        let org_id_opt: Option<String> = row.get(1)?;
        let organization_id = org_id_opt.ok_or_else(|| {
            RegisterError::Database(format!(
                "Register '{register_id}' has corrupt or NULL organization_id"
            ))
        })?;
        let branch_id_opt: Option<String> = row.get(2)?;
        let branch_id = branch_id_opt.ok_or_else(|| {
            RegisterError::Database(format!(
                "Register '{register_id}' has corrupt or NULL branch_id"
            ))
        })?;
        let name: String = row.get(3)?;
        let code: Option<String> = row.get(4)?;
        let is_active_int: i32 = row.get(5)?;
        let created_at: String = row.get(6)?;

        Ok(Some(Register {
            id: register_id,
            organization_id,
            branch_id,
            name,
            code,
            is_active: is_active_int != 0,
            created_at,
        }))
    } else {
        Ok(None)
    }
}

/// Updates an existing register.
pub fn update_register(
    conn: &Connection,
    input: UpdateRegisterInput,
) -> Result<Register, RegisterError> {
    let id = validate_id(&input.id)?;
    let name = validate_name(&input.name)?;
    let code = input
        .code
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty());
    let is_active_int = if input.is_active { 1 } else { 0 };

    let rows_affected = conn.execute(
        "UPDATE registers
         SET name = ?2,
             code = ?3,
             is_active = ?4
         WHERE id = ?1",
        params![id, name, code, is_active_int],
    )?;

    if rows_affected == 0 {
        return Err(RegisterError::NotFound(format!(
            "Register with ID '{id}' does not exist"
        )));
    }

    match get_register(conn, &id)? {
        Some(r) => Ok(r),
        None => Err(RegisterError::NotFound(format!(
            "Register '{id}' disappeared"
        ))),
    }
}

/// Lists all registers belonging to a specific branch.
pub fn list_registers(conn: &Connection, branch_id: &str) -> Result<Vec<Register>, RegisterError> {
    let b_id = branch_id.trim();
    if b_id.is_empty() {
        return Err(RegisterError::Validation(
            "Branch ID cannot be empty".into(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, organization_id, branch_id, name, code, is_active, created_at
         FROM registers
         WHERE branch_id = ?1
         ORDER BY created_at ASC, id ASC",
    )?;

    let mut rows = stmt.query(params![b_id])?;
    let mut registers = Vec::new();
    while let Some(row) = rows.next()? {
        let register_id: String = row.get(0)?;
        let org_id_opt: Option<String> = row.get(1)?;
        let organization_id = org_id_opt.ok_or_else(|| {
            RegisterError::Database(format!(
                "Register '{register_id}' has corrupt or NULL organization_id"
            ))
        })?;
        let branch_id_opt: Option<String> = row.get(2)?;
        let reg_branch_id = branch_id_opt.ok_or_else(|| {
            RegisterError::Database(format!(
                "Register '{register_id}' has corrupt or NULL branch_id"
            ))
        })?;
        let name: String = row.get(3)?;
        let code: Option<String> = row.get(4)?;
        let is_active_int: i32 = row.get(5)?;
        let created_at: String = row.get(6)?;

        registers.push(Register {
            id: register_id,
            organization_id,
            branch_id: reg_branch_id,
            name,
            code,
            is_active: is_active_int != 0,
            created_at,
        });
    }

    Ok(registers)
}
