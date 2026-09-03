// Tauri IPC command handlers for Weighted Products domain operations.
// F2.06 — Weighted Products (ADR-0008)

use crate::db::DbState;
use crate::weighted::{ProductWeightConfig, UpsertWeightConfigInput, WeightedCalculationResult};
use rusqlite::Connection;

use super::{
    authorize_catalog_mutation as authorize_weighted_mutation,
    authorize_catalog_read as authorize_weighted_read,
};

// =========================================================================
// INTERNAL COMMAND LOGIC (DIRECTLY TESTABLE WITHOUT TAURI RUNTIME)
// =========================================================================

pub fn set_product_weight_config_impl(
    conn: &Connection,
    session_id: &str,
    request: &UpsertWeightConfigInput,
) -> Result<ProductWeightConfig, String> {
    authorize_weighted_mutation(conn, session_id)?;
    crate::weighted::upsert_product_weight_config(conn, request).map_err(|e| e.to_string())
}

pub fn get_product_weight_config_impl(
    conn: &Connection,
    session_id: &str,
    product_id: &str,
) -> Result<Option<ProductWeightConfig>, String> {
    authorize_weighted_read(conn, session_id)?;
    crate::weighted::get_product_weight_config(conn, product_id).map_err(|e| e.to_string())
}

pub fn delete_product_weight_config_impl(
    conn: &Connection,
    session_id: &str,
    product_id: &str,
) -> Result<(), String> {
    authorize_weighted_mutation(conn, session_id)?;
    crate::weighted::delete_product_weight_config(conn, product_id).map_err(|e| e.to_string())
}

pub fn calculate_weighted_item_impl(
    conn: &Connection,
    session_id: &str,
    product_id: &str,
    gross_weight_milli: i64,
    custom_tare_milli: Option<i64>,
    source_unit: Option<&str>,
) -> Result<WeightedCalculationResult, String> {
    authorize_weighted_read(conn, session_id)?;
    crate::weighted::calculate_weighted_item(
        conn,
        product_id,
        gross_weight_milli,
        custom_tare_milli,
        source_unit,
    )
    .map_err(|e| e.to_string())
}

// =========================================================================
// TAURI IPC COMMANDS
// =========================================================================

#[tauri::command]
pub fn set_product_weight_config(
    state: tauri::State<DbState>,
    session_id: String,
    request: UpsertWeightConfigInput,
) -> Result<ProductWeightConfig, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    set_product_weight_config_impl(&conn, &session_id, &request)
}

#[tauri::command]
pub fn get_product_weight_config(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
) -> Result<Option<ProductWeightConfig>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    get_product_weight_config_impl(&conn, &session_id, &product_id)
}

#[tauri::command]
pub fn delete_product_weight_config(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    delete_product_weight_config_impl(&conn, &session_id, &product_id)
}

#[tauri::command]
pub fn calculate_weighted_item(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
    gross_weight_milli: i64,
    custom_tare_milli: Option<i64>,
    source_unit: Option<String>,
) -> Result<WeightedCalculationResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;
    calculate_weighted_item_impl(
        &conn,
        &session_id,
        &product_id,
        gross_weight_milli,
        custom_tare_milli,
        source_unit.as_deref(),
    )
}
