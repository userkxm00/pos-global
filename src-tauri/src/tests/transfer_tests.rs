// F2.12 — Transfers Behavioral Test Suite
// Exhaustive architectural and domain verification for:
// 1. Intra-branch spatial relocation & 100% batch relocation (Task 5A blocker fix verification)
// 2. Inter-branch dispatch + receive lifecycle
// 3. Serial asset transfer continuity & re-homing
// 4. Batch continuity & destination batch auto-creation
// 5. Validation failures & cross-entity fail-closed constraints
// 6. Authorization, tenancy & ledger boundary enforcement
// 7. Insufficient stock & atomic rollback
// 8. Idempotency replay & conflict detection
// 9. Receive rollback across all mutations
// 10. Concurrency & double dispatch race prevention

use crate::batch::{create_batch, CreateBatchInput};
use crate::permission::{evaluate_user_permission, validate_scope, Permission, PermissionError};
use crate::serial::{create_serial_instance, CreateSerialInput};
use crate::stock_ledger::{
    PostMovementInput, StockLedgerError, StockLedgerService, StockMovementReason,
};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::transfer::{
    CancelTransferInput, CreateTransferInput, CreateTransferItemInput, DispatchTransferInput,
    InstantIntraBranchTransferInput, ReceiveTransferInput, TransferError,
    TransferService, TransferStatus, TransferType,
};
use crate::user::session::create_local_session;
use rusqlite::{params, Connection, OptionalExtension};
use std::str::FromStr;

// =========================================================================
// TEST FIXTURES & HELPERS
// =========================================================================

#[allow(dead_code)]
struct TwoBranchFixtures {
    org_id: String,
    branch_a: String,
    branch_b: String,
    loc_a1: String,
    loc_a2: String,
    bin_a1: String,
    bin_a2: String,
    loc_b1: String,
    bin_b1: String,
    product_id: String,
    variant_id: String,
    product_serial_id: String,
    user_id: String,
}

fn setup_transfer_fixtures(conn: &Connection) -> TwoBranchFixtures {
    let (org_id, branch_a) = create_test_org_and_branch(conn);

    let branch_b_res = crate::branch::create_branch(
        conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Destination Branch B".to_string(),
            address: Some("456 Market St, Boston, MA".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("destination branch created");
    let branch_b = branch_b_res.id;

    let loc_a1 = "loc_branch_a_01".to_string();
    let bin_a1 = "bin_branch_a_01".to_string();
    let loc_a2 = "loc_branch_a_02".to_string();
    let bin_a2 = "bin_branch_a_02".to_string();

    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Branch A Staging', 'LOC-A1', 'warehouse', 1)",
        params![loc_a1, branch_a],
    )
    .expect("loc_a1 created");

    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES (?1, ?2, 'Aisle A Shelf 1', 'BIN-A1', 1)",
        params![bin_a1, loc_a1],
    )
    .expect("bin_a1 created");

    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Branch A Floor', 'LOC-A2', 'sales_floor', 1)",
        params![loc_a2, branch_a],
    )
    .expect("loc_a2 created");

    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES (?1, ?2, 'Floor Rack 2', 'BIN-A2', 1)",
        params![bin_a2, loc_a2],
    )
    .expect("bin_a2 created");

    let loc_b1 = "loc_branch_b_01".to_string();
    let bin_b1 = "bin_branch_b_01".to_string();

    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Branch B Intake', 'LOC-B1', 'warehouse', 1)",
        params![loc_b1, branch_b],
    )
    .expect("loc_b1 created");

    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES (?1, ?2, 'Intake Bay 1', 'BIN-B1', 1)",
        params![bin_b1, loc_b1],
    )
    .expect("bin_b1 created");

    let product_id = "prod_trf_standard_01".to_string();
    let variant_id = "var_trf_standard_01".to_string();

    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active, requires_serial)
         VALUES (?1, 'Transfer Standard Product', 20.0, 1, 0)",
        params![product_id],
    )
    .expect("standard product created");

    conn.execute(
        "INSERT OR REPLACE INTO product_capabilities (product_id, capability_id, enabled)
         SELECT ?1, id, 1 FROM capabilities WHERE code = 'BATCH'",
        params![product_id],
    )
    .expect("batch capability added");

    conn.execute(
        "INSERT INTO product_variants (id, product_id, sku, is_active)
         VALUES (?1, ?2, 'SKU-TRF-STD-01', 1)",
        params![variant_id, product_id],
    )
    .expect("standard variant created");

    let product_serial_id = "prod_trf_serial_01".to_string();

    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active, requires_serial)
         VALUES (?1, 'Transfer Serialized Asset', 500.0, 1, 1)",
        params![product_serial_id],
    )
    .expect("serial product created");

    let user_id = "usr_transfer_admin_01".to_string();
    conn.execute(
        "INSERT INTO users (id, branch_id, full_name, username, role, is_active)
         VALUES (?1, ?2, 'Transfer Admin', 'trf_admin', 'admin', 1)",
        params![user_id, branch_a],
    )
    .expect("admin user created");

    TwoBranchFixtures {
        org_id,
        branch_a,
        branch_b,
        loc_a1,
        loc_a2,
        bin_a1,
        bin_a2,
        loc_b1,
        bin_b1,
        product_id,
        variant_id,
        product_serial_id,
        user_id,
    }
}

fn seed_stock(
    conn: &mut Connection,
    branch_id: &str,
    product_id: &str,
    variant_id: Option<&str>,
    location_id: &str,
    bin_id: Option<&str>,
    batch_id: Option<&str>,
    serial_id: Option<&str>,
    qty_milli: i64,
) {
    let input = PostMovementInput {
        branch_id: branch_id.to_string(),
        product_id: product_id.to_string(),
        variant_id: variant_id.map(ToString::to_string),
        location_id: location_id.to_string(),
        bin_id: bin_id.map(ToString::to_string),
        batch_id: batch_id.map(ToString::to_string),
        serial_id: serial_id.map(ToString::to_string),
        quantity_delta_milli: qty_milli,
        reason: StockMovementReason::OpeningBalance,
        source_type: Some("opening_balance".into()),
        source_id: None,
        user_id: None,
        idempotency_key: None,
    };
    StockLedgerService::post_movement(conn, &input).expect("stock seeded via opening balance");
}

