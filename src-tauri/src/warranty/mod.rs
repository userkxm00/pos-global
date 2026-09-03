// F2.09 — Warranty Core Domain Engine
// Implements ADR-0011: Lightweight Core, Canonical Date Contract, and Coverage Evaluation.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// =========================================================================
// ERROR TYPES
// =========================================================================

#[derive(Debug, thiserror::Error)]
pub enum WarrantyError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Date parse error: {0}")]
    DateParse(String),

    #[error("Database error: {0}")]
    Database(String),
}

impl From<rusqlite::Error> for WarrantyError {
    fn from(err: rusqlite::Error) -> Self {
        WarrantyError::Database(err.to_string())
    }
}

// =========================================================================
// DOMAIN MODELS & DTOs
// =========================================================================

/// Authoritative status of warranty coverage evaluated against a reference date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WarrantyCoverageStatus {
    /// Coverage is active on the reference date.
    Active {
        expiry_date: String,
        days_remaining: i64,
    },
    /// Coverage has expired prior to the reference date.
    Expired {
        expiry_date: String,
        days_elapsed: i64,
    },
    /// Product has warranty capability/duration, but no expiration date is registered on the instance.
    NotRegistered,
    /// Product has no warranty capability and no registered expiration date.
    NotCovered,
}

/// Input payload for registering or updating warranty on a serialized instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterWarrantyInput {
    pub serial_number_id: String,
    pub start_date: Option<String>,
    pub duration_months: Option<u32>,
    pub warranty_expires_at: Option<String>,
}

/// Output record for instance warranty status and coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceWarrantyRecord {
    pub serial_number_id: String,
    pub product_id: String,
    pub branch_id: String,
    pub warranty_expires_at: Option<String>,
    pub coverage: WarrantyCoverageStatus,
}

// =========================================================================
// CALENDAR & DATE ARITHMETIC (CANONICAL YYYY-MM-DD)
// =========================================================================

/// Checks whether a year is a leap year under the Gregorian calendar rules.
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Returns the number of days in a given calendar month for a specified year.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Parses and validates a canonical `YYYY-MM-DD` date string.
/// Enforces range constraints: year 1970..=9999, month 1..=12, day 1..=days_in_month.
pub fn parse_canonical_date(s: &str) -> Result<(i32, u32, u32), WarrantyError> {
    let trimmed = s.trim();
    if trimmed.len() != 10 {
        return Err(WarrantyError::DateParse(format!(
            "Invalid date length '{}'; expected exact 'YYYY-MM-DD' format",
            trimmed
        )));
    }

    let bytes = trimmed.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(WarrantyError::DateParse(format!(
            "Invalid date separator in '{}'; expected 'YYYY-MM-DD'",
            trimmed
        )));
    }

    let year: i32 = trimmed[0..4]
        .parse()
        .map_err(|_| WarrantyError::DateParse(format!("Invalid year in '{}'", trimmed)))?;
    let month: u32 = trimmed[5..7]
        .parse()
        .map_err(|_| WarrantyError::DateParse(format!("Invalid month in '{}'", trimmed)))?;
    let day: u32 = trimmed[8..10]
        .parse()
        .map_err(|_| WarrantyError::DateParse(format!("Invalid day in '{}'", trimmed)))?;

    if !(1970..=9999).contains(&year) {
        return Err(WarrantyError::DateParse(format!(
            "Year {} out of valid range (1970-9999)",
            year
        )));
    }

    if !(1..=12).contains(&month) {
        return Err(WarrantyError::DateParse(format!(
            "Month {} out of valid range (1-12)",
            month
        )));
    }

    let max_days = days_in_month(year, month);
    if day < 1 || day > max_days {
        return Err(WarrantyError::DateParse(format!(
            "Day {} out of range for year {} month {} (max {})",
            day, year, month, max_days
        )));
    }

    Ok((year, month, day))
}

/// Normalizes an incoming date or timestamp string to canonical `YYYY-MM-DD`.
/// If caller supplies an ISO 8601 timestamp (e.g. `2026-09-03T12:00:00Z` or `2026-09-03 14:00:00`),
/// the calendar date prefix is extracted and validated.
pub fn normalize_to_canonical_date(input: &str) -> Result<String, WarrantyError> {
    let trimmed = input.trim();
    if trimmed.len() < 10 {
        return Err(WarrantyError::DateParse(format!(
            "Input date '{}' is too short; expected at least 10 characters 'YYYY-MM-DD'",
            trimmed
        )));
    }

    let date_part = &trimmed[0..10];
    let (year, month, day) = parse_canonical_date(date_part)?;

    // If longer than 10 characters, ensure the delimiter is valid for an ISO timestamp
    if trimmed.len() > 10 {
        let delimiter = trimmed.as_bytes()[10];
        if delimiter != b'T' && delimiter != b' ' {
            return Err(WarrantyError::DateParse(format!(
                "Malformed timestamp delimiter in '{}'; expected 'T' or space after YYYY-MM-DD",
                trimmed
            )));
        }
    }

    Ok(format!("{:04}-{:02}-{:02}", year, month, day))
}

