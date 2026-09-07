// Comprehensive test suite for F2.11 — Stock Movement Ledger & Spatial Inventory Architecture
// Covers ADR-0013: Migration 020, spatial balances, 8 partial unique indexes, immutability triggers,
// single write authority, fail-closed negative stock prevention, batch/serial linkage, idempotency,
// and legacy unallocated stock preservation.

use crate::commands::stock::{
    get_batch_summary_impl, get_product_spatial_balances_impl, get_stock_summary_impl,
    post_stock_movement_impl, PostMovementInput,
};
use crate::location::{create_bin, create_location, CreateBinInput, CreateLocationInput};
use crate::stock::{
    MovementReason, PostMovementRequest, StockLedgerError, StockLedgerService,
};
use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
    setup_test_db_up_to,
};
use rusqlite::{params, Connection};
use uuid::Uuid;

// =========================================================================
// TEST FIXTURE HELPERS
// =========================================================================

struct TestContext {
    conn: Connection,
    org_id: String,
    branch_id: String,
    branch_2_id: String,
    admin_session: String,
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
            organization_id: org_id.clone(),
            name: "Branch 2 Warehouse".to_string(),
            address: Some("789 Secondary St".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch 2 created");
    let branch_2_id = branch_2.id;

    // Admin user session with InventoryAdjust
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
    let admin_session = crate::user::session::create_local_session(&conn, &admin_user.id)
        .expect("admin session created")
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
    let cashier_session = crate::user::session::create_local_session(&conn, &cashier_user.id)
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
        org_id,
        branch_id,
        branch_2_id,
        admin_session,
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
    assert_eq!(loc_inv_count, 0, "Migration 020 must not backfill location_inventory");

    // 5. Verify unallocated balance derivation
    let summary = StockLedgerService::get_stock_summary(&conn, &branch_id, &product_id, None).unwrap();
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
        notes: None,
    };

    let result = StockLedgerService::post_movement(&mut ctx.conn, &req).unwrap();

    // Direct UPDATE must be aborted by database trigger
    let update_res = ctx.conn.execute(
        "UPDATE stock_movements SET quantity_delta_milli = 99999 WHERE id = ?1",
        params![result.movement_id],
    );
    assert!(update_res.is_err(), "Trigger must block UPDATE on stock_movements");
    let err_str = update_res.unwrap_err().to_string();
    assert!(err_str.contains("Historical stock_movements rows are immutable"));

    // Direct DELETE must be aborted by database trigger
    let delete_res = ctx.conn.execute(
        "DELETE FROM stock_movements WHERE id = ?1",
        params![result.movement_id],
    );
    assert!(delete_res.is_err(), "Trigger must block DELETE on stock_movements");
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
    assert!(err1.unwrap_err().to_string().contains("Movement bin cannot be specified without a location"));

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
    assert!(err2.unwrap_err().to_string().contains("Movement location branch does not match movement branch"));

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
    assert!(err3.unwrap_err().to_string().contains("Movement bin does not belong to movement location"));

    // Trigger check 4: Zero delta rejected
    let err4 = ctx.conn.execute(
        "INSERT INTO stock_movements (id, branch_id, product_id, quantity_delta, quantity_delta_milli, reason, location_id, created_at)
         VALUES ('mov_err_4', ?1, ?2, 0.0, 0, 'manual', ?3, datetime('now'))",
        params![ctx.branch_id, ctx.product_id, ctx.location_id],
    );
    assert!(err4.is_err());
    assert!(err4.unwrap_err().to_string().contains("Stock movement quantity delta cannot be zero"));

    // Trigger check 5: after != before + delta rejected
    let err5 = ctx.conn.execute(
        "INSERT INTO stock_movements (id, branch_id, product_id, quantity_delta, quantity_delta_milli, quantity_before_milli, quantity_after_milli, reason, location_id, created_at)
         VALUES ('mov_err_5', ?1, ?2, 1.0, 1000, 500, 9999, 'manual', ?3, datetime('now'))",
        params![ctx.branch_id, ctx.product_id, ctx.location_id],
    );
    assert!(err5.is_err());
    assert!(err5.unwrap_err().to_string().contains("after quantity must equal before quantity plus delta"));
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
            "INSERT INTO product_variants (id, product_id, sku, name, created_at, updated_at)
             VALUES (?1, ?2, 'SKU-VAR-1', 'Large Variant', datetime('now'), datetime('now'))",
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
    test_slot_collision!(None::<&str>, None::<&str>, None::<&str>, "u1 (NULL, NULL, NULL)");

