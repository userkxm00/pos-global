// Branch domain model, validation rules, and database operations.
// F1.02 — Branch Model

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Branch {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub address: Option<String>,
    pub currency: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBranchInput {
    pub organization_id: String,
    pub name: String,
    pub address: Option<String>,
    pub currency: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBranchInput {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub currency: String,
    pub is_active: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BranchError {
    Validation(String),
    NotFound(String),
    InvalidOrganization(String),
    Database(String),
}

impl std::fmt::Display for BranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchError::Validation(msg) => {
                write!(f, "Validation error: {msg}")
            }
            BranchError::NotFound(msg) => {
                write!(f, "Branch not found: {msg}")
            }
            BranchError::InvalidOrganization(msg) => {
                write!(f, "Invalid organization: {msg}")
            }
            BranchError::Database(msg) => {
                write!(f, "Database error: {msg}")
            }
        }
    }
}

impl std::error::Error for BranchError {}

impl From<rusqlite::Error> for BranchError {
    fn from(e: rusqlite::Error) -> Self {
        BranchError::Database(e.to_string())
    }
}

/// Validates branch name. Must be non-empty and maximum 255 characters.
pub fn validate_name(name: &str) -> Result<String, BranchError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(BranchError::Validation(
            "Branch name cannot be empty".into(),
        ));
    }
    if trimmed.chars().count() > 255 {
        return Err(BranchError::Validation(
            "Branch name exceeds maximum length of 255 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates currency code. Must be a 3-character uppercase ISO code.
pub fn validate_currency(currency: &str) -> Result<String, BranchError> {
    let trimmed = currency.trim();
    if trimmed.len() != 3 || !trimmed.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(BranchError::Validation(format!(
            "Invalid ISO currency code '{trimmed}'"
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates branch identifier.
pub fn validate_id(id: &str) -> Result<String, BranchError> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(BranchError::Validation("Branch ID cannot be empty".into()));
    }
    Ok(trimmed.to_string())
}

/// Validates organization identifier and verifies that the organization exists.
pub fn validate_organization_id(
    conn: &Connection,
    organization_id: &str,
) -> Result<String, BranchError> {
    let trimmed = organization_id.trim();
    if trimmed.is_empty() {
        return Err(BranchError::Validation(
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
        return Err(BranchError::InvalidOrganization(format!(
            "Organization with ID '{trimmed}' does not exist"
        )));
    }

    Ok(trimmed.to_string())
}

/// Creates a new branch attached to an existing organization in the local database.
pub fn create_branch(conn: &Connection, input: CreateBranchInput) -> Result<Branch, BranchError> {
    let org_id = validate_organization_id(conn, &input.organization_id)?;
    let name = validate_name(&input.name)?;

    let currency = match input.currency {
        Some(ref c) => validate_currency(c)?,
        None => {
            let org_curr: Option<String> = conn
                .query_row(
                    "SELECT default_currency FROM organizations WHERE id = ?1",
                    params![org_id],
                    |row| row.get(0),
                )
                .optional()?;

            match org_curr {
                Some(curr) => curr,
                None => {
                    return Err(BranchError::InvalidOrganization(format!(
                        "Organization with ID '{org_id}' does not exist"
                    )))
                }
            }
        }
    };

    let address = input
        .address
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());
    let is_active = input.is_active.unwrap_or(true);
    let is_active_int = if is_active { 1 } else { 0 };

    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO branches (id, organization_id, name, address, currency, is_active, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
        params![id, org_id, name, address, currency, is_active_int],
    )?;

    match get_branch(conn, &id)? {
        Some(b) => Ok(b),
        None => Err(BranchError::Database(
            "Failed to load newly created branch".into(),
        )),
    }
}

/// Retrieves a branch by ID.
pub fn get_branch(conn: &Connection, id: &str) -> Result<Option<Branch>, BranchError> {
    let id = validate_id(id)?;

    let mut stmt = conn.prepare(
        "SELECT id, organization_id, name, address, currency, is_active, created_at
         FROM branches
         WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let branch_id: String = row.get(0)?;
        let org_id_opt: Option<String> = row.get(1)?;
        let organization_id = org_id_opt.ok_or_else(|| {
            BranchError::Database(format!(
                "Branch '{branch_id}' has corrupt or NULL organization_id"
            ))
        })?;
        let name: String = row.get(2)?;
        let address: Option<String> = row.get(3)?;
        let currency: String = row.get(4)?;
        let is_active_int: i32 = row.get(5)?;
        let created_at: String = row.get(6)?;

        Ok(Some(Branch {
            id: branch_id,
            organization_id,
            name,
            address,
            currency,
            is_active: is_active_int != 0,
            created_at,
        }))
    } else {
        Ok(None)
    }
}

/// Updates an existing branch.
pub fn update_branch(conn: &Connection, input: UpdateBranchInput) -> Result<Branch, BranchError> {
    let id = validate_id(&input.id)?;
    let name = validate_name(&input.name)?;
    let currency = validate_currency(&input.currency)?;
    let address = input
        .address
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());
    let is_active_int = if input.is_active { 1 } else { 0 };

    let rows_affected = conn.execute(
        "UPDATE branches
         SET name = ?2,
             address = ?3,
             currency = ?4,
             is_active = ?5
         WHERE id = ?1",
        params![id, name, address, currency, is_active_int],
    )?;

    if rows_affected == 0 {
        return Err(BranchError::NotFound(format!(
            "Branch with ID '{id}' does not exist"
        )));
    }

    match get_branch(conn, &id)? {
        Some(b) => Ok(b),
        None => Err(BranchError::NotFound(format!("Branch '{id}' disappeared"))),
    }
}

/// Lists all branches belonging to a specific organization.
pub fn list_branches(conn: &Connection, organization_id: &str) -> Result<Vec<Branch>, BranchError> {
    let org_id = organization_id.trim();
    if org_id.is_empty() {
        return Err(BranchError::Validation(
            "Organization ID cannot be empty".into(),
        ));
    }

    let mut stmt = conn.prepare(
        "SELECT id, organization_id, name, address, currency, is_active, created_at
         FROM branches
         WHERE organization_id = ?1
         ORDER BY created_at ASC, id ASC",
    )?;

    let mut rows = stmt.query(params![org_id])?;
    let mut branches = Vec::new();
    while let Some(row) = rows.next()? {
        let branch_id: String = row.get(0)?;
        let org_id_opt: Option<String> = row.get(1)?;
        let branch_org_id = org_id_opt.ok_or_else(|| {
            BranchError::Database(format!(
                "Branch '{branch_id}' has corrupt or NULL organization_id"
            ))
        })?;
        let name: String = row.get(2)?;
        let address: Option<String> = row.get(3)?;
        let currency: String = row.get(4)?;
        let is_active_int: i32 = row.get(5)?;
        let created_at: String = row.get(6)?;

        branches.push(Branch {
            id: branch_id,
            organization_id: branch_org_id,
            name,
            address,
            currency,
            is_active: is_active_int != 0,
            created_at,
        });
    }

    Ok(branches)
}
