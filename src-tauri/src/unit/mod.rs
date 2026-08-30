// Unit of Measure (UOM) and Unit Conversion domain model, repository, and evaluation engine.
// F2.04 — Units & Conversions

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Supported unit dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitDimension {
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "mass")]
    Mass,
    #[serde(rename = "volume")]
    Volume,
    #[serde(rename = "length")]
    Length,
    #[serde(rename = "area")]
    Area,
    #[serde(rename = "custom")]
    Custom,
}

impl UnitDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnitDimension::Count => "count",
            UnitDimension::Mass => "mass",
            UnitDimension::Volume => "volume",
            UnitDimension::Length => "length",
            UnitDimension::Area => "area",
            UnitDimension::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Result<Self, UnitError> {
        match s.trim().to_lowercase().as_str() {
            "count" => Ok(UnitDimension::Count),
            "mass" => Ok(UnitDimension::Mass),
            "volume" => Ok(UnitDimension::Volume),
            "length" => Ok(UnitDimension::Length),
            "area" => Ok(UnitDimension::Area),
            "custom" => Ok(UnitDimension::Custom),
            other => Err(UnitError::Validation(format!(
                "Invalid unit dimension '{other}'. Allowed: count, mass, volume, length, area, custom"
            ))),
        }
    }
}

impl std::fmt::Display for UnitDimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Canonical Unit of Measure entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub id: String,
    pub code: String,
    pub name: String,
    pub dimension: UnitDimension,
    pub precision: u32,
    pub is_base: bool,
    pub created_at: String,
}

/// Canonical Unit Conversion rule entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitConversion {
    pub from_unit_id: String,
    pub to_unit_id: String,
    pub multiplier: f64,
    pub created_at: String,
}

/// Enriched Unit Conversion view with unit codes and names for UI/diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitConversionView {
    pub from_unit_id: String,
    pub from_unit_code: String,
    pub from_unit_name: String,
    pub to_unit_id: String,
    pub to_unit_code: String,
    pub to_unit_name: String,
    pub multiplier: f64,
    pub created_at: String,
}

/// Conversion calculation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversionResult {
    pub from_unit_id: String,
    pub from_unit_code: String,
    pub to_unit_id: String,
    pub to_unit_code: String,
    pub original_quantity: f64,
    pub converted_quantity: f64,
    pub effective_multiplier: f64,
}

/// Input payload for creating a unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUnitInput {
    pub code: String,
    pub name: String,
    pub dimension: String,
    pub precision: Option<u32>,
    pub is_base: Option<bool>,
}

/// Input payload for updating a unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUnitInput {
    pub id: String,
    pub code: String,
    pub name: String,
    pub dimension: String,
    pub precision: u32,
    pub is_base: bool,
}

/// Filter parameters for listing units.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitFilter {
    pub dimension: Option<String>,
    pub is_base: Option<bool>,
    pub query: Option<String>,
}

/// Input payload for creating/upserting a unit conversion rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUnitConversionInput {
    pub from_unit_id: String,
    pub to_unit_id: String,
    pub multiplier: f64,
}

/// Input payload for converting a quantity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertQuantityInput {
    pub from_unit_id: String,
    pub to_unit_id: String,
    pub quantity: f64,
}

/// Domain errors for Units & Conversions.
#[derive(Debug, PartialEq)]
pub enum UnitError {
    Validation(String),
    NotFound(String),
    DuplicateCode(String),
    IncompatibleDimensions {
        from_dimension: String,
        to_dimension: String,
    },
    ConversionPathNotFound {
        from_unit: String,
        to_unit: String,
    },
    ConversionCycleDetected(String),
    Database(String),
}