/// Converts a valid Gregorian calendar date to days relative to 1970-01-01 (Civil Days Algorithm).
/// Pure integer arithmetic; zero floating-point operations.
fn civil_to_days(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    let (y_adj, m_adj) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let era = y_adj.div_euclid(400);
    let yoe = y_adj.rem_euclid(400);
    let doy = (153 * (m_adj - 3) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Calculates the exact signed number of days between two canonical date strings (`date_b - date_a`).
pub fn days_between(date_a: &str, date_b: &str) -> Result<i64, WarrantyError> {
    let (y1, m1, d1) = parse_canonical_date(date_a)?;
    let (y2, m2, d2) = parse_canonical_date(date_b)?;

    let days1 = civil_to_days(y1, m1, d1);
    let days2 = civil_to_days(y2, m2, d2);

    Ok(days2 - days1)
}

/// Computes warranty expiration date from a canonical start date and duration in months.
///
/// Implements exact month-end clamping:
/// - `2026-01-31` + 1 month = `2026-02-28`
/// - `2028-01-31` + 1 month = `2028-02-29`
/// - `2026-05-31` + 1 month = `2026-06-30`
pub fn calculate_warranty_expiration(
    start_date: &str,
    duration_months: u32,
) -> Result<String, WarrantyError> {
    if duration_months < 1 {
        return Err(WarrantyError::Validation(
            "Warranty duration must be at least 1 month".to_string(),
        ));
    }

    let (start_y, start_m, start_d) = parse_canonical_date(start_date)?;

    let zero_based_m = (start_m - 1) + duration_months;
    let target_y = start_y + (zero_based_m / 12) as i32;
    let target_m = (zero_based_m % 12) + 1;

    let max_days = days_in_month(target_y, target_m);
    let target_d = start_d.min(max_days);

    Ok(format!("{:04}-{:02}-{:02}", target_y, target_m, target_d))
}

// =========================================================================
// COVERAGE EVALUATION
// =========================================================================

/// Evaluates warranty coverage status given an instance expiration date and reference date.
///
/// Semantics (ADR-0011):
/// - `days_remaining = max(0, expiry_date - as_of_date)`
/// - `days_elapsed   = max(0, as_of_date - expiry_date)`
/// - `Active`: `as_of_date <= expiry_date`. On the expiration day, `days_remaining = 0` and status is Active.
/// - `Expired`: `as_of_date > expiry_date`. `days_elapsed > 0`.
/// - `NotRegistered`: warranty capability or product duration exists, but `expiry_date` is None.
/// - `NotCovered`: no warranty capability and no registered expiration date.
pub fn evaluate_warranty_coverage(
    expiry_date: Option<&str>,
    as_of_date: Option<&str>,
    is_tracked: bool,
) -> Result<WarrantyCoverageStatus, WarrantyError> {
    let expiry = match expiry_date {
        Some(s) if !s.trim().is_empty() => normalize_to_canonical_date(s)?,
        _ => {
            return Ok(if is_tracked {
                WarrantyCoverageStatus::NotRegistered
            } else {
                WarrantyCoverageStatus::NotCovered
            });
        }
    };

    let canonical_as_of = match as_of_date {
        Some(s) if !s.trim().is_empty() => normalize_to_canonical_date(s)?,
        _ => current_utc_date(),
    };

    let diff = days_between(&canonical_as_of, &expiry)?;

    if diff >= 0 {
        // as_of_date <= expiry_date
        Ok(WarrantyCoverageStatus::Active {
            expiry_date: expiry,
            days_remaining: diff,
        })
    } else {
        // as_of_date > expiry_date
        Ok(WarrantyCoverageStatus::Expired {
            expiry_date: expiry,
            days_elapsed: -diff,
        })
    }
}

/// Returns the current date in UTC as a canonical `YYYY-MM-DD` string.
pub fn current_utc_date() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0));
    let secs = duration.as_secs();

    // Convert epoch seconds to days since 1970-01-01
    let days = (secs / 86400) as i64;

    // Convert days since epoch back to Gregorian (y, m, d)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}", y, m, d)
}

// =========================================================================
// CAPABILITY & PRODUCT ELIGIBILITY
// =========================================================================

/// Validates product warranty duration input.
/// Rejects negative values (`warranty_months < 0`).
pub fn validate_warranty_months(months: Option<i32>) -> Result<(), WarrantyError> {
    if let Some(m) = months {
        if m < 0 {
            return Err(WarrantyError::Validation(
                "Product warranty_months cannot be negative".to_string(),
            ));
        }
    }
    Ok(())
}

