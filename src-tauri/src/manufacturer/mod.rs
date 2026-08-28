// Manufacturer domain model, validation rules, and SQLite database operations.
// F2.02 — Categories, Brands, Manufacturers

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Canonical Manufacturer entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manufacturer {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub support_phone: Option<String>,
    pub support_email: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input payload for creating a manufacturer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateManufacturerInput {
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub support_phone: Option<String>,
    pub support_email: Option<String>,
}

/// Input payload for updating a manufacturer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManufacturerInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub support_phone: Option<String>,
    pub support_email: Option<String>,
    pub is_active: bool,
}

/// Filter parameters for listing manufacturers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManufacturerFilter {
    pub query: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ManufacturerError {
    Validation(String),
    NotFound(String),
    DuplicateName(String),
    Database(String),
}

impl std::fmt::Display for ManufacturerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManufacturerError::Validation(msg) => write!(f, "Validation error: {msg}"),
            ManufacturerError::NotFound(msg) => write!(f, "Not found: {msg}"),
            ManufacturerError::DuplicateName(msg) => write!(f, "Duplicate name error: {msg}"),
            ManufacturerError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for ManufacturerError {}

impl From<rusqlite::Error> for ManufacturerError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref f, Some(ref msg)) = e {
            if f.code == rusqlite::ffi::ErrorCode::ConstraintViolation
                && (msg.contains("idx_manufacturers_name_active")
                    || msg.contains("UNIQUE constraint failed: manufacturers.name"))
            {
                return ManufacturerError::DuplicateName(
                    "An active manufacturer with this name already exists".into(),
                );
            }
        }
        ManufacturerError::Database(e.to_string())
    }
}

