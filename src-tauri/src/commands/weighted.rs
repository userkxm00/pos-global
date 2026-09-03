// Tauri IPC command handlers for Weighted Products domain operations.
// F2.06 — Weighted Products (ADR-0008)

use crate::db::DbState;
use crate::weighted::{ProductWeightConfig, UpsertWeightConfigInput, WeightedCalculationResult};

use super::{
    authorize_catalog_mutation as authorize_weighted_mutation,
    authorize_catalog_read as authorize_weighted_read,
};

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

    authorize_weighted_mutation(&conn, &session_id)?;
    crate::weighted::upsert_product_weight_config(&conn, &request).map_err(|e| e.to_string())
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

    authorize_weighted_read(&conn, &session_id)?;
    crate::weighted::get_product_weight_config(&conn, &product_id).map_err(|e| e.to_string())
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

    authorize_weighted_mutation(&conn, &session_id)?;
    crate::weighted::delete_product_weight_config(&conn, &product_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn calculate_weighted_item(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
    gross_weight_milli: i64,
    custom_tare_milli: Option<i64>,
) -> Result<WeightedCalculationResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_weighted_read(&conn, &session_id)?;
    crate::weighted::calculate_weighted_item(
        &conn,
        &product_id,
        gross_weight_milli,
        custom_tare_milli,
    )
    .map_err(|e| e.to_string())
}
