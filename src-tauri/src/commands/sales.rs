use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SaleItem {
    pub product_id: String,
    pub variant_id: Option<String>,
    /// Quantity in thousandths. 2500 = 2.5 units.
    pub quantity_milli: i64,
    pub unit_price_minor: i64,
}

impl SaleItem {
    fn quantity(&self) -> f64 { self.quantity_milli as f64 / 1000.0 }

    fn line_total_minor(&self) -> Result<i64, String> {
        if self.quantity_milli <= 0 || self.unit_price_minor < 0 {
            return Err("invalid sale quantity or price".into());
        }
        let product = i128::from(self.quantity_milli)
            .checked_mul(i128::from(self.unit_price_minor))
            .ok_or_else(|| "line total overflow".to_string())?;
        i64::try_from((product + 500) / 1000)
            .map_err(|_| "line total exceeds i64 range".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSaleRequest {
    pub branch_id: String,
    pub shift_id: String,
    pub user_id: String,
    pub idempotency_key: String,
    pub items: Vec<SaleItem>,
    pub payment_method: String,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesReportSummary { pub sales_count: i64, pub total_minor: i64 }

#[tauri::command]
pub fn create_sale(state: tauri::State<crate::db::DbState>, request: CreateSaleRequest) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| format!("database lock failed: {e}"))?;
    execute_create_sale(&conn, &request)
}

pub fn execute_create_sale(conn: &Connection, request: &CreateSaleRequest) -> Result<String, String> {
    validate_request(request)?;
    let tx = conn.unchecked_transaction().map_err(|e| format!("failed to start transaction: {e}"))?;

    if let Some(result_json) = tx.query_row(
        "SELECT result_json FROM idempotency_keys WHERE key=?1 AND operation='create_sale'",
        [request.idempotency_key.as_str()], |row| row.get::<_, String>(0)
    ).optional().map_err(|e| format!("failed to check idempotency key: {e}"))? {
        let sale_id = serde_json::from_str::<serde_json::Value>(&result_json).ok()
            .and_then(|v| v.get("sale_id").and_then(|v| v.as_str()).map(str::to_owned))
            .ok_or_else(|| "stored idempotency result is invalid".to_string())?;
        tx.rollback().map_err(|e| format!("failed to finish idempotent read: {e}"))?;
        return Ok(sale_id);
    }

    let shift_ok: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM shifts WHERE id=?1 AND branch_id=?2 AND user_id=?3 AND status='open')",
        params![request.shift_id, request.branch_id, request.user_id], |row| row.get(0)
    ).map_err(|e| format!("failed to validate shift: {e}"))?;
    if !shift_ok { return Err("shift is not open or does not belong to this user/branch".into()); }

    let sale_id = uuid::Uuid::new_v4().to_string();
    let subtotal_minor = request.items.iter().map(SaleItem::line_total_minor).try_fold(0_i64, |acc, value| {
        acc.checked_add(value?).ok_or_else(|| "sale total overflow".to_string())
    })?;

    tx.execute(
        "INSERT INTO sales (id,branch_id,shift_id,user_id,currency,subtotal,discount_amount,tax_amount,total,subtotal_minor,discount_amount_minor,tax_amount_minor,total_minor,status)
         VALUES (?1,?2,?3,?4,?5,0,0,0,0,?6,0,0,?6,'completed')",
        params![sale_id, request.branch_id, request.shift_id, request.user_id, request.currency, subtotal_minor]
    ).map_err(|e| format!("failed to create sale: {e}"))?;

