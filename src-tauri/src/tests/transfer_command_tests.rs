// F2.12 — Stock Transfers IPC Command Layer Tests
// Verifies security boundary, session authentication, InventoryTransfer authorization,
// branch scope tenancy, DTO conversion, error mapping, and idempotency propagation.

use crate::commands::transfer::{
    cancel_stock_transfer_impl, create_stock_transfer_impl, dispatch_stock_transfer_impl,
    get_stock_transfer_impl, instant_intra_branch_transfer_impl, list_stock_transfers_impl,
    receive_stock_transfer_impl, CancelStockTransferRequest, CreateStockTransferRequest,
    DispatchStockTransferRequest, InstantIntraBranchTransferRequest, ListStockTransfersRequest,
    ReceiveStockTransferRequest,
};
use crate::stock_ledger::{PostMovementInput, StockLedgerService, StockMovementReason};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::transfer::{CreateTransferItemInput, TransferStatus, TransferType};
use crate::user::session::create_local_session;
use rusqlite::Connection;

// =========================================================================
// TEST FIXTURES & HELPERS
// =========================================================================

struct TransferCommandFixtures {
    org_id: String,
    branch_a: String,
    branch_b: String,
    branch_c: String,
    product_id: String,
    loc_a1: String,
    bin_a1: String,
    loc_a2: String,
    bin_a2: String,
    loc_b1: String,
    bin_b1: String,
    admin_session_a: String,
    cashier_session_a: String,
    admin_session_b: String,
    admin_session_c: String,
}

fn create_auth_session(conn: &Connection, branch_id: &str, role: &str) -> String {
    let username = format!("cmd_user_{}_{}", role, &branch_id[..6]);
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

fn setup_command_fixtures(conn: &mut Connection) -> TransferCommandFixtures {
    let (org_id, branch_a) = create_test_org_and_branch(conn);

    let branch_b_obj = crate::branch::create_branch(
        conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Branch B Destination".to_string(),
            address: Some("456 Branch B Ave".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch b created");
    let branch_b = branch_b_obj.id;

    let branch_c_obj = crate::branch::create_branch(
        conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Branch C Third Party".to_string(),
            address: Some("789 Branch C Blvd".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch c created");
    let branch_c = branch_c_obj.id;

    let product_id = "prod_trf_cmd_01".to_string();
    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active)
         VALUES (?1, 'Transfer IPC Test Product', 50.0, 1)",
        [&product_id],
    )
    .expect("product created");

    // Locations & Bins for Branch A
    let loc_a1 = "loc_trf_cmd_a1".to_string();
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Warehouse A1', 'LOC-A1', 'warehouse', 1)",
        [&loc_a1, &branch_a],
    )
    .expect("loc_a1 created");

    let bin_a1 = "bin_trf_cmd_a1".to_string();
    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES (?1, ?2, 'Aisle 1', 'BIN-A1', 1)",
        [&bin_a1, &loc_a1],
    )
    .expect("bin_a1 created");

    let loc_a2 = "loc_trf_cmd_a2".to_string();
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Showroom A2', 'LOC-A2', 'sales_floor', 1)",
        [&loc_a2, &branch_a],
    )
    .expect("loc_a2 created");

    let bin_a2 = "bin_trf_cmd_a2".to_string();
    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES (?1, ?2, 'Shelf 2', 'BIN-A2', 1)",
        [&bin_a2, &loc_a2],
    )
    .expect("bin_a2 created");

    // Locations & Bins for Branch B
    let loc_b1 = "loc_trf_cmd_b1".to_string();
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Warehouse B1', 'LOC-B1', 'warehouse', 1)",
        [&loc_b1, &branch_b],
    )
    .expect("loc_b1 created");

    let bin_b1 = "bin_trf_cmd_b1".to_string();
    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES (?1, ?2, 'Aisle B1', 'BIN-B1', 1)",
        [&bin_b1, &loc_b1],
    )
    .expect("bin_b1 created");

    // Sessions
    let admin_session_a = create_auth_session(conn, &branch_a, "admin");
    let cashier_session_a = create_auth_session(conn, &branch_a, "cashier");
    let admin_session_b = create_auth_session(conn, &branch_b, "admin");
    let admin_session_c = create_auth_session(conn, &branch_c, "admin");

    // Seed stock at loc_a1 / bin_a1: 100,000 milli (100 units)
    let seed_input = PostMovementInput {
        branch_id: branch_a.clone(),
        product_id: product_id.clone(),
        variant_id: None,
        location_id: loc_a1.clone(),
        bin_id: Some(bin_a1.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 100_000,
        reason: StockMovementReason::OpeningStock,
        source_type: None,
        source_id: None,
        user_id: None,
        idempotency_key: None,
    };
    StockLedgerService::post_movement(conn, &seed_input).expect("seed stock posted");

    TransferCommandFixtures {
        org_id,
        branch_a,
        branch_b,
        branch_c,
        product_id,
        loc_a1,
        bin_a1,
        loc_a2,
        bin_a2,
        loc_b1,
        bin_b1,
        admin_session_a,
        cashier_session_a,
        admin_session_b,
        admin_session_c,
    }
}

