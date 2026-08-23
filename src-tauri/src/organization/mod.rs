// Organization domain model, validation rules, and database operations.
// F1.01 — Organization Model

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub default_currency: String,
    pub default_language: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationInput {
    pub name: String,
    pub default_currency: Option<String>,
    pub default_language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganizationInput {
    pub id: String,
    pub name: String,
    pub default_currency: String,
    pub default_language: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OrganizationError {
    Validation(String),
    NotFound(String),
    Database(String),
}

impl std::fmt::Display for OrganizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrganizationError::Validation(msg) => write!(f, "Validation error: {msg}"),
            OrganizationError::NotFound(msg) => write!(f, "Organization not found: {msg}"),
            OrganizationError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for OrganizationError {}

impl From<rusqlite::Error> for OrganizationError {
    fn from(e: rusqlite::Error) -> Self {
        OrganizationError::Database(e.to_string())
    }
}

/// Validates organization name. Must be non-empty and within reasonable length.
pub fn validate_name(name: &str) -> Result<String, OrganizationError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(OrganizationError::Validation(
            "Organization name cannot be empty".into(),
        ));
    }
    if trimmed.len() > 255 {
        return Err(OrganizationError::Validation(
            "Organization name exceeds maximum length of 255 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates currency code. Must be a 3-character uppercase ISO code.
pub fn validate_currency(currency: &str) -> Result<String, OrganizationError> {
    let trimmed = currency.trim();
    if trimmed.len() != 3 || !trimmed.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(OrganizationError::Validation(format!(
            "Invalid ISO currency code '{trimmed}'. Expected 3 uppercase ASCII letters (e.g. 'USD', 'DZD', 'EUR')"
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates language code. Must be 2-10 ASCII characters (e.g. 'en', 'fr', 'ar').
pub fn validate_language(language: &str) -> Result<String, OrganizationError> {
    let trimmed = language.trim();
    if trimmed.len() < 2
        || trimmed.len() > 10
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(OrganizationError::Validation(format!(
            "Invalid language code '{trimmed}'. Expected 2-10 alphanumeric characters (e.g. 'en', 'fr', 'ar')"
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates organization identifier.
pub fn validate_id(id: &str) -> Result<String, OrganizationError> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(OrganizationError::Validation(
            "Organization ID cannot be empty".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Creates a new organization in the local SQLite database.
pub fn create_organization(
    conn: &Connection,
    input: CreateOrganizationInput,
) -> Result<Organization, OrganizationError> {
    let name = validate_name(&input.name)?;
    let default_currency = match input.default_currency {
        Some(ref c) => validate_currency(c)?,
        None => "USD".to_string(),
    };
    let default_language = match input.default_language {
        Some(ref l) => validate_language(l)?,
        None => "en".to_string(),
    };

    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO organizations (id, name, default_currency, default_language, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![id, name, default_currency, default_language],
    )?;

    get_organization(conn, &id)?.ok_or_else(|| {
        OrganizationError::Database("Failed to load newly created organization".into())
    })
}

/// Retrieves an organization by ID.
pub fn get_organization(
    conn: &Connection,
    id: &str,
) -> Result<Option<Organization>, OrganizationError> {
    let id = validate_id(id)?;

    let org = conn
        .query_row(
            "SELECT id, name, default_currency, default_language, created_at
             FROM organizations
             WHERE id = ?1",
            params![id],
            |row| {
                Ok(Organization {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    default_currency: row.get(2)?,
                    default_language: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()?;

    Ok(org)
}

/// Updates an existing organization.
pub fn update_organization(
    conn: &Connection,
    input: UpdateOrganizationInput,
) -> Result<Organization, OrganizationError> {
    let id = validate_id(&input.id)?;
    let name = validate_name(&input.name)?;
    let default_currency = validate_currency(&input.default_currency)?;
    let default_language = validate_language(&input.default_language)?;

    let rows_affected = conn.execute(
        "UPDATE organizations
         SET name = ?2,
             default_currency = ?3,
             default_language = ?4
         WHERE id = ?1",
        params![id, name, default_currency, default_language],
    )?;

    if rows_affected == 0 {
        return Err(OrganizationError::NotFound(format!(
            "Organization with ID '{id}' does not exist"
        )));
    }

    get_organization(conn, &id)?
        .ok_or_else(|| OrganizationError::NotFound(format!("Organization '{id}' disappeared")))
}

/// Lists all organizations ordered by creation date.
pub fn list_organizations(conn: &Connection) -> Result<Vec<Organization>, OrganizationError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, default_currency, default_language, created_at
         FROM organizations
         ORDER BY created_at ASC, id ASC",
    )?;

    let org_iter = stmt.query_map([], |row| {
        Ok(Organization {
            id: row.get(0)?,
            name: row.get(1)?,
            default_currency: row.get(2)?,
            default_language: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;

    let mut organizations = Vec::new();
    for org in org_iter {
        organizations.push(org?);
    }

    Ok(organizations)
}
