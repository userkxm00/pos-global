// Tauri command boundary for Barcode and SKU operations.
// F2.03 — SKU / Barcode

use crate::barcode::{
    detect_symbology, generate_internal_ean13, generate_next_sku, validate_barcode_symbology,
    AddBarcodeRequest, BarcodeIntegrityMismatch, BarcodeSymbology, ProductBarcode,
};
use crate::db::DbState;
use crate::product::Product;

use super::{
    authorize_catalog_mutation as authorize_barcode_mutation,
    authorize_catalog_read as authorize_barcode_read,
};

#[tauri::command]
pub fn get_product_by_barcode(
    state: tauri::State<DbState>,
    session_id: String,
    barcode: String,
) -> Result<(Product, Option<ProductBarcode>), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_read(&conn, &session_id)?;
    match crate::barcode::get_product_by_barcode(&conn, &barcode).map_err(|e| e.to_string())? {
        Some(res) => Ok(res),
        None => Err(format!("Product with barcode '{barcode}' not found")),
    }
}

#[tauri::command]
pub fn add_product_barcode(
    state: tauri::State<DbState>,
    session_id: String,
    request: AddBarcodeRequest,
) -> Result<ProductBarcode, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_mutation(&conn, &session_id)?;
    crate::barcode::add_product_barcode(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_product_barcode(
    state: tauri::State<DbState>,
    session_id: String,
    barcode_id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_mutation(&conn, &session_id)?;
    crate::barcode::remove_product_barcode(&conn, &barcode_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_primary_barcode(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
    barcode_id: String,
) -> Result<ProductBarcode, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_mutation(&conn, &session_id)?;
    crate::barcode::set_primary_barcode(&conn, &product_id, &barcode_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reassign_product_barcode(
    state: tauri::State<DbState>,
    session_id: String,
    barcode_id: String,
    target_product_id: String,
    as_primary: Option<bool>,
) -> Result<ProductBarcode, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_mutation(&conn, &session_id)?;
    crate::barcode::reassign_product_barcode(
        &conn,
        &barcode_id,
        &target_product_id,
        as_primary.unwrap_or(false),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_product_barcodes(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
    include_inactive: Option<bool>,
) -> Result<Vec<ProductBarcode>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_read(&conn, &session_id)?;
    crate::barcode::list_product_barcodes(&conn, &product_id, include_inactive.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_barcode_string(
    state: tauri::State<DbState>,
    session_id: String,
    barcode: String,
    symbology: Option<String>,
) -> Result<String, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_read(&conn, &session_id)?;

    let sym = match symbology.as_deref() {
        Some(s) => BarcodeSymbology::parse(s).unwrap_or(BarcodeSymbology::Unknown),
        None => detect_symbology(&barcode),
    };

    validate_barcode_symbology(&barcode, sym).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_internal_barcode(
    state: tauri::State<DbState>,
    session_id: String,
    prefix: Option<String>,
) -> Result<String, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_mutation(&conn, &session_id)?;
    generate_internal_ean13(&conn, prefix.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_product_sku(
    state: tauri::State<DbState>,
    session_id: String,
    prefix: Option<String>,
) -> Result<String, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_mutation(&conn, &session_id)?;
    generate_next_sku(&conn, prefix.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn verify_catalog_barcode_integrity(
    state: tauri::State<DbState>,
    session_id: String,
) -> Result<Vec<BarcodeIntegrityMismatch>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_read(&conn, &session_id)?;
    crate::barcode::verify_catalog_barcode_integrity(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reconcile_catalog_barcode_mirrors(
    state: tauri::State<DbState>,
    session_id: String,
) -> Result<usize, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_barcode_mutation(&conn, &session_id)?;
    crate::barcode::reconcile_catalog_barcode_mirrors(&conn).map_err(|e| e.to_string())
}