    for item in &request.items {
        let line_total_minor = item.line_total_minor()?;
        tx.execute(
            "INSERT INTO sale_items (id,sale_id,product_id,variant_id,quantity,unit_price,line_total,unit_price_minor,line_total_minor)
             VALUES (?1,?2,?3,?4,?5,0,0,?6,?7)",
            params![uuid::Uuid::new_v4().to_string(), sale_id, item.product_id, item.variant_id, item.quantity(), item.unit_price_minor, line_total_minor]
        ).map_err(|e| format!("failed to create sale item: {e}"))?;

        let updated = tx.execute(
            "UPDATE inventory SET quantity=quantity-?1,updated_at=datetime('now')
             WHERE branch_id=?2 AND product_id=?3
               AND ((variant_id=?4) OR (variant_id IS NULL AND ?4 IS NULL))
               AND quantity>=?1",
            params![item.quantity(), request.branch_id, item.product_id, item.variant_id]
        ).map_err(|e| format!("failed to update inventory: {e}"))?;
        if updated != 1 { return Err(format!("insufficient stock or inventory row missing for product {}", item.product_id)); }

        tx.execute(
            "INSERT INTO stock_movements (id,branch_id,product_id,variant_id,quantity_delta,quantity_before,quantity_after,reason,source_type,source_id,user_id)
             SELECT ?1,branch_id,product_id,variant_id,-?2,quantity+?2,quantity,'sale','sale',?3,?4
             FROM inventory WHERE branch_id=?5 AND product_id=?6
               AND ((variant_id=?7) OR (variant_id IS NULL AND ?7 IS NULL))",
            params![uuid::Uuid::new_v4().to_string(), item.quantity(), sale_id, request.user_id, request.branch_id, item.product_id, item.variant_id]
        ).map_err(|e| format!("failed to create stock movement: {e}"))?;
    }

    tx.execute(
        "INSERT INTO sale_payments (id,sale_id,payment_method,amount,amount_minor) VALUES (?1,?2,?3,0,?4)",
        params![uuid::Uuid::new_v4().to_string(), sale_id, request.payment_method, subtotal_minor]
    ).map_err(|e| format!("failed to create payment: {e}"))?;

    tx.execute(
        "INSERT INTO idempotency_keys (key,operation,result_json) VALUES (?1,'create_sale',?2)",
        params![request.idempotency_key, format!(r#"{{"sale_id":"{}"}}"#, sale_id)]
    ).map_err(|e| format!("failed to record idempotency key: {e}"))?;

    tx.execute(
        "INSERT INTO outbox_events (event_id,aggregate_type,aggregate_id,event_type,schema_version,branch_id,payload_json)
         VALUES (?1,'sale',?2,'sale.completed',1,?3,?4)",
        params![uuid::Uuid::new_v4().to_string(), sale_id, request.branch_id, format!(r#"{{"sale_id":"{}"}}"#, sale_id)]
    ).map_err(|e| format!("failed to enqueue outbox event: {e}"))?;

    tx.commit().map_err(|e| format!("failed to commit sale: {e}"))?;
    Ok(sale_id)
}

#[tauri::command]
pub fn get_sales_report(state: tauri::State<crate::db::DbState>) -> Result<SalesReportSummary, String> {
    let conn = state.0.lock().map_err(|e| format!("database lock failed: {e}"))?;
    let sales_count = conn.query_row("SELECT COUNT(*) FROM sales", [], |row| row.get(0)).map_err(|e| format!("failed to read sales report: {e}"))?;
    let total_minor = conn.query_row("SELECT COALESCE(SUM(total_minor),0) FROM sales", [], |row| row.get(0)).map_err(|e| format!("failed to read sales total: {e}"))?;
    Ok(SalesReportSummary { sales_count, total_minor })
}

fn validate_request(request: &CreateSaleRequest) -> Result<(), String> {
    if request.branch_id.trim().is_empty() || request.shift_id.trim().is_empty() || request.user_id.trim().is_empty()
        || request.idempotency_key.trim().is_empty() || request.currency.trim().is_empty() || request.payment_method.trim().is_empty() {
        return Err("required sale fields are missing".into());
    }
    if request.items.is_empty() { return Err("at least one sale item is required".into()); }
    for item in &request.items {
        if item.product_id.trim().is_empty() { return Err("product_id is required".into()); }
        if item.quantity_milli <= 0 { return Err("quantity must be positive".into()); }
        if item.unit_price_minor < 0 { return Err("unit_price_minor must be non-negative".into()); }
        item.line_total_minor()?;
    }
    Ok(())
}