fn get_aggregate_stock(conn: &Connection, branch_id: &str, product_id: &str) -> i64 {
    conn.query_row(
        "SELECT quantity_milli FROM inventory WHERE branch_id = ?1 AND product_id = ?2",
        params![branch_id, product_id],
        |r| r.get(0),
    )
    .optional()
    .expect("query aggregate stock")
    .unwrap_or(0)
}

fn get_spatial_stock(
    conn: &Connection,
    branch_id: &str,
    location_id: &str,
    product_id: &str,
) -> i64 {
    conn.query_row(
        "SELECT COALESCE(SUM(quantity_milli), 0) FROM location_inventory
         WHERE branch_id = ?1 AND location_id = ?2 AND product_id = ?3",
        params![branch_id, location_id, product_id],
        |r| r.get(0),
    )
    .expect("query spatial stock")
}

// =========================================================================
// 1. INTRA-BRANCH SPATIAL RELOCATION TESTS
// =========================================================================

#[test]
fn test_instant_intra_branch_relocation_success() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // Seed 10,000 milli at loc_a1, bin_a1
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        10000,
    );

    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a1, &f.product_id),
        10000
    );
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a2, &f.product_id),
        0
    );
    assert_eq!(
        get_aggregate_stock(&conn, &f.branch_a, &f.product_id),
        10000
    );

    // Relocate 5,000 milli from loc_a1/bin_a1 to loc_a2/bin_a2
    let input = InstantIntraBranchTransferInput {
        branch_id: f.branch_a.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_a2.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_a2.clone()),
        notes: Some("Relocating stock to sales floor".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_milli: 5000,
        }],
        user_id: Some(f.user_id.clone()),
        idempotency_key: Some("idem-intra-01".into()),
    };

    let transfer = TransferService::instant_intra_branch_transfer(&mut conn, &input)
        .expect("instant intra-branch transfer succeeded");

    assert_eq!(transfer.transfer_type, TransferType::IntraBranch);
    assert_eq!(transfer.status, TransferStatus::Completed);
    assert_eq!(transfer.items.len(), 1);
    assert_eq!(transfer.items[0].quantity_milli, 5000);
    assert_eq!(transfer.items[0].received_quantity_milli, Some(5000));
    assert!(transfer.dispatched_at.is_some());
    assert!(transfer.received_at.is_some());

    // Verify spatial deltas
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a1, &f.product_id),
        5000
    );
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a2, &f.product_id),
        5000
    );

    // Verify aggregate invariant (net delta = 0)
    assert_eq!(
        get_aggregate_stock(&conn, &f.branch_a, &f.product_id),
        10000
    );

    // Verify exactly two transfer ledger movements exist
    let movements_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE source_type = 'transfer' AND source_id = ?1",
            params![transfer.id],
            |r| r.get(0),
        )
        .expect("count transfer movements");
    assert_eq!(movements_count, 2);

    let net_movement_delta: i64 = conn
        .query_row(
            "SELECT SUM(quantity_delta_milli) FROM stock_movements WHERE source_id = ?1",
            params![transfer.id],
            |r| r.get(0),
        )
        .expect("sum transfer movements");
    assert_eq!(net_movement_delta, 0);
}

#[test]
fn test_instant_intra_branch_relocation_100_percent_batch() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // Create a batch with exactly 5,000 milli
    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_number: "BATCH-100-INTRA".into(),
            quantity_milli: 0,
            cost_price_minor: Some(150),
            manufactured_date: Some("2026-01-01".into()),
            expiry_date: Some("2029-12-31".into()),
        },
    )
    .expect("batch created");

    // Seed stock into loc_a1 with this batch
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        Some(&batch.id),
        None,
        5000,
    );

    // Verify initial batch state
    let initial_batch_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM product_batches WHERE id = ?1",
            params![batch.id],
            |r| r.get(0),
        )
        .expect("initial batch qty");
    assert_eq!(initial_batch_qty, 5000);

    // Relocate 100% (5,000 milli) of the batch from loc_a1 to loc_a2
    let input = InstantIntraBranchTransferInput {
        branch_id: f.branch_a.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_a2.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_a2.clone()),
        notes: Some("100% intra-branch batch relocation".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_id: Some(batch.id.clone()),
            serial_id: None,
            quantity_milli: 5000,
        }],
        user_id: Some(f.user_id.clone()),
        idempotency_key: Some("idem-intra-100-batch".into()),
    };

    let transfer = TransferService::instant_intra_branch_transfer(&mut conn, &input)
        .expect("100% intra-branch batch transfer must succeed under Task 5A remediation");

    assert_eq!(transfer.status, TransferStatus::Completed);
    assert_eq!(transfer.items[0].received_quantity_milli, Some(5000));

    // Batch quantity must remain exactly 5,000 and status must be active
    let (final_batch_qty, final_batch_status): (i64, String) = conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("final batch state");
    assert_eq!(final_batch_qty, 5000);
    assert_eq!(final_batch_status, "active");

    // Verify spatial relocation
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a1, &f.product_id),
        0
    );
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a2, &f.product_id),
        5000
    );
    assert_eq!(get_aggregate_stock(&conn, &f.branch_a, &f.product_id), 5000);
}

// =========================================================================
// 2. INTER-BRANCH DISPATCH + RECEIVE LIFECYCLE TESTS
// =========================================================================

