// Inventory command boundary recovered from the earlier implementation snapshot.
// Business logic must remain in service/repository layers; SQL does not belong here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductSummary {
    pub id: String,
    pub name: String,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductSearchRequest {
    pub branch_id: String,
    pub query: Option<String>,
    pub category_id: Option<String>,
    pub limit: Option<u32>,
}

#[tauri::command]
pub fn list_products(_request: ProductSearchRequest) -> Result<Vec<ProductSummary>, String> {
    Err("Product repository/service is not implemented yet".into())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertProductRequest {
    pub id: Option<String>,
    pub name: String,
    pub category_id: Option<String>,
    pub barcode: Option<String>,
    pub sku: Option<String>,
    pub description: Option<String>,
}

#[tauri::command]
pub fn upsert_product(_request: UpsertProductRequest) -> Result<String, String> {
    Err("Product service is not implemented yet".into())
}