// =========================================================================
// 1. AUTHENTICATION & AUTHORIZATION DENIAL TESTS
// =========================================================================

#[test]
fn test_authorization_denial_unauthenticated() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&mut conn);

    let req = CreateStockTransferRequest {
        transfer_type: "inter_branch".to_string(),
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 5000,
        }],
        idempotency_key: None,
    };

    // Missing / invalid session fails authentication
    let err = create_stock_transfer_impl(&mut conn, "invalid-session-token", req).unwrap_err();
    assert!(
        err.contains("Authentication required") || err.contains("Session not found"),
        "Unexpected error: {err}"
    );
}

#[test]
fn test_authorization_denial_missing_permission() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&mut conn);

    let req = CreateStockTransferRequest {
        transfer_type: "inter_branch".to_string(),
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 5000,
        }],
        idempotency_key: None,
    };

    // Cashier role lacks Permission::InventoryTransfer
    let err = create_stock_transfer_impl(&mut conn, &f.cashier_session_a, req).unwrap_err();
    assert!(
        err.contains("Permission denied") && err.contains("inventory.transfer"),
        "Unexpected error: {err}"
    );
}

#[test]
fn test_authorization_denial_branch_scope_mismatch() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&mut conn);

    // 1. Create: Admin in Branch A attempts to create transfer where source is Branch B
    let create_req = CreateStockTransferRequest {
        transfer_type: "inter_branch".to_string(),
        source_branch_id: f.branch_b.clone(), // caller belongs to Branch A
        destination_branch_id: f.branch_a.clone(),
        source_location_id: f.loc_b1.clone(),
        destination_location_id: f.loc_a1.clone(),
        source_bin_id: Some(f.bin_b1.clone()),
        destination_bin_id: Some(f.bin_a1.clone()),
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 5000,
        }],
        idempotency_key: None,
    };

    let err = create_stock_transfer_impl(&mut conn, &f.admin_session_a, create_req).unwrap_err();
    assert!(err.contains("Scope mismatch"), "Unexpected error: {err}");

    // Create a valid transfer from A to B
    let valid_req = CreateStockTransferRequest {
        transfer_type: "inter_branch".to_string(),
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 5000,
        }],
        idempotency_key: None,
    };
    let transfer =
        create_stock_transfer_impl(&mut conn, &f.admin_session_a, valid_req).expect("created");

    // 2. Dispatch: Admin in Branch B attempts to dispatch transfer originating from Branch A
    let disp_req = DispatchStockTransferRequest {
        transfer_id: transfer.id.clone(),
        idempotency_key: None,
    };
    let err = dispatch_stock_transfer_impl(&mut conn, &f.admin_session_b, disp_req).unwrap_err();
    assert!(err.contains("Scope mismatch"), "Unexpected error: {err}");

    // Now dispatch properly from Branch A
    let disp_req_a = DispatchStockTransferRequest {
        transfer_id: transfer.id.clone(),
        idempotency_key: None,
    };
    dispatch_stock_transfer_impl(&mut conn, &f.admin_session_a, disp_req_a).expect("dispatched");

    // 3. Receive: Admin in Branch A attempts to receive transfer destined for Branch B
    let recv_req = ReceiveStockTransferRequest {
        transfer_id: transfer.id.clone(),
        destination_location_id: None,
        destination_bin_id: None,
        idempotency_key: None,
    };
    let err = receive_stock_transfer_impl(&mut conn, &f.admin_session_a, recv_req).unwrap_err();
    assert!(err.contains("Scope mismatch"), "Unexpected error: {err}");

    // 4. Get: Admin in Branch C (unrelated third branch) attempts to get transfer between A and B
    let err = get_stock_transfer_impl(&conn, &f.admin_session_c, &transfer.id).unwrap_err();
    assert!(err.contains("Scope mismatch"), "Unexpected error: {err}");

    // 5. List: Admin in Branch A attempts to list transfers with branch_id = Branch B
    let list_req = ListStockTransfersRequest {
        branch_id: Some(f.branch_b.clone()),
        source_branch_id: None,
        destination_branch_id: None,
        transfer_type: None,
        status: None,
        limit: None,
        offset: None,
    };
    let err = list_stock_transfers_impl(&conn, &f.admin_session_a, list_req).unwrap_err();
    assert!(err.contains("Scope mismatch"), "Unexpected error: {err}");
}