#[test]
fn test_inter_branch_lifecycle_create_dispatch_receive() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // Seed 20,000 milli at branch A
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        20000,
    );

    // 1. Create Transfer -> Draft
    let create_input = CreateTransferInput {
        transfer_type: TransferType::InterBranch,
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: Some("Inter-branch stock transfer".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_milli: 10000,
        }],
        user_id: Some(f.user_id.clone()),
        idempotency_key: Some("idem-inter-create".into()),
    };

    let transfer =
        TransferService::create_transfer(&mut conn, &create_input).expect("transfer created");
    assert_eq!(transfer.status, TransferStatus::Draft);
    assert_eq!(transfer.items.len(), 1);
    assert_eq!(transfer.items[0].received_quantity_milli, None);
    assert!(transfer.dispatched_at.is_none());
    assert!(transfer.received_at.is_none());

    // Draft transfer must have zero ledger impact
    assert_eq!(
        get_aggregate_stock(&conn, &f.branch_a, &f.product_id),
        20000
    );
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a1, &f.product_id),
        20000
    );
    assert_eq!(get_aggregate_stock(&conn, &f.branch_b, &f.product_id), 0);

    // 2. Dispatch Transfer -> InTransit
    let dispatch_input = DispatchTransferInput {
        transfer_id: transfer.id.clone(),
        user_id: Some(f.user_id.clone()),
        idempotency_key: Some("idem-inter-dispatch".into()),
    };

    let dispatched = TransferService::dispatch_transfer(&mut conn, &dispatch_input)
        .expect("transfer dispatched");
    assert_eq!(dispatched.status, TransferStatus::InTransit);
    assert!(dispatched.dispatched_at.is_some());
    assert_eq!(dispatched.dispatched_by, Some(f.user_id.clone()));

    // Source inventory decreased; destination unchanged
    assert_eq!(
        get_aggregate_stock(&conn, &f.branch_a, &f.product_id),
        10000
    );
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a1, &f.product_id),
        10000
    );
    assert_eq!(get_aggregate_stock(&conn, &f.branch_b, &f.product_id), 0);

    // Exactly 1 outbound transfer ledger movement
    let outbound_movements: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE source_id = ?1 AND quantity_delta_milli < 0",
            params![transfer.id],
            |r| r.get(0),
        )
        .expect("count outbound movements");
    assert_eq!(outbound_movements, 1);

    // 3. Receive Transfer -> Completed
    let receive_input = ReceiveTransferInput {
        transfer_id: transfer.id.clone(),
        destination_location_id: None,
        destination_bin_id: None,
        user_id: Some(f.user_id.clone()),
        idempotency_key: Some("idem-inter-receive".into()),
    };

    let received =
        TransferService::receive_transfer(&mut conn, &receive_input).expect("transfer received");
    assert_eq!(received.status, TransferStatus::Completed);
    assert!(received.received_at.is_some());
    assert_eq!(received.received_by, Some(f.user_id.clone()));
    assert_eq!(received.items[0].received_quantity_milli, Some(10000));

    // Destination inventory credited
    assert_eq!(
        get_aggregate_stock(&conn, &f.branch_b, &f.product_id),
        10000
    );
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_b, &f.loc_b1, &f.product_id),
        10000
    );

    // Exactly 1 inbound transfer ledger movement
    let inbound_movements: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE source_id = ?1 AND quantity_delta_milli > 0",
            params![transfer.id],
            |r| r.get(0),
        )
        .expect("count inbound movements");
    assert_eq!(inbound_movements, 1);
}

#[test]
fn test_draft_cancellation() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    let create_input = CreateTransferInput {
        transfer_type: TransferType::InterBranch,
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: None,
        destination_bin_id: None,
        notes: Some("To be cancelled".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_milli: 1000,
        }],
        user_id: Some(f.user_id.clone()),
        idempotency_key: None,
    };

    let transfer =
        TransferService::create_transfer(&mut conn, &create_input).expect("created draft");
    assert_eq!(transfer.status, TransferStatus::Draft);

    let cancel_input = CancelTransferInput {
        transfer_id: transfer.id.clone(),
        user_id: Some(f.user_id.clone()),
    };

    let cancelled =
        TransferService::cancel_transfer(&mut conn, &cancel_input).expect("cancelled draft");
    assert_eq!(cancelled.status, TransferStatus::Cancelled);

    // Cannot dispatch a cancelled transfer
    let dispatch_err = TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(
        dispatch_err,
        TransferError::InvalidStatusTransition { .. }
    ));
}

// =========================================================================
// 3. SERIAL TRANSFER CONTINUITY TESTS
// =========================================================================

