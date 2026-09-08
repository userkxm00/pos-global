// F2.11 — Stock Movement Ledger & Spatial Balances Acceptance Test Suite
// ADR-0013: Spatial balance tracking, immutable ledger, single write authority.

use crate::batch::{create_batch, BatchStatus, CreateBatchInput};
use crate::commands::stock::{
    get_batch_summary_impl, get_product_spatial_balances_impl, get_stock_summary_impl,
    post_stock_movement_impl, PostMovementInput,
};
use crate::location::{create_bin, create_location, CreateBinInput, CreateLocationInput};
use crate::product::{create_product, CreateProductInput};
use crate::serial::{
    create_serial_instance, update_serial_status, CreateSerialInput, SerialError, SerialStatus,
    UpdateSerialStatusInput,
};
use crate::stock::{MovementReason, PostMovementRequest, StockLedgerError, StockLedgerService};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_hierarchy, create_test_user_with_creds,
    setup_test_db,
};
use crate::user::session::create_local_session;
use crate::variant::{create_variant, CreateVariantInput};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use std::thread;

// =========================================================================
// TEST FIXTURES & HELPERS
// =========================================================================

struct TestStockContext {
    branch_id: String,
    location_id: String,
    bin_id: String,
    product_id: String,
}

fn setup_stock_context(conn: &Connection) -> TestStockContext {
    let (_, branch_id) = create_test_org_and_branch(conn);

    let loc = create_location(
        conn,
        &CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "Main Warehouse".into(),
            code: "WH-01".into(),
            location_type: "warehouse".into(),
        },
    )
    .expect("create location");

    let bin = create_bin(
        conn,
        &CreateBinInput {
            location_id: loc.id.clone(),
            name: "Shelf A1".into(),
            code: "BIN-A1".into(),
        },
    )
    .expect("create bin");

    let prod = create_product(
        conn,
        CreateProductInput {
            name: "Enterprise Server".into(),
            description: None,
            category_id: None,
            sku: Some("SRV-ENT-01".into()),
            barcode: None,
            product_type: Some("simple".into()),
            base_price_minor: 500000,
            cost_price_minor: Some(300000),
            unit_type: Some("piece".into()),
            requires_expiry: Some(true),
            requires_serial: Some(true),
            warranty_months: Some(24),
            custom_attributes: None,
        },
    )
    .expect("create product");

    conn.execute(
        "INSERT OR REPLACE INTO product_capabilities (product_id, capability_id, enabled)
         SELECT ?1, id, 1 FROM capabilities WHERE code = 'BATCH'",
        params![prod.id],
    )
    .expect("enable batch capability");

    TestStockContext {
        branch_id,
        location_id: loc.id,
        bin_id: bin.id,
        product_id: prod.id,
    }
}

// =========================================================================
// 1. CORE OPERATIONAL MOVEMENTS & BALANCE TRANSITIONS
// =========================================================================

#[test]
fn test_post_movement_opening_balance_positive() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-open-01".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let result = StockLedgerService::post_movement(&mut conn, &req).expect("post opening balance");
    assert_eq!(result.quantity_before_milli, 0);
    assert_eq!(result.quantity_after_milli, 10_000);
    assert_eq!(result.quantity_delta_milli, 10_000);

    // Verify aggregate balance in inventory
    let agg: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE branch_id = ?1 AND product_id = ?2",
            params![ctx.branch_id, ctx.product_id],
            |r| r.get(0),
        )
        .expect("query aggregate");
    assert_eq!(agg, 10_000);

    // Verify spatial balance in location_inventory
    let spatial: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE location_id = ?1 AND bin_id = ?2",
            params![ctx.location_id, ctx.bin_id],
            |r| r.get(0),
        )
        .expect("query spatial");
    assert_eq!(spatial, 10_000);

    // Verify audit record in stock_movements
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE id = ?1",
            params![result.movement_id],
            |r| r.get(0),
        )
        .expect("query movement");
    assert_eq!(count, 1);
}

#[test]
fn test_post_movement_opening_balance_negative_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-open-neg".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -5_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::NegativeStock(_)));
}

