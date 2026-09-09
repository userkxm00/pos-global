// F2.11 — Stock Ledger & Spatial Balances IPC Command Layer Tests
// Verifies security boundary, session authentication, InventoryAdjust authorization,
// branch scope tenancy, DTO conversion, error mapping, and idempotency.

use crate::commands::stock_ledger::{
    get_stock_balance_impl, get_stock_movement_impl, list_location_inventory_impl,
    list_stock_movements_impl, post_stock_movement_impl, LocationInventoryFilterRequest,
    PostMovementRequest, StockMovementFilterRequest,
};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::create_local_session;
use rusqlite::Connection;

// =========================================================================
// TEST FIXTURES & HELPERS
// =========================================================================

#[allow(dead_code)]
struct CommandTestFixtures {
    org_id: String,
    branch_a: String,
    branch_b: String,
    product_id: String,
    variant_id: String,
    location_id: String,
    bin_id: String,
    admin_session_a: String,
    manager_session_a: String,
    cashier_session_a: String,
    admin_session_b: String,
}

fn create_auth_session(conn: &Connection, branch_id: &str, role: &str) -> String {
    let username = format!("user_{}_{}", role, &branch_id[..6]);
    let user = create_test_user_with_creds(
        conn,
        branch_id,
        &format!("Test {}", role),
        Some(&username),
        Some("Password123!"),
        Some("1234"),
        role,
    )
    .expect("user created");

    let session =
        create_local_session(conn, &user.id, branch_id, "password", None).expect("session created");
    session.id
}

fn setup_command_fixtures(conn: &Connection) -> CommandTestFixtures {
    let (org_id, branch_a) = create_test_org_and_branch(conn);

    let branch_b_obj = crate::branch::create_branch(
        conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Branch B".to_string(),
            address: Some("456 Other St".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch b created");
    let branch_b = branch_b_obj.id;

    let product_id = "prod_cmd_01";
    let variant_id = "var_cmd_01";

    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active)
         VALUES (?1, 'Command Test Product', 30.0, 1)",
        [product_id],
    )
    .expect("product created");

    conn.execute(
        "INSERT INTO product_variants (id, product_id, sku, is_active)
         VALUES (?1, ?2, 'SKU-CMD-01', 1)",
        [variant_id, product_id],
    )
    .expect("variant created");

    let location_id = "loc_cmd_01";
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Main Warehouse', 'LOC-CMD-01', 'warehouse', 1)",
        [location_id, &branch_a],
    )
    .expect("location created");

    let bin_id = "bin_cmd_01";
    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES (?1, ?2, 'Aisle 1 Slot 1', 'BIN-CMD-01', 1)",
        [bin_id, location_id],
    )
    .expect("bin created");

    let admin_session_a = create_auth_session(conn, &branch_a, "admin");
    let manager_session_a = create_auth_session(conn, &branch_a, "manager");
    let cashier_session_a = create_auth_session(conn, &branch_a, "cashier");
    let admin_session_b = create_auth_session(conn, &branch_b, "admin");

    CommandTestFixtures {
        org_id,
        branch_a,
        branch_b,
        product_id: product_id.to_string(),
        variant_id: variant_id.to_string(),
        location_id: location_id.to_string(),
        bin_id: bin_id.to_string(),
        admin_session_a,
        manager_session_a,
        cashier_session_a,
        admin_session_b,
    }
}

// =========================================================================
// GATE 3 FOCUSED COMMAND LAYER TESTS
// =========================================================================

#[test]
fn test_command_authorized_write_succeeds() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    let req = PostMovementRequest {
        branch_id: f.branch_a.clone(),
        product_id: f.product_id.clone(),
        variant_id: Some(f.variant_id.clone()),
        location_id: f.location_id.clone(),
        bin_id: Some(f.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10000,
        reason: "opening_balance".to_string(),
        source_type: Some("initial_setup".into()),
        source_id: None,
        idempotency_key: None,
    };

    // Manager has Permission::InventoryAdjust -> succeeds
    let movement = post_stock_movement_impl(&mut conn, &f.manager_session_a, req)
        .expect("authorized post movement succeeds");

    assert_eq!(movement.quantity_delta_milli, 10000);
    assert_eq!(movement.quantity_before_milli, Some(0));
    assert_eq!(movement.quantity_after_milli, Some(10000));
}