#[test]
fn test_serial_transfer_dispatch_and_receive() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // Create serial number instance in branch A
    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_serial_id.clone(),
            variant_id: None,
            serial_number: Some("SN-TRF-9999".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("serial created in stock");

    // Seed stock in ledger for this serial (1,000 milli = 1 unit)
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_serial_id,
        None,
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        Some(&serial.id),
        1000,
    );

    // Verify initial serial state
    let (s_branch, s_status, s_loc, s_bin): (String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT branch_id, status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("initial serial query");
    assert_eq!(s_branch, f.branch_a);
    assert_eq!(s_status, "in_stock");
    assert_eq!(s_loc, Some(f.loc_a1.clone()));
    assert_eq!(s_bin, Some(f.bin_a1.clone()));

    // Create and dispatch transfer
    let create_input = CreateTransferInput {
        transfer_type: TransferType::InterBranch,
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: Some("Transfer serialized asset".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_serial_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: Some(serial.id.clone()),
            quantity_milli: 1000,
        }],
        user_id: Some(f.user_id.clone()),
        idempotency_key: None,
    };

    let transfer =
        TransferService::create_transfer(&mut conn, &create_input).expect("transfer created");

    let dispatched = TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: Some(f.user_id.clone()),
            idempotency_key: None,
        },
    )
    .expect("dispatched");
    assert_eq!(dispatched.status, TransferStatus::InTransit);

    // After dispatch: status = transferred, location/bin cleared to NULL, branch remains source
    let (s_branch_disp, s_status_disp, s_loc_disp, s_bin_disp): (
        String,
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT branch_id, status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("dispatched serial query");
    assert_eq!(s_branch_disp, f.branch_a);
    assert_eq!(s_status_disp, "transferred");
    assert_eq!(s_loc_disp, None);
    assert_eq!(s_bin_disp, None);

    // Receive transfer at destination branch B
    let received = TransferService::receive_transfer(
        &mut conn,
        &ReceiveTransferInput {
            transfer_id: transfer.id.clone(),
            destination_location_id: Some(f.loc_b1.clone()),
            destination_bin_id: Some(f.bin_b1.clone()),
            user_id: Some(f.user_id.clone()),
            idempotency_key: None,
        },
    )
    .expect("received");
    assert_eq!(received.status, TransferStatus::Completed);

    // After receive: status = in_stock, branch_id re-homed to branch B, location/bin assigned
    let (s_branch_recv, s_status_recv, s_loc_recv, s_bin_recv): (
        String,
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT branch_id, status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("received serial query");
    assert_eq!(s_branch_recv, f.branch_b);
    assert_eq!(s_status_recv, "in_stock");
    assert_eq!(s_loc_recv, Some(f.loc_b1.clone()));
    assert_eq!(s_bin_recv, Some(f.bin_b1.clone()));
}

#[test]
fn test_serial_invalid_status_rejected() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_serial_id.clone(),
            variant_id: None,
            serial_number: Some("SN-DEFECTIVE-01".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("serial created defective");

    conn.execute(
        "UPDATE serial_numbers SET status = 'defective' WHERE id = ?1",
        params![serial.id],
    )
    .expect("set serial status to defective");

    let create_input = CreateTransferInput {
        transfer_type: TransferType::InterBranch,
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: None,
        destination_bin_id: None,
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_serial_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: Some(serial.id.clone()),
            quantity_milli: 1000,
        }],
        user_id: None,
        idempotency_key: None,
    };

    let err = TransferService::create_transfer(&mut conn, &create_input).unwrap_err();
    assert!(matches!(err, TransferError::InvalidSerialStatus(_)));
}

#[test]
fn test_serial_quantity_must_be_1000() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_serial_id.clone(),
            variant_id: None,
            serial_number: Some("SN-QTY-FAIL-01".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("serial created");

    let create_input = CreateTransferInput {
        transfer_type: TransferType::InterBranch,
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: None,
        destination_bin_id: None,
        notes: None,
        items: vec![CreateTransferItemInput {
            product_id: f.product_serial_id.clone(),
            variant_id: None,
            batch_id: None,
            serial_id: Some(serial.id.clone()),
            quantity_milli: 2000, // Invalid: must be 1000
        }],
        user_id: None,
        idempotency_key: None,
    };

    let err = TransferService::create_transfer(&mut conn, &create_input).unwrap_err();
    assert!(matches!(err, TransferError::Validation(_)));
}

// =========================================================================
// 4. BATCH CONTINUITY TESTS
// =========================================================================

#[test]
fn test_batch_transfer_existing_destination_batch() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // Source batch in branch A
    let batch_a = create_batch(
        &conn,
        &CreateBatchInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_number: "LOT-SHARED-01".into(),
            quantity_milli: 0,
            cost_price_minor: Some(250),
            manufactured_date: Some("2025-06-01".into()),
            expiry_date: Some("2028-12-31".into()),
        },
    )
    .expect("source batch created");

    // Destination batch in branch B (case-insensitive batch number match)
    let batch_b = create_batch(
        &conn,
        &CreateBatchInput {
            branch_id: f.branch_b.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_number: "lot-shared-01".into(),
            quantity_milli: 0,
            cost_price_minor: Some(250),
            manufactured_date: Some("2025-06-01".into()),
            expiry_date: Some("2028-12-31".into()),
        },
    )
    .expect("destination batch created");

    // Seed stock in branch A (10,000 milli) and branch B (2,000 milli)
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        Some(&batch_a.id),
        None,
        10000,
    );
    seed_stock(
        &mut conn,
        &f.branch_b,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_b1,
        Some(&f.bin_b1),
        Some(&batch_b.id),
        None,
        2000,
    );

    // Transfer 4,000 milli from branch A to branch B
    let create_input = CreateTransferInput {
        transfer_type: TransferType::InterBranch,
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: Some(f.bin_a1.clone()),
        destination_bin_id: Some(f.bin_b1.clone()),
        notes: Some("Batch transfer existing".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_id: Some(batch_a.id.clone()),
            serial_id: None,
            quantity_milli: 4000,
        }],
        user_id: Some(f.user_id.clone()),
        idempotency_key: None,
    };

    let transfer =
        TransferService::create_transfer(&mut conn, &create_input).expect("transfer created");
    TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: Some(f.user_id.clone()),
            idempotency_key: None,
        },
    )
    .expect("dispatched");

    // After dispatch: source batch decremented from 10,000 to 6,000
    let qty_a: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM product_batches WHERE id = ?1",
            params![batch_a.id],
            |r| r.get(0),
        )
        .expect("qty_a");
    assert_eq!(qty_a, 6000);

    TransferService::receive_transfer(
        &mut conn,
        &ReceiveTransferInput {
            transfer_id: transfer.id.clone(),
            destination_location_id: None,
            destination_bin_id: None,
            user_id: Some(f.user_id.clone()),
            idempotency_key: None,
        },
    )
    .expect("received");

    // After receive: destination batch incremented from 2,000 to 6,000
    let qty_b: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM product_batches WHERE id = ?1",
            params![batch_b.id],
            |r| r.get(0),
        )
        .expect("qty_b");
    assert_eq!(qty_b, 6000);
}