#[test]
fn test_post_movement_adjustment_positive_and_negative() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Initial stock: 10_000
    let req_init = PostMovementRequest {
        idempotency_key: "idem-adj-init".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_init).unwrap();

    // Positive adjustment: +5_000 -> 15_000
    let req_adj_pos = PostMovementRequest {
        idempotency_key: "idem-adj-pos".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let res_pos = StockLedgerService::post_movement(&mut conn, &req_adj_pos).unwrap();
    assert_eq!(res_pos.quantity_before_milli, 10_000);
    assert_eq!(res_pos.quantity_after_milli, 15_000);

    // Negative adjustment: -7_000 -> 8_000
    let req_adj_neg = PostMovementRequest {
        idempotency_key: "idem-adj-neg".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -7_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let res_neg = StockLedgerService::post_movement(&mut conn, &req_adj_neg).unwrap();
    assert_eq!(res_neg.quantity_before_milli, 15_000);
    assert_eq!(res_neg.quantity_after_milli, 8_000);
}

#[test]
fn test_post_movement_adjustment_to_zero() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req_init = PostMovementRequest {
        idempotency_key: "idem-zero-init".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_init).unwrap();

    let req_zero = PostMovementRequest {
        idempotency_key: "idem-zero-adj".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -5_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let res_zero = StockLedgerService::post_movement(&mut conn, &req_zero).unwrap();
    assert_eq!(res_zero.quantity_after_milli, 0);

    let spatial: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE location_id = ?1 AND bin_id = ?2",
            params![ctx.location_id, ctx.bin_id],
            |r| r.get(0),
        )
        .expect("query spatial");
    assert_eq!(spatial, 0);
}

#[test]
fn test_post_movement_damage_and_loss_negative() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req_init = PostMovementRequest {
        idempotency_key: "idem-dmg-init".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_init).unwrap();

    // Damage write-off: -2_000
    let req_dmg = PostMovementRequest {
        idempotency_key: "idem-dmg-post".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -2_000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    let res_dmg = StockLedgerService::post_movement(&mut conn, &req_dmg).unwrap();
    assert_eq!(res_dmg.quantity_after_milli, 8_000);

    // Loss write-off: -3_000
    let req_loss = PostMovementRequest {
        idempotency_key: "idem-loss-post".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -3_000,
        reason: MovementReason::Loss,
        user_id: None,
    };
    let res_loss = StockLedgerService::post_movement(&mut conn, &req_loss).unwrap();
    assert_eq!(res_loss.quantity_after_milli, 5_000);
}

#[test]
fn test_post_movement_damage_positive_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-dmg-pos".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::Damage,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(
        matches!(err, StockLedgerError::Database(msg) if msg.contains("must have negative deltas"))
    );
}

#[test]
fn test_post_movement_loss_positive_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-loss-pos".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::Loss,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(
        matches!(err, StockLedgerError::Database(msg) if msg.contains("must have negative deltas"))
    );
}

#[test]
fn test_post_movement_zero_delta_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-zero".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 0,
        reason: MovementReason::Adjustment,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert_eq!(err, StockLedgerError::ZeroQuantityDelta);
}

#[test]
fn test_post_movement_negative_aggregate_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Initial stock: 5_000
    let req_init = PostMovementRequest {
        idempotency_key: "idem-neg-init".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_init).unwrap();

    // Deduction: -6_000 -> exceeds aggregate balance
    let req_over = PostMovementRequest {
        idempotency_key: "idem-neg-over".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -6_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut conn, &req_over).unwrap_err();
    assert!(matches!(err, StockLedgerError::NegativeStock(_)));
}

#[test]
fn test_post_movement_negative_spatial_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Add 10_000 to bin_id
    let req_init = PostMovementRequest {
        idempotency_key: "idem-sp-init".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_init).unwrap();

    // Create Bin 2 (has 0 balance)
    let bin2 = create_bin(
        &conn,
        &CreateBinInput {
            location_id: ctx.location_id.clone(),
            name: "Shelf B2".into(),
            code: "BIN-B2".into(),
        },
    )
    .expect("create bin 2");

    // Attempt to deduct 1_000 from bin2 (aggregate has 10_000, but bin2 has 0)
    let req_deduct_bin2 = PostMovementRequest {
        idempotency_key: "idem-sp-deduct".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(bin2.id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: -1_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut conn, &req_deduct_bin2).unwrap_err();
    assert!(matches!(err, StockLedgerError::NegativeStock(_)));
}

// =========================================================================
// 2. RELATIONAL REFERENCE VALIDATIONS
// =========================================================================

#[test]
fn test_validation_missing_location_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-miss-loc".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: "   ".into(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert_eq!(err, StockLedgerError::MissingLocation);
}

#[test]
fn test_validation_location_not_found_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-nonexist-loc".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: "nonexistent-loc-id".into(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::LocationNotFound(_)));
}