#[test]
fn test_command_unauthorized_write_fails_with_inventory_adjust_error() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    let req = PostMovementRequest {
        branch_id: f.branch_a.clone(),
        product_id: f.product_id.clone(),
        variant_id: None,
        location_id: f.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: "opening_balance".to_string(),
        source_type: None,
        source_id: None,
        idempotency_key: None,
    };

    // Cashier lacks Permission::InventoryAdjust -> rejected with Permission denied
    let err = post_stock_movement_impl(&mut conn, &f.cashier_session_a, req).unwrap_err();

    assert!(
        err.contains("Permission denied") || err.contains("inventory.adjust"),
        "Error must clearly indicate missing InventoryAdjust permission, got: {err}"
    );

    // Assert zero stock mutations occurred
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_command_unauthenticated_write_fails() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    let req = PostMovementRequest {
        branch_id: f.branch_a.clone(),
        product_id: f.product_id.clone(),
        variant_id: None,
        location_id: f.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: "opening_balance".to_string(),
        source_type: None,
        source_id: None,
        idempotency_key: None,
    };

    let err = post_stock_movement_impl(&mut conn, "invalid-session-token", req).unwrap_err();

    assert!(
        err.to_lowercase().contains("session") || err.contains("Unauthorized"),
        "Unauthenticated session must fail closed, got: {err}"
    );
}

#[test]
fn test_command_branch_mismatch_write_fails() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    // Admin of Branch B attempts to mutate stock for Branch A
    let req = PostMovementRequest {
        branch_id: f.branch_a.clone(), // Target is Branch A
        product_id: f.product_id.clone(),
        variant_id: None,
        location_id: f.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: "opening_balance".to_string(),
        source_type: None,
        source_id: None,
        idempotency_key: None,
    };

    let err = post_stock_movement_impl(&mut conn, &f.admin_session_b, req).unwrap_err();

    assert!(
        err.contains("Scope mismatch"),
        "Cross-branch write must be blocked by tenancy boundary with Scope mismatch, got: {err}"
    );
    assert!(
        err.contains(&f.branch_a) && err.contains(&f.branch_b),
        "Scope mismatch error must detail expected and actual branches: {err}"
    );

    // Verify no stock mutated in Branch A
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_command_authorized_read_succeeds() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    // Seed stock via authorized write
    let m = post_stock_movement_impl(
        &mut conn,
        &f.admin_session_a,
        PostMovementRequest {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 8000,
            reason: "opening_balance".to_string(),
            source_type: None,
            source_id: None,
            idempotency_key: None,
        },
    )
    .expect("seed movement");

    // 1. get_stock_balance_impl
    let bal = get_stock_balance_impl(
        &conn,
        &f.admin_session_a,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
    )
    .expect("get stock balance succeeds");
    assert_eq!(bal.aggregate_quantity_milli, 8000);
    assert_eq!(bal.allocated_spatial_milli, 8000);
    assert_eq!(bal.unallocated_milli, 0);

    // 2. list_location_inventory_impl
    let loc_list = list_location_inventory_impl(
        &conn,
        &f.admin_session_a,
        LocationInventoryFilterRequest {
            branch_id: f.branch_a.clone(),
            product_id: Some(f.product_id.clone()),
            ..Default::default()
        },
    )
    .expect("list location inventory succeeds");
    assert_eq!(loc_list.len(), 1);
    assert_eq!(loc_list[0].quantity_milli, 8000);

    // 3. list_stock_movements_impl
    let mov_list = list_stock_movements_impl(
        &conn,
        &f.admin_session_a,
        StockMovementFilterRequest {
            branch_id: f.branch_a.clone(),
            product_id: Some(f.product_id.clone()),
            ..Default::default()
        },
    )
    .expect("list stock movements succeeds");
    assert_eq!(mov_list.len(), 1);
    assert_eq!(mov_list[0].id, m.id);

    // 4. get_stock_movement_impl
    let single_mov = get_stock_movement_impl(&conn, &f.admin_session_a, &f.branch_a, &m.id)
        .expect("get single movement succeeds");
    assert!(single_mov.is_some());
    assert_eq!(single_mov.unwrap().id, m.id);
}