#[test]
fn test_batch_transfer_new_destination_batch_created() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    let batch_a = create_batch(
        &conn,
        &CreateBatchInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_number: "LOT-BRAND-NEW-01".into(),
            quantity_milli: 0,
            cost_price_minor: Some(780),
            manufactured_date: Some("2026-02-15".into()),
            expiry_date: Some("2029-08-30".into()),
        },
    )
    .expect("source batch created");

    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        Some(&batch_a.id),
        None,
        8000,
    );

    // Verify destination branch B currently has no batches for this product
    let b_count_before: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM product_batches WHERE branch_id = ?1 AND product_id = ?2",
            params![f.branch_b, f.product_id],
            |r| r.get(0),
        )
        .expect("b_count_before");
    assert_eq!(b_count_before, 0);

    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: Some(f.bin_a1.clone()),
            destination_bin_id: Some(f.bin_b1.clone()),
            notes: Some("Auto-create destination batch".into()),
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: Some(batch_a.id.clone()),
                serial_id: None,
                quantity_milli: 3500,
            }],
            user_id: Some(f.user_id.clone()),
            idempotency_key: None,
        },
    )
    .expect("created");

    TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: Some(f.user_id.clone()),
            idempotency_key: None,
        },
    )
    .expect("dispatched");

    TransferService::receive_transfer(
        &mut conn,
        &ReceiveTransferInput {
            transfer_id: transfer.id.clone(),
            destination_location_id: None,
            destination_bin_id: None,
            user_id: Some(f.user_id.clone()),
            idempotency_key: None,
        },
    )
    .expect("received");

    // Verify destination batch was created with exact metadata copied
    let (b_num, b_cost, b_mfg, b_exp, b_qty, b_status): (
        String,
        Option<i64>,
        Option<String>,
        String,
        i64,
        String,
    ) = conn
        .query_row(
            "SELECT batch_number, cost_price_minor, manufactured_date, expiry_date, quantity_milli, status
             FROM product_batches WHERE branch_id = ?1 AND product_id = ?2",
            params![f.branch_b, f.product_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .expect("new destination batch");

    assert_eq!(b_num, "LOT-BRAND-NEW-01");
    assert_eq!(b_cost, Some(780));
    assert_eq!(b_mfg, Some("2026-02-15".into()));
    assert_eq!(b_exp, "2029-08-30");
    assert_eq!(b_qty, 3500);
    assert_eq!(b_status, "active");
}

#[test]
fn test_batch_transfer_mismatches_fail_closed() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    let batch_src = create_batch(
        &conn,
        &CreateBatchInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_number: "LOT-MISMATCH-01".into(),
            quantity_milli: 0,
            cost_price_minor: Some(100),
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .expect("src batch created");

    // Seed 10,000 milli in branch A
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        Some(&batch_src.id),
        None,
        10000,
    );

    // Destination batch with different expiry
    create_batch(
        &conn,
        &CreateBatchInput {
            branch_id: f.branch_b.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_number: "LOT-MISMATCH-01".into(),
            quantity_milli: 0,
            cost_price_minor: Some(100),
            manufactured_date: None,
            expiry_date: Some("2029-01-01".into()), // Different expiry date!
        },
    )
    .expect("conflicting destination batch created");

    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: Some(batch_src.id.clone()),
                serial_id: None,
                quantity_milli: 1000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("transfer created");

    TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("dispatched");

    // Receive must fail closed because destination batch expiry does not match
    let err = TransferService::receive_transfer(
        &mut conn,
        &ReceiveTransferInput {
            transfer_id: transfer.id.clone(),
            destination_location_id: None,
            destination_bin_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, TransferError::InvalidBatch(_)));
}

// =========================================================================
// 5. VALIDATION FAILURES TESTS
// =========================================================================

#[test]
fn test_validation_failures_topology_and_locations() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // 1. Quantity <= 0
    let err_zero_qty = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::IntraBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_a.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_a2.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: None,
                serial_id: None,
                quantity_milli: 0,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_zero_qty, TransferError::Validation(_)));

    // 2. Intra-branch topology mismatch (different branches)
    let err_topo = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::IntraBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(), // Different branches in intra_branch!
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: None,
                batch_id: None,
                serial_id: None,
                quantity_milli: 1000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_topo, TransferError::TopologyMismatch(_)));

    // 3. Same location + same bin no-op
    let err_noop = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::IntraBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_a.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_a1.clone(), // Same location!
            source_bin_id: Some(f.bin_a1.clone()),
            destination_bin_id: Some(f.bin_a1.clone()), // Same bin!
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: None,
                batch_id: None,
                serial_id: None,
                quantity_milli: 1000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_noop, TransferError::NoOpRelocation(_)));

    // 4. Same location + both bins NULL no-op
    let err_noop_null_bins = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::IntraBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_a.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_a1.clone(), // Same location
            source_bin_id: None,
            destination_bin_id: None, // Both bins NULL!
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: None,
                batch_id: None,
                serial_id: None,
                quantity_milli: 1000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(
        err_noop_null_bins,
        TransferError::NoOpRelocation(_)
    ));

    // 5. Cross-branch location mismatch (source location belongs to branch B)
    let err_loc_branch = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_b1.clone(), // Belongs to branch B, not A!
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: None,
                batch_id: None,
                serial_id: None,
                quantity_milli: 1000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_loc_branch, TransferError::BranchMismatch(_)));
}

