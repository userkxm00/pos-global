// Comprehensive test suite for F2.11 — Stock Movement Ledger & Spatial Inventory Architecture
// Covers ADR-0013: Migration 020, spatial balances, 8 partial unique indexes, immutability triggers,
// single write authority, fail-closed negative stock prevention, batch/serial linkage, idempotency,
// and legacy unallocated stock preservation.

use crate::commands::stock::{
    get_batch_summary_impl, get_product_spatial_balances_impl, get_stock_summary_impl,
    post_stock_movement_impl, PostMovementInput,
};
use crate::location::{create_bin, create_location, CreateBinInput, CreateLocationInput};
use crate::serial::{create_serial_instance, CreateSerialInput, SerialStatus};
use crate::stock::{MovementReason, PostMovementRequest, StockLedgerError, StockLedgerService};
use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
    setup_test_db_up_to,
};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

// =========================================================================
// TEST FIXTURE HELPERS
// =========================================================================

struct TestContext {
    conn: Connection,
    branch_id: String,
    branch_2_id: String,
    admin_session: String,
    admin_b2_session: String,
    cashier_session: String,
    product_id: String,
    location_id: String,
    bin_id: String,
}

fn setup_stock_test_context() -> TestContext {
    let conn = setup_test_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn);

    // Secondary branch for isolation tests
    let branch_2 = crate::branch::create_branch(
        &conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id,
            name: "Branch 2 Warehouse".to_string(),
            address: Some("789 Secondary St".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch 2 created");
    let branch_2_id = branch_2.id;

    // Admin user session with InventoryAdjust for Branch 1
    let admin_user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Admin User",
        Some("stock_admin"),
        Some("Password123!"),
        Some("1234"),
        "admin",
    )
    .expect("admin user created");
    let admin_session = crate::user::session::create_local_session(
        &conn,
        &admin_user.id,
        &branch_id,
        "password",
        None,
    )
    .expect("admin session created")
    .id;

    // Admin user session with InventoryAdjust for Branch 2 (for spatial cross-branch testing)
    let admin_b2_user = create_test_user_with_creds(
        &conn,
        &branch_2_id,
        "Admin B2 User",
        Some("stock_admin_b2"),
        Some("Password123!"),
        Some("5678"),
        "admin",
    )
    .expect("admin b2 user created");
    let admin_b2_session = crate::user::session::create_local_session(
        &conn,
        &admin_b2_user.id,
        &branch_2_id,
        "password",
        None,
    )
    .expect("admin b2 session created")
    .id;

    // Cashier user session (no InventoryAdjust)
    let cashier_user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier User",
        Some("stock_cashier"),
        Some("Password123!"),
        Some("4321"),
        "cashier",
    )
    .expect("cashier user created");
    let cashier_session = crate::user::session::create_local_session(
        &conn,
        &cashier_user.id,
        &branch_id,
        "password",
        None,
    )
    .expect("cashier session created")
    .id;

    // Test product
    let product_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO products (id, name, is_active, created_at, updated_at) VALUES (?1, 'Industrial Widget', 1, datetime('now'), datetime('now'))",
        params![product_id],
    )
    .expect("product created");

    // Test location and bin
    let loc = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "Main Storage Bay".to_string(),
            code: "BAY-01".to_string(),
            location_type: Some("warehouse_bay".to_string()),
        },
    )
    .expect("location created");
    let location_id = loc.id;

    let bin = create_bin(
        &conn,
        CreateBinInput {
            location_id: location_id.clone(),
            name: "Shelf Slot A".to_string(),
            code: "SLOT-A".to_string(),
        },
    )
    .expect("bin created");
    let bin_id = bin.id;

    TestContext {
        conn,
        branch_id,
        branch_2_id,
        admin_session,
        admin_b2_session,
        cashier_session,
        product_id,
        location_id,
        bin_id,
    }
}

// =========================================================================
// 1. MIGRATION 020 CUTOVER & LEGACY PRESERVATION
// =========================================================================