#[test]
fn test_validation_location_branch_mismatch_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Create branch 2 with location 2
    let (_, branch_2) = create_test_org_and_branch(&conn);
    let loc_b2 = create_location(
        &conn,
        &CreateLocationInput {
            branch_id: branch_2,
            parent_id: None,
            name: "Branch 2 Store".into(),
            code: "B2-ST".into(),
            location_type: "store".into(),
        },
    )
    .unwrap();

    // Request specifies branch 1, but location from branch 2
    let req = PostMovementRequest {
        idempotency_key: "idem-loc-branch-mis".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: loc_b2.id,
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::LocationBranchMismatch(_)));
}

#[test]
fn test_validation_inactive_location_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    conn.execute(
        "UPDATE locations SET is_active = 0 WHERE id = ?1",
        params![ctx.location_id],
    )
    .unwrap();

    let req = PostMovementRequest {
        idempotency_key: "idem-inact-loc".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::LocationNotFound(_)));
}

#[test]
fn test_validation_bin_not_found_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-nonexist-bin".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some("nonexistent-bin-id".into()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::BinNotFound(_)));
}

#[test]
fn test_validation_bin_location_mismatch_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Create location 2
    let loc2 = create_location(
        &conn,
        &CreateLocationInput {
            branch_id: ctx.branch_id.clone(),
            parent_id: None,
            name: "Location 2".into(),
            code: "LOC-02".into(),
            location_type: "store".into(),
        },
    )
    .unwrap();

    // Create bin in location 2
    let bin_loc2 = create_bin(
        &conn,
        &CreateBinInput {
            location_id: loc2.id,
            name: "Bin in Loc 2".into(),
            code: "BIN-L2".into(),
        },
    )
    .unwrap();

    // Request specifies location 1, but bin belonging to location 2
    let req = PostMovementRequest {
        idempotency_key: "idem-bin-loc-mis".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(bin_loc2.id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::BinLocationMismatch(_)));
}

#[test]
fn test_validation_inactive_bin_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    conn.execute(
        "UPDATE bins SET is_active = 0 WHERE id = ?1",
        params![ctx.bin_id],
    )
    .unwrap();

    let req = PostMovementRequest {
        idempotency_key: "idem-inact-bin".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::BinNotFound(_)));
}