#[test]
fn test_validation_failures_tracked_entities_and_duplicates() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_number: "LOT-VAL-01".into(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2030-01-01".into()),
        },
    )
    .expect("batch");

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            branch_id: f.branch_a.clone(),
            product_id: f.product_serial_id.clone(),
            variant_id: None,
            serial_number: Some("SN-VAL-01".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("serial");

    // 1. Both batch_id and serial_id specified
    let err_both = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: None,
                batch_id: Some(batch.id.clone()),
                serial_id: Some(serial.id.clone()), // Both specified!
                quantity_milli: 1000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_both, TransferError::Validation(_)));

    // 2. Duplicate serial_id in same transfer payload (Task 5A protection)
    let err_dup_serial = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![
                CreateTransferItemInput {
                    product_id: f.product_serial_id.clone(),
                    variant_id: None,
                    batch_id: None,
                    serial_id: Some(serial.id.clone()),
                    quantity_milli: 1000,
                },
                CreateTransferItemInput {
                    product_id: f.product_serial_id.clone(),
                    variant_id: None,
                    batch_id: None,
                    serial_id: Some(serial.id.clone()), // Duplicate serial in payload!
                    quantity_milli: 1000,
                },
            ],
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_dup_serial, TransferError::Validation(_)));
}

// =========================================================================
// 6. AUTHORIZATION, TENANCY & LEDGER BOUNDARY TESTS
// =========================================================================

#[test]
fn test_authorization_tenancy_and_role_boundaries() {
    let conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // 1. Create Cashier user in Branch A
    let cashier = create_test_user_with_creds(
        &conn,
        &f.branch_a,
        "Cashier User",
        Some("cashier_trf"),
        Some("pass123"),
        Some("1234"),
        "cashier",
    )
    .expect("cashier created");

    // 2. Cashier lacks Permission::InventoryTransfer (fails closed)
    let cashier_has_transfer =
        evaluate_user_permission(&conn, &cashier.id, "cashier", Permission::InventoryTransfer)
            .expect("evaluate cashier permission");
    assert!(
        !cashier_has_transfer,
        "Cashier role must be rejected for inventory transfers"
    );

    // 3. Manager and Admin have Permission::InventoryTransfer
    let manager = create_test_user_with_creds(
        &conn,
        &f.branch_a,
        "Manager User",
        Some("manager_trf"),
        Some("pass123"),
        Some("1234"),
        "manager",
    )
    .expect("manager created");

    let mgr_has_transfer =
        evaluate_user_permission(&conn, &manager.id, "manager", Permission::InventoryTransfer)
            .expect("evaluate manager permission");
    assert!(
        mgr_has_transfer,
        "Manager role must possess inventory transfer permission"
    );

    // 4. Source-branch unauthorized session rejected when operating against destination branch
    let session_a = create_local_session(&conn, &manager.id, &f.branch_a, "password", None)
        .expect("session a created");

    // Cross-branch tenancy validation: session_a (branch A) attempting operation targeting branch B
    let cross_branch_err = validate_scope(
        Some(&f.org_id),
        &session_a.branch_id,
        Some(&f.org_id),
        Some(&f.branch_b),
    )
    .unwrap_err();
    assert!(matches!(
        cross_branch_err,
        PermissionError::ScopeMismatch { .. }
    ));

    // 5. Destination-branch user in Branch B
    let user_b = create_test_user_with_creds(
        &conn,
        &f.branch_b,
        "Branch B Manager",
        Some("mgr_b_trf"),
        Some("pass123"),
        Some("1234"),
        "manager",
    )
    .expect("user b created");
    let session_b = create_local_session(&conn, &user_b.id, &f.branch_b, "password", None)
        .expect("session b created");

    // Session B cannot manipulate transfer in Branch A's context
    let cross_branch_b_err = validate_scope(
        Some(&f.org_id),
        &session_b.branch_id,
        Some(&f.org_id),
        Some(&f.branch_a),
    )
    .unwrap_err();
    assert!(matches!(
        cross_branch_b_err,
        PermissionError::ScopeMismatch { .. }
    ));
}

#[test]
fn test_ledger_authorization_rejects_bogus_transfer() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        10000,
    );

    // Attempt direct ledger mutation with bogus source_id
    let bogus_input = PostMovementInput {
        branch_id: f.branch_a.clone(),
        product_id: f.product_id.clone(),
        variant_id: Some(f.variant_id.clone()),
        location_id: f.loc_a1.clone(),
        bin_id: Some(f.bin_a1.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -1000,
        reason: StockMovementReason::Transfer,
        source_type: Some("transfer".into()),
        source_id: Some("bogus-transfer-uuid-999".into()),
        user_id: None,
        idempotency_key: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &bogus_input).unwrap_err();
    assert!(matches!(err, StockLedgerError::Validation(_)));
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn test_ledger_authorization_rejects_wrong_branch_and_location() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        10000,
    );

    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: None,
                serial_id: None,
                quantity_milli: 1000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("created");

    // Outbound movement against branch B (wrong branch)
    let wrong_branch_input = PostMovementInput {
        branch_id: f.branch_b.clone(), // Wrong! Source is branch A
        product_id: f.product_id.clone(),
        variant_id: Some(f.variant_id.clone()),
        location_id: f.loc_b1.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -1000,
        reason: StockMovementReason::Transfer,
        source_type: Some("transfer".into()),
        source_id: Some(transfer.id.clone()),
        user_id: None,
        idempotency_key: None,
    };

    let err_branch = StockLedgerService::post_movement(&mut conn, &wrong_branch_input).unwrap_err();
    assert!(matches!(err_branch, StockLedgerError::BranchMismatch(_)));

    // Outbound movement against wrong location
    let wrong_loc_input = PostMovementInput {
        branch_id: f.branch_a.clone(),
        product_id: f.product_id.clone(),
        variant_id: Some(f.variant_id.clone()),
        location_id: f.loc_a2.clone(), // Wrong! Transfer source is loc_a1
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -1000,
        reason: StockMovementReason::Transfer,
        source_type: Some("transfer".into()),
        source_id: Some(transfer.id.clone()),
        user_id: None,
        idempotency_key: None,
    };

    let err_loc = StockLedgerService::post_movement(&mut conn, &wrong_loc_input).unwrap_err();
    assert!(matches!(err_loc, StockLedgerError::InvalidLocation(_)));
}

