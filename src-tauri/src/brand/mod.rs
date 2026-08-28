// Brand domain model, validation rules, and SQLite database operations.
// F2.02 — Categories, Brands, Manufacturers

use crate::db::escape_like_pattern;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Canonical Brand entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Brand {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Input payload for creating a brand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBrandInput {
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

/// Input payload for updating a brand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBrandInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub is_active: bool,
}

/// Filter parameters for listing brands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrandFilter {
    pub query: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BrandError {
    Validation(String),
    NotFound(String),
    DuplicateName(String),
    Database(String),
}

impl std::fmt::Display for BrandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrandError::Validation(msg) => write!(f, "Validation error: {msg}"),
            BrandError::NotFound(msg) => write!(f, "Not found: {msg}"),
            BrandError::DuplicateName(msg) => write!(f, "Duplicate name error: {msg}"),
            BrandError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for BrandError {}

impl From<rusqlite::Error> for BrandError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(ref f, Some(ref msg)) = e {
            if f.code == rusqlite::ffi::ErrorCode::ConstraintViolation
                && (msg.contains("idx_brands_name_active")
                    || msg.contains("UNIQUE constraint failed: brands.name"))
            {
                return BrandError::DuplicateName(
                    "An active brand with this name already exists".into(),
                );
            }
        }
        BrandError::Database(e.to_string())
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

/// Validates brand name. Must be non-empty and <= 255 Unicode characters.
pub fn validate_name(name: &str) -> Result<String, BrandError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(BrandError::Validation("Brand name cannot be empty".into()));
    }
    if trimmed.chars().count() > 255 {
        return Err(BrandError::Validation(
            "Brand name exceeds maximum length of 255 characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates brand description. Trims whitespace and normalizes empty string to None.
pub fn validate_description(desc: Option<&str>) -> Option<String> {
    desc.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// Validates brand website. Conservative check: <= 2048 chars, no spaces, valid host.
pub fn validate_website(url: Option<&str>) -> Result<Option<String>, BrandError> {
    crate::db::validate_url_syntax(url).map_err(|e| BrandError::Validation(e.to_string()))
}

const BRAND_COLUMNS: &str = "id, name, description, website, is_active, created_at, updated_at";

fn map_brand_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Brand> {
    let is_active_int: i64 = row.get("is_active")?;
    Ok(Brand {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        website: row.get("website")?,
        is_active: is_active_int != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Creates a new brand.
pub fn create_brand(conn: &Connection, input: CreateBrandInput) -> Result<Brand, BrandError> {
    let name = validate_name(&input.name)?;
    let description = validate_description(input.description.as_deref());
    let website = validate_website(input.website.as_deref())?;

    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO brands (
            id, name, description, website, is_active, created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, 1, datetime('now'), datetime('now')
        )",
        params![id, name, description, website],
    )?;

    get_brand(conn, &id)?
        .ok_or_else(|| BrandError::Database("Failed to retrieve created brand".into()))
}

/// Retrieves a brand by unique ID.
pub fn get_brand(conn: &Connection, id: &str) -> Result<Option<Brand>, BrandError> {
    let sql = format!("SELECT {BRAND_COLUMNS} FROM brands WHERE id = ?1");
    let result = conn.query_row(&sql, [id], map_brand_row).optional()?;
    Ok(result)
}

/// Updates an existing brand.
pub fn update_brand(conn: &Connection, input: UpdateBrandInput) -> Result<Brand, BrandError> {
    let name = validate_name(&input.name)?;
    let description = validate_description(input.description.as_deref());
    let website = validate_website(input.website.as_deref())?;

    let is_active_int = if input.is_active { 1 } else { 0 };

    let affected = conn.execute(
        "UPDATE brands SET
            name = ?1,
            description = ?2,
            website = ?3,
            is_active = ?4,
            updated_at = datetime('now')
        WHERE id = ?5",
        params![name, description, website, is_active_int, input.id],
    )?;

    if affected == 0 {
        return Err(BrandError::NotFound(format!(
            "Brand '{}' not found",
            input.id
        )));
    }

    get_brand(conn, &input.id)?
        .ok_or_else(|| BrandError::Database("Failed to retrieve updated brand".into()))
}

/// Soft-deletes a brand by setting `is_active = 0`.
pub fn delete_brand(conn: &Connection, id: &str) -> Result<(), BrandError> {
    let existing = get_brand(conn, id)?
        .ok_or_else(|| BrandError::NotFound(format!("Brand '{id}' not found")))?;

    if !existing.is_active {
        return Ok(()); // Idempotent soft delete
    }

    conn.execute(
        "UPDATE brands SET
            is_active = 0,
            updated_at = datetime('now')
        WHERE id = ?1 AND is_active = 1",
        [id],
    )?;

    Ok(())
}

/// Lists brands matching the specified filter.
pub fn list_brands(conn: &Connection, filter: &BrandFilter) -> Result<Vec<Brand>, BrandError> {
    let mut sql = format!("SELECT {BRAND_COLUMNS} FROM brands WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(active) = filter.is_active {
        sql.push_str(" AND is_active = ?");
        params_vec.push(Box::new(if active { 1 } else { 0 }));
    }

    crate::db::append_name_or_description_search(
        &mut sql,
        &mut params_vec,
        filter.query.as_deref(),
    );

    sql.push_str(" ORDER BY name COLLATE NOCASE ASC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();
    let mut stmt = conn.prepare(&sql)?;
    let brand_iter = stmt.query_map(params_slice.as_slice(), map_brand_row)?;

    let mut brands = Vec::new();
    for b in brand_iter {
        brands.push(b?);
    }
    Ok(brands)
}