/// Checks whether a product has active warranty tracking.
///
/// Canonical Rule (ADR-0011):
/// `is_warranty_tracked(P) <=> (products.warranty_months > 0) OR has_capability(P, 'WARRANTY')`
pub fn is_warranty_tracked(conn: &Connection, product_id: &str) -> Result<bool, WarrantyError> {
    let mut stmt = conn.prepare_cached(
        "SELECT p.warranty_months,
                EXISTS(
                    SELECT 1 FROM product_capabilities pc
                    JOIN capabilities c ON pc.capability_id = c.id
                    WHERE pc.product_id = p.id
                      AND c.code = 'WARRANTY'
                      AND pc.enabled = 1
                ) as has_cap
         FROM products p
         WHERE p.id = ?1",
    )?;

    let row = stmt.query_row(params![product_id], |row| {
        let warranty_months: Option<i32> = row.get(0)?;
        let has_cap: i64 = row.get(1)?;
        let months_positive = warranty_months.map(|m| m > 0).unwrap_or(false);
        Ok(months_positive || (has_cap != 0))
    });

    match row {
        Ok(tracked) => Ok(tracked),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(WarrantyError::Validation(format!(
            "Product '{product_id}' not found"
        ))),
        Err(e) => Err(WarrantyError::Database(e.to_string())),
    }
}

// =========================================================================
// INSTANCE REGISTRATION & RETRIEVAL
// =========================================================================

/// Registers or updates the warranty expiration date on a serialized instance.
///
/// Semantics:
/// - If `warranty_expires_at` is directly specified, validates and normalizes it to `YYYY-MM-DD`.
/// - If `duration_months` is provided, calculates expiration from `start_date + duration_months`.
/// - If `duration_months` is absent, loads default `products.warranty_months`. If missing or <= 0, errors.
/// - Start date defaults to current UTC date if omitted.
/// - Directly updates `serial_numbers.warranty_expires_at`.
pub fn register_instance_warranty(
    conn: &Connection,
    input: &RegisterWarrantyInput,
) -> Result<InstanceWarrantyRecord, WarrantyError> {
    // 1. Fetch serial instance
    let mut stmt = conn.prepare_cached(
        "SELECT s.product_id, s.branch_id, s.warranty_expires_at
         FROM serial_numbers s
         WHERE s.id = ?1",
    )?;

    let (product_id, branch_id, existing_expires_at): (String, String, Option<String>) = stmt
        .query_row(params![input.serial_number_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => WarrantyError::Validation(format!(
                "Serial instance '{}' not found",
                input.serial_number_id
            )),
            other => WarrantyError::Database(other.to_string()),
        })?;

    // 2. Verify warranty capability on parent product
    let is_tracked = is_warranty_tracked(conn, &product_id)?;
    if !is_tracked {
        return Err(WarrantyError::Validation(format!(
            "Product '{}' is not eligible for warranty tracking",
            product_id
        )));
    }

    // 3. Resolve expiration date
    let target_expiry = if let Some(ref direct_expiry) = input.warranty_expires_at {
        normalize_to_canonical_date(direct_expiry)?
    } else {
        let start = match input.start_date.as_deref() {
            Some(s) if !s.trim().is_empty() => normalize_to_canonical_date(s)?,
            _ => current_utc_date(),
        };

        let duration = if let Some(d) = input.duration_months {
            d
        } else {
            // Load default from product
            let mut prod_stmt =
                conn.prepare_cached("SELECT warranty_months FROM products WHERE id = ?1")?;
            let prod_months: Option<i32> = prod_stmt
                .query_row(params![product_id], |row| row.get(0))
                .map_err(|e| WarrantyError::Database(e.to_string()))?;

            match prod_months {
                Some(m) if m > 0 => m as u32,
                _ => {
                    return Err(WarrantyError::Validation(format!(
                        "Product '{}' has no default warranty duration; explicit duration_months or warranty_expires_at must be provided",
                        product_id
                    )));
                }
            }
        };

        calculate_warranty_expiration(&start, duration)?
    };

    // 4. Update serial_numbers.warranty_expires_at
    conn.execute(
        "UPDATE serial_numbers
         SET warranty_expires_at = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![target_expiry, input.serial_number_id],
    )?;

    // 5. Evaluate coverage on the registered date
    let coverage = evaluate_warranty_coverage(Some(&target_expiry), None, true)?;

    let _ = existing_expires_at;

    Ok(InstanceWarrantyRecord {
        serial_number_id: input.serial_number_id.clone(),
        product_id,
        branch_id,
        warranty_expires_at: Some(target_expiry),
        coverage,
    })
}

/// Retrieves the warranty state and evaluated coverage for a serialized instance.
pub fn get_instance_warranty(
    conn: &Connection,
    serial_number_id: &str,
    as_of_date: Option<&str>,
) -> Result<InstanceWarrantyRecord, WarrantyError> {
    let mut stmt = conn.prepare_cached(
        "SELECT s.product_id, s.branch_id, s.warranty_expires_at
         FROM serial_numbers s
         WHERE s.id = ?1",
    )?;

    let (product_id, branch_id, expires_at): (String, String, Option<String>) = stmt
        .query_row(params![serial_number_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => WarrantyError::Validation(format!(
                "Serial instance '{}' not found",
                serial_number_id
            )),
            other => WarrantyError::Database(other.to_string()),
        })?;

    let is_tracked = is_warranty_tracked(conn, &product_id)?;
    let coverage = evaluate_warranty_coverage(expires_at.as_deref(), as_of_date, is_tracked)?;

    Ok(InstanceWarrantyRecord {
        serial_number_id: serial_number_id.to_string(),
        product_id,
        branch_id,
        warranty_expires_at: expires_at,
        coverage,
    })
}
