use crate::commands::sales::{execute_create_sale, CreateSaleRequest, SaleItem};
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    crate::db::init_database(&conn).expect("apply all migrations");
    conn.execute("INSERT INTO organizations (id, name) VALUES ('org-1', 'Test Org')", [])
        .unwrap();
    conn.execute(
        "INSERT INTO business_settings (id, business_name, default_currency, organization_id)
         VALUES ('biz-1', 'Test Business', 'DZD', 'org-1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO branches (id, name, currency, organization_id) VALUES ('branch-1', 'Main', 'DZD', 'org-1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO users (id, branch_id, full_name, username, role) VALUES ('user-1', 'branch-1', 'Cashier', 'cashier', 'cashier')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO products (id, name, barcode) VALUES ('product-1', 'Test Product', '123')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO inventory (id, branch_id, product_id, variant_id, quantity, low_stock_threshold)
         VALUES ('inventory-1', 'branch-1', 'product-1', NULL, 10, 2)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO shifts (id, branch_id, user_id, status, opening_balance, opening_balance_minor)
         VALUES ('shift-1', 'branch-1', 'user-1', 'open', 0, 0)",
        [],
    )
    .unwrap();
    conn
}

fn request(quantity: f64) -> CreateSaleRequest {
    CreateSaleRequest {
        branch_id: "branch-1".into(),
        shift_id: "shift-1".into(),
        user_id: "user-1".into(),
        items: vec![SaleItem {
            product_id: "product-1".into(),
            variant_id: None,
            quantity,
            unit_price: 1.25,
            unit_price_minor: 125,
        }],
        payment_method: "cash".into(),
        currency: "DZD".into(),
    }
}

#[test]
fn rejects_empty_shift_id() {
    let conn = setup_db();
    let mut req = request(1.0);
    req.shift_id.clear();
    let result = execute_create_sale(&conn, &req);
    assert!(result.is_err());
}

#[test]
fn rejects_closed_shift() {
    let conn = setup_db();
    conn.execute("UPDATE shifts SET status = 'closed' WHERE id = 'shift-1'", [])
        .unwrap();
    assert!(execute_create_sale(&conn, &request(1.0)).is_err());
}

#[test]
fn rejects_wrong_shift_owner() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO users (id, branch_id, full_name, username, role) VALUES ('user-2', 'branch-1', 'Other', 'other', 'cashier')",
        [],
    )
    .unwrap();
    let mut req = request(1.0);
    req.user_id = "user-2".into();
    assert!(execute_create_sale(&conn, &req).is_err());
}

#[test]
fn rejects_overselling() {
    let conn = setup_db();
    assert!(execute_create_sale(&conn, &request(11.0)).is_err());
    let qty: f64 = conn
        .query_row("SELECT quantity FROM inventory WHERE id = 'inventory-1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(qty, 10.0);
    let sales: i64 = conn.query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0)).unwrap();
    assert_eq!(sales, 0);
}

#[test]
fn successful_sale_decrements_stock() {
    let conn = setup_db();
    let sale_id = execute_create_sale(&conn, &request(2.0)).expect("sale should succeed");
    assert!(!sale_id.is_empty());
    let qty: f64 = conn
        .query_row("SELECT quantity FROM inventory WHERE id = 'inventory-1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(qty, 8.0);
}

#[test]
fn successful_sale_records_stock_movement() {
    let conn = setup_db();
    let sale_id = execute_create_sale(&conn, &request(2.0)).unwrap();
    let (delta, before, after): (f64, f64, f64) = conn
        .query_row(
            "SELECT quantity_delta, quantity_before, quantity_after FROM stock_movements WHERE source_id = ?1",
            [&sale_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(delta, -2.0);
    assert_eq!(before, 10.0);
    assert_eq!(after, 8.0);
}

#[test]
fn successful_sale_records_payment_idempotency_and_outbox() {
    let conn = setup_db();
    let sale_id = execute_create_sale(&conn, &request(1.0)).unwrap();
    let payment_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sale_payments WHERE sale_id = ?1", [&sale_id], |r| r.get(0))
        .unwrap();
    let idem_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM idempotency_keys WHERE key = ?1", [&sale_id], |r| r.get(0))
        .unwrap();
    let outbox_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM outbox_events WHERE aggregate_id = ?1", [&sale_id], |r| r.get(0))
        .unwrap();
    assert_eq!(payment_count, 1);
    assert_eq!(idem_count, 1);
    assert_eq!(outbox_count, 1);
}

#[test]
fn successful_sale_uses_minor_money_columns() {
    let conn = setup_db();
    let sale_id = execute_create_sale(&conn, &request(2.0)).unwrap();
    let total_minor: i64 = conn
        .query_row("SELECT total_minor FROM sales WHERE id = ?1", [&sale_id], |r| r.get(0))
        .unwrap();
    let line_minor: i64 = conn
        .query_row("SELECT line_total_minor FROM sale_items WHERE sale_id = ?1", [&sale_id], |r| r.get(0))
        .unwrap();
    let payment_minor: i64 = conn
        .query_row("SELECT amount_minor FROM sale_payments WHERE sale_id = ?1", [&sale_id], |r| r.get(0))
        .unwrap();
    assert_eq!(total_minor, 250);
    assert_eq!(line_minor, 250);
    assert_eq!(payment_minor, 250);
}