// =========================================================================
// 2. VALID INTER-BRANCH COMMAND LIFECYCLE TESTS
// =========================================================================

#[test]
fn test_valid_inter_branch_command_lifecycle() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&mut conn);

    // 1. Create Stock Transfer
    let create_req = CreateStockTransferRequest {
        transfer_type: "inter_branch".to_string(),
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: Some("Command lifecycle verification".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 10_000,
        }],
        idempotency_key: None,
    };

    let transfer =
        create_stock_transfer_impl(&mut conn, &f.admin_session_a, create_req).expect("created");
    assert_eq!(transfer.status, TransferStatus::Draft);
    assert_eq!(transfer.items.len(), 1);
    assert_eq!(transfer.items[0].quantity_milli, 10_000);

    // 2. Dispatch Stock Transfer (Branch A Admin)
    let disp_req = DispatchStockTransferRequest {
        transfer_id: transfer.id.clone(),
        idempotency_key: None,
    };
    let dispatched =
        dispatch_stock_transfer_impl(&mut conn, &f.admin_session_a, disp_req).expect("dispatched");
    assert_eq!(dispatched.status, TransferStatus::InTransit);
    assert!(dispatched.dispatched_at.is_some());

    // 3. Receive Stock Transfer (Branch B Admin)
    let recv_req = ReceiveStockTransferRequest {
        transfer_id: transfer.id.clone(),
        destination_location_id: None,
        destination_bin_id: None,
        idempotency_key: None,
    };
    let received =
        receive_stock_transfer_impl(&mut conn, &f.admin_session_b, recv_req).expect("received");
    assert_eq!(received.status, TransferStatus::Completed);
    assert!(received.received_at.is_some());
    assert_eq!(received.items[0].received_quantity_milli, Some(10_000));

    // 4. Get Stock Transfer (accessible by both Branch A and Branch B)
    let get_a = get_stock_transfer_impl(&conn, &f.admin_session_a, &transfer.id)
        .expect("get a")
        .expect("found");
    assert_eq!(get_a.id, transfer.id);
    assert_eq!(get_a.status, TransferStatus::Completed);

    let get_b = get_stock_transfer_impl(&conn, &f.admin_session_b, &transfer.id)
        .expect("get b")
        .expect("found");
    assert_eq!(get_b.id, transfer.id);

    // 5. List Stock Transfers
    let list_req = ListStockTransfersRequest {
        branch_id: None,
        source_branch_id: None,
        destination_branch_id: None,
        transfer_type: Some("inter_branch".into()),
        status: Some("completed".into()),
        limit: Some(10),
        offset: Some(0),
    };
    let list = list_stock_transfers_impl(&conn, &f.admin_session_a, list_req).expect("list a");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, transfer.id);
}

// =========================================================================
// 3. INSTANT INTRA-BRANCH RELOCATION & DRAFT CANCELLATION
// =========================================================================