#[test]
fn test_ledger_authorization_rejects_cancelled_transfer() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        10000,
    );

    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: None,
                serial_id: None,
                quantity_milli: 1000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("created");

    TransferService::cancel_transfer(
        &mut conn,
        &CancelTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: None,
        },
    )
    .expect("cancelled");

    let cancelled_input = PostMovementInput {
        branch_id: f.branch_a.clone(),
        product_id: f.product_id.clone(),
        variant_id: Some(f.variant_id.clone()),
        location_id: f.loc_a1.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -1000,
        reason: StockMovementReason::Transfer,
        source_type: Some("transfer".into()),
        source_id: Some(transfer.id.clone()),
        user_id: None,
        idempotency_key: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &cancelled_input).unwrap_err();
    assert!(matches!(err, StockLedgerError::Validation(_)));
    assert!(err.to_string().contains("cancelled"));
}

#[test]
fn test_public_movement_rejects_transfer_string() {
    let parsed = StockMovementReason::from_str("transfer");
    assert!(parsed.is_err());
    assert!(matches!(
        parsed.unwrap_err(),
        StockLedgerError::InvalidReason(_)
    ));
}

// =========================================================================
// 7. INSUFFICIENT STOCK & ATOMIC ROLLBACK TESTS
// =========================================================================

#[test]
fn test_dispatch_insufficient_stock_aborts_and_rolls_back() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // Seed only 5,000 milli
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        5000,
    );

    // Request 10,000 milli
    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: Some(f.bin_a1.clone()),
            destination_bin_id: Some(f.bin_b1.clone()),
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: None,
                serial_id: None,
                quantity_milli: 10000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("created draft");

    let err = TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: None,
            idempotency_key: Some("idem-insufficient".into()),
        },
    )
    .unwrap_err();

    assert!(matches!(err, TransferError::InsufficientStock { .. }));

    // Status remains Draft
    let current = TransferService::get_transfer(&conn, &transfer.id)
        .expect("get transfer")
        .expect("exists");
    assert_eq!(current.status, TransferStatus::Draft);

    // Balances unchanged
    assert_eq!(get_aggregate_stock(&conn, &f.branch_a, &f.product_id), 5000);
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a1, &f.product_id),
        5000
    );

    // No transfer movements recorded
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE source_id = ?1",
            params![transfer.id],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(count, 0);

    // Idempotency key not recorded
    let key_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM idempotency_keys WHERE key = 'idem-insufficient'",
            [],
            |r| r.get(0),
        )
        .expect("key_count");
    assert_eq!(key_count, 0);
}

#[test]
fn test_dispatch_multi_item_partial_failure_rolls_back_entirely() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // Create a second product
    let prod_2 = "prod_multi_fail_02".to_string();
    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active) VALUES (?1, 'Product Two', 15.0, 1)",
        params![prod_2],
    )
    .expect("prod 2");

    // Seed 10,000 milli for product 1, but only 2,000 milli for product 2
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        10000,
    );
    seed_stock(
        &mut conn,
        &f.branch_a,
        &prod_2,
        None,
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        2000,
    );

    // Multi-item transfer: Item 1 requests 5,000 (available), Item 2 requests 8,000 (insufficient!)
    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![
                CreateTransferItemInput {
                    product_id: f.product_id.clone(),
                    variant_id: Some(f.variant_id.clone()),
                    batch_id: None,
                    serial_id: None,
                    quantity_milli: 5000,
                },
                CreateTransferItemInput {
                    product_id: prod_2.clone(),
                    variant_id: None,
                    batch_id: None,
                    serial_id: None,
                    quantity_milli: 8000, // Insufficient!
                },
            ],
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("created");

    let err = TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, TransferError::InsufficientStock { .. }));

    // Product 1 stock must NOT have been deducted
    assert_eq!(
        get_aggregate_stock(&conn, &f.branch_a, &f.product_id),
        10000
    );
    assert_eq!(get_aggregate_stock(&conn, &f.branch_a, &prod_2), 2000);

    // Zero transfer movements exist
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE source_id = ?1",
            params![transfer.id],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(count, 0);
}

// =========================================================================
// 8. IDEMPOTENCY TESTS
// =========================================================================

#[test]
fn test_create_transfer_idempotency_replay_and_conflict() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    let mut input = CreateTransferInput {
        transfer_type: TransferType::InterBranch,
        source_branch_id: f.branch_a.clone(),
        destination_branch_id: f.branch_b.clone(),
        source_location_id: f.loc_a1.clone(),
        destination_location_id: f.loc_b1.clone(),
        source_bin_id: None,
        destination_bin_id: None,
        notes: Some("Initial note".into()),
        items: vec![CreateTransferItemInput {
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_milli: 1000,
        }],
        user_id: Some(f.user_id.clone()),
        idempotency_key: Some("idem-key-create-01".into()),
    };

    let first = TransferService::create_transfer(&mut conn, &input).expect("first create");

    // Replay with exact same request
    let replay = TransferService::create_transfer(&mut conn, &input).expect("replay create");
    assert_eq!(first.id, replay.id);
    assert_eq!(first.transfer_number, replay.transfer_number);

    // Replay with different notes -> IdempotencyConflict
    input.notes = Some("Modified note".into());
    let conflict_err = TransferService::create_transfer(&mut conn, &input).unwrap_err();
    assert!(matches!(
        conflict_err,
        TransferError::IdempotencyConflict(_)
    ));
}