    // Index 2: bin NOT NULL, var NULL, batch NULL
    test_slot_collision!(Some(&ctx.bin_id), None::<&str>, None::<&str>, "u2 (bin, NULL, NULL)");

    // Index 3: bin NULL, var NOT NULL, batch NULL
    test_slot_collision!(None::<&str>, Some(&variant_id), None::<&str>, "u3 (NULL, var, NULL)");

    // Index 4: bin NULL, var NULL, batch NOT NULL
    test_slot_collision!(None::<&str>, None::<&str>, Some(&batch_id), "u4 (NULL, NULL, batch)");

    // Index 5: bin NOT NULL, var NOT NULL, batch NULL
    test_slot_collision!(Some(&ctx.bin_id), Some(&variant_id), None::<&str>, "u5 (bin, var, NULL)");

    // Index 6: bin NOT NULL, var NULL, batch NOT NULL
    test_slot_collision!(Some(&ctx.bin_id), None::<&str>, Some(&batch_id), "u6 (bin, NULL, batch)");

    // Index 7: bin NULL, var NOT NULL, batch NOT NULL
    test_slot_collision!(None::<&str>, Some(&variant_id), Some(&batch_id), "u7 (NULL, var, batch)");

    // Index 8: bin NOT NULL, var NOT NULL, batch NOT NULL
    test_slot_collision!(Some(&ctx.bin_id), Some(&variant_id), Some(&batch_id), "u8 (bin, var, batch)");
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
        StockLedgerService::get_stock_summary(&ctx.conn, &ctx.branch_id, &ctx.product_id, None).unwrap();
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
        notes: Some("Initial slot establishment".to_string()),
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
        StockLedgerService::get_stock_summary(&ctx.conn, &ctx.branch_id, &ctx.product_id, None).unwrap();
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
    };
    let err_over = StockLedgerService::post_movement(&mut ctx.conn, &req_overdraft).unwrap_err();
    assert!(matches!(err_over, StockLedgerError::NegativeStockBlocked(_)));

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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        .query_row("SELECT COUNT(*) FROM idempotency_keys", [], |row| row.get(0))
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
        notes: None,
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
        .query_row("SELECT COUNT(*) FROM idempotency_keys", [], |row| row.get(0))
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
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
        notes: None,
    };

    // 1. Cashier lacks InventoryAdjust permission -> rejected
    let cashier_err = post_stock_movement_impl(&mut ctx.conn, &ctx.cashier_session, input.clone());
    assert!(cashier_err.is_err());
    assert!(cashier_err.unwrap_err().contains("inventory.adjust"));

    // 2. Admin has InventoryAdjust for Branch 1 -> succeeds
    let admin_ok = post_stock_movement_impl(&mut ctx.conn, &ctx.admin_session, input);
    assert!(admin_ok.is_ok());

    // 3. Attempting to post movement in Branch 2 using Location from Branch 1 -> rejected
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
        notes: None,
    };
    let cross_err = post_stock_movement_impl(&mut ctx.conn, &ctx.admin_session, input_cross_branch);
    assert!(cross_err.is_err());

    // 4. Query spatial balances via IPC impl
    let balances =
        get_product_spatial_balances_impl(&ctx.conn, &ctx.admin_session, &ctx.branch_id, &ctx.product_id)
            .unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].quantity_milli, 1000);

    // Summary for Branch 2 is 0
    let b2_summary =
        get_stock_summary_impl(&ctx.conn, &ctx.admin_session, &ctx.branch_2_id, &ctx.product_id, None)
            .unwrap();
    assert_eq!(b2_summary.total_quantity_milli, 0);
    assert_eq!(b2_summary.spatial_quantity_milli, 0);
}