#[test]
fn test_instant_intra_branch_and_cancellation() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&mut conn);

    // 1. Instant Intra-Branch Relocation
    let instant_req = InstantIntraBranchTransferRequest {
        branch_id: f.branch_a.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_a2.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_a2.clone()),
        notes: Some("Move to showroom".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 15_000,
        }],
        idempotency_key: None,
    };

    let instant = instant_intra_branch_transfer_impl(&mut conn, &f.admin_session_a, instant_req)
        .expect("instant relocation");
    assert_eq!(instant.status, TransferStatus::Completed);
    assert_eq!(instant.transfer_type, TransferType::IntraBranch);
    assert_eq!(instant.items[0].received_quantity_milli, Some(15_000));

    // 2. Draft Cancellation
    let create_req = CreateStockTransferRequest {
        transfer_type: "inter_branch".to_string(),
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 5000,
        }],
        idempotency_key: None,
    };
    let draft =
        create_stock_transfer_impl(&mut conn, &f.admin_session_a, create_req).expect("created");
    assert_eq!(draft.status, TransferStatus::Draft);

    let cancel_req = CancelStockTransferRequest {
        transfer_id: draft.id.clone(),
    };
    let cancelled =
        cancel_stock_transfer_impl(&mut conn, &f.admin_session_a, cancel_req).expect("cancelled");
    assert_eq!(cancelled.status, TransferStatus::Cancelled);
}

// =========================================================================
// 4. IDEMPOTENCY PROPAGATION TESTS
// =========================================================================

#[test]
fn test_idempotency_key_propagation() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&mut conn);

    let mut create_req = CreateStockTransferRequest {
        transfer_type: "inter_branch".to_string(),
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 5000,
        }],
        idempotency_key: Some("idem_cmd_create_01".into()),
    };

    // First call succeeds
    let first = create_stock_transfer_impl(&mut conn, &f.admin_session_a, create_req.clone())
        .expect("first create");

    // Replay with exact same idempotency key returns cached result
    let replay = create_stock_transfer_impl(&mut conn, &f.admin_session_a, create_req.clone())
        .expect("replay create");
    assert_eq!(first.id, replay.id);

    // Call with same idempotency key but different quantity triggers conflict
    create_req.items[0].quantity_milli = 8000;
    let conflict =
        create_stock_transfer_impl(&mut conn, &f.admin_session_a, create_req).unwrap_err();
    assert!(
        conflict.contains("Idempotency conflict"),
        "Unexpected error: {conflict}"
    );

    // Dispatch idempotency propagation
    let disp_req = DispatchStockTransferRequest {
        transfer_id: first.id.clone(),
        idempotency_key: Some("idem_cmd_disp_01".into()),
    };
    let disp_first = dispatch_stock_transfer_impl(&mut conn, &f.admin_session_a, disp_req.clone())
        .expect("disp first");
    let disp_replay =
        dispatch_stock_transfer_impl(&mut conn, &f.admin_session_a, disp_req).expect("disp replay");
    assert_eq!(disp_first.id, disp_replay.id);
}

// =========================================================================
// 5. DOMAIN ERROR CONVERSION TESTS
// =========================================================================

#[test]
fn test_domain_error_conversion() {
    let mut conn = setup_test_db();
    let f = setup_command_fixtures(&mut conn);

    // 1. Insufficient stock mapping
    let huge_req = CreateStockTransferRequest {
        transfer_type: "inter_branch".to_string(),
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: None,
            quantity_milli: 999_999_000, // exceeds 100,000 seeded
        }],
        idempotency_key: None,
    };
    let transfer =
        create_stock_transfer_impl(&mut conn, &f.admin_session_a, huge_req).expect("created draft");

    let disp_req = DispatchStockTransferRequest {
        transfer_id: transfer.id.clone(),
        idempotency_key: None,
    };
    let err = dispatch_stock_transfer_impl(&mut conn, &f.admin_session_a, disp_req).unwrap_err();
    assert!(
        err.contains("Insufficient stock for product"),
        "Expected clean domain error, got: {err}"
    );
    assert!(!err.contains("sqlite"), "Raw SQLite error leaked: {err}");

    // 2. Entity Not Found mapping
    let not_found_cancel = CancelStockTransferRequest {
        transfer_id: "non-existent-transfer-id".into(),
    };
    let err =
        cancel_stock_transfer_impl(&mut conn, &f.admin_session_a, not_found_cancel).unwrap_err();
    assert!(
        err.contains("Entity not found"),
        "Expected Entity not found, got: {err}"
    );

    // 3. Invalid Status Transition mapping
    let double_cancel = CancelStockTransferRequest {
        transfer_id: transfer.id.clone(),
    };
    cancel_stock_transfer_impl(&mut conn, &f.admin_session_a, double_cancel.clone())
        .expect("cancelled first time");
    let err = cancel_stock_transfer_impl(&mut conn, &f.admin_session_a, double_cancel).unwrap_err();
    assert!(
        err.contains("Invalid status transition"),
        "Expected Invalid status transition, got: {err}"
    );
}