#[test]
fn test_validation_product_not_found_or_inactive_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Non-existent product
    let req_nonexist = PostMovementRequest {
        idempotency_key: "idem-prod-nonexist".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: "nonexistent-prod".into(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err1 = StockLedgerService::post_movement(&mut conn, &req_nonexist).unwrap_err();
    assert!(matches!(err1, StockLedgerError::ProductNotFound(_)));

    // Inactive product
    conn.execute(
        "UPDATE products SET is_active = 0 WHERE id = ?1",
        params![ctx.product_id],
    )
    .unwrap();

    let req_inact = PostMovementRequest {
        idempotency_key: "idem-prod-inact".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err2 = StockLedgerService::post_movement(&mut conn, &req_inact).unwrap_err();
    assert!(matches!(err2, StockLedgerError::ProductNotFound(_)));
}

#[test]
fn test_validation_variant_product_mismatch_or_inactive_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let prod2 = create_product(
        &conn,
        CreateProductInput {
            name: "Other Product".into(),
            description: None,
            category_id: None,
            sku: Some("OTHER-PROD".into()),
            barcode: None,
            product_type: Some("simple".into()),
            base_price_minor: 1000,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .unwrap();

    let var2 = create_variant(
        &conn,
        CreateVariantInput {
            product_id: prod2.id,
            sku: Some("VAR-PROD-2".into()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    )
    .unwrap();

    // Mismatched variant: variant belongs to prod2, but request specifies product 1
    let req_mis = PostMovementRequest {
        idempotency_key: "idem-var-mis".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: Some(var2.variant.id.clone()),
        location_id: ctx.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err = StockLedgerService::post_movement(&mut conn, &req_mis).unwrap_err();
    assert!(matches!(err, StockLedgerError::VariantProductMismatch(_)));

    // Inactive variant
    let var1 = create_variant(
        &conn,
        CreateVariantInput {
            product_id: ctx.product_id.clone(),
            sku: Some("VAR-PROD-1".into()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    )
    .unwrap();

    conn.execute(
        "UPDATE product_variants SET is_active = 0 WHERE id = ?1",
        params![var1.variant.id],
    )
    .unwrap();

    let req_inact_var = PostMovementRequest {
        idempotency_key: "idem-var-inact".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: Some(var1.variant.id),
        location_id: ctx.location_id,
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let err2 = StockLedgerService::post_movement(&mut conn, &req_inact_var).unwrap_err();
    assert!(matches!(err2, StockLedgerError::VariantInactive(_)));
}

// =========================================================================
// 3. BATCH LOT BALANCE TRACKING & VALIDATIONS
// =========================================================================

#[test]
fn test_batch_movement_positive_intake_activates_depleted_batch() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            batch_number: "LOT-2026-A".into(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2028-12-31".into()),
        },
    )
    .expect("create batch");

    assert_eq!(batch.status, BatchStatus::Depleted);
    assert_eq!(batch.quantity_milli, 0);

    // Initial intake of 5_000 units via stock ledger
    let req = PostMovementRequest {
        idempotency_key: "idem-batch-intake".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: Some(batch.id.clone()),
        serial_id: None,
        quantity_delta_milli: 5_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    StockLedgerService::post_movement(&mut conn, &req).expect("post batch movement");

    // Verify batch status transitioned to active and quantity incremented
    let (qty, status): (i64, String) = conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();

    assert_eq!(qty, 5_000);
    assert_eq!(status, "active");

    // Verify spatial inventory has batch_id
    let sp_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM location_inventory WHERE batch_id = ?1",
            params![batch.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(sp_qty, 5_000);
}

#[test]
fn test_batch_movement_negative_deduction_and_depletion() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            batch_number: "LOT-2026-B".into(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2028-12-31".into()),
        },
    )
    .unwrap();

    // Intake 10_000
    let req_intake = PostMovementRequest {
        idempotency_key: "idem-b-intake".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: Some(batch.id.clone()),
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_intake).unwrap();

    // Deduct 10_000 (reaches zero)
    let req_deduct = PostMovementRequest {
        idempotency_key: "idem-b-deplete".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: Some(batch.id.clone()),
        serial_id: None,
        quantity_delta_milli: -10_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_deduct).unwrap();

    let (qty, status): (i64, String) = conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            params![batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();

    assert_eq!(qty, 0);
    assert_eq!(status, "depleted");
}

#[test]
fn test_batch_movement_insufficient_stock_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            batch_number: "LOT-INSUFF".into(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2028-12-31".into()),
        },
    )
    .unwrap();

    // Attempt to deduct from batch with 0 balance
    let req = PostMovementRequest {
        idempotency_key: "idem-b-over".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: Some(batch.id),
        serial_id: None,
        quantity_delta_milli: -1_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::BatchInsufficientStock(_)));
}

#[test]
fn test_batch_movement_product_branch_variant_mismatch_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Create branch 2 with batch
    let (_, branch_2) = create_test_org_and_branch(&conn);
    let p2 = create_product(
        &conn,
        CreateProductInput {
            name: "Prod B2".into(),
            description: None,
            category_id: None,
            sku: Some("SKU-B2".into()),
            barcode: None,
            product_type: Some("simple".into()),
            base_price_minor: 100,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: Some(true),
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .unwrap();

    let batch_b2 = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: p2.id,
            branch_id: branch_2,
            variant_id: None,
            batch_number: "LOT-B2".into(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2028-12-31".into()),
        },
    )
    .unwrap();

    // Request specifies product 1 and branch 1, but batch from branch 2
    let req = PostMovementRequest {
        idempotency_key: "idem-batch-mis".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: Some(batch_b2.id),
        serial_id: None,
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::BatchProductMismatch(_)));
}

// =========================================================================
// 4. SERIAL ASSET LIFECYCLE & VALIDATIONS
// =========================================================================

#[test]
fn test_serial_creation_reserved_and_positive_movement_instock() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-SRV-001".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: Some(250000),
        },
    )
    .expect("create serial");

    // Invariant: Registered with status reserved and NULL coordinates
    assert_eq!(serial.status, SerialStatus::Reserved);
    assert_eq!(serial.location_id, None);
    assert_eq!(serial.bin_id, None);

    // Stock intake via stock ledger: +1000 milli
    let req = PostMovementRequest {
        idempotency_key: "idem-sn-intake".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    StockLedgerService::post_movement(&mut conn, &req).expect("post serial intake");

    // Verify serial is now in_stock with assigned coordinates
    let (status, loc, bin): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();

    assert_eq!(status, "in_stock");
    assert_eq!(loc, Some(ctx.location_id));
    assert_eq!(bin, Some(ctx.bin_id));
}

#[test]
fn test_serial_movement_invalid_quantity_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-INVALID-QTY".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Delta != 1000 or -1000
    let req = PostMovementRequest {
        idempotency_key: "idem-sn-badqty".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: Some(serial.id),
        quantity_delta_milli: 2_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::SerialInvalidQuantity(_)));
}

#[test]
fn test_serial_movement_already_instock_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-ALREADY-IN".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    let req1 = PostMovementRequest {
        idempotency_key: "idem-sn-in1".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req1).unwrap();

    // Second positive movement for same serial must fail fail-closed
    let req2 = PostMovementRequest {
        idempotency_key: "idem-sn-in2".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: Some(serial.id),
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req2).unwrap_err();
    assert!(matches!(err, StockLedgerError::SerialInvalidStatus(_)));
}

