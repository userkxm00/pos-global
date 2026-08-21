// Sales command boundary recovered from the implementation snapshot.
// This is intentionally preserved as a foundation implementation; future
// production work must reconcile it with current domain contracts before
// expanding taxes, discounts, costing, debt, loyalty, or sync semantics.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SaleItem {
    pub product_id: String,
    pub variant_id: Option<String>,
    pub quantity: f64,
    /// Transitional input only. Financial truth uses unit_price_minor.
    pub unit_price: f64,
    pub unit_price_minor: i64,
}

impl SaleItem {
    fn quantity_minor_total(&self) -> i64 {
        (self.quantity * self.unit_price_minor as f64).round() as i64
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSaleRequest {
    pub branch_id: String,
    pub shift_id: String,
    pub user_id: String,
    pub items: Vec<SaleItem>,
    pub payment_method: String,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesReportSummary {
    pub sales_count: i64,
    pub total_minor: i64,
}

#[tauri::command]
pub fn create_sale(
    state: tauri::State<crate::db::DbState>,
    request: CreateSaleRequest,
) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| format!("database lock failed: {e}"))?;
    execute_create_sale(&conn, &request)
}

/// Foundation transaction implementation recovered from the earlier tested
/// snapshot. It verifies shift ownership/open state, uses exact minor-unit
/// monetary columns, prevents overselling, records stock movements, writes an
/// idempotency record, and emits an outbox event in one transaction.
pub fn execute_create_sale(
    conn: &Connection,
    request: &CreateSaleRequest,
) -> Result<String, String> {
    validate_request(request)?;

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("failed to start transaction: {e}"))?;

    let shift_ok: bool = tx
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM shifts
                WHERE id = ?1
                  AND branch_id = ?2
                  AND user_id = ?3
                  AND status = 'open'
            )",
            params![request.shift_id, request.branch_id, request.user_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("failed to validate shift: {e}"))?;

    if !shift_ok {
        return Err("shift is not open or does not belong to this user/branch".into());
    }

    let sale_id = uuid::Uuid::new_v4().to_string();
    let subtotal_minor: i64 = request
        .items
        .iter()
        .map(SaleItem::quantity_minor_total)
        .try_fold(0_i64, |acc, value| acc.checked_add(value))
        .ok_or_else(|| "sale total overflow".to_string())?;

    tx.execute(
        "INSERT INTO sales
            (id, branch_id, shift_id, user_id, currency, subtotal, discount_amount,
             tax_amount, total, subtotal_minor, discount_amount_minor, tax_amount_minor,
             total_minor, status)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 0, ?6, 0, 0, ?6, 'completed')",
        params![
            sale_id,
            request.branch_id,
            request.shift_id,
            request.user_id,
            request.currency,
            subtotal_minor
        ],
    )
    .map_err(|e| format!("failed to create sale: {e}"))?;

    for item in &request.items {
        let line_total_minor = item.quantity_minor_total();
        tx.execute(
            "INSERT INTO sale_items
                (id, sale_id, product_id, variant_id, quantity, unit_price, line_total,
                 unit_price_minor, line_total_minor)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, ?7)",
            params![
                uuid::Uuid::new_v4().to_string(),
                sale_id,
                item.product_id,
                item.variant_id,
                item.quantity,
                item.unit_price_minor,
                line_total_minor
            ],
        )
        .map_err(|e| format!("failed to create sale item: {e}"))?;

        let updated = tx
            .execute(
                "UPDATE inventory
                 SET quantity = quantity - ?1,
                     updated_at = datetime('now')
                 WHERE branch_id = ?2
                   AND product_id = ?3
                   AND ((variant_id = ?4) OR (variant_id IS NULL AND ?4 IS NULL))
                   AND quantity >= ?1",
                params![item.quantity, request.branch_id, item.product_id, item.variant_id],
            )
            .map_err(|e| format!("failed to update inventory: {e}"))?;

        if updated != 1 {
            return Err(format!(
                "insufficient stock or inventory row missing for product {}",
                item.product_id
            ));
        }

        tx.execute(
            "INSERT INTO stock_movements
                (id, branch_id, product_id, variant_id, quantity_delta,
                 quantity_before, quantity_after, reason, source_type, source_id, user_id)
             SELECT
                ?1, branch_id, product_id, variant_id, -?2,
                quantity + ?2, quantity, 'sale', 'sale', ?3, ?4
             FROM inventory
             WHERE branch_id = ?5
               AND product_id = ?6
               AND ((variant_id = ?7) OR (variant_id IS NULL AND ?7 IS NULL))",
            params![
                uuid::Uuid::new_v4().to_string(),
                item.quantity,
                sale_id,
                request.user_id,
                request.branch_id,
                item.product_id,
                item.variant_id
            ],
        )
        .map_err(|e| format!("failed to create stock movement: {e}"))?;
    }

    tx.execute(
        "INSERT INTO sale_payments (id, sale_id, payment_method, amount, amount_minor)
         VALUES (?1, ?2, ?3, 0, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            sale_id,
            request.payment_method,
            subtotal_minor
        ],
    )
    .map_err(|e| format!("failed to create payment: {e}"))?;

    tx.execute(
        "INSERT INTO idempotency_keys (key, operation, result_json)
         VALUES (?1, 'create_sale', ?2)",
        params![sale_id, format!(r#"{{"sale_id":"{}"}}"#, sale_id)],
    )
    .map_err(|e| format!("failed to record idempotency key: {e}"))?;

    tx.execute(
        "INSERT INTO outbox_events
            (event_id, aggregate_type, aggregate_id, event_type, schema_version,
             branch_id, payload_json)
         VALUES (?1, 'sale', ?2, 'sale.completed', 1, ?3, ?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            sale_id,
            request.branch_id,
            format!(r#"{{"sale_id":"{}"}}"#, sale_id)
        ],
    )
    .map_err(|e| format!("failed to enqueue outbox event: {e}"))?;

    tx.commit().map_err(|e| format!("failed to commit sale: {e}"))?;
    Ok(sale_id)
}

#[tauri::command]
pub fn get_sales_report(
    state: tauri::State<crate::db::DbState>,
) -> Result<SalesReportSummary, String> {
    let conn = state.0.lock().map_err(|e| format!("database lock failed: {e}"))?;
    let sales_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sales", [], |row| row.get(0))
        .map_err(|e| format!("failed to read sales report: {e}"))?;
    let total_minor: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total_minor), 0) FROM sales",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("failed to read sales total: {e}"))?;
    Ok(SalesReportSummary { sales_count, total_minor })
}

fn validate_request(request: &CreateSaleRequest) -> Result<(), String> {
    if request.branch_id.trim().is_empty() { return Err("branch_id is required".into()); }
    if request.shift_id.trim().is_empty() { return Err("shift_id is required".into()); }
    if request.user_id.trim().is_empty() { return Err("user_id is required".into()); }
    if request.currency.trim().is_empty() { return Err("currency is required".into()); }
    if request.items.is_empty() { return Err("at least one sale item is required".into()); }
    if request.payment_method.trim().is_empty() { return Err("payment_method is required".into()); }
    for item in &request.items {
        if item.product_id.trim().is_empty() { return Err("product_id is required".into()); }
        if !item.quantity.is_finite() || item.quantity <= 0.0 { return Err("quantity must be positive and finite".into()); }
        if item.unit_price_minor < 0 { return Err("unit_price_minor must be non-negative".into()); }
    }
    Ok(())
}
