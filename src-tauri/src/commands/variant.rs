// Tauri command boundary for Product Variants and Matrix operations.
// F2.05 — Variants / Matrix

use crate::db::DbState;
use crate::variant::{
    bulk_update_variant_prices as domain_bulk_update_variant_prices,
    bulk_update_variant_status as domain_bulk_update_variant_status,
    create_attribute_definition as domain_create_attribute_definition,
    create_attribute_value as domain_create_attribute_value,
    create_variant as domain_create_variant,
    generate_variant_matrix as domain_generate_variant_matrix,
    get_attribute_definition as domain_get_attribute_definition, get_variant as domain_get_variant,
    get_variant_attribute_values as domain_get_variant_attribute_values,
    get_variant_by_barcode as domain_get_variant_by_barcode,
    get_variant_by_sku as domain_get_variant_by_sku,
    list_attribute_definitions as domain_list_attribute_definitions,
    list_attribute_values_by_definition as domain_list_attribute_values_by_definition,
    list_variants_by_product as domain_list_variants_by_product,
    preview_variant_matrix as domain_preview_variant_matrix,
    search_variants as domain_search_variants, soft_delete_variant as domain_soft_delete_variant,
    update_variant as domain_update_variant, AttributeDefinition, AttributeValue,
    BulkOperationResult, BulkUpdateVariantPricesInput, BulkUpdateVariantStatusInput,
    CreateAttributeDefinitionInput, CreateAttributeValueInput, CreateVariantInput,
    GenerateMatrixInput, MatrixGenerationResult, MatrixPreviewResult, PreviewMatrixInput,
    ProductVariant, UpdateVariantInput, VariantWithAttributes,
};

use super::{
    authorize_catalog_mutation as authorize_variant_mutation,
    authorize_catalog_read as authorize_variant_read,
};

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

    authorize_variant_mutation(&conn, &session_id)?;
    domain_create_attribute_definition(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_attribute_definition(
    state: tauri::State<DbState>,
    session_id: String,
    attribute_definition_id: String,
) -> Result<AttributeDefinition, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_variant_read(&conn, &session_id)?;
    match domain_get_attribute_definition(&conn, &attribute_definition_id)
        .map_err(|e| e.to_string())?
    {
        Some(def) => Ok(def),
        None => Err(format!(
            "Attribute definition with ID '{attribute_definition_id}' not found"
        )),
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

    authorize_variant_read(&conn, &session_id)?;
    domain_list_attribute_definitions(&conn).map_err(|e| e.to_string())
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

    authorize_variant_mutation(&conn, &session_id)?;
    domain_create_attribute_value(&conn, request).map_err(|e| e.to_string())
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

    authorize_variant_read(&conn, &session_id)?;
    domain_list_attribute_values_by_definition(&conn, &attribute_definition_id)
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

    authorize_variant_mutation(&conn, &session_id)?;
    domain_create_variant(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_variant(
    state: tauri::State<DbState>,
    session_id: String,
    variant_id: String,
) -> Result<VariantWithAttributes, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_variant_read(&conn, &session_id)?;
    let variant = domain_get_variant(&conn, &variant_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Variant with ID '{variant_id}' not found"))?;

    let attribute_values =
        domain_get_variant_attribute_values(&conn, &variant_id).map_err(|e| e.to_string())?;

    Ok(VariantWithAttributes {
        variant,
        attribute_values,
    })
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

    authorize_variant_mutation(&conn, &session_id)?;
    domain_update_variant(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_variant(
    state: tauri::State<DbState>,
    session_id: String,
    variant_id: String,
) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("database lock failed: {e}"))?;

    authorize_variant_mutation(&conn, &session_id)?;
    domain_soft_delete_variant(&conn, &variant_id).map_err(|e| e.to_string())
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

    authorize_variant_read(&conn, &session_id)?;
    let variants = domain_list_variants_by_product(&conn, &product_id, active_only)
        .map_err(|e| e.to_string())?;

    let mut results = Vec::with_capacity(variants.len());
    for v in variants {
        let attribute_values =
            domain_get_variant_attribute_values(&conn, &v.id).map_err(|e| e.to_string())?;
        results.push(VariantWithAttributes {
            variant: v,
            attribute_values,
        });
    }

    Ok(results)
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

    authorize_variant_read(&conn, &session_id)?;
    domain_preview_variant_matrix(&conn, request).map_err(|e| e.to_string())
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

    authorize_variant_mutation(&conn, &session_id)?;
    domain_generate_variant_matrix(&conn, request).map_err(|e| e.to_string())
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

    authorize_variant_mutation(&conn, &session_id)?;
    domain_bulk_update_variant_status(&conn, request).map_err(|e| e.to_string())
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

    authorize_variant_mutation(&conn, &session_id)?;
    domain_bulk_update_variant_prices(&conn, request).map_err(|e| e.to_string())
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

    authorize_variant_read(&conn, &session_id)?;
    domain_get_variant_by_barcode(&conn, &barcode).map_err(|e| e.to_string())
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

    authorize_variant_read(&conn, &session_id)?;
    domain_get_variant_by_sku(&conn, &sku).map_err(|e| e.to_string())
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

    authorize_variant_read(&conn, &session_id)?;
    domain_search_variants(&conn, product_id.as_deref(), &query).map_err(|e| e.to_string())
}