#[test]
fn test_serial_movement_terminal_status_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-DISPOSED-TEST".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    conn.execute(
        "UPDATE serial_numbers SET status = 'disposed' WHERE id = ?1",
        params![serial.id],
    )
    .unwrap();

    let req = PostMovementRequest {
        idempotency_key: "idem-sn-disp".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: Some(serial.id),
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::SerialInvalidStatus(_)));
}

#[test]
fn test_serial_movement_defective_repair_intake() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-REPAIR-01".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Simulate serial returning from repair (defective -> in_stock via positive adjustment)
    conn.execute(
        "UPDATE serial_numbers SET status = 'defective' WHERE id = ?1",
        params![serial.id],
    )
    .unwrap();

    let req = PostMovementRequest {
        idempotency_key: "idem-sn-repair-in".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: 1_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };

    StockLedgerService::post_movement(&mut conn, &req).expect("repair return intake succeeds");

    let status: String = conn
        .query_row(
            "SELECT status FROM serial_numbers WHERE id = ?1",
            params![serial.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "in_stock");
}

#[test]
fn test_serial_movement_outbound_damage_defective() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-DMG-OUT".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Intake into stock
    let req_in = PostMovementRequest {
        idempotency_key: "idem-sn-dmg-in".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_in).unwrap();

    // Outbound damage: -1000 -> status becomes defective and coordinates cleared
    let req_out = PostMovementRequest {
        idempotency_key: "idem-sn-dmg-out".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: -1_000,
        reason: MovementReason::Damage,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_out).unwrap();

    let (status, loc, bin): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(status, "defective");
    assert_eq!(loc, None);
    assert_eq!(bin, None);
}

#[test]
fn test_serial_movement_outbound_loss_disposed() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-LOSS-OUT".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    let req_in = PostMovementRequest {
        idempotency_key: "idem-sn-loss-in".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_in).unwrap();

    // Outbound loss: -1000 -> status becomes disposed and coordinates cleared
    let req_out = PostMovementRequest {
        idempotency_key: "idem-sn-loss-out".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: -1_000,
        reason: MovementReason::Loss,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_out).unwrap();

    let (status, loc, bin): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(status, "disposed");
    assert_eq!(loc, None);
    assert_eq!(bin, None);
}

#[test]
fn test_serial_movement_outbound_adjustment_reserved() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-ADJ-OUT".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    let req_in = PostMovementRequest {
        idempotency_key: "idem-sn-adj-in".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_in).unwrap();

    // Outbound adjustment: -1000 -> status becomes reserved and coordinates cleared
    let req_out = PostMovementRequest {
        idempotency_key: "idem-sn-adj-out".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: -1_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_out).unwrap();

    let (status, loc, bin): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            params![serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(status, "reserved");
    assert_eq!(loc, None);
    assert_eq!(bin, None);
}

#[test]
fn test_serial_movement_outbound_non_instock_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-NON-INSTOCK".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Attempt negative movement on reserved serial (not in_stock)
    let req = PostMovementRequest {
        idempotency_key: "idem-sn-non-in".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: Some(serial.id),
        quantity_delta_milli: -1_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert!(matches!(err, StockLedgerError::SerialInvalidStatus(_)));
}

#[test]
fn test_serial_movement_coordinate_mismatch_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-COORD-MIS".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Intake into Location 1, Bin 1
    let req_in = PostMovementRequest {
        idempotency_key: "idem-sn-c-in".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id.clone()),
        quantity_delta_milli: 1_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req_in).unwrap();

    // Create Bin 2
    let bin2 = create_bin(
        &conn,
        &CreateBinInput {
            location_id: ctx.location_id.clone(),
            name: "Shelf B2".into(),
            code: "BIN-C-B2".into(),
        },
    )
    .unwrap();

    // Attempt outbound deduction specifying Bin 2 (stored at Bin 1)
    let req_out = PostMovementRequest {
        idempotency_key: "idem-sn-c-out".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(bin2.id),
        batch_id: None,
        serial_id: Some(serial.id),
        quantity_delta_milli: -1_000,
        reason: MovementReason::Damage,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req_out).unwrap_err();
    assert!(matches!(err, StockLedgerError::SerialCoordinateMismatch(_)));
}

