use crate::commands::sales::{
    execute_create_sale, execute_sales_report, CreateSaleRequest, SaleItem,
};
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    crate::db::init_database(&conn).expect("apply all migrations");

    conn.execute(
        "INSERT INTO organizations (id, name) VALUES ('org-1', 'Test Org')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO business_settings (
            id, business_name, default_currency, organization_id
         ) VALUES ('biz-1', 'Test Business', 'DZD', 'org-1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO branches (id, name, currency, organization_id)
         VALUES ('branch-1', 'Main', 'DZD', 'org-1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO users (id, branch_id, full_name, username, role)
         VALUES ('user-1', 'branch-1', 'Cashier', 'cashier', 'cashier')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO products (id, name, barcode)
         VALUES ('product-1', 'Test Product', '123')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO inventory (
            id, branch_id, product_id, variant_id, quantity, quantity_milli, low_stock_threshold
         ) VALUES ('inventory-1', 'branch-1', 'product-1', NULL, 10, 10000, 2)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO shifts (
            id, branch_id, user_id, status, opening_balance, opening_balance_minor
         ) VALUES ('shift-1', 'branch-1', 'user-1', 'open', 0, 0)",
        [],
    )
    .unwrap();

    conn
}

fn request(quantity_milli: i64, key: &str) -> CreateSaleRequest {
    CreateSaleRequest {
        branch_id: "branch-1".into(),
        shift_id: "shift-1".into(),
        user_id: "user-1".into(),
        idempotency_key: key.into(),
        items: vec![SaleItem {
            product_id: "product-1".into(),
            variant_id: None,
            quantity_milli,
            unit_price_minor: 125,
        }],
        payment_method: "cash".into(),
        currency: "DZD".into(),
    }
}

#[test]
fn rejects_empty_shift_id() {
    let conn = setup_db();
    let mut req = request(1000, "idem-1");
    req.shift_id.clear();

    assert!(execute_create_sale(&conn, &req).is_err());
}

#[test]
fn rejects_closed_shift() {
    let conn = setup_db();
    conn.execute(
        "UPDATE shifts SET status = 'closed' WHERE id = 'shift-1'",
        [],
    )
    .unwrap();

    assert!(execute_create_sale(&conn, &request(1000, "idem-2")).is_err());
}

#[test]
fn rejects_wrong_shift_owner() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO users (id, branch_id, full_name, username, role)
         VALUES ('user-2', 'branch-1', 'Other', 'other', 'cashier')",
        [],
    )
    .unwrap();

    let mut req = request(1000, "idem-3");
    req.user_id = "user-2".into();

    assert!(execute_create_sale(&conn, &req).is_err());
}

#[test]
fn rejects_overselling_and_rolls_back() {
    let conn = setup_db();

    assert!(execute_create_sale(&conn, &request(11000, "idem-4")).is_err());

    let qty_milli: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE id = 'inventory-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let sales: i64 = conn
        .query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0))
        .unwrap();

    assert_eq!(qty_milli, 10000);
    assert_eq!(sales, 0);
}

#[test]
fn successful_sale_decrements_stock_using_integer_units() {
    let conn = setup_db();
    let sale_id =
        execute_create_sale(&conn, &request(2000, "idem-5")).expect("sale should succeed");

    assert!(!sale_id.is_empty());

    let qty_milli: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE id = 'inventory-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(qty_milli, 8000);
}