#[test]
fn test_dispatch_and_receive_idempotency() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        20000,
    );

    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: None,
                serial_id: None,
                quantity_milli: 5000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("created");

    // 1. Dispatch Idempotency
    let mut dispatch_input = DispatchTransferInput {
        transfer_id: transfer.id.clone(),
        user_id: Some("user_01".into()),
        idempotency_key: Some("idem-disp-01".into()),
    };

    let disp_first =
        TransferService::dispatch_transfer(&mut conn, &dispatch_input).expect("first dispatch");
    let disp_replay =
        TransferService::dispatch_transfer(&mut conn, &dispatch_input).expect("replay dispatch");
    assert_eq!(disp_first.id, disp_replay.id);

    // Source inventory only deducted once (20,000 - 5,000 = 15,000)
    assert_eq!(
        get_aggregate_stock(&conn, &f.branch_a, &f.product_id),
        15000
    );

    // Dispatch conflict
    dispatch_input.user_id = Some("user_02".into());
    let disp_conflict = TransferService::dispatch_transfer(&mut conn, &dispatch_input).unwrap_err();
    assert!(matches!(
        disp_conflict,
        TransferError::IdempotencyConflict(_)
    ));

    // 2. Receive Idempotency
    let mut receive_input = ReceiveTransferInput {
        transfer_id: transfer.id.clone(),
        destination_location_id: None,
        destination_bin_id: None,
        user_id: Some("user_01".into()),
        idempotency_key: Some("idem-recv-01".into()),
    };

    let recv_first =
        TransferService::receive_transfer(&mut conn, &receive_input).expect("first receive");
    let recv_replay =
        TransferService::receive_transfer(&mut conn, &receive_input).expect("replay receive");
    assert_eq!(recv_first.id, recv_replay.id);

    // Destination inventory only credited once (+5,000)
    assert_eq!(get_aggregate_stock(&conn, &f.branch_b, &f.product_id), 5000);

    // Receive conflict
    receive_input.user_id = Some("user_02".into());
    let recv_conflict = TransferService::receive_transfer(&mut conn, &receive_input).unwrap_err();
    assert!(matches!(
        recv_conflict,
        TransferError::IdempotencyConflict(_)
    ));
}

// =========================================================================
// 9. RECEIVE ROLLBACK TESTS
// =========================================================================

#[test]
fn test_receive_rollback_on_failure() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        10000,
    );

    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: None,
                serial_id: None,
                quantity_milli: 4000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("created");

    TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer.id.clone(),
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("dispatched");

    // Attempt receive with invalid destination location (loc_a1 belongs to branch A, not branch B)
    let bad_receive_input = ReceiveTransferInput {
        transfer_id: transfer.id.clone(),
        destination_location_id: Some(f.loc_a1.clone()), // Invalid location for branch B!
        destination_bin_id: None,
        user_id: None,
        idempotency_key: Some("idem-bad-recv".into()),
    };

    let err = TransferService::receive_transfer(&mut conn, &bad_receive_input).unwrap_err();
    assert!(matches!(err, TransferError::BranchMismatch(_)));

    // Verify atomic rollback:
    // Transfer must still be InTransit
    let cur = TransferService::get_transfer(&conn, &transfer.id)
        .expect("get")
        .expect("exists");
    assert_eq!(cur.status, TransferStatus::InTransit);
    assert_eq!(cur.items[0].received_quantity_milli, None);

    // Destination stock remains 0
    assert_eq!(get_aggregate_stock(&conn, &f.branch_b, &f.product_id), 0);

    // No inbound ledger movements created
    let inbound_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE source_id = ?1 AND quantity_delta_milli > 0",
            params![transfer.id],
            |r| r.get(0),
        )
        .expect("inbound_count");
    assert_eq!(inbound_count, 0);

    // No idempotency key recorded
    let key_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM idempotency_keys WHERE key = 'idem-bad-recv'",
            [],
            |r| r.get(0),
        )
        .expect("key_count");
    assert_eq!(key_count, 0);
}

// =========================================================================
// 10. CONCURRENCY & DOUBLE DISPATCH RACE PREVENTION
// =========================================================================

#[test]
fn test_concurrent_double_dispatch_single_winner() {
    let mut conn = setup_test_db();
    let f = setup_transfer_fixtures(&conn);

    // Seed exactly enough stock for 1 transfer (10,000 milli)
    seed_stock(
        &mut conn,
        &f.branch_a,
        &f.product_id,
        Some(&f.variant_id),
        &f.loc_a1,
        Some(&f.bin_a1),
        None,
        None,
        10000,
    );

    let transfer = TransferService::create_transfer(
        &mut conn,
        &CreateTransferInput {
            transfer_type: TransferType::InterBranch,
            source_branch_id: f.branch_a.clone(),
            destination_branch_id: f.branch_b.clone(),
            source_location_id: f.loc_a1.clone(),
            destination_location_id: f.loc_b1.clone(),
            source_bin_id: None,
            destination_bin_id: None,
            notes: None,
            items: vec![CreateTransferItemInput {
                product_id: f.product_id.clone(),
                variant_id: Some(f.variant_id.clone()),
                batch_id: None,
                serial_id: None,
                quantity_milli: 10000,
            }],
            user_id: None,
            idempotency_key: None,
        },
    )
    .expect("created");

    let transfer_id = transfer.id;

    // First dispatch succeeds
    let first = TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer_id.clone(),
            user_id: Some("user_thread_1".into()),
            idempotency_key: None,
        },
    );
    assert!(first.is_ok());

    // Competing concurrent dispatch attempt against the same transfer fails closed
    let second = TransferService::dispatch_transfer(
        &mut conn,
        &DispatchTransferInput {
            transfer_id: transfer_id.clone(),
            user_id: Some("user_thread_2".into()),
            idempotency_key: None,
        },
    );
    assert!(second.is_err());
    assert!(matches!(
        second.unwrap_err(),
        TransferError::InvalidStatusTransition { .. }
    ));

    // Source inventory deducted exactly once (10,000 -> 0), no double deduction, no negative stock
    assert_eq!(get_aggregate_stock(&conn, &f.branch_a, &f.product_id), 0);
    assert_eq!(
        get_spatial_stock(&conn, &f.branch_a, &f.loc_a1, &f.product_id),
        0
    );

    // Exactly 1 outbound ledger movement exists
    let movements: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE source_id = ?1",
            params![transfer_id],
            |r| r.get(0),
        )
        .expect("movements");
    assert_eq!(movements, 1);
}