#[test]
fn test_serial_movement_null_coordinates_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-NULL-COORD".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Simulate an in_stock serial with NULL location_id (e.g. corrupt state)
    conn.execute(
        "UPDATE serial_numbers SET status = 'in_stock', location_id = NULL, bin_id = NULL WHERE id = ?1",
        params![serial.id],
    )
    .unwrap();

    let req = PostMovementRequest {
        idempotency_key: "idem-sn-null-loc".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: Some(serial.id),
        quantity_delta_milli: -1_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req).unwrap_err();
    assert_eq!(
        err,
        StockLedgerError::SerialCoordinateMismatch(
            "Serial has no physical location assigned".into()
        )
    );
}

#[test]
fn test_update_serial_status_firewall_rejection() {
    let conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let serial = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-FIREWALL-TEST".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    let err = update_serial_status(
        &conn,
        &UpdateSerialStatusInput {
            id: serial.id,
            branch_id: ctx.branch_id,
            status: SerialStatus::InStock,
        },
    )
    .unwrap_err();

    assert!(matches!(err, SerialError::Validation(msg) if msg.contains("in_stock")));
}

// =========================================================================
// 5. TRANSACTION-SCOPED IDEMPOTENCY
// =========================================================================

#[test]
fn test_idempotency_exact_replay_returns_cached_result() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-replay-01".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let res1 = StockLedgerService::post_movement(&mut conn, &req).expect("first post");
    let res2 = StockLedgerService::post_movement(&mut conn, &req).expect("replayed post");

    // Exact match of cached result
    assert_eq!(res1.movement_id, res2.movement_id);
    assert_eq!(res1.quantity_after_milli, res2.quantity_after_milli);

    // Verify balance was NOT mutated twice
    let agg: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE branch_id = ?1 AND product_id = ?2",
            params![ctx.branch_id, ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(agg, 10_000);

    // Verify only ONE movement record exists
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM stock_movements WHERE branch_id = ?1",
            params![ctx.branch_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_idempotency_conflicting_request_rejected() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req1 = PostMovementRequest {
        idempotency_key: "idem-conflict-key".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req1).unwrap();

    // Conflicting request with same idempotency key but different quantity
    let req2 = PostMovementRequest {
        idempotency_key: "idem-conflict-key".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 20_000, // Changed!
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let err = StockLedgerService::post_movement(&mut conn, &req2).unwrap_err();
    assert!(matches!(err, StockLedgerError::IdempotencyConflict(_)));
}

#[test]
fn test_idempotency_inside_same_transaction_no_orphan_rows() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Create a request that will fail validation (e.g. non-existent bin)
    let req_fail = PostMovementRequest {
        idempotency_key: "idem-fail-tx".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some("nonexistent-bin".into()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };

    let _ = StockLedgerService::post_movement(&mut conn, &req_fail);

    // Verify idempotency key was NOT persisted on transaction failure
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM idempotency_keys WHERE key = 'idem-fail-tx'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 0,
        "Idempotency key must not be persisted on failed transaction"
    );
}

// =========================================================================
// 6. LEGACY STOCK PRESERVATION & QUERY BREAKDOWNS
// =========================================================================

#[test]
fn test_legacy_stock_preservation_and_unallocated_queries() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Simulate pre-020 legacy stock inserted directly into inventory
    conn.execute(
        "INSERT INTO inventory (branch_id, product_id, variant_id, quantity_milli, updated_at)
         VALUES (?1, ?2, NULL, 50000, datetime('now'))",
        params![ctx.branch_id, ctx.product_id],
    )
    .unwrap();

    // Query summary before any spatial movement
    let summary_before =
        StockLedgerService::get_stock_summary(&conn, &ctx.branch_id, &ctx.product_id, None)
            .unwrap();

    assert_eq!(summary_before.aggregate_quantity_milli, 50_000);
    assert_eq!(summary_before.allocated_spatial_milli, 0);
    assert_eq!(summary_before.unallocated_quantity_milli, 50_000);

    // Allocate 10_000 units into Location 1 via stock movement
    let req = PostMovementRequest {
        idempotency_key: "idem-legacy-alloc".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: MovementReason::Adjustment,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req).unwrap();

    let summary_after =
        StockLedgerService::get_stock_summary(&conn, &ctx.branch_id, &ctx.product_id, None)
            .unwrap();

    assert_eq!(summary_after.aggregate_quantity_milli, 60_000);
    assert_eq!(summary_after.allocated_spatial_milli, 10_000);
    assert_eq!(summary_after.unallocated_quantity_milli, 50_000);
}