#[test]
fn test_command_authenticated_read_from_another_branch_is_rejected() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    // Seed stock in Branch A
    let m = post_stock_movement_impl(
        &mut conn,
        &f.admin_session_a,
        PostMovementRequest {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 4000,
            reason: "opening_balance".to_string(),
            source_type: None,
            source_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // User authenticated for Branch B attempts to read Branch A data
    let err_bal =
        get_stock_balance_impl(&conn, &f.admin_session_b, &f.branch_a, &f.product_id, None);
    assert!(err_bal.is_err(), "Cross-branch balance read must fail");

    let err_loc = list_location_inventory_impl(
        &conn,
        &f.admin_session_b,
        LocationInventoryFilterRequest {
            branch_id: f.branch_a.clone(),
            ..Default::default()
        },
    );
    assert!(
        err_loc.is_err(),
        "Cross-branch location inventory read must fail"
    );

    let err_mov = list_stock_movements_impl(
        &conn,
        &f.admin_session_b,
        StockMovementFilterRequest {
            branch_id: f.branch_a.clone(),
            ..Default::default()
        },
    );
    assert!(err_mov.is_err(), "Cross-branch movement list must fail");

    let err_single = get_stock_movement_impl(&conn, &f.admin_session_b, &f.branch_a, &m.id);
    assert!(
        err_single.is_err(),
        "Cross-branch single movement read must fail"
    );
}

#[test]
fn test_command_read_does_not_require_inventory_adjust() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    // Seed stock in Branch A
    post_stock_movement_impl(
        &mut conn,
        &f.admin_session_a,
        PostMovementRequest {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 5000,
            reason: "opening_balance".to_string(),
            source_type: None,
            source_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Cashier (who lacks Permission::InventoryAdjust) reads balance for own branch -> succeeds!
    let bal = get_stock_balance_impl(
        &conn,
        &f.cashier_session_a,
        &f.branch_a,
        &f.product_id,
        None,
    )
    .expect("cashier can read stock balance without InventoryAdjust");

    assert_eq!(bal.aggregate_quantity_milli, 5000);
}

#[test]
fn test_command_ipc_request_correctly_reaches_stock_ledger_service() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    let req = PostMovementRequest {
        branch_id: f.branch_a.clone(),
        product_id: f.product_id.clone(),
        variant_id: Some(f.variant_id.clone()),
        location_id: f.location_id.clone(),
        bin_id: Some(f.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 6500,
        reason: "opening_balance".to_string(),
        source_type: Some("custom_source".into()),
        source_id: Some("SRC-7711".into()),
        idempotency_key: Some("ipc-unique-01".into()),
    };

    let movement = post_stock_movement_impl(&mut conn, &f.manager_session_a, req)
        .expect("post stock movement succeeds");

    assert_eq!(movement.quantity_delta_milli, 6500);
    assert_eq!(movement.source_type.as_deref(), Some("custom_source"));
    assert_eq!(movement.source_id.as_deref(), Some("SRC-7711"));
    assert!(
        movement.user_id.is_some(),
        "user_id must be populated from authenticated session"
    );

    // Verify directly in DB that StockLedgerService processed it
    let (db_delta, db_reason): (i64, String) = conn
        .query_row(
            "SELECT quantity_delta_milli, reason FROM stock_movements WHERE id = ?1",
            [&movement.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(db_delta, 6500);
    assert_eq!(db_reason, "opening_balance");
}

#[test]
fn test_command_stock_ledger_error_mapped_consistently() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    // 1. Insufficient stock error mapping
    let err_underflow = post_stock_movement_impl(
        &mut conn,
        &f.admin_session_a,
        PostMovementRequest {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: -50000,
            reason: "adjustment".to_string(),
            source_type: None,
            source_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(err_underflow.starts_with("Insufficient stock:"));

    // 2. Invalid reason error mapping
    let err_reason = post_stock_movement_impl(
        &mut conn,
        &f.admin_session_a,
        PostMovementRequest {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 1000,
            reason: "invalid_unsupported_reason".to_string(),
            source_type: None,
            source_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(err_reason.starts_with("Invalid reason:"));
}

#[test]
fn test_command_idempotent_repeated_ipc_request_returns_same_result_without_duplicate() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&conn);

    let req = PostMovementRequest {
        branch_id: f.branch_a.clone(),
        product_id: f.product_id.clone(),
        variant_id: None,
        location_id: f.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 4000,
        reason: "opening_balance".to_string(),
        source_type: None,
        source_id: None,
        idempotency_key: Some("ipc-repeat-key-99".to_string()),
    };

    // First call
    let m1 = post_stock_movement_impl(&mut conn, &f.manager_session_a, req.clone())
        .expect("first call succeeds");

    // Second repeated call with same key & payload
    let m2 = post_stock_movement_impl(&mut conn, &f.manager_session_a, req)
        .expect("second call replays cached result");

    assert_eq!(m1.id, m2.id);
    assert_eq!(m1.quantity_delta_milli, m2.quantity_delta_milli);

    // Exactly 1 movement row in DB
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);

    // Stock only mutated once
    let bal = get_stock_balance_impl(
        &conn,
        &f.manager_session_a,
        &f.branch_a,
        &f.product_id,
        None,
    )
    .unwrap();
    assert_eq!(bal.aggregate_quantity_milli, 4000);
}
