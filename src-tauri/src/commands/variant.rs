// Tauri command boundary for Product Variants and Matrix operations.
// F2.05 — Variants / Matrix

use crate::db::DbState;
use crate::variant::{
    AttributeDefinition, AttributeValue, BulkOperationResult, BulkUpdateVariantPricesInput,
    BulkUpdateVariantStatusInput, CreateAttributeDefinitionInput, CreateAttributeValueInput,
    CreateVariantInput, GenerateMatrixInput, MatrixGenerationResult, MatrixPreviewResult,
    PreviewMatrixInput, ProductVariant, UpdateVariantInput, VariantWithAttributes,
};

use super::{authorize_catalog_mutation, authorize_catalog_read};

#[tauri::command]
pub fn create_attribute_definition(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateAttributeDefinitionInput,
) -> Result<AttributeDefinition, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_catalog_mutation(&conn, &session_id)?;
    crate::variant::create_attribute_definition(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_attribute_definition(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<AttributeDefinition, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_catalog_read(&conn, &session_id)?;
    match crate::variant::get_attribute_definition(&conn, &id).map_err(|e| e.to_string())? {
        Some(def) => Ok(def),
        None => Err(format!("Attribute definition with ID '{id}' not found")),
    }
}

#[tauri::command]
pub fn list_attribute_definitions(
    state: tauri::State<DbState>,
    session_id: String,
) -> Result<Vec<AttributeDefinition>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_catalog_read(&conn, &session_id)?;
    crate::variant::list_attribute_definitions(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_attribute_value(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateAttributeValueInput,
) -> Result<AttributeValue, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Verify parent attribute definition exists before mutation
    crate::variant::get_attribute_definition(&conn, &request.attribute_definition_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Attribute definition with ID '{}' not found",
                request.attribute_definition_id
            )
        })?;

    authorize_catalog_mutation(&conn, &session_id)?;
    crate::variant::create_attribute_value(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_attribute_value(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<AttributeValue, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_catalog_read(&conn, &session_id)?;
    match crate::variant::get_attribute_value(&conn, &id).map_err(|e| e.to_string())? {
        Some(val) => Ok(val),
        None => Err(format!("Attribute value with ID '{id}' not found")),
    }
}

#[tauri::command]
pub fn list_attribute_values_by_definition(
    state: tauri::State<DbState>,
    session_id: String,
    attribute_definition_id: String,
) -> Result<Vec<AttributeValue>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_catalog_read(&conn, &session_id)?;
    crate::variant::list_attribute_values_by_definition(&conn, &attribute_definition_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_variant(
    state: tauri::State<DbState>,
    session_id: String,
    request: CreateVariantInput,
) -> Result<VariantWithAttributes, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Verify target parent product exists before performing mutation
    crate::product::get_product(&conn, &request.product_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Parent product with ID '{}' not found", request.product_id))?;

    authorize_catalog_mutation(&conn, &session_id)?;
    crate::variant::create_variant(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_variant(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<VariantWithAttributes, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_catalog_read(&conn, &session_id)?;
    let variant = match crate::variant::get_variant(&conn, &id).map_err(|e| e.to_string())? {
        Some(v) => v,
        None => return Err(format!("Variant with ID '{id}' not found")),
    };
    let attribute_values = crate::variant::get_variant_attribute_values(&conn, &variant.id)
        .map_err(|e| e.to_string())?;

    Ok(VariantWithAttributes {
        variant,
        attribute_values,
    })
}

#[tauri::command]
pub fn list_variants_by_product(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: String,
    active_only: Option<bool>,
) -> Result<Vec<VariantWithAttributes>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Verify target parent product exists before reading variants
    crate::product::get_product(&conn, &product_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Product with ID '{product_id}' not found"))?;

    authorize_catalog_read(&conn, &session_id)?;
    let variants = crate::variant::list_variants_by_product(&conn, &product_id, active_only)
        .map_err(|e| e.to_string())?;

    let mut results = Vec::with_capacity(variants.len());
    for v in variants {
        let attribute_values = crate::variant::get_variant_attribute_values(&conn, &v.id)
            .map_err(|e| e.to_string())?;
        results.push(VariantWithAttributes {
            variant: v,
            attribute_values,
        });
    }

    Ok(results)
}

#[tauri::command]
pub fn update_variant(
    state: tauri::State<DbState>,
    session_id: String,
    request: UpdateVariantInput,
) -> Result<ProductVariant, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Resolve target variant exists before performing mutation
    crate::variant::get_variant(&conn, &request.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Variant with ID '{}' not found", request.id))?;

    authorize_catalog_mutation(&conn, &session_id)?;
    crate::variant::update_variant(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_variant(
    state: tauri::State<DbState>,
    session_id: String,
    id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Resolve target variant exists before performing mutation
    crate::variant::get_variant(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Variant with ID '{id}' not found"))?;

    authorize_catalog_mutation(&conn, &session_id)?;
    crate::variant::soft_delete_variant(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn preview_variant_matrix(
    state: tauri::State<DbState>,
    session_id: String,
    request: PreviewMatrixInput,
) -> Result<MatrixPreviewResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Resolve target parent product exists before preview
    crate::product::get_product(&conn, &request.product_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Product with ID '{}' not found", request.product_id))?;

    authorize_catalog_read(&conn, &session_id)?;
    crate::variant::preview_variant_matrix(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn generate_variant_matrix(
    state: tauri::State<DbState>,
    session_id: String,
    request: GenerateMatrixInput,
) -> Result<MatrixGenerationResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Resolve target parent product exists before matrix generation
    crate::product::get_product(&conn, &request.product_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Product with ID '{}' not found", request.product_id))?;

    authorize_catalog_mutation(&conn, &session_id)?;
    crate::variant::generate_variant_matrix(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn bulk_update_variant_status(
    state: tauri::State<DbState>,
    session_id: String,
    request: BulkUpdateVariantStatusInput,
) -> Result<BulkOperationResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Resolve all target variants exist before bulk status update
    for id in &request.variant_ids {
        crate::variant::get_variant(&conn, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Variant with ID '{id}' not found"))?;
    }

    authorize_catalog_mutation(&conn, &session_id)?;
    crate::variant::bulk_update_variant_status(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn bulk_update_variant_prices(
    state: tauri::State<DbState>,
    session_id: String,
    request: BulkUpdateVariantPricesInput,
) -> Result<BulkOperationResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    // Resolve all target variants exist before bulk price update
    for id in &request.variant_ids {
        crate::variant::get_variant(&conn, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Variant with ID '{id}' not found"))?;
    }

    authorize_catalog_mutation(&conn, &session_id)?;
    crate::variant::bulk_update_variant_prices(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_variant_by_barcode(
    state: tauri::State<DbState>,
    session_id: String,
    barcode: String,
) -> Result<Option<VariantWithAttributes>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_catalog_read(&conn, &session_id)?;
    crate::variant::get_variant_by_barcode(&conn, &barcode).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_variant_by_sku(
    state: tauri::State<DbState>,
    session_id: String,
    sku: String,
) -> Result<Option<VariantWithAttributes>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_catalog_read(&conn, &session_id)?;
    crate::variant::get_variant_by_sku(&conn, &sku).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_variants(
    state: tauri::State<DbState>,
    session_id: String,
    product_id: Option<String>,
    query: String,
) -> Result<Vec<VariantWithAttributes>, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    if let Some(ref pid) = product_id {
        crate::product::get_product(&conn, pid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Product with ID '{pid}' not found"))?;
    }

    authorize_catalog_read(&conn, &session_id)?;
    crate::variant::search_variants(&conn, product_id.as_deref(), &query).map_err(|e| e.to_string())
}