#[test]
fn test_migration_020_applies_cleanly_and_preserves_legacy_unallocated_stock() {
    // 1. Migrate only up to 019
    let conn = setup_test_db_up_to("019_locations_bins");
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    let product_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO products (id, name, is_active, created_at, updated_at) VALUES (?1, 'Legacy Widget', 1, datetime('now'), datetime('now'))",
        params![product_id],
    )
    .unwrap();

    // Insert legacy aggregate inventory (pre-020)
    let legacy_inv_id = Uuid::new_v4().to_string();
    let legacy_qty_milli = 12500i64; // 12.5 units
    conn.execute(
        "INSERT INTO inventory (id, branch_id, product_id, quantity, quantity_milli, updated_at)
         VALUES (?1, ?2, ?3, ?4 / 1000.0, ?4, datetime('now'))",
        params![legacy_inv_id, branch_id, product_id, legacy_qty_milli],
    )
    .unwrap();

    // Insert legacy batch (pre-020)
    let legacy_batch_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity_milli, status, expiry_date, received_at)
         VALUES (?1, ?2, ?3, 'LEGACY-LOT-99', 5000, 'active', '2028-12-31', datetime('now'))",
        params![legacy_batch_id, product_id, branch_id],
    )
    .unwrap();

    // Insert legacy serial (pre-020)
    let legacy_serial_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'SN-LEGACY-001', 'in_stock', datetime('now'), datetime('now'))",
        params![legacy_serial_id, product_id, branch_id],
    )
    .unwrap();

    // 2. Now apply Migration 020
    apply_migrations_up_to(&conn, "020_stock_ledger_and_spatial_balances");

    // 3. Verify legacy data is preserved without corruption
    let inv_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE id = ?1",
            params![legacy_inv_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(inv_qty, legacy_qty_milli);

    let batch_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM product_batches WHERE id = ?1",
            params![legacy_batch_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(batch_qty, 5000);

    let (serial_status, s_loc, s_bin): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![legacy_serial_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(serial_status, "in_stock");
    assert!(s_loc.is_none(), "Legacy serial must retain NULL location");
    assert!(s_bin.is_none(), "Legacy serial must retain NULL bin");

    // 4. Verify location_inventory is completely empty (no synthetic backfill)
    let loc_inv_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM location_inventory", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        loc_inv_count, 0,
        "Migration 020 must not backfill location_inventory"
    );

    // 5. Verify unallocated balance derivation
    let summary =
        StockLedgerService::get_stock_summary(&conn, &branch_id, &product_id, None).unwrap();
    assert_eq!(summary.total_quantity_milli, legacy_qty_milli);
    assert_eq!(summary.spatial_quantity_milli, 0);
    assert_eq!(summary.unallocated_quantity_milli, legacy_qty_milli);
}

// =========================================================================
// 2. IMMUTABILITY ENGINE TRIGGERS (UPDATE / DELETE ABORT)
// =========================================================================

#[test]
fn test_immutable_stock_movements_triggers_block_update_and_delete() {
    let mut ctx = setup_stock_test_context();

    let req = PostMovementRequest {
        idempotency_key: "immut-test-01".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let result = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap();

    // Direct UPDATE must be aborted by database trigger
    let update_res = ctx.conn.execute(
        "UPDATE stock_movements SET quantity_delta_milli = 99999 WHERE id = ?1",
        params![result.movement_id],
    );
    assert!(
        update_res.is_err(),
        "Trigger must block UPDATE on stock_movements"
    );
    let err_str = update_res.unwrap_err().to_string();
    assert!(err_str.contains("Historical stock_movements rows are immutable"));

    // Direct DELETE must be aborted by database trigger
    let delete_res = ctx.conn.execute(
        "DELETE FROM stock_movements WHERE id = ?1",
        params![result.movement_id],
    );
    assert!(
        delete_res.is_err(),
        "Trigger must block DELETE on stock_movements"
    );
    let err_str = delete_res.unwrap_err().to_string();
    assert!(err_str.contains("Historical stock_movements rows are immutable"));
}

// =========================================================================
// 3. REFERENTIAL & SPATIAL ENGINE TRIGGERS
// =========================================================================

#[test]
fn test_stock_movements_spatial_guard_triggers() {
    let ctx = setup_stock_test_context();

    // Trigger check 1: Bin specified without a location
    let err1 = ctx.conn.execute(
        "INSERT INTO stock_movements (id, branch_id, product_id, quantity_delta, quantity_delta_milli, reason, bin_id, created_at)
         VALUES ('mov_err_1', ?1, ?2, 1.0, 1000, 'manual', ?3, datetime('now'))",
        params![ctx.branch_id, ctx.product_id, ctx.bin_id],
    );
    assert!(err1.is_err());
    assert!(err1
        .unwrap_err()
        .to_string()
        .contains("Movement bin cannot be specified without a location"));

    // Trigger check 2: Location from another branch
    let loc_branch_2 = create_location(
        &ctx.conn,
        CreateLocationInput {
            branch_id: ctx.branch_2_id.clone(),
            parent_id: None,
            name: "Branch 2 Bay".to_string(),
            code: "B2-BAY".to_string(),
            location_type: None,
        },
    )
    .unwrap();

    let err2 = ctx.conn.execute(
        "INSERT INTO stock_movements (id, branch_id, product_id, quantity_delta, quantity_delta_milli, reason, location_id, created_at)
         VALUES ('mov_err_2', ?1, ?2, 1.0, 1000, 'manual', ?3, datetime('now'))",
        params![ctx.branch_id, ctx.product_id, loc_branch_2.id],
    );
    assert!(err2.is_err());
    assert!(err2
        .unwrap_err()
        .to_string()
        .contains("Movement location branch does not match movement branch"));

    // Trigger check 3: Bin does not belong to location
    let loc_another = create_location(
        &ctx.conn,
        CreateLocationInput {
            branch_id: ctx.branch_id.clone(),
            parent_id: None,
            name: "Other Bay".to_string(),
            code: "OTHER-BAY".to_string(),
            location_type: None,
        },
    )
    .unwrap();

    let err3 = ctx.conn.execute(
        "INSERT INTO stock_movements (id, branch_id, product_id, quantity_delta, quantity_delta_milli, reason, location_id, bin_id, created_at)
         VALUES ('mov_err_3', ?1, ?2, 1.0, 1000, 'manual', ?3, ?4, datetime('now'))",
        params![ctx.branch_id, ctx.product_id, loc_another.id, ctx.bin_id],
    );
    assert!(err3.is_err());
    assert!(err3
        .unwrap_err()
        .to_string()
        .contains("Movement bin does not belong to movement location"));

    // Trigger check 4: Zero delta rejected
    let err4 = ctx.conn.execute(
        "INSERT INTO stock_movements (id, branch_id, product_id, quantity_delta, quantity_delta_milli, reason, location_id, created_at)
         VALUES ('mov_err_4', ?1, ?2, 0.0, 0, 'manual', ?3, datetime('now'))",
        params![ctx.branch_id, ctx.product_id, ctx.location_id],
    );
    assert!(err4.is_err());
    assert!(err4
        .unwrap_err()
        .to_string()
        .contains("Stock movement quantity delta cannot be zero"));

    // Trigger check 5: after != before + delta rejected
    let err5 = ctx.conn.execute(
        "INSERT INTO stock_movements (id, branch_id, product_id, quantity_delta, quantity_delta_milli, quantity_before_milli, quantity_after_milli, reason, location_id, created_at)
         VALUES ('mov_err_5', ?1, ?2, 1.0, 1000, 500, 9999, 'manual', ?3, datetime('now'))",
        params![ctx.branch_id, ctx.product_id, ctx.location_id],
    );
    assert!(err5.is_err());
    assert!(err5
        .unwrap_err()
        .to_string()
        .contains("after quantity must equal before quantity plus delta"));
}

// =========================================================================
// 4. 8 MUTUALLY EXCLUSIVE PARTIAL UNIQUE INDEXES ON LOCATION_INVENTORY
// =========================================================================

#[test]
fn test_location_inventory_8_mutually_exclusive_partial_unique_indexes() {
    let ctx = setup_stock_test_context();

    // Create a variant
    let variant_id = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_variants (id, product_id, sku, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'SKU-VAR-1', 1, datetime('now'), datetime('now'))",
            params![variant_id, ctx.product_id],
        )
        .unwrap();

    // Create a batch
    let batch_id = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity_milli, status, expiry_date, received_at)
             VALUES (?1, ?2, ?3, 'LOT-INDEX-01', 1000, 'active', '2029-01-01', datetime('now'))",
            params![batch_id, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    // Helper macro to test uniqueness
    macro_rules! test_slot_collision {
        ($bin:expr, $var:expr, $batch:expr, $desc:expr) => {
            let id1 = Uuid::new_v4().to_string();
            let id2 = Uuid::new_v4().to_string();
            let res1 = ctx.conn.execute(
                "INSERT INTO location_inventory (id, branch_id, location_id, bin_id, product_id, variant_id, batch_id, quantity_milli)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1000)",
                params![id1, ctx.branch_id, ctx.location_id, $bin, ctx.product_id, $var, $batch],
            );
            assert!(res1.is_ok(), "First insert must succeed for {}", $desc);

            let res2 = ctx.conn.execute(
                "INSERT INTO location_inventory (id, branch_id, location_id, bin_id, product_id, variant_id, batch_id, quantity_milli)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 2000)",
                params![id2, ctx.branch_id, ctx.location_id, $bin, ctx.product_id, $var, $batch],
            );
            assert!(res2.is_err(), "Duplicate insert must fail unique index for {}", $desc);
        };
    }

    // Index 1: bin NULL, var NULL, batch NULL
    test_slot_collision!(
        None::<&str>,
        None::<&str>,
        None::<&str>,
        "u1 (NULL, NULL, NULL)"
    );

    // Index 2: bin NOT NULL, var NULL, batch NULL
    test_slot_collision!(
        Some(&ctx.bin_id),
        None::<&str>,
        None::<&str>,
        "u2 (bin, NULL, NULL)"
    );

    // Index 3: bin NULL, var NOT NULL, batch NULL
    test_slot_collision!(
        None::<&str>,
        Some(&variant_id),
        None::<&str>,
        "u3 (NULL, var, NULL)"
    );

    // Index 4: bin NULL, var NULL, batch NOT NULL
    test_slot_collision!(
        None::<&str>,
        None::<&str>,
        Some(&batch_id),
        "u4 (NULL, NULL, batch)"
    );

    // Index 5: bin NOT NULL, var NOT NULL, batch NULL
    test_slot_collision!(
        Some(&ctx.bin_id),
        Some(&variant_id),
        None::<&str>,
        "u5 (bin, var, NULL)"
    );

    // Index 6: bin NOT NULL, var NULL, batch NOT NULL
    test_slot_collision!(
        Some(&ctx.bin_id),
        None::<&str>,
        Some(&batch_id),
        "u6 (bin, NULL, batch)"
    );

    // Index 7: bin NULL, var NOT NULL, batch NOT NULL
    test_slot_collision!(
        None::<&str>,
        Some(&variant_id),
        Some(&batch_id),
        "u7 (NULL, var, batch)"
    );

    // Index 8: bin NOT NULL, var NOT NULL, batch NOT NULL
    test_slot_collision!(
        Some(&ctx.bin_id),
        Some(&variant_id),
        Some(&batch_id),
        "u8 (bin, var, batch)"
    );
}

// =========================================================================
// 5. OPENING BALANCE & UNALLOCATED PRESERVATION
// =========================================================================

#[test]
fn test_opening_balance_happy_path_and_unallocated_invariance() {
    let mut ctx = setup_stock_test_context();

    // Establish pre-existing unallocated inventory in `inventory`
    let legacy_unallocated = 4000i64;
    ctx.conn
        .execute(
            "INSERT INTO inventory (id, branch_id, product_id, quantity, quantity_milli, updated_at)
             VALUES ('legacy_inv', ?1, ?2, 4.0, ?3, datetime('now'))",
            params![ctx.branch_id, ctx.product_id, legacy_unallocated],
        )
        .unwrap();

    let initial_summary =
        StockLedgerService::get_stock_summary(&ctx.conn, &ctx.branch_id, &ctx.product_id, None)
            .unwrap();
    assert_eq!(initial_summary.total_quantity_milli, 4000);
    assert_eq!(initial_summary.spatial_quantity_milli, 0);
    assert_eq!(initial_summary.unallocated_quantity_milli, 4000);

    // Post a new opening balance to physical slot
    let delta = 6000i64;
    let req = PostMovementRequest {
        idempotency_key: "open_bal_01".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: delta,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let result = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap();
    assert_eq!(result.quantity_delta_milli, delta);
    assert_eq!(result.quantity_before_milli, 4000);
    assert_eq!(result.quantity_after_milli, 10000);
    assert_eq!(result.slot_before_milli, 0);
    assert_eq!(result.slot_after_milli, 6000);

    // Mathematical invariant proof:
    // aggregate += delta (4000 -> 10000)
    // spatial += delta (0 -> 6000)
    // unallocated remains STRICTLY unchanged at 4000!
    let final_summary =
        StockLedgerService::get_stock_summary(&ctx.conn, &ctx.branch_id, &ctx.product_id, None)
            .unwrap();
    assert_eq!(final_summary.total_quantity_milli, 10000);
    assert_eq!(final_summary.spatial_quantity_milli, 6000);
    assert_eq!(final_summary.unallocated_quantity_milli, 4000);
}

// =========================================================================
// 6. ADJUSTMENTS, DAMAGE, LOSS, AND FAIL-CLOSED NEGATIVE STOCK
// =========================================================================

#[test]
fn test_adjustments_damage_loss_and_negative_stock_prevention() {
    let mut ctx = setup_stock_test_context();

    // 1. Establish initial stock of 10,000 milli
    let req_init = PostMovementRequest {
        idempotency_key: "k_init".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_init).unwrap();

    // 2. Adjustment (+2000)
    let req_adj_pos = PostMovementRequest {
        idempotency_key: "k_adj_pos".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 2000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let res_adj = StockLedgerService::post_movement(&mut ctx.conn, &req_adj_pos).unwrap();
    assert_eq!(res_adj.quantity_after_milli, 12000);

    // 3. Damage (-1000)
    let req_damage = PostMovementRequest {
        idempotency_key: "k_damage".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    let res_dmg = StockLedgerService::post_movement(&mut ctx.conn, &req_damage).unwrap();
    assert_eq!(res_dmg.quantity_after_milli, 11000);

    // 4. Loss (-2000)
    let req_loss = PostMovementRequest {
        idempotency_key: "k_loss".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -2000,
        reason: MovementReason::Loss,
        user_id: None,
    };
    let res_loss = StockLedgerService::post_movement(&mut ctx.conn, &req_loss).unwrap();
    assert_eq!(res_loss.quantity_after_milli, 9000);

    // 5. Fail-Closed: Damage with positive delta rejected
    let req_dmg_invalid = PostMovementRequest {
        idempotency_key: "k_dmg_inv".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1000, // Invalid positive
        reason: MovementReason::Damage,
        user_id: None,
    };
    let err_dmg = StockLedgerService::post_movement(&mut ctx.conn, &req_dmg_invalid).unwrap_err();
    assert!(matches!(err_dmg, StockLedgerError::Validation(_)));

    // 6. Fail-Closed: Excessive deduction resulting in negative spatial balance
    let req_overdraft = PostMovementRequest {
        idempotency_key: "k_overdraft".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -20000, // exceeds 9,000 on hand
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err_over = StockLedgerService::post_movement(&mut ctx.conn, &req_overdraft).unwrap_err();
    assert!(matches!(
        err_over,
        StockLedgerError::NegativeStockBlocked(_)
    ));

    // Balance remains exactly 9,000
    let cur_qty = StockLedgerService::get_location_balance(
        &ctx.conn,
        &ctx.branch_id,
        &ctx.location_id,
        Some(&ctx.bin_id),
        &ctx.product_id,
        None,
        None,
    )
    .unwrap();
    assert_eq!(cur_qty, 9000);
}

// =========================================================================
// 7. NEGATIVE AGGREGATE REJECTION
// =========================================================================

#[test]
fn test_negative_aggregate_rejection() {
    let mut ctx = setup_stock_test_context();

    // 1. Initial 2000 aggregate, but slot has 2000
    let req_init = PostMovementRequest {
        idempotency_key: "k_agg_init".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 2000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_init).unwrap();

    // 2. Try deducting 5000: aggregate would become -3000
    let req_neg_agg = PostMovementRequest {
        idempotency_key: "k_neg_agg".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -5000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut ctx.conn, &req_neg_agg).unwrap_err();
    assert!(matches!(err, StockLedgerError::NegativeStockBlocked(_)));
}

// =========================================================================
// 8. NEGATIVE SPATIAL BALANCE REJECTION
// =========================================================================

#[test]
fn test_negative_spatial_balance_rejection() {
    let mut ctx = setup_stock_test_context();

    // Create a second location in the same branch
    let loc_b = create_location(
        &ctx.conn,
        CreateLocationInput {
            branch_id: ctx.branch_id.clone(),
            parent_id: None,
            name: "Secondary Bay".to_string(),
            code: "BAY-02".to_string(),
            location_type: None,
        },
    )
    .unwrap();

    // Put 10,000 into Location A
    let req_init = PostMovementRequest {
        idempotency_key: "k_spat_init".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_init).unwrap();

    // Aggregate inventory is 10,000. But Location B has 0!
    // Trying to deduct 1000 from Location B must fail because Location B spatial slot cannot be negative
    let req_neg_slot = PostMovementRequest {
        idempotency_key: "k_neg_slot".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: loc_b.id,
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -1000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut ctx.conn, &req_neg_slot).unwrap_err();
    assert!(matches!(err, StockLedgerError::NegativeStockBlocked(_)));
}

// =========================================================================
// 9. NEGATIVE BATCH BALANCE REJECTION
// =========================================================================

#[test]
fn test_negative_batch_balance_rejection() {
    let mut ctx = setup_stock_test_context();

    let batch_id = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity_milli, status, expiry_date, received_at)
             VALUES (?1, ?2, ?3, 'LOT-NEG-TEST', 1000, 'active', '2030-01-01', datetime('now'))",
            params![batch_id, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    // Put 1000 into slot with that batch
    let req_init = PostMovementRequest {
        idempotency_key: "k_batch_init_2".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(batch_id.clone()),
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_init).unwrap();

    // Now try to deduct 3000 from batch: batch has only 2000 total!
    let req_neg_b = PostMovementRequest {
        idempotency_key: "k_batch_over".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(batch_id),
        serial_id: None,
        quantity_delta_milli: -3000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut ctx.conn, &req_neg_b).unwrap_err();
    assert!(matches!(err, StockLedgerError::NegativeStockBlocked(_)));
}

// =========================================================================
// 10. ZERO DELTA REJECTION & MISSING LOCATION REJECTION
// =========================================================================

#[test]
fn test_zero_delta_rejection() {
    let mut ctx = setup_stock_test_context();

    let req = PostMovementRequest {
        idempotency_key: "k_zero_delta".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 0, // Zero delta
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap_err();
    assert_eq!(err, StockLedgerError::ZeroQuantityDelta);
}

#[test]
fn test_missing_location_rejection() {
    let mut ctx = setup_stock_test_context();

    let req = PostMovementRequest {
        idempotency_key: "k_missing_loc".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: "".to_string(), // Empty location
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap_err();
    assert_eq!(err, StockLedgerError::MissingLocation);
}

// =========================================================================
// 11. INVALID MOVEMENT REASON REJECTION
// =========================================================================

#[test]
fn test_invalid_movement_reason_rejection() {
    // Authorized F2.11 reasons: opening_balance, adjustment, damage, loss
    assert!(MovementReason::from_str("opening_balance").is_ok());
    assert!(MovementReason::from_str("adjustment").is_ok());
    assert!(MovementReason::from_str("damage").is_ok());
    assert!(MovementReason::from_str("loss").is_ok());

    // Unauthorized reasons (future phases) must be rejected
    assert!(MovementReason::from_str("sale").is_err());
    assert!(MovementReason::from_str("refund").is_err());
    assert!(MovementReason::from_str("transfer_in").is_err());
    assert!(MovementReason::from_str("transfer_out").is_err());
    assert!(MovementReason::from_str("count_reconciliation").is_err());
    assert!(MovementReason::from_str("random_junk").is_err());
}

// =========================================================================
// 12. ATOMIC ROLLBACK ON PARTIAL FAILURE
// =========================================================================

#[test]
fn test_atomic_rollback_on_partial_failure() {
    let mut ctx = setup_stock_test_context();

    // Establish initial state
    let req_init = PostMovementRequest {
        idempotency_key: "k_init_rollback".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_init).unwrap();

    let initial_movements_count: i64 = ctx
        .conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
        .unwrap();
    let initial_agg: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    let initial_slot: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    let initial_idemp_count: i64 = ctx
        .conn
        .query_row("SELECT COUNT(*) FROM idempotency_keys", [], |row| {
            row.get(0)
        })
        .unwrap();

    // Attempt a transaction that fails during validation/invariants (e.g. negative stock)
    let req_fail = PostMovementRequest {
        idempotency_key: "k_fail_rollback".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -10000, // Insufficient: fails check
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut ctx.conn, &req_fail).unwrap_err();
    assert!(matches!(err, StockLedgerError::NegativeStockBlocked(_)));

    // Verify complete atomic rollback: absolutely nothing changed!
    let final_movements_count: i64 = ctx
        .conn
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
        .unwrap();
    let final_agg: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    let final_slot: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    let final_idemp_count: i64 = ctx
        .conn
        .query_row("SELECT COUNT(*) FROM idempotency_keys", [], |row| {
            row.get(0)
        })
        .unwrap();

    assert_eq!(initial_movements_count, final_movements_count);
    assert_eq!(initial_agg, final_agg);
    assert_eq!(initial_slot, final_slot);
    assert_eq!(initial_idemp_count, final_idemp_count);
}

// =========================================================================
// 13. IDEMPOTENT REPLAY & CONFLICT DETECTION
// =========================================================================

#[test]
fn test_idempotent_replay_with_same_key_and_hash() {
    let mut ctx = setup_stock_test_context();

    let req = PostMovementRequest {
        idempotency_key: "idemp_k_replay".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 3000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let res1 = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap();
    let res2 = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap();
    assert_eq!(res1, res2);

    let count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "Replay must not duplicate stock_movement rows");
}

#[test]
fn test_idempotency_conflict_with_same_key_and_different_hash() {
    let mut ctx = setup_stock_test_context();

    let req = PostMovementRequest {
        idempotency_key: "idemp_k_conflict".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 3000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap();

    let mut req_altered = req.clone();
    req_altered.quantity_delta_milli = 9000; // Altered payload

    let err = StockLedgerService::post_movement(&mut ctx.conn, &req_altered).unwrap_err();
    assert!(matches!(err, StockLedgerError::IdempotencyConflict(_)));
}

// =========================================================================
// 14. SERIALIZED ASSET INVARIANTS
// =========================================================================

#[test]
fn test_serialized_opening_balance_requires_exactly_1000_milli_and_location() {
    let mut ctx = setup_stock_test_context();

    let serial_id = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'SN-UNIT-XYZ', 'reserved', datetime('now'), datetime('now'))",
            params![serial_id, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    // 1. Non-1000 delta rejected
    let req_bad_delta = PostMovementRequest {
        idempotency_key: "k_ser_delta_bad".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial_id.clone()),
        quantity_delta_milli: 500, // Invalid: must be 1000
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut ctx.conn, &req_bad_delta).unwrap_err();
    assert!(matches!(err, StockLedgerError::SerialInvalidQuantity(_)));

    // 2. Exactly 1000 delta with location succeeds
    let req_ok = PostMovementRequest {
        idempotency_key: "k_ser_delta_ok".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial_id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let res = StockLedgerService::post_movement(&mut ctx.conn, &req_ok).unwrap();
    assert_eq!(res.quantity_delta_milli, 1000);

    // 3. Serial now in_stock cannot be re-added via opening balance
    let req_dup = PostMovementRequest {
        idempotency_key: "k_ser_dup".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial_id),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err_dup = StockLedgerService::post_movement(&mut ctx.conn, &req_dup).unwrap_err();
    assert!(matches!(err_dup, StockLedgerError::SerialInvalidStatus(_)));
}

// =========================================================================
// 15. BRANCH ISOLATION & PERMISSION ENFORCEMENT
// =========================================================================

#[test]
fn test_branch_isolation_and_permission_enforcement() {
    let mut ctx = setup_stock_test_context();

    let input = PostMovementInput {
        idempotency_key: "ipc_branch_iso_01".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: "opening_balance".to_string(),
    };

    // 1. Cashier lacks InventoryAdjust permission -> rejected
    let cashier_err = post_stock_movement_impl(&mut ctx.conn, &ctx.cashier_session, input.clone());
    assert!(cashier_err.is_err());
    assert!(cashier_err.unwrap_err().contains("inventory.adjust"));

    // 2. Admin has InventoryAdjust for Branch 1 -> succeeds
    let admin_ok = post_stock_movement_impl(&mut ctx.conn, &ctx.admin_session, input);
    assert!(admin_ok.is_ok());

    // 3. Attempting to post movement in Branch 2 using Location from Branch 1 -> rejected by spatial guard
    let input_cross_branch = PostMovementInput {
        idempotency_key: "ipc_cross_01".to_string(),
        branch_id: ctx.branch_2_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(), // Location from Branch 1!
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: "opening_balance".to_string(),
    };
    let cross_err =
        post_stock_movement_impl(&mut ctx.conn, &ctx.admin_b2_session, input_cross_branch);
    assert!(cross_err.is_err());
    let err_msg = cross_err.unwrap_err();
    assert!(
        err_msg.contains("does not belong to branch")
            || err_msg.contains("Location branch mismatch")
            || err_msg.contains("Movement location branch does not match movement branch"),
        "Expected location branch mismatch error, got: {err_msg}"
    );

    // 4. Query spatial balances via IPC impl
    let balances = get_product_spatial_balances_impl(
        &ctx.conn,
        &ctx.admin_session,
        &ctx.branch_id,
        &ctx.product_id,
    )
    .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].quantity_milli, 1000);

    // Cross-branch read with Branch 1 session attempting to read Branch 2 is rejected
    let cross_read_err = get_stock_summary_impl(
        &ctx.conn,
        &ctx.admin_session,
        &ctx.branch_2_id,
        &ctx.product_id,
        None,
    )
    .unwrap_err();
    assert!(cross_read_err.contains("Scope mismatch"));

    // Summary for Branch 2 with Branch 2 session is 0
    let b2_summary = get_stock_summary_impl(
        &ctx.conn,
        &ctx.admin_b2_session,
        &ctx.branch_2_id,
        &ctx.product_id,
        None,
    )
    .unwrap();
    assert_eq!(b2_summary.total_quantity_milli, 0);
    assert_eq!(b2_summary.spatial_quantity_milli, 0);

    // 5. Query batch summary via IPC impl (verifies get_batch_summary_impl)
    let b_id = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity_milli, status, expiry_date, received_at)
             VALUES (?1, ?2, ?3, 'IPC-BATCH-01', 5000, 'active', '2030-01-01', datetime('now'))",
            params![b_id, ctx.product_id, ctx.branch_id],
        )
        .unwrap();
    let batch_sum =
        get_batch_summary_impl(&ctx.conn, &ctx.admin_session, &ctx.branch_id, &b_id).unwrap();
    assert_eq!(batch_sum.total_quantity_milli, 5000);
    assert_eq!(batch_sum.unallocated_quantity_milli, 5000);
}

// =========================================================================
// 16. BATCH VARIANT CONSISTENCY (NULL-SAFE CASES)
// =========================================================================

#[test]
fn test_batch_variant_consistency_all_four_null_safe_cases() {
    let mut ctx = setup_stock_test_context();

    let var_a = Uuid::new_v4().to_string();
    let var_b = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_variants (id, product_id, sku, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'SKU-VAR-A', 1, datetime('now'), datetime('now'))",
            params![var_a, ctx.product_id],
        )
        .unwrap();
    ctx.conn
        .execute(
            "INSERT INTO product_variants (id, product_id, sku, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'SKU-VAR-B', 1, datetime('now'), datetime('now'))",
            params![var_b, ctx.product_id],
        )
        .unwrap();

    let batch_null = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_batches (id, product_id, branch_id, variant_id, batch_number, quantity_milli, status, expiry_date, received_at)
             VALUES (?1, ?2, ?3, NULL, 'LOT-NULL-VAR', 10000, 'active', '2030-01-01', datetime('now'))",
            params![batch_null, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    let batch_a = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_batches (id, product_id, branch_id, variant_id, batch_number, quantity_milli, status, expiry_date, received_at)
             VALUES (?1, ?2, ?3, ?4, 'LOT-VAR-A', 10000, 'active', '2030-01-01', datetime('now'))",
            params![batch_a, ctx.product_id, ctx.branch_id, var_a],
        )
        .unwrap();

    // Case 1: Both variant NULL => succeeds
    let req_null_null = PostMovementRequest {
        idempotency_key: "k_batch_v_null_null".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(batch_null.clone()),
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let res1 = StockLedgerService::post_movement(&mut ctx.conn, &req_null_null);
    assert!(res1.is_ok());

    // Case 2: Both variant equal (Variant A == Variant A) => succeeds
    let req_a_a = PostMovementRequest {
        idempotency_key: "k_batch_v_a_a".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: Some(var_a.clone()),
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(batch_a.clone()),
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let res2 = StockLedgerService::post_movement(&mut ctx.conn, &req_a_a);
    assert!(res2.is_ok());

    // Case 3: Batch has Variant A, request has NULL variant => rejected
    let req_a_null = PostMovementRequest {
        idempotency_key: "k_batch_v_a_null".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(batch_a.clone()),
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err3 = StockLedgerService::post_movement(&mut ctx.conn, &req_a_null).unwrap_err();
    assert!(matches!(err3, StockLedgerError::BatchMismatch(_)));

    // Case 4: Batch has NULL variant, request has Variant A => rejected
    let req_null_a = PostMovementRequest {
        idempotency_key: "k_batch_v_null_a".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: Some(var_a.clone()),
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(batch_null.clone()),
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err4 = StockLedgerService::post_movement(&mut ctx.conn, &req_null_a).unwrap_err();
    assert!(matches!(err4, StockLedgerError::BatchMismatch(_)));

    // Case 5: Batch has Variant A, request has Variant B => rejected
    let req_a_b = PostMovementRequest {
        idempotency_key: "k_batch_v_a_b".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: Some(var_b.clone()),
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(batch_a),
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err5 = StockLedgerService::post_movement(&mut ctx.conn, &req_a_b).unwrap_err();
    assert!(matches!(err5, StockLedgerError::BatchMismatch(_)));

    // Verify Variant B has 0 balance (no leaked mutation)
    let var_b_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT COALESCE(quantity_milli, 0) FROM inventory WHERE product_id = ?1 AND variant_id = ?2",
            params![ctx.product_id, var_b],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
        .unwrap_or(0);
    assert_eq!(var_b_qty, 0);
}

// =========================================================================
// 17. SERIAL VARIANT CONSISTENCY
// =========================================================================

#[test]
fn test_serial_variant_consistency_rejection() {
    let mut ctx = setup_stock_test_context();

    let var_a = Uuid::new_v4().to_string();
    let var_b = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_variants (id, product_id, sku, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'SKU-SVAR-A', 1, datetime('now'), datetime('now'))",
            params![var_a, ctx.product_id],
        )
        .unwrap();
    ctx.conn
        .execute(
            "INSERT INTO product_variants (id, product_id, sku, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'SKU-SVAR-B', 1, datetime('now'), datetime('now'))",
            params![var_b, ctx.product_id],
        )
        .unwrap();

    let serial_id = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO serial_numbers (id, product_id, branch_id, variant_id, serial_number, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'SN-VAR-001', 'reserved', datetime('now'), datetime('now'))",
            params![serial_id, ctx.product_id, ctx.branch_id, var_a],
        )
        .unwrap();

    // Attempt to post movement for Variant B using Serial for Variant A
    let req_mismatch = PostMovementRequest {
        idempotency_key: "k_ser_var_mismatch".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: Some(var_b.clone()),
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: Some(serial_id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut ctx.conn, &req_mismatch).unwrap_err();
    assert!(matches!(err, StockLedgerError::SerialMismatch(_)));

    // Verify neither aggregate nor spatial balance was mutated for Variant B
    let agg_b: i64 = ctx
        .conn
        .query_row(
            "SELECT COALESCE(quantity_milli, 0) FROM inventory WHERE product_id = ?1 AND variant_id = ?2",
            params![ctx.product_id, var_b],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
        .unwrap_or(0);
    assert_eq!(agg_b, 0);

    let spat_b: i64 = ctx
        .conn
        .query_row(
            "SELECT COALESCE(SUM(quantity_milli), 0) FROM location_inventory WHERE product_id = ?1 AND variant_id = ?2",
            params![ctx.product_id, var_b],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(spat_b, 0);

    // Serial status remains unchanged
    let s_status: String = ctx
        .conn
        .query_row(
            "SELECT status FROM serial_numbers WHERE id = ?1",
            params![serial_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(s_status, "reserved");
}

// =========================================================================
// 18. SERIAL BIN CONSISTENCY
// =========================================================================

#[test]
fn test_serial_bin_consistency_cases() {
    let mut ctx = setup_stock_test_context();

    // Create a second bin in the same location
    let bin_b = create_bin(
        &ctx.conn,
        CreateBinInput {
            location_id: ctx.location_id.clone(),
            name: "Shelf Slot B".to_string(),
            code: "SLOT-B".to_string(),
        },
    )
    .expect("bin b created");

    // 1. Establish serial in Bin A
    let serial_id = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'SN-BIN-TEST-1', 'reserved', datetime('now'), datetime('now'))",
            params![serial_id, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    let req_open = PostMovementRequest {
        idempotency_key: "k_bin_open_a".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()), // Bin A
        batch_id: None,
        serial_id: Some(serial_id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_open).unwrap();

    // 2. Try deducting serial from Bin B (mismatch: physically in Bin A!)
    let req_deduct_bad_bin = PostMovementRequest {
        idempotency_key: "k_bin_deduct_b".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(bin_b.id.clone()), // Mismatched Bin B!
        batch_id: None,
        serial_id: Some(serial_id.clone()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    let err_bin =
        StockLedgerService::post_movement(&mut ctx.conn, &req_deduct_bad_bin).unwrap_err();
    assert!(matches!(
        err_bin,
        StockLedgerError::SerialCoordinateMismatch(_)
    ));

    // Balances remain intact: Bin A has 1000, Bin B has 0
    let bal_a: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE bin_id = ?1",
            params![ctx.bin_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(bal_a, 1000);

    // 3. Deduct serial from correct Bin A -> succeeds
    let req_deduct_ok = PostMovementRequest {
        idempotency_key: "k_bin_deduct_a_ok".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()), // Correct Bin A
        batch_id: None,
        serial_id: Some(serial_id.clone()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    let res_ok = StockLedgerService::post_movement(&mut ctx.conn, &req_deduct_ok);
    assert!(res_ok.is_ok());

    // Serial is now defective and coordinates cleared
    let (s_stat, s_loc, s_bin): (String, Option<String>, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![serial_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(s_stat, "defective");
    assert!(s_loc.is_none());
    assert!(s_bin.is_none());

    // 4. Serial with NULL bin in location -> deducting with NULL bin succeeds
    let serial_null_bin = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'SN-BIN-NULL-1', 'reserved', datetime('now'), datetime('now'))",
            params![serial_null_bin, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    let req_open_null_bin = PostMovementRequest {
        idempotency_key: "k_bin_open_null".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None, // NULL bin
        batch_id: None,
        serial_id: Some(serial_null_bin.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_open_null_bin).unwrap();

    let req_deduct_null_bin = PostMovementRequest {
        idempotency_key: "k_bin_deduct_null".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None, // NULL bin
        batch_id: None,
        serial_id: Some(serial_null_bin),
        quantity_delta_milli: -1000,
        reason: MovementReason::Loss,
        user_id: None,
    };
    let res_null_bin = StockLedgerService::post_movement(&mut ctx.conn, &req_deduct_null_bin);
    assert!(res_null_bin.is_ok());
}

// =========================================================================
// 19. BATCH STATUS PRESERVATION & DEPLETION
// =========================================================================

#[test]
fn test_batch_status_preservation_and_depletion() {
    let mut ctx = setup_stock_test_context();

    // 1. Recalled batch: negative deduction preserves 'recalled' status
    let b_recalled = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity_milli, status, expiry_date, received_at)
             VALUES (?1, ?2, ?3, 'LOT-REC-01', 5000, 'recalled', '2030-01-01', datetime('now'))",
            params![b_recalled, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    // Seed location and aggregate inventory for that batch so negative movement can occur
    ctx.conn
        .execute(
            "INSERT INTO inventory (id, branch_id, product_id, quantity, quantity_milli, updated_at)
             VALUES ('inv_rec', ?1, ?2, 5.0, 5000, datetime('now'))",
            params![ctx.branch_id, ctx.product_id],
        )
        .unwrap();
    ctx.conn
        .execute(
            "INSERT INTO location_inventory (id, branch_id, location_id, product_id, batch_id, quantity_milli)
             VALUES ('loc_rec', ?1, ?2, ?3, ?4, 5000)",
            params![ctx.branch_id, ctx.location_id, ctx.product_id, b_recalled],
        )
        .unwrap();

    let req_rec_deduct = PostMovementRequest {
        idempotency_key: "k_rec_deduct".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(b_recalled.clone()),
        serial_id: None,
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_rec_deduct).unwrap();

    let (rec_qty, rec_stat): (i64, String) = ctx
        .conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![b_recalled],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(rec_qty, 4000);
    assert_eq!(rec_stat, "recalled", "Recalled status must be preserved");

    // 2. Quarantined batch: negative deduction preserves 'quarantined' status
    let b_quarantine = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity_milli, status, expiry_date, received_at)
             VALUES (?1, ?2, ?3, 'LOT-QUAR-01', 3000, 'quarantined', '2030-01-01', datetime('now'))",
            params![b_quarantine, ctx.product_id, ctx.branch_id],
        )
        .unwrap();
    ctx.conn
        .execute(
            "UPDATE inventory SET quantity_milli = quantity_milli + 3000 WHERE id = 'inv_rec'",
            [],
        )
        .unwrap();
    ctx.conn
        .execute(
            "INSERT INTO location_inventory (id, branch_id, location_id, product_id, batch_id, quantity_milli)
             VALUES ('loc_quar', ?1, ?2, ?3, ?4, 3000)",
            params![ctx.branch_id, ctx.location_id, ctx.product_id, b_quarantine],
        )
        .unwrap();

    let req_quar_deduct = PostMovementRequest {
        idempotency_key: "k_quar_deduct".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(b_quarantine.clone()),
        serial_id: None,
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_quar_deduct).unwrap();

    let (quar_qty, quar_stat): (i64, String) = ctx
        .conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![b_quarantine],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(quar_qty, 2000);
    assert_eq!(
        quar_stat, "quarantined",
        "Quarantined status must be preserved"
    );

    // 3. Batch reaching zero quantity transitions to 'depleted'
    let req_quar_deplete = PostMovementRequest {
        idempotency_key: "k_quar_deplete".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(b_quarantine.clone()),
        serial_id: None,
        quantity_delta_milli: -2000, // consumes remaining 2000
        reason: MovementReason::Loss,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_quar_deplete).unwrap();

    let (dep_qty, dep_stat): (i64, String) = ctx
        .conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![b_quarantine],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(dep_qty, 0);
    assert_eq!(
        dep_stat, "depleted",
        "Zero quantity must transition to depleted"
    );

    // 4. Normal active batch remains active when deducted, and revives depleted when added
    let b_active = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity_milli, status, expiry_date, received_at)
             VALUES (?1, ?2, ?3, 'LOT-ACT-01', 0, 'depleted', '2030-01-01', datetime('now'))",
            params![b_active, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    let req_revive = PostMovementRequest {
        idempotency_key: "k_dep_revive".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: Some(b_active.clone()),
        serial_id: None,
        quantity_delta_milli: 2500,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &req_revive).unwrap();

    let (act_qty, act_stat): (i64, String) = ctx
        .conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![b_active],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(act_qty, 2500);
    assert_eq!(
        act_stat, "active",
        "Depleted batch revives to active on positive movement"
    );
}

// =========================================================================
// 20. SERIAL TERMINAL STATUS REVIVAL BLOCKED
// =========================================================================

#[test]
fn test_serial_terminal_status_revival_blocked() {
    let mut ctx = setup_stock_test_context();

    for (status, name) in &[
        ("disposed", "SN-TERM-DISPOSED"),
        ("recalled", "SN-TERM-RECALLED"),
        ("sold", "SN-TERM-SOLD"),
        ("transferred", "SN-TERM-TRANSFERRED"),
    ] {
        let s_id = Uuid::new_v4().to_string();
        ctx.conn
            .execute(
                "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))",
                params![s_id, ctx.product_id, ctx.branch_id, name, status],
            )
            .unwrap();

        let req = PostMovementRequest {
            idempotency_key: format!("k_term_revive_{status}"),
            branch_id: ctx.branch_id.clone(),
            product_id: ctx.product_id.clone(),
            variant_id: None,
            location_id: ctx.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: Some(s_id.clone()),
            quantity_delta_milli: 1000,
            reason: MovementReason::OpeningBalance,
            user_id: None,
        };

        let err = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap_err();
        assert!(
            matches!(err, StockLedgerError::SerialInvalidStatus(_)),
            "Status {status} must be rejected from revival"
        );

        // Verify status remains unchanged
        let cur_stat: String = ctx
            .conn
            .query_row(
                "SELECT status FROM serial_numbers WHERE id = ?1",
                params![s_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cur_stat, *status);
    }

    // Verify aggregate stock remains 0
    let agg_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT COALESCE(quantity_milli, 0) FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .optional()
        .unwrap()
        .unwrap_or(0);
    assert_eq!(agg_qty, 0);
}

#[test]
fn test_whitespace_serial_positive_movement_fails_closed_without_side_effects() {
    let mut ctx = setup_stock_test_context();

    let req = PostMovementRequest {
        idempotency_key: "k_whitespace_serial_pos".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: Some("   \t  ".to_string()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap_err();
    assert!(
        matches!(err, StockLedgerError::Validation(ref msg) if msg.contains("serial_id cannot be whitespace-only")),
        "Expected validation error for whitespace serial, got: {err:?}"
    );

    // Atomicity check: verify NO table was modified
    let inv_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(inv_count, 0, "inventory must not be created");

    let loc_inv_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM location_inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(loc_inv_count, 0, "location_inventory must not be created");

    let mov_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(mov_count, 0, "stock_movements must not record movement");

    let idem_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM idempotency_keys WHERE key = ?1",
            params!["k_whitespace_serial_pos"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        idem_count, 0,
        "idempotency_keys must not be recorded on validation failure"
    );

    let sn_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM serial_numbers WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sn_count, 0, "serial_numbers must not be mutated");
}

#[test]
fn test_whitespace_serial_negative_movement_fails_closed_without_side_effects() {
    let mut ctx = setup_stock_test_context();

    // First establish genuine initial baseline stock via valid opening balance
    let open_req = PostMovementRequest {
        idempotency_key: "k_base_open_for_neg_ws".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &open_req).expect("baseline stock posted");

    // Attempt negative movement with whitespace serial_id
    let req = PostMovementRequest {
        idempotency_key: "k_whitespace_serial_neg".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: Some(" \n  ".to_string()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap_err();
    assert!(
        matches!(err, StockLedgerError::Validation(ref msg) if msg.contains("serial_id cannot be whitespace-only")),
        "Expected validation error for whitespace serial, got: {err:?}"
    );

    // Atomicity check: inventory must remain exactly at baseline 5000
    let inv_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(inv_qty, 5000, "inventory must remain unchanged at 5000");

    let loc_inv_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE product_id = ?1 AND location_id = ?2",
            params![ctx.product_id, ctx.location_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        loc_inv_qty, 5000,
        "location_inventory must remain unchanged at 5000"
    );

    let mov_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        mov_count, 1,
        "only the opening baseline movement should exist"
    );

    let idem_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM idempotency_keys WHERE key = ?1",
            params!["k_whitespace_serial_neg"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        idem_count, 0,
        "failed request idempotency key must not be saved"
    );
}

#[test]
fn test_valid_serial_positive_and_negative_movement_lifecycle_succeeds() {
    let mut ctx = setup_stock_test_context();

    let s_id = Uuid::new_v4().to_string();
    ctx.conn
        .execute(
            "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'SN-VALID-LIFECYCLE', 'reserved', datetime('now'), datetime('now'))",
            params![s_id, ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    // 1. Positive serialized movement (+1000)
    let pos_req = PostMovementRequest {
        idempotency_key: "k_valid_serial_pos".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: Some(s_id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let pos_res = StockLedgerService::post_movement(&mut ctx.conn, &pos_req)
        .expect("valid serial positive succeeds");
    assert_eq!(pos_res.serial_id, Some(s_id.clone()));

    // Verify serial status is now 'in_stock' at location
    let (stat_after_pos, loc_after_pos): (String, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id FROM serial_numbers WHERE id = ?1",
            params![s_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(stat_after_pos, "in_stock");
    assert_eq!(loc_after_pos, Some(ctx.location_id.clone()));

    // 2. Negative serialized movement (-1000) with Damage
    let neg_req = PostMovementRequest {
        idempotency_key: "k_valid_serial_neg".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: Some(s_id.clone()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    let neg_res = StockLedgerService::post_movement(&mut ctx.conn, &neg_req)
        .expect("valid serial negative succeeds");
    assert_eq!(neg_res.serial_id, Some(s_id.clone()));

    // Verify serial status transitioned to 'defective' with location cleared
    let (stat_after_neg, loc_after_neg): (String, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id FROM serial_numbers WHERE id = ?1",
            params![s_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(stat_after_neg, "defective");
    assert_eq!(loc_after_neg, None);
}

#[test]
fn test_ordinary_non_serialized_movement_succeeds_with_none_serial() {
    let mut ctx = setup_stock_test_context();

    let req = PostMovementRequest {
        idempotency_key: "k_none_serial_pos".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 2500,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let res =
        StockLedgerService::post_movement(&mut ctx.conn, &req).expect("non-serialized succeeds");
    assert_eq!(res.serial_id, None);
    assert_eq!(res.quantity_after_milli, 2500);

    // Verify movement recorded with NULL serial_id
    let saved_serial: Option<String> = ctx
        .conn
        .query_row(
            "SELECT serial_id FROM stock_movements WHERE id = ?1",
            params![res.movement_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(saved_serial, None);
}

#[test]
fn test_whitespace_optional_identifiers_fail_closed() {
    let mut ctx = setup_stock_test_context();

    for (field, var, bin, batch) in [
        ("variant_id", Some("   ".to_string()), None, None),
        ("bin_id", None, Some("\t ".to_string()), None),
        ("batch_id", None, None, Some("  \n".to_string())),
    ] {
        let req = PostMovementRequest {
            idempotency_key: format!("k_ws_opt_{field}"),
            branch_id: ctx.branch_id.clone(),
            product_id: ctx.product_id.clone(),
            variant_id: var,
            location_id: ctx.location_id.clone(),
            bin_id: bin,
            batch_id: batch,
            serial_id: None,
            quantity_delta_milli: 1000,
            reason: MovementReason::OpeningBalance,
            user_id: None,
        };
        let err = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap_err();
        assert!(
            matches!(err, StockLedgerError::Validation(ref msg) if msg.contains(&format!("{field} cannot be whitespace-only"))),
            "Expected validation error for {field}, got: {err:?}"
        );
    }
}

#[test]
fn test_serial_creation_reconciliation_and_stock_ledger_intake_atomicity() {
    let mut ctx = setup_stock_test_context();

    // Mark product as serial-tracked so serial creation is authorized
    ctx.conn
        .execute(
            "UPDATE products SET requires_serial = 1 WHERE id = ?1",
            params![ctx.product_id],
        )
        .unwrap();

    // 1. Serial Creation (F2.08) creates instance in 'reserved' pre-stock state
    let serial_input = CreateSerialInput {
        product_id: ctx.product_id.clone(),
        branch_id: ctx.branch_id.clone(),
        variant_id: None,
        serial_number: Some("SN-RECON-001".into()),
        imei: None,
        asset_tag: None,
        cost_price_minor: Some(150000),
    };
    let instance = create_serial_instance(&ctx.conn, &serial_input)
        .expect("serial creation must succeed in pre-stock state");

    assert_eq!(instance.status, SerialStatus::Reserved);
    let (init_loc, init_bin): (Option<String>, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![instance.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(init_loc, None);
    assert_eq!(init_bin, None);

    // Verify creation alone cannot create stock-on-hand unintentionally
    let inv_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        inv_count, 0,
        "No inventory row may exist from serial registration alone"
    );

    let loc_inv_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM location_inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        loc_inv_count, 0,
        "No spatial inventory row may exist from serial registration alone"
    );

    let movements_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        movements_count, 0,
        "No stock movements may exist from serial registration alone"
    );

    // 2. Stock Entry (F2.11) via StockLedgerService::post_movement inducts unit into stock
    let intake_req = PostMovementRequest {
        idempotency_key: "k_serial_recon_intake_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(instance.id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let intake_res = StockLedgerService::post_movement(&mut ctx.conn, &intake_req)
        .expect("Stock intake through ledger must succeed");
    assert_eq!(intake_res.serial_id, Some(instance.id.clone()));
    assert_eq!(intake_res.quantity_after_milli, 1000);

    // Verify atomic updates: serial becomes in_stock with assigned coordinates
    let (stat, loc, bin): (String, Option<String>, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![instance.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(stat, "in_stock");
    assert_eq!(loc, Some(ctx.location_id.clone()));
    assert_eq!(bin, Some(ctx.bin_id.clone()));

    // Aggregate inventory balance is exactly 1000
    let agg_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(agg_qty, 1000);

    // Spatial inventory balance is exactly 1000
    let loc_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE product_id = ?1 AND location_id = ?2 AND bin_id = ?3",
            params![ctx.product_id, ctx.location_id, ctx.bin_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(loc_qty, 1000);

    // Exactly one movement row in stock_movements
    let mov_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mov_count, 1);

    // 3. Double-intake prevention: Attempting to intake the same serial again must fail
    let duplicate_intake_req = PostMovementRequest {
        idempotency_key: "k_serial_recon_intake_dup".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(instance.id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let dup_err =
        StockLedgerService::post_movement(&mut ctx.conn, &duplicate_intake_req).unwrap_err();
    assert!(
        matches!(dup_err, StockLedgerError::SerialInvalidStatus(ref msg) if msg.contains("already in_stock")),
        "Expected SerialInvalidStatus, got: {dup_err:?}"
    );

    // 4. Failure / Rollback atomicity test:
    // Create another serial in 'reserved' state
    let serial_input_2 = CreateSerialInput {
        product_id: ctx.product_id.clone(),
        branch_id: ctx.branch_id.clone(),
        variant_id: None,
        serial_number: Some("SN-RECON-FAIL-002".into()),
        imei: None,
        asset_tag: None,
        cost_price_minor: None,
    };
    let instance_2 = create_serial_instance(&ctx.conn, &serial_input_2)
        .expect("second serial creation succeeds");
    assert_eq!(instance_2.status, SerialStatus::Reserved);

    // Create an inactive bin to trigger validation error during transaction
    let inactive_bin_id = crate::location::create_bin(
        &ctx.conn,
        CreateBinInput {
            location_id: ctx.location_id.clone(),
            code: "BIN-INACTIVE-RECON".into(),
            name: "Inactive Bin".into(),
        },
    )
    .unwrap()
    .id;
    ctx.conn
        .execute(
            "UPDATE bins SET is_active = 0 WHERE id = ?1",
            params![inactive_bin_id],
        )
        .unwrap();

    let fail_req = PostMovementRequest {
        idempotency_key: "k_serial_recon_fail_tx".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(inactive_bin_id),
        batch_id: None,
        serial_id: Some(instance_2.id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let fail_err = StockLedgerService::post_movement(&mut ctx.conn, &fail_req).unwrap_err();
    assert!(matches!(fail_err, StockLedgerError::BinInactive(_)));

    // Verify full rollback: serial remains 'reserved', no coordinates assigned
    let (stat_2, loc_2, bin_2): (String, Option<String>, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![instance_2.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(stat_2, "reserved");
    assert_eq!(loc_2, None);
    assert_eq!(bin_2, None);

    // Balances remained unchanged from previous successful intake (1000 aggregate, 1 movement)
    let final_agg: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(final_agg, 1000);

    let final_mov: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(final_mov, 1);

    let idem_check: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM idempotency_keys WHERE key = 'k_serial_recon_fail_tx'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        idem_check, 0,
        "Idempotency key for rolled back transaction must not exist"
    );
}

#[test]
fn test_serial_reversible_adjustment_roundtrip() {
    let mut ctx = setup_stock_test_context();

    // Mark product as serial-tracked
    ctx.conn
        .execute(
            "UPDATE products SET requires_serial = 1 WHERE id = ?1",
            params![ctx.product_id],
        )
        .unwrap();

    // 1. Create serial in Reserved state
    let serial_input = CreateSerialInput {
        product_id: ctx.product_id.clone(),
        branch_id: ctx.branch_id.clone(),
        variant_id: None,
        serial_number: Some("SN-REVERSIBLE-001".into()),
        imei: None,
        asset_tag: None,
        cost_price_minor: Some(100000),
    };
    let instance =
        create_serial_instance(&ctx.conn, &serial_input).expect("create serial instance");
    assert_eq!(instance.status, SerialStatus::Reserved);

    // 2. Initial Intake (+1000 OpeningBalance) -> in_stock
    let intake_req = PostMovementRequest {
        idempotency_key: "k_rev_intake_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(instance.id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &intake_req).expect("intake succeeds");

    // Stage 1 Verification: in_stock with coordinates, 1000 aggregate, 1000 spatial, 1 movement
    let (stat_1, loc_1, bin_1): (String, Option<String>, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![instance.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(stat_1, "in_stock");
    assert_eq!(loc_1, Some(ctx.location_id.clone()));
    assert_eq!(bin_1, Some(ctx.bin_id.clone()));

    let agg_1: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(agg_1, 1000);

    let spat_1: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE product_id = ?1 AND location_id = ?2",
            params![ctx.product_id, ctx.location_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(spat_1, 1000);

    let mov_1: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mov_1, 1);

    // 3. Negative Adjustment (-1000 Adjustment) -> serial becomes 'reserved', coordinates cleared
    let neg_adj_req = PostMovementRequest {
        idempotency_key: "k_rev_neg_adj_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(instance.id.clone()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &neg_adj_req)
        .expect("negative adjustment succeeds");

    // Stage 2 Verification: serial is reserved with NULL coordinates, balances 0, 2 movements
    let (stat_2, loc_2, bin_2): (String, Option<String>, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![instance.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        stat_2, "reserved",
        "Negative adjustment must transition serial to reserved"
    );
    assert_eq!(loc_2, None, "Negative adjustment must clear location_id");
    assert_eq!(bin_2, None, "Negative adjustment must clear bin_id");

    let agg_2: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(agg_2, 0);

    let spat_2: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE product_id = ?1 AND location_id = ?2",
            params![ctx.product_id, ctx.location_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(spat_2, 0);

    let mov_2: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mov_2, 2);

    // 4. Positive Adjustment (+1000 Adjustment) -> serial restored to 'in_stock', coordinates assigned
    let pos_adj_req = PostMovementRequest {
        idempotency_key: "k_rev_pos_adj_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(instance.id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &pos_adj_req)
        .expect("positive adjustment succeeds");

    // Stage 3 Verification: serial is restored to in_stock, coordinates reinstated, balances 1000, 3 movements
    let (stat_3, loc_3, bin_3): (String, Option<String>, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![instance.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        stat_3, "in_stock",
        "Positive adjustment must restore serial to in_stock"
    );
    assert_eq!(
        loc_3,
        Some(ctx.location_id.clone()),
        "Location must be reinstated"
    );
    assert_eq!(bin_3, Some(ctx.bin_id.clone()), "Bin must be reinstated");

    let agg_3: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(agg_3, 1000);

    let spat_3: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE product_id = ?1 AND location_id = ?2",
            params![ctx.product_id, ctx.location_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(spat_3, 1000);

    let mov_3: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mov_3, 3);

    // 5. Verification: Loss is NOT reversible
    let loss_req = PostMovementRequest {
        idempotency_key: "k_rev_loss_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(instance.id.clone()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Loss,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &loss_req).expect("loss succeeds");

    let stat_loss: String = ctx
        .conn
        .query_row(
            "SELECT status FROM serial_numbers WHERE id = ?1",
            params![instance.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        stat_loss, "disposed",
        "Loss must transition serial to disposed"
    );

    // Attempting to revive a disposed serial via +1000 Adjustment MUST FAIL
    let revive_loss_req = PostMovementRequest {
        idempotency_key: "k_rev_loss_attempt_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(instance.id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut ctx.conn, &revive_loss_req).unwrap_err();
    assert!(
        matches!(err, StockLedgerError::SerialInvalidStatus(ref msg) if msg.contains("terminal or non-revivable status 'disposed'")),
        "Loss must not be reversible, got error: {err:?}"
    );

    // Serial remains disposed and movements count remains 4
    let stat_after_failed_revive: String = ctx
        .conn
        .query_row(
            "SELECT status FROM serial_numbers WHERE id = ?1",
            params![instance.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stat_after_failed_revive, "disposed");

    let mov_final: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mov_final, 4);
}

#[test]
fn test_inactive_variant_movement_rejected_fail_closed() {
    let mut ctx = setup_stock_test_context();

    // 1. Create a variant for the product
    let variant_id = "var-test-inactive-001";
    ctx.conn
        .execute(
            "INSERT INTO product_variants (id, product_id, sku, barcode, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'SKU-INACT-001', 'BAR-INACT-001', 1, datetime('now'), datetime('now'))",
            params![variant_id, ctx.product_id],
        )
        .unwrap();

    // Deactivate the variant
    ctx.conn
        .execute(
            "UPDATE product_variants SET is_active = 0 WHERE id = ?1",
            params![variant_id],
        )
        .unwrap();

    // 2. Attempt positive movement with inactive variant
    let inact_req = PostMovementRequest {
        idempotency_key: "k_inact_var_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: Some(variant_id.to_string()),
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut ctx.conn, &inact_req).unwrap_err();
    match err {
        StockLedgerError::Validation(ref msg) => {
            assert!(
                msg.contains("inactive and cannot accept stock movements"),
                "Expected inactive variant validation error, got: {msg}"
            );
        }
        other => panic!("Expected StockLedgerError::Validation, got: {other:?}"),
    }

    // 3. Strict Fail-Closed Verification:
    // - no inventory mutation
    // - no spatial mutation
    // - no batch mutation
    // - no serial mutation
    // - no stock movement
    // - no successful idempotency record
    let inv_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM inventory WHERE product_id = ?1 AND variant_id = ?2",
            params![ctx.product_id, variant_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        inv_count, 0,
        "No inventory row must exist for inactive variant"
    );

    let loc_inv_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM location_inventory WHERE product_id = ?1 AND variant_id = ?2",
            params![ctx.product_id, variant_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        loc_inv_count, 0,
        "No spatial row must exist for inactive variant"
    );

    let batch_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM product_batches WHERE variant_id = ?1",
            params![variant_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        batch_count, 0,
        "No batch row must exist for inactive variant"
    );

    let serial_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM serial_numbers WHERE variant_id = ?1",
            params![variant_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serial_count, 0,
        "No serial row must exist for inactive variant"
    );

    let mov_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE variant_id = ?1",
            params![variant_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        mov_count, 0,
        "No stock movement must exist for inactive variant"
    );

    let idemp_count: i64 = ctx
        .conn
        .query_row(
            "SELECT count(*) FROM idempotency_keys WHERE key = 'k_inact_var_001'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        idemp_count, 0,
        "No successful idempotency key must exist for inactive variant"
    );

    // 4. Reactivate variant and confirm active-variant behavior is preserved
    ctx.conn
        .execute(
            "UPDATE product_variants SET is_active = 1 WHERE id = ?1",
            params![variant_id],
        )
        .unwrap();

    let active_req = PostMovementRequest {
        idempotency_key: "k_act_var_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: Some(variant_id.to_string()),
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let active_res = StockLedgerService::post_movement(&mut ctx.conn, &active_req)
        .expect("Active variant movement must succeed");
    assert_eq!(active_res.quantity_after_milli, 5000);

    let active_agg: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1 AND variant_id = ?2",
            params![ctx.product_id, variant_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(active_agg, 5000);
}

#[test]
fn test_batch_positive_stock_intake_via_ledger_roundtrip() {
    let mut ctx = setup_stock_test_context();

    // Enable batch tracking on product
    ctx.conn
        .execute(
            "UPDATE products SET requires_expiry = 1 WHERE id = ?1",
            params![ctx.product_id],
        )
        .unwrap();

    // 1. Create batch using production create_batch API (initializes with quantity 0, 'depleted')
    let batch = crate::batch::create_batch(
        &ctx.conn,
        &crate::batch::CreateBatchInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            batch_number: "BATCH-LEDGER-001".into(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2099-12-31".into()),
        },
    )
    .expect("batch creation with 0 quantity must succeed");

    assert_eq!(batch.quantity_milli, 0);
    assert_eq!(batch.status, crate::batch::BatchStatus::Depleted);

    // 2. Perform positive stock intake exclusively via StockLedgerService::post_movement
    let intake_req = PostMovementRequest {
        idempotency_key: "k_batch_intake_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: Some(batch.id.clone()),
        serial_id: None,
        quantity_delta_milli: 10000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let result = StockLedgerService::post_movement(&mut ctx.conn, &intake_req)
        .expect("Stock intake for batch must succeed");
    assert_eq!(result.quantity_after_milli, 10000);

    // 3. Verify batch quantity and status updated to 'active'
    let (batch_qty, batch_status): (i64, String) = ctx
        .conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(batch_qty, 10000);
    assert_eq!(batch_status, "active");

    // 4. Verify aggregate inventory
    let agg_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE product_id = ?1 AND branch_id = ?2",
            params![ctx.product_id, ctx.branch_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(agg_qty, 10000);

    // 5. Verify spatial inventory
    let spatial_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE product_id = ?1 AND location_id = ?2 AND bin_id = ?3 AND batch_id = ?4",
            params![ctx.product_id, ctx.location_id, ctx.bin_id, batch.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(spatial_qty, 10000);

    // 6. Verify stock movement row
    let mov_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE batch_id = ?1 AND quantity_delta_milli = 10000",
            params![batch.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mov_count, 1);
}

#[test]
fn test_batch_stock_intake_failed_no_partial_writes() {
    let mut ctx = setup_stock_test_context();

    ctx.conn
        .execute(
            "UPDATE products SET requires_expiry = 1 WHERE id = ?1",
            params![ctx.product_id],
        )
        .unwrap();

    let batch = crate::batch::create_batch(
        &ctx.conn,
        &crate::batch::CreateBatchInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            batch_number: "BATCH-FAIL-001".into(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2099-12-31".into()),
        },
    )
    .expect("batch creation with 0 quantity must succeed");

    // Location belongs to Branch 2, but request specifies Branch 1 -> location mismatch
    let loc_b2 = create_location(
        &ctx.conn,
        CreateLocationInput {
            branch_id: ctx.branch_2_id.clone(),
            parent_id: None,
            name: "Branch 2 Bay".to_string(),
            code: "BAY-B2".to_string(),
            location_type: Some("warehouse_bay".to_string()),
        },
    )
    .expect("b2 location created");

    let fail_req = PostMovementRequest {
        idempotency_key: "k_batch_fail_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: loc_b2.id,
        bin_id: None,
        batch_id: Some(batch.id.clone()),
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut ctx.conn, &fail_req).unwrap_err();
    assert!(matches!(err, StockLedgerError::LocationBranchMismatch(_)));

    // Batch quantity remains 0 and status remains 'depleted'
    let (batch_qty, batch_status): (i64, String) = ctx
        .conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(batch_qty, 0);
    assert_eq!(batch_status, "depleted");

    // No movements or inventory created
    let mov_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE batch_id = ?1",
            params![batch.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mov_count, 0);
}

#[test]
fn test_legacy_unallocated_serial_boundary_and_rejection() {
    let mut ctx = setup_stock_test_context();

    // Insert historical pre-020 serial row directly: in_stock with NULL location and bin
    ctx.conn
        .execute(
            "INSERT INTO serial_numbers (
                id, product_id, branch_id, serial_number, status, location_id, bin_id, created_at, updated_at
            ) VALUES (
                'leg-sn-001', ?1, ?2, 'HISTORICAL-SN-999', 'in_stock', NULL, NULL, datetime('now', '-30 days'), datetime('now', '-30 days')
            )",
            params![ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    // 1. Attempting spatial deduction via StockLedgerService fails fail-closed
    let deduct_req = PostMovementRequest {
        idempotency_key: "k_legacy_deduct_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some("leg-sn-001".to_string()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };

    let err_ledger = StockLedgerService::post_movement(&mut ctx.conn, &deduct_req).unwrap_err();
    assert!(
        matches!(err_ledger, StockLedgerError::SerialCoordinateMismatch(ref msg) if msg.contains("Serial has no physical location assigned")),
        "Deduction of unallocated serial must fail with SerialCoordinateMismatch: {err_ledger:?}"
    );

    // 2. Direct outbound status mutation via update_serial_status fails fail-closed
    let err_status = crate::serial::update_serial_status(
        &ctx.conn,
        &crate::serial::UpdateSerialStatusInput {
            id: "leg-sn-001".to_string(),
            branch_id: ctx.branch_id.clone(),
            status: SerialStatus::Sold,
        },
    )
    .unwrap_err();
    assert!(
        matches!(err_status, crate::serial::SerialError::Validation(ref msg) if msg.contains("Direct transition from 'in_stock' is prohibited")),
        "Direct outbound status transition must be rejected: {err_status:?}"
    );

    // 3. Verify serial remains completely uncorrupted in historical state
    let (status, loc, bin): (String, Option<String>, Option<String>) = ctx
        .conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = 'leg-sn-001'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(status, "in_stock");
    assert!(loc.is_none(), "Legacy serial must retain NULL location");
    assert!(bin.is_none(), "Legacy serial must retain NULL bin");

    // 4. Verify zero movements were fabricated
    let mov_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE serial_id = 'leg-sn-001'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        mov_count, 0,
        "No stock movement must be fabricated for legacy serial"
    );
}

// =========================================================================
// 27. REMEDIATION REGRESSION TESTS (PR #79)
// =========================================================================

#[test]
fn test_focused_idempotency_regression_concurrency_and_replay() {
    let mut ctx = setup_stock_test_context();

    let req = PostMovementRequest {
        idempotency_key: "idemp_reg_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 2000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    // 1. First execution succeeds and mutates balances
    let res1 = StockLedgerService::post_movement(&mut ctx.conn, &req)
        .expect("Initial movement must succeed");
    assert_eq!(res1.quantity_delta_milli, 2000);
    assert_eq!(res1.quantity_before_milli, 0);
    assert_eq!(res1.quantity_after_milli, 2000);

    // 2. Same key + same canonical request => replayed existing result
    let res2 = StockLedgerService::post_movement(&mut ctx.conn, &req).expect("Replay must succeed");
    assert_eq!(res1, res2, "Replay must return identical cached result");

    // 3. Verify no duplicate stock movement is created (exactly 1 exists)
    let movement_count: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        movement_count, 1,
        "Must create exactly 1 stock movement row"
    );

    // 4. Verify no duplicate balance mutation occurs
    let agg_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE branch_id = ?1 AND product_id = ?2",
            params![ctx.branch_id, ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        agg_qty, 2000,
        "Aggregate balance must remain exactly 2000 milli"
    );

    let slot_qty: i64 = ctx
        .conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE location_id = ?1 AND bin_id = ?2",
            params![ctx.location_id, ctx.bin_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        slot_qty, 2000,
        "Spatial slot balance must remain exactly 2000 milli"
    );

    // 5. Same key + different canonical request => IdempotencyConflict
    let mut req_conflict = req.clone();
    req_conflict.quantity_delta_milli = 5000;
    let err_conflict = StockLedgerService::post_movement(&mut ctx.conn, &req_conflict).unwrap_err();
    assert!(
        matches!(err_conflict, StockLedgerError::IdempotencyConflict(_)),
        "Expected IdempotencyConflict, got: {err_conflict:?}"
    );

    // Verify balance and movement count remained completely unchanged after conflict
    let movement_count_after: i64 = ctx
        .conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?1",
            params![ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        movement_count_after, 1,
        "Conflict must not create new movement"
    );

    // 6. Deterministic concurrency test: two competing requests with same key on Mutex<Connection>
    // Simulate concurrent IPC requests arriving at DbState
    let conn_arc = std::sync::Arc::new(std::sync::Mutex::new(ctx.conn));
    let c1 = std::sync::Arc::clone(&conn_arc);
    let c2 = std::sync::Arc::clone(&conn_arc);

    let concurrent_req = PostMovementInput {
        idempotency_key: "idemp_concurrent_race_001".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: "opening_balance".to_string(),
    };

    let req_t1 = concurrent_req.clone();
    let session_t1 = ctx.admin_session.clone();
    let handle1 = std::thread::spawn(move || {
        let mut conn_guard = c1.lock().unwrap();
        post_stock_movement_impl(&mut conn_guard, &session_t1, req_t1)
    });

    let req_t2 = concurrent_req;
    let session_t2 = ctx.admin_session.clone();
    let handle2 = std::thread::spawn(move || {
        let mut conn_guard = c2.lock().unwrap();
        post_stock_movement_impl(&mut conn_guard, &session_t2, req_t2)
    });

    let res_t1 = handle1
        .join()
        .unwrap()
        .expect("Thread 1 should succeed or replay");
    let res_t2 = handle2
        .join()
        .unwrap()
        .expect("Thread 2 should succeed or replay");

    assert_eq!(
        res_t1.movement_id, res_t2.movement_id,
        "Both concurrent requests must return the exact same movement_id (one executed, one replayed)"
    );

    let final_conn = conn_arc.lock().unwrap();
    let final_mov_count: i64 = final_conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE id = ?1",
            params![res_t1.movement_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        final_mov_count, 1,
        "Exactly one movement row must be committed across competing concurrent requests"
    );

    let final_agg: i64 = final_conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE branch_id = ?1 AND product_id = ?2",
            params![ctx.branch_id, ctx.product_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        final_agg, 3000,
        "Aggregate balance must increase by exactly 1000 (from 2000 to 3000), not double-mutated"
    );
}

#[test]
fn test_focused_serial_coordinate_mismatch_regression() {
    let mut ctx = setup_stock_test_context();

    // Secondary location in Branch 1
    let loc_alt = create_location(
        &ctx.conn,
        CreateLocationInput {
            branch_id: ctx.branch_id.clone(),
            parent_id: None,
            name: "Alternate Location".to_string(),
            code: "ALT-LOC".to_string(),
            location_type: Some("staging".to_string()),
        },
    )
    .expect("alt location created");

    let bin_alt = create_bin(
        &ctx.conn,
        CreateBinInput {
            location_id: loc_alt.id.clone(),
            name: "Alt Bin".to_string(),
            code: "ALT-BIN".to_string(),
        },
    )
    .expect("alt bin created");

    // Register serial instance in reserved status
    let s_inst = create_serial_instance(
        &ctx.conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("FOCUSED-SN-12345".to_string()),
            imei: None,
            asset_tag: None,
            cost_price_minor: Some(15000),
        },
    )
    .expect("serial created in reserved status");

    // Intake serial into Location 1, Bin 1 via StockLedgerService
    let intake_req = PostMovementRequest {
        idempotency_key: "k_serial_focus_intake".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(s_inst.id.clone()),
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut ctx.conn, &intake_req).expect("Serial intake succeeds");

    // 1. Deduction specifying wrong location -> SerialCoordinateMismatch
    let wrong_loc_req = PostMovementRequest {
        idempotency_key: "k_serial_wrong_loc".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: loc_alt.id.clone(), // Wrong location!
        bin_id: Some(bin_alt.id.clone()),
        batch_id: None,
        serial_id: Some(s_inst.id.clone()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    let err_loc = StockLedgerService::post_movement(&mut ctx.conn, &wrong_loc_req).unwrap_err();
    assert!(
        matches!(err_loc, StockLedgerError::SerialCoordinateMismatch(ref msg) if msg.contains("physically located at location")),
        "Expected SerialCoordinateMismatch for location mismatch, got: {err_loc:?}"
    );

    // 2. Deduction specifying correct location but wrong bin -> SerialCoordinateMismatch
    let wrong_bin_req = PostMovementRequest {
        idempotency_key: "k_serial_wrong_bin".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(), // Correct location
        bin_id: None,                         // Mismatched Bin (was in bin_id)
        batch_id: None,
        serial_id: Some(s_inst.id.clone()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    let err_bin = StockLedgerService::post_movement(&mut ctx.conn, &wrong_bin_req).unwrap_err();
    assert!(
        matches!(err_bin, StockLedgerError::SerialCoordinateMismatch(ref msg) if msg.contains("physically located at bin")),
        "Expected SerialCoordinateMismatch for bin mismatch, got: {err_bin:?}"
    );

    // 3. Unallocated legacy serial with NULL coordinates -> SerialCoordinateMismatch("Serial has no physical location assigned")
    ctx.conn
        .execute(
            "INSERT INTO serial_numbers (
                id, product_id, branch_id, serial_number, status, location_id, bin_id, created_at, updated_at
            ) VALUES (
                'sn-unallocated-999', ?1, ?2, 'HIST-UNALLOC-999', 'in_stock', NULL, NULL, datetime('now'), datetime('now')
            )",
            params![ctx.product_id, ctx.branch_id],
        )
        .unwrap();

    let unalloc_req = PostMovementRequest {
        idempotency_key: "k_unalloc_deduct".to_string(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some("sn-unallocated-999".to_string()),
        quantity_delta_milli: -1000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err_unalloc = StockLedgerService::post_movement(&mut ctx.conn, &unalloc_req).unwrap_err();
    assert_eq!(
        err_unalloc,
        StockLedgerError::SerialCoordinateMismatch(
            "Serial has no physical location assigned".to_string()
        ),
        "Missing coordinates must yield exact SerialCoordinateMismatch message"
    );

    // 4. Genuine Location-vs-Branch mismatch must still return LocationBranchMismatch
    let loc_b2 = create_location(
        &ctx.conn,
        CreateLocationInput {
            branch_id: ctx.branch_2_id.clone(),
            parent_id: None,
            name: "Branch 2 Location".to_string(),
            code: "B2-LOC".to_string(),
            location_type: Some("warehouse_bay".to_string()),
        },
    )
    .expect("b2 location created");

    let branch_mismatch_req = PostMovementRequest {
        idempotency_key: "k_branch_mismatch".to_string(),
        branch_id: ctx.branch_id.clone(), // Branch 1
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: loc_b2.id.clone(), // Location belonging to Branch 2!
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err_branch =
        StockLedgerService::post_movement(&mut ctx.conn, &branch_mismatch_req).unwrap_err();
    assert!(
        matches!(err_branch, StockLedgerError::LocationBranchMismatch(_)),
        "Location-vs-branch mismatch must still return LocationBranchMismatch, got: {err_branch:?}"
    );
}