#[test]
fn successful_sale_records_integer_stock_movement() {
    let conn = setup_db();
    let sale_id = execute_create_sale(&conn, &request(2000, "idem-6")).unwrap();

    let (delta_milli, before_milli, after_milli): (i64, i64, i64) = conn
        .query_row(
            "SELECT quantity_delta_milli, quantity_before_milli, quantity_after_milli
             FROM stock_movements
             WHERE source_id = ?1",
            [&sale_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();

    assert_eq!(delta_milli, -2000);
    assert_eq!(before_milli, 10000);
    assert_eq!(after_milli, 8000);
}

#[test]
fn retry_with_same_idempotency_key_returns_same_sale_without_duplicate_side_effects() {
    let conn = setup_db();
    let first = execute_create_sale(&conn, &request(1000, "idem-retry")).unwrap();
    let second = execute_create_sale(&conn, &request(1000, "idem-retry")).unwrap();

    assert_eq!(first, second);

    let sales: i64 = conn
        .query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0))
        .unwrap();
    let movements: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0))
        .unwrap();
    let outbox: i64 = conn
        .query_row("SELECT COUNT(*) FROM outbox_events", [], |r| r.get(0))
        .unwrap();
    let qty_milli: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE id = 'inventory-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(sales, 1);
    assert_eq!(movements, 1);
    assert_eq!(outbox, 1);
    assert_eq!(qty_milli, 9000);
}

#[test]
fn successful_sale_records_payment_idempotency_and_outbox() {
    let conn = setup_db();
    let sale_id = execute_create_sale(&conn, &request(1000, "idem-7")).unwrap();

    let payment: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sale_payments WHERE sale_id = ?1",
            [&sale_id],
            |r| r.get(0),
        )
        .unwrap();
    let idem: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM idempotency_keys WHERE key = 'idem-7'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let outbox: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM outbox_events WHERE aggregate_id = ?1",
            [&sale_id],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(payment, 1);
    assert_eq!(idem, 1);
    assert_eq!(outbox, 1);
}

#[test]
fn successful_sale_uses_minor_money_columns() {
    let conn = setup_db();
    let sale_id = execute_create_sale(&conn, &request(2000, "idem-8")).unwrap();

    let total: i64 = conn
        .query_row(
            "SELECT total_minor FROM sales WHERE id = ?1",
            [&sale_id],
            |r| r.get(0),
        )
        .unwrap();
    let line: i64 = conn
        .query_row(
            "SELECT line_total_minor FROM sale_items WHERE sale_id = ?1",
            [&sale_id],
            |r| r.get(0),
        )
        .unwrap();
    let payment: i64 = conn
        .query_row(
            "SELECT amount_minor FROM sale_payments WHERE sale_id = ?1",
            [&sale_id],
            |r| r.get(0),
        )
        .unwrap();
    let quantity_milli: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM sale_items WHERE sale_id = ?1",
            [&sale_id],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(total, 250);
    assert_eq!(line, 250);
    assert_eq!(payment, 250);
    assert_eq!(quantity_milli, 2000);
}

#[test]
fn sales_report_is_branch_scoped() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO branches (id, name, currency, organization_id)
         VALUES ('branch-2', 'Other', 'DZD', 'org-1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO users (id, branch_id, full_name, username, role)
         VALUES ('user-2', 'branch-2', 'Other Cashier', 'other-cashier', 'cashier')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO shifts (
            id, branch_id, user_id, status, opening_balance, opening_balance_minor
         ) VALUES ('shift-2', 'branch-2', 'user-2', 'open', 0, 0)",
        [],
    )
    .unwrap();

    execute_create_sale(&conn, &request(1000, "report-branch-1")).unwrap();

    conn.execute(
        "INSERT INTO sales (
            id, branch_id, shift_id, user_id, currency,
            subtotal, discount_amount, tax_amount, total,
            subtotal_minor, discount_amount_minor, tax_amount_minor, total_minor, status
         ) VALUES (
            'sale-branch-2', 'branch-2', 'shift-2', 'user-2', 'DZD',
            0, 0, 0, 0, 999, 0, 0, 999, 'completed'
         )",
        [],
    )
    .unwrap();

    let branch_one = execute_sales_report(&conn, "branch-1").unwrap();
    let branch_two = execute_sales_report(&conn, "branch-2").unwrap();

    assert_eq!(branch_one.sales_count, 1);
    assert_eq!(branch_one.total_minor, 125);
    assert_eq!(branch_two.sales_count, 1);
    assert_eq!(branch_two.total_minor, 999);
}