#[test]
fn test_get_product_spatial_balances_query() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let bin2 = create_bin(
        &conn,
        &CreateBinInput {
            location_id: ctx.location_id.clone(),
            name: "Shelf A2".into(),
            code: "BIN-A2".into(),
        },
    )
    .unwrap();

    // Post movement into Bin 1 (5_000)
    let req1 = PostMovementRequest {
        idempotency_key: "idem-sp-b1".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req1).unwrap();

    // Post movement into Bin 2 (3_000)
    let req2 = PostMovementRequest {
        idempotency_key: "idem-sp-b2".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(bin2.id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 3_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req2).unwrap();

    let balances =
        StockLedgerService::get_product_spatial_balances(&conn, &ctx.branch_id, &ctx.product_id)
            .unwrap();

    assert_eq!(balances.len(), 2);
    assert_eq!(balances[0].quantity_milli, 5_000);
    assert_eq!(balances[1].quantity_milli, 3_000);
}

#[test]
fn test_get_batch_summary_query() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: ctx.product_id.clone(),
            branch_id: ctx.branch_id.clone(),
            variant_id: None,
            batch_number: "LOT-SUMMARY-01".into(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2028-12-31".into()),
        },
    )
    .unwrap();

    // Intake 15_000 units into batch and location
    let req = PostMovementRequest {
        idempotency_key: "idem-b-sum".into(),
        branch_id: ctx.branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: Some(batch.id.clone()),
        serial_id: None,
        quantity_delta_milli: 15_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    StockLedgerService::post_movement(&mut conn, &req).unwrap();

    let summary = StockLedgerService::get_batch_summary(&conn, &ctx.branch_id, &batch.id).unwrap();

    assert_eq!(summary.total_quantity_milli, 15_000);
    assert_eq!(summary.spatial_quantity_milli, 15_000);
    assert_eq!(summary.unallocated_quantity_milli, 0);
    assert_eq!(summary.status, "active");
}

// =========================================================================
// 7. DATABASE ENGINE TRIGGERS
// =========================================================================

#[test]
fn test_database_trigger_immutability_update_prevented() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-trg-upd".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let res = StockLedgerService::post_movement(&mut conn, &req).unwrap();

    // SQL UPDATE must be rejected by trg_stock_movements_no_update
    let err = conn
        .execute(
            "UPDATE stock_movements SET quantity_delta_milli = 999 WHERE id = ?1",
            params![res.movement_id],
        )
        .unwrap_err();

    assert!(err.to_string().contains("IMMUTABLE_LEDGER"));
}

#[test]
fn test_database_trigger_immutability_delete_prevented() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let req = PostMovementRequest {
        idempotency_key: "idem-trg-del".into(),
        branch_id: ctx.branch_id,
        product_id: ctx.product_id,
        variant_id: None,
        location_id: ctx.location_id,
        bin_id: Some(ctx.bin_id),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5_000,
        reason: MovementReason::OpeningBalance,
        user_id: None,
    };
    let res = StockLedgerService::post_movement(&mut conn, &req).unwrap();

    // SQL DELETE must be rejected by trg_stock_movements_no_delete
    let err = conn
        .execute(
            "DELETE FROM stock_movements WHERE id = ?1",
            params![res.movement_id],
        )
        .unwrap_err();

    assert!(err.to_string().contains("IMMUTABLE_LEDGER"));
}

#[test]
fn test_database_trigger_spatial_guard_prevents_corrupt_inserts() {
    let conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    // Zero delta insert direct SQL
    let err_zero = conn
        .execute(
            "INSERT INTO stock_movements (
            branch_id, product_id, quantity_delta, quantity_delta_milli,
            quantity_before, quantity_before_milli, quantity_after, quantity_after_milli,
            reason, source_type, location_id
         ) VALUES (?1, ?2, 0.0, 0, 0.0, 0, 0.0, 0, 'adjustment', 'manual', ?3)",
            params![ctx.branch_id, ctx.product_id, ctx.location_id],
        )
        .unwrap_err();
    assert!(err_zero.to_string().contains("SPATIAL_GUARD_ZERO_DELTA"));

    // Arithmetic discontinuity: before=10, delta=5, after=20 (should be 15)
    let err_arith = conn
        .execute(
            "INSERT INTO stock_movements (
            branch_id, product_id, quantity_delta, quantity_delta_milli,
            quantity_before, quantity_before_milli, quantity_after, quantity_after_milli,
            reason, source_type, location_id
         ) VALUES (?1, ?2, 5.0, 5000, 10.0, 10000, 20.0, 20000, 'adjustment', 'manual', ?3)",
            params![ctx.branch_id, ctx.product_id, ctx.location_id],
        )
        .unwrap_err();
    assert!(err_arith
        .to_string()
        .contains("SPATIAL_GUARD_ARITHMETIC_MISMATCH"));

    // Orphan bin (bin without location)
    let err_orphan = conn
        .execute(
            "INSERT INTO stock_movements (
            branch_id, product_id, quantity_delta, quantity_delta_milli,
            quantity_before, quantity_before_milli, quantity_after, quantity_after_milli,
            reason, source_type, bin_id
         ) VALUES (?1, ?2, 5.0, 5000, 0.0, 0, 5.0, 5000, 'adjustment', 'manual', ?3)",
            params![ctx.branch_id, ctx.product_id, ctx.bin_id],
        )
        .unwrap_err();
    assert!(err_orphan.to_string().contains("SPATIAL_GUARD_ORPHAN_BIN"));
}