impl std::fmt::Display for UnitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitError::Validation(msg) => write!(f, "Validation error: {msg}"),
            UnitError::NotFound(msg) => write!(f, "Not found: {msg}"),
            UnitError::DuplicateCode(msg) => write!(f, "Duplicate unit code: {msg}"),
            UnitError::IncompatibleDimensions {
                from_dimension,
                to_dimension,
            } => write!(
                f,
                "Incompatible dimensions: cannot convert between '{from_dimension}' and '{to_dimension}' without an explicit conversion rule"
            ),
            UnitError::ConversionPathNotFound {
                from_unit,
                to_unit,
            } => write!(
                f,
                "No conversion path found from unit '{from_unit}' to unit '{to_unit}'"
            ),
            UnitError::ConversionCycleDetected(msg) => {
                write!(f, "Conversion cycle detected: {msg}")
            }
            UnitError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl std::error::Error for UnitError {}

impl From<rusqlite::Error> for UnitError {
    fn from(e: rusqlite::Error) -> Self {
        UnitError::Database(e.to_string())
    }
}

/// Validates unit code.
/// Requirements: 1 to 32 characters, allowed: alphanumeric, hyphens, underscores, dots, slashes.
pub fn validate_unit_code(code: &str) -> Result<String, UnitError> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return Err(UnitError::Validation("Unit code cannot be empty".into()));
    }
    if trimmed.len() > 32 {
        return Err(UnitError::Validation(format!(
            "Unit code cannot exceed 32 characters, got {}",
            trimmed.len()
        )));
    }
    let is_valid_char =
        |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '%');
    if !trimmed.chars().all(is_valid_char) {
        return Err(UnitError::Validation(
            "Unit code contains invalid characters. Allowed: alphanumeric, hyphens (-), underscores (_), dots (.), slashes (/), percent (%)".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validates unit name.
/// Requirements: 1 to 128 characters.
pub fn validate_unit_name(name: &str) -> Result<String, UnitError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(UnitError::Validation("Unit name cannot be empty".into()));
    }
    if trimmed.len() > 128 {
        return Err(UnitError::Validation(format!(
            "Unit name cannot exceed 128 characters, got {}",
            trimmed.len()
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates unit precision.
/// Requirements: 0 to 6 decimal places.
pub fn validate_unit_precision(precision: u32) -> Result<u32, UnitError> {
    if precision > 6 {
        return Err(UnitError::Validation(format!(
            "Unit precision must be between 0 and 6 decimal places, got {precision}"
        )));
    }
    Ok(precision)
}

/// Validates conversion multiplier.
/// Requirements: strictly positive, finite, non-NaN.
pub fn validate_multiplier(multiplier: f64) -> Result<f64, UnitError> {
    if !multiplier.is_finite() || multiplier.is_nan() || multiplier <= 0.0 {
        return Err(UnitError::Validation(format!(
            "Conversion multiplier must be a positive finite number greater than 0, got {multiplier}"
        )));
    }
    Ok(multiplier)
}

fn map_unit_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Unit> {
    let dim_str: String = row.get("dimension")?;
    let is_base_int: i64 = row.get("is_base")?;
    let precision_int: i64 = row.get("precision")?;
    let dimension = UnitDimension::parse(&dim_str).unwrap_or(UnitDimension::Custom);

    Ok(Unit {
        id: row.get("id")?,
        code: row.get("code")?,
        name: row.get("name")?,
        dimension,
        precision: precision_int.clamp(0, 6) as u32,
        is_base: is_base_int != 0,
        created_at: row.get("created_at")?,
    })
}

// =========================================================================
// REPOSITORY CRUD OPERATIONS — UNITS
// =========================================================================

/// Creates a new unit in the catalog.
pub fn create_unit(conn: &Connection, input: CreateUnitInput) -> Result<Unit, UnitError> {
    let clean_code = validate_unit_code(&input.code)?;
    let clean_name = validate_unit_name(&input.name)?;
    let dimension = UnitDimension::parse(&input.dimension)?;
    let precision = validate_unit_precision(input.precision.unwrap_or(3))?;
    let is_base = input.is_base.unwrap_or(false);

    // Check code uniqueness (case-insensitive)
    let existing_id: Option<String> = conn
        .query_row(
            "SELECT id FROM units WHERE code = ?1 COLLATE NOCASE",
            params![clean_code],
            |row| row.get(0),
        )
        .optional()?;

    if existing_id.is_some() {
        return Err(UnitError::DuplicateCode(format!(
            "A unit with code '{clean_code}' already exists"
        )));
    }

    let unit_id = uuid::Uuid::new_v4().to_string();

    conn.execute("BEGIN IMMEDIATE;", [])?;

    let res: Result<Unit, UnitError> = (|| {
        // If this unit is marked as base, demote other units in the same dimension
        if is_base {
            conn.execute(
                "UPDATE units SET is_base = 0 WHERE dimension = ?1",
                params![dimension.as_str()],
            )?;
        }

        conn.execute(
            "INSERT INTO units (id, code, name, dimension, precision, is_base, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
            params![
                unit_id,
                clean_code,
                clean_name,
                dimension.as_str(),
                precision,
                if is_base { 1 } else { 0 }
            ],
        )?;

        let created = get_unit(conn, &unit_id)?
            .ok_or_else(|| UnitError::Database("Failed to load newly created unit".into()))?;

        Ok(created)
    })();

    match res {
        Ok(unit) => {
            conn.execute("COMMIT;", [])?;
            Ok(unit)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            Err(e)
        }
    }
}

/// Retrieves a unit by ID.
pub fn get_unit(conn: &Connection, id: &str) -> Result<Option<Unit>, UnitError> {
    let result = conn
        .query_row(
            "SELECT id, code, name, dimension, precision, is_base, created_at FROM units WHERE id = ?1",
            params![id.trim()],
            map_unit_row,
        )
        .optional()?;

    Ok(result)
}

/// Retrieves a unit by code (case-insensitive).
pub fn get_unit_by_code(conn: &Connection, code: &str) -> Result<Option<Unit>, UnitError> {
    let result = conn
        .query_row(
            "SELECT id, code, name, dimension, precision, is_base, created_at FROM units WHERE code = ?1 COLLATE NOCASE",
            params![code.trim()],
            map_unit_row,
        )
        .optional()?;

    Ok(result)
}

/// Lists all units matching the filter criteria.
pub fn list_units(conn: &Connection, filter: UnitFilter) -> Result<Vec<Unit>, UnitError> {
    let mut sql =
        "SELECT id, code, name, dimension, precision, is_base, created_at FROM units WHERE 1=1"
            .to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(dim_str) = filter.dimension {
        let dim = UnitDimension::parse(&dim_str)?;
        sql.push_str(" AND dimension = ?");
        params_vec.push(Box::new(dim.as_str().to_string()));
    }

    if let Some(is_base) = filter.is_base {
        sql.push_str(" AND is_base = ?");
        params_vec.push(Box::new(if is_base { 1 } else { 0 }));
    }

    if let Some(query) = filter.query {
        let trimmed = query.trim().to_string();
        if !trimmed.is_empty() {
            sql.push_str(" AND (code LIKE ? OR name LIKE ?)");
            let pattern = format!("%{trimmed}%");
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }
    }

    sql.push_str(" ORDER BY dimension ASC, is_base DESC, code ASC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_slice.as_slice(), map_unit_row)?;

    let mut list = Vec::new();
    for row in rows {
        list.push(row?);
    }

    Ok(list)
}

/// Updates an existing unit.
pub fn update_unit(conn: &Connection, input: UpdateUnitInput) -> Result<Unit, UnitError> {
    let unit_id = input.id.trim();
    let clean_code = validate_unit_code(&input.code)?;
    let clean_name = validate_unit_name(&input.name)?;
    let dimension = UnitDimension::parse(&input.dimension)?;
    let precision = validate_unit_precision(input.precision)?;

    // Verify existence
    let existing = get_unit(conn, unit_id)?
        .ok_or_else(|| UnitError::NotFound(format!("Unit with ID '{unit_id}' not found")))?;

    // Check code uniqueness if code changed
    let conflict: Option<String> = conn
        .query_row(
            "SELECT id FROM units WHERE code = ?1 COLLATE NOCASE AND id != ?2",
            params![clean_code, unit_id],
            |row| row.get(0),
        )
        .optional()?;

    if conflict.is_some() {
        return Err(UnitError::DuplicateCode(format!(
            "A unit with code '{clean_code}' already exists"
        )));
    }

    conn.execute("BEGIN IMMEDIATE;", [])?;

    let res: Result<Unit, UnitError> = (|| {
        if input.is_base && !existing.is_base {
            // Demote other units in the same dimension
            conn.execute(
                "UPDATE units SET is_base = 0 WHERE dimension = ?1 AND id != ?2",
                params![dimension.as_str(), unit_id],
            )?;
        }

        conn.execute(
            "UPDATE units SET code = ?1, name = ?2, dimension = ?3, precision = ?4, is_base = ?5 WHERE id = ?6",
            params![
                clean_code,
                clean_name,
                dimension.as_str(),
                precision,
                if input.is_base { 1 } else { 0 },
                unit_id
            ],
        )?;

        let updated = get_unit(conn, unit_id)?
            .ok_or_else(|| UnitError::Database("Failed to load updated unit".into()))?;

        Ok(updated)
    })();

    match res {
        Ok(unit) => {
            conn.execute("COMMIT;", [])?;
            Ok(unit)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            Err(e)
        }
    }
}

/// Deletes a unit and its associated conversion rules.
pub fn delete_unit(conn: &Connection, id: &str) -> Result<(), UnitError> {
    let unit_id = id.trim();

    let _existing = get_unit(conn, unit_id)?
        .ok_or_else(|| UnitError::NotFound(format!("Unit with ID '{unit_id}' not found")))?;

    conn.execute("BEGIN IMMEDIATE;", [])?;

    let res: Result<(), UnitError> = (|| {
        // Remove conversion rules referencing this unit
        conn.execute(
            "DELETE FROM unit_conversions WHERE from_unit_id = ?1 OR to_unit_id = ?1",
            params![unit_id],
        )?;

        // Remove the unit itself
        conn.execute("DELETE FROM units WHERE id = ?1", params![unit_id])?;

        Ok(())
    })();

    match res {
        Ok(()) => {
            conn.execute("COMMIT;", [])?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK;", []);
            Err(e)
        }
    }
}

// =========================================================================
// REPOSITORY CRUD OPERATIONS — UNIT CONVERSIONS
// =========================================================================

/// Creates or updates a unit conversion rule.
pub fn create_unit_conversion(
    conn: &Connection,
    input: CreateUnitConversionInput,
) -> Result<UnitConversion, UnitError> {
    let from_id = input.from_unit_id.trim();
    let to_id = input.to_unit_id.trim();
    let multiplier = validate_multiplier(input.multiplier)?;

    if from_id == to_id {
        return Err(UnitError::Validation(
            "Cannot create conversion rule from a unit to itself; self-conversion is identity 1.0"
                .into(),
        ));
    }

    // Verify both units exist
    let from_unit = get_unit(conn, from_id)?
        .ok_or_else(|| UnitError::NotFound(format!("Source unit with ID '{from_id}' not found")))?;
    let to_unit = get_unit(conn, to_id)?
        .ok_or_else(|| UnitError::NotFound(format!("Target unit with ID '{to_id}' not found")))?;

    // Check dimensions: allow cross-dimension only if both are custom or user explicitly sets a bridge
    // (the repository permits inserting the rule, which then serves as the authorized bridge)
    conn.execute(
        "INSERT INTO unit_conversions (from_unit_id, to_unit_id, multiplier, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(from_unit_id, to_unit_id) DO UPDATE SET
             multiplier = excluded.multiplier,
             created_at = datetime('now')",
        params![from_unit.id, to_unit_unit_id_param(&to_unit), multiplier],
    )?;

    Ok(UnitConversion {
        from_unit_id: from_unit.id,
        to_unit_id: to_unit.id,
        multiplier,
        created_at: chrono_now_iso(),
    })
}

fn to_unit_unit_id_param(u: &Unit) -> &str {
    &u.id
}

fn chrono_now_iso() -> String {
    // SQLite datetime('now') equivalent timestamp placeholder
    "now".to_string()
}

/// Deletes a unit conversion rule.
pub fn delete_unit_conversion(
    conn: &Connection,
    from_unit_id: &str,
    to_unit_id: &str,
) -> Result<(), UnitError> {
    let from_id = from_unit_id.trim();
    let to_id = to_unit_id.trim();

    let affected = conn.execute(
        "DELETE FROM unit_conversions WHERE from_unit_id = ?1 AND to_unit_id = ?2",
        params![from_id, to_id],
    )?;

    if affected == 0 {
        return Err(UnitError::NotFound(format!(
            "No conversion rule found from unit '{from_id}' to '{to_id}'"
        )));
    }

    Ok(())
}

/// Lists unit conversions, optionally filtered by a specific unit ID (as source or target).
pub fn list_unit_conversions(
    conn: &Connection,
    unit_id: Option<&str>,
) -> Result<Vec<UnitConversionView>, UnitError> {
    let mut sql = "SELECT uc.from_unit_id, u1.code as from_code, u1.name as from_name,
                          uc.to_unit_id, u2.code as to_code, u2.name as to_name,
                          uc.multiplier, uc.created_at
                   FROM unit_conversions uc
                   JOIN units u1 ON uc.from_unit_id = u1.id
                   JOIN units u2 ON uc.to_unit_id = u2.id"
        .to_string();

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(uid) = unit_id {
        let trimmed = uid.trim().to_string();
        if !trimmed.is_empty() {
            sql.push_str(" WHERE uc.from_unit_id = ? OR uc.to_unit_id = ?");
            params_vec.push(Box::new(trimmed.clone()));
            params_vec.push(Box::new(trimmed));
        }
    }

    sql.push_str(" ORDER BY u1.code ASC, u2.code ASC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_slice.as_slice(), |row| {
        Ok(UnitConversionView {
            from_unit_id: row.get(0)?,
            from_unit_code: row.get(1)?,
            from_unit_name: row.get(2)?,
            to_unit_id: row.get(3)?,
            to_unit_code: row.get(4)?,
            to_unit_name: row.get(5)?,
            multiplier: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    let mut list = Vec::new();
    for row in rows {
        list.push(row?);
    }

    Ok(list)
}

// =========================================================================
// CONVERSION EVALUATION ENGINE
// =========================================================================

/// Evaluates conversion factor between two units using identity, direct, inverse, and transitive resolution.
/// Max traversal depth: 5 hops. Cycle-protected with visited-node tracking.
pub fn find_conversion_factor(
    conn: &Connection,
    from_unit: &Unit,
    to_unit: &Unit,
) -> Result<f64, UnitError> {
    // 1. Identity: Same unit always returns 1.0
    if from_unit.id == to_unit.id || from_unit.code.eq_ignore_ascii_case(&to_unit.code) {
        return Ok(1.0);
    }

    // 2. Direct lookup: A -> B
    let direct: Option<f64> = conn
        .query_row(
            "SELECT multiplier FROM unit_conversions WHERE from_unit_id = ?1 AND to_unit_id = ?2",
            params![from_unit.id, to_unit.id],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(m) = direct {
        return validate_multiplier(m);
    }

    // 3. Inverse lookup: B -> A (if A -> B not defined, inverse is 1 / M_inv)
    let inverse: Option<f64> = conn
        .query_row(
            "SELECT multiplier FROM unit_conversions WHERE from_unit_id = ?1 AND to_unit_id = ?2",
            params![to_unit.id, from_unit.id],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(m_inv) = inverse {
        let valid_inv = validate_multiplier(m_inv)?;
        return Ok(1.0 / valid_inv);
    }

    // 4. Transitive BFS Graph Search
    // Build adjacency graph of all conversion rules
    let mut stmt =
        conn.prepare("SELECT from_unit_id, to_unit_id, multiplier FROM unit_conversions")?;
    let rows = stmt.query_map([], |row| {
        let f: String = row.get(0)?;
        let t: String = row.get(1)?;
        let m: f64 = row.get(2)?;
        Ok((f, t, m))
    })?;

    let mut adj: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for row in rows {
        let (f, t, m) = row?;
        if m > 0.0 && m.is_finite() {
            // Direct edge
            adj.entry(f.clone()).or_default().push((t.clone(), m));
            // Inverse edge (if reverse not already explicitly present, add inverse)
            adj.entry(t).or_default().push((f, 1.0 / m));
        }
    }

    // BFS queue: (current_unit_id, accumulated_multiplier, hop_count)
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    queue.push_back((from_unit.id.clone(), 1.0, 0));
    visited.insert(from_unit.id.clone());

    const MAX_HOPS: usize = 5;

    while let Some((curr_id, curr_multiplier, hops)) = queue.pop_front() {
        if curr_id == to_unit.id {
            return validate_multiplier(curr_multiplier);
        }

        if hops >= MAX_HOPS {
            continue;
        }

        if let Some(neighbors) = adj.get(&curr_id) {
            for (next_id, edge_multiplier) in neighbors {
                if !visited.contains(next_id) {
                    visited.insert(next_id.clone());
                    let next_mult = curr_multiplier * edge_multiplier;
                    if next_mult.is_finite() && !next_mult.is_nan() && next_mult > 0.0 {
                        queue.push_back((next_id.clone(), next_mult, hops + 1));
                    }
                }
            }
        }
    }

    // If dimensions differ and no explicit bridge was found
    if from_unit.dimension != to_unit.dimension {
        return Err(UnitError::IncompatibleDimensions {
            from_dimension: from_unit.dimension.to_string(),
            to_dimension: to_unit.dimension.to_string(),
        });
    }

    Err(UnitError::ConversionPathNotFound {
        from_unit: from_unit.code.clone(),
        to_unit: to_unit.code.clone(),
    })
}

/// Converts a quantity from one unit to another, rounding to the target unit's precision.
pub fn convert_quantity(
    conn: &Connection,
    input: ConvertQuantityInput,
) -> Result<ConversionResult, UnitError> {
    if !input.quantity.is_finite() || input.quantity.is_nan() {
        return Err(UnitError::Validation(format!(
            "Quantity must be a finite number, got {}",
            input.quantity
        )));
    }

    let from_id = input.from_unit_id.trim();
    let to_id = input.to_unit_id.trim();

    let from_unit = get_unit(conn, from_id)?
        .ok_or_else(|| UnitError::NotFound(format!("Source unit with ID '{from_id}' not found")))?;
    let to_unit = get_unit(conn, to_id)?
        .ok_or_else(|| UnitError::NotFound(format!("Target unit with ID '{to_id}' not found")))?;

    let effective_multiplier = find_conversion_factor(conn, &from_unit, &to_unit)?;
    let raw_converted = input.quantity * effective_multiplier;

    if !raw_converted.is_finite() || raw_converted.is_nan() {
        return Err(UnitError::Validation(
            "Calculated converted quantity resulted in overflow or non-finite number".into(),
        ));
    }

    // Round according to target unit precision
    let precision_factor = 10f64.powi(to_unit.precision as i32);
    let converted_quantity = (raw_converted * precision_factor).round() / precision_factor;

    Ok(ConversionResult {
        from_unit_id: from_unit.id,
        from_unit_code: from_unit.code,
        to_unit_id: to_unit.id,
        to_unit_code: to_unit.code,
        original_quantity: input.quantity,
        converted_quantity,
        effective_multiplier,
    })
}