/// Escapes wildcard characters (% and _) in user queries for SQL LIKE with ESCAPE '\\'.
pub fn escape_like_pattern(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        if c == '%' || c == '_' || c == '\\' {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// Validates manufacturer name. Must be non-empty and <= 255 Unicode characters.
pub fn validate_name(name: &str) -> Result<String, ManufacturerError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ManufacturerError::Validation(
            "Manufacturer name cannot be empty".into(),
        ));
    }
    if trimmed.chars().count() > 255 {
        return Err(ManufacturerError::Validation(
            "Manufacturer name exceeds maximum length of 255 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates manufacturer description. Trims whitespace and normalizes empty string to None.
pub fn validate_description(desc: Option<&str>) -> Option<String> {
    desc.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// Validates manufacturer website. Conservative check: <= 2048 chars, no spaces.
pub fn validate_website(url: Option<&str>) -> Result<Option<String>, ManufacturerError> {
    match url.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => {
            if s.chars().count() > 2048 {
                return Err(ManufacturerError::Validation(
                    "Website URL exceeds maximum length of 2048 characters".into(),
                ));
            }
            if s.contains(char::is_whitespace) {
                return Err(ManufacturerError::Validation(
                    "Website URL cannot contain whitespace".into(),
                ));
            }
            if !s.starts_with("http://") && !s.starts_with("https://") && !s.contains('.') {
                return Err(ManufacturerError::Validation(
                    "Website URL must be a valid web address or domain".into(),
                ));
            }
            Ok(Some(s.to_string()))
        }
    }
}

/// Validates manufacturer support phone. Conservative international format check.
pub fn validate_phone(phone: Option<&str>) -> Result<Option<String>, ManufacturerError> {
    match phone.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => {
            if s.chars().count() > 50 {
                return Err(ManufacturerError::Validation(
                    "Support phone exceeds maximum length of 50 characters".into(),
                ));
            }
            let valid = s.chars().all(|c| {
                c.is_ascii_digit()
                    || c == '+'
                    || c == '-'
                    || c == ' '
                    || c == '('
                    || c == ')'
                    || c == '.'
            });
            if !valid {
                return Err(ManufacturerError::Validation(
                    "Support phone contains invalid characters".into(),
                ));
            }
            Ok(Some(s.to_string()))
        }
    }
}

/// Validates manufacturer support email. Conservative format check.
pub fn validate_email(email: Option<&str>) -> Result<Option<String>, ManufacturerError> {
    match email.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => {
            if s.chars().count() > 255 {
                return Err(ManufacturerError::Validation(
                    "Support email exceeds maximum length of 255 characters".into(),
                ));
            }
            if s.contains(char::is_whitespace) {
                return Err(ManufacturerError::Validation(
                    "Support email cannot contain whitespace".into(),
                ));
            }
            let parts: Vec<&str> = s.split('@').collect();
            if parts.len() != 2
                || parts[0].is_empty()
                || parts[1].is_empty()
                || !parts[1].contains('.')
            {
                return Err(ManufacturerError::Validation(
                    "Support email must be a valid email address".into(),
                ));
            }
            Ok(Some(s.to_string()))
        }
    }
}

const MANUFACTURER_COLUMNS: &str =
    "id, name, description, website, support_phone, support_email, is_active, created_at, updated_at";

fn map_manufacturer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Manufacturer> {
    let is_active_int: i64 = row.get("is_active")?;
    Ok(Manufacturer {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        website: row.get("website")?,
        support_phone: row.get("support_phone")?,
        support_email: row.get("support_email")?,
        is_active: is_active_int != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Creates a new manufacturer.
pub fn create_manufacturer(
    conn: &Connection,
    input: CreateManufacturerInput,
) -> Result<Manufacturer, ManufacturerError> {
    let name = validate_name(&input.name)?;
    let description = validate_description(input.description.as_deref());
    let website = validate_website(input.website.as_deref())?;
    let support_phone = validate_phone(input.support_phone.as_deref())?;
    let support_email = validate_email(input.support_email.as_deref())?;

    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(

        "INSERT INTO manufacturers (
            id, name, description, website, support_phone, support_email, is_active, created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, 1, datetime('now'), datetime('now')
        )",
        params![id, name, description, website, support_phone, support_email],
    )?;

    get_manufacturer(conn, &id)?.ok_or_else(|| {
        ManufacturerError::Database("Failed to retrieve created manufacturer".into())
    })
}

/// Retrieves a manufacturer by unique ID.
pub fn get_manufacturer(
    conn: &Connection,
    id: &str,
) -> Result<Option<Manufacturer>, ManufacturerError> {
    let sql = format!("SELECT {MANUFACTURER_COLUMNS} FROM manufacturers WHERE id = ?1");
    let result = conn
        .query_row(&sql, [id], map_manufacturer_row)
        .optional()?;
    Ok(result)
}

/// Updates an existing manufacturer.
pub fn update_manufacturer(
    conn: &Connection,
    input: UpdateManufacturerInput,
) -> Result<Manufacturer, ManufacturerError> {
    let name = validate_name(&input.name)?;
    let description = validate_description(input.description.as_deref());
    let website = validate_website(input.website.as_deref())?;
    let support_phone = validate_phone(input.support_phone.as_deref())?;
    let support_email = validate_email(input.support_email.as_deref())?;

    let is_active_int = if input.is_active { 1 } else { 0 };

    let affected = conn.execute(
        "UPDATE manufacturers SET
            name = ?1,
            description = ?2,
            website = ?3,
            support_phone = ?4,
            support_email = ?5,
            is_active = ?6,
            updated_at = datetime('now')
        WHERE id = ?7",
        params![
            name,
            description,
            website,
            support_phone,
            support_email,
            is_active_int,
            input.id
        ],
    )?;

    if affected == 0 {
        return Err(ManufacturerError::NotFound(format!(
            "Manufacturer '{}' not found",
            input.id
        )));
    }

    get_manufacturer(conn, &input.id)?.ok_or_else(|| {
        ManufacturerError::Database("Failed to retrieve updated manufacturer".into())
    })
}

/// Soft-deletes a manufacturer by setting `is_active = 0`.
pub fn delete_manufacturer(conn: &Connection, id: &str) -> Result<(), ManufacturerError> {
    let existing = get_manufacturer(conn, id)?
        .ok_or_else(|| ManufacturerError::NotFound(format!("Manufacturer '{id}' not found")))?;

    if !existing.is_active {
        return Ok(()); // Idempotent soft delete
    }

    conn.execute(
        "UPDATE manufacturers SET
            is_active = 0,
            updated_at = datetime('now')
        WHERE id = ?1 AND is_active = 1",
        [id],
    )?;

    Ok(())
}

/// Lists manufacturers matching the specified filter.
pub fn list_manufacturers(
    conn: &Connection,
    filter: &ManufacturerFilter,
) -> Result<Vec<Manufacturer>, ManufacturerError> {
    let mut sql = format!("SELECT {MANUFACTURER_COLUMNS} FROM manufacturers WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(active) = filter.is_active {
        sql.push_str(" AND is_active = ?");
        params_vec.push(Box::new(if active { 1 } else { 0 }));
    }

    if let Some(ref q) = filter.query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            sql.push_str(" AND (name LIKE ? ESCAPE '\\' OR description LIKE ? ESCAPE '\\')");
            let pattern = format!("%{}%", escape_like_pattern(trimmed));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }
    }

    sql.push_str(" ORDER BY name COLLATE NOCASE ASC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();
    let mut stmt = conn.prepare(&sql)?;
    let mfr_iter = stmt.query_map(params_slice.as_slice(), map_manufacturer_row)?;

    let mut manufacturers = Vec::new();
    for m in mfr_iter {
        manufacturers.push(m?);
    }
    Ok(manufacturers)
}