// =========================================================================
// 8. COMMAND AUTHORIZATION & TENANCY
// =========================================================================

#[test]
fn test_post_stock_movement_command_permission() {
    let mut conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let (org_id, branch_id, cashier) = create_test_user_hierarchy(&conn);
    let _ = org_id;

    let manager = create_test_user_with_creds(
        &conn,
        "mgr_user",
        "manager",
        &branch_id,
        "manager@example.com",
    );

    let session_cashier =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).unwrap();
    let session_mgr = create_local_session(&conn, &manager.id, &branch_id, "pin", None).unwrap();

    let input = PostMovementInput {
        idempotency_key: "idem-cmd-perm".into(),
        branch_id: branch_id.clone(),
        product_id: ctx.product_id.clone(),
        variant_id: None,
        location_id: ctx.location_id.clone(),
        bin_id: Some(ctx.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10_000,
        reason: "opening_balance".into(),
    };

    // 1. Cashier without InventoryAdjust permission is denied
    let err = post_stock_movement_impl(&mut conn, &session_cashier.id, &input).unwrap_err();
    assert!(err.contains("Permission denied") || err.contains("permission"));

    // 2. Manager with InventoryAdjust permission succeeds
    let res =
        post_stock_movement_impl(&mut conn, &session_mgr.id, &input).expect("manager succeeds");
    assert_eq!(res.quantity_after_milli, 10_000);
}

#[test]
fn test_query_commands_enforce_branch_isolation() {
    let conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let (_, branch_2, cashier_b2) = create_test_user_hierarchy(&conn);
    let session_b2 = create_local_session(&conn, &cashier_b2.id, &branch_2, "pin", None).unwrap();

    // User in branch 2 attempts to query product stock summary in branch 1
    let err = get_stock_summary_impl(&conn, &session_b2.id, &ctx.branch_id, &ctx.product_id, None)
        .unwrap_err();

    assert!(err.contains("Scope violation") || err.contains("denied") || err.contains("Branch"));
}

// =========================================================================
// 9. CONCURRENCY & SERIALIZED ACCESS
// =========================================================================

#[test]
fn test_concurrent_idempotency_racing_threads_models_tauri_mutex() {
    let conn = setup_test_db();
    let ctx = setup_stock_context(&conn);

    let shared_db = Arc::new(Mutex::new(conn));
    let mut handles = vec![];

    let shared_ctx = Arc::new(ctx);

    for _ in 0..5 {
        let db_ref = Arc::clone(&shared_db);
        let ctx_ref = Arc::clone(&shared_ctx);

        let handle = thread::spawn(move || {
            let req = PostMovementRequest {
                idempotency_key: "concurrent-race-idem-01".into(),
                branch_id: ctx_ref.branch_id.clone(),
                product_id: ctx_ref.product_id.clone(),
                variant_id: None,
                location_id: ctx_ref.location_id.clone(),
                bin_id: Some(ctx_ref.bin_id.clone()),
                batch_id: None,
                serial_id: None,
                quantity_delta_milli: 10_000,
                reason: MovementReason::OpeningBalance,
                user_id: None,
            };

            let mut guard = db_ref.lock().unwrap();
            StockLedgerService::post_movement(&mut guard, &req)
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    let mut movement_ids = Vec::new();

    for h in handles {
        let res = h.join().unwrap();
        if let Ok(m) = res {
            success_count += 1;
            movement_ids.push(m.movement_id);
        }
    }

    assert_eq!(
        success_count, 5,
        "All 5 concurrent requests must succeed via replay"
    );
    // All 5 must have received the exact same movement_id
    assert!(movement_ids.iter().all(|id| id == &movement_ids[0]));

    let guard = shared_db.lock().unwrap();
    let agg: i64 = guard
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE branch_id = ?1 AND product_id = ?2",
            params![shared_ctx.branch_id, shared_ctx.product_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        agg, 10_000,
        "Inventory must be mutated exactly once despite 5 competing threads"
    );
}
