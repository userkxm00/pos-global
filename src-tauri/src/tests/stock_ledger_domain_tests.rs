// F2.11 — Stock Ledger & Spatial Balances Domain Service Test Suite
// Exhaustive invariant validation for Gate 2:
// - Reason directionality & rejection
// - Aggregate & spatial atomicity & rollback
// - Unallocated balance invariants
// - Branch, variant, batch, and serial isolation
// - Batch lifecycle (active -> depleted, quarantined/recalled preservation)
// - Serial exact +/-1000 unit delta, physical location, and lifecycle firewall
// - Idempotency replay and conflict detection
// - Stock movements immutability triggers (UPDATE / DELETE rejected)
// - Legacy pre-020 untouched unallocated stock
// - Serialized concurrency with Arc<Mutex<Connection>>

use crate::batch::{create_batch, update_batch_status, BatchStatus, CreateBatchInput, UpdateBatchStatusInput};
use crate::serial::{create_serial_instance, update_serial_status, CreateSerialInstanceInput, UpdateSerialStatusInput};
use crate::stock_ledger::{
    compute_request_hash, LocationInventoryFilter, PostMovementInput, StockLedgerError,
    StockLedgerService, StockMovementFilter, StockMovementReason,
};
use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, setup_test_db, setup_test_db_up_to,
};
use rusqlite::{params, Connection};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;

// =========================================================================
// TEST FIXTURES & HELPERS
// =========================================================================

struct TestFixtures {
    org_id: String,
    branch_id: String,
    product_id: String,
    variant_id: String,
    location_id: String,
    bin_id: String,
}

fn setup_fixtures(conn: &Connection) -> TestFixtures {
    let (org_id, branch_id) = create_test_org_and_branch(conn);

    let product_id = "prod_ledger_01";
    let variant_id = "var_ledger_01";

    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active)
         VALUES (?1, 'Ledger Test Product', 25.0, 1)",
        [product_id],
    )
    .expect("product created");

    conn.execute(
        "INSERT INTO product_variants (id, product_id, sku, is_active)
         VALUES (?1, ?2, 'SKU-LEDGER-01', 1)",
        [variant_id, product_id],
    )
    .expect("variant created");

    let location_id = "loc_ledger_01";
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Main Warehouse', 'LOC-01', 'warehouse', 1)",
        [location_id, &branch_id],
    )
    .expect("location created");

    let bin_id = "bin_ledger_01";
    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES (?1, ?2, 'Aisle 1 Shelf A', 'BIN-01', 1)",
        [bin_id, location_id],
    )
    .expect("bin created");

    TestFixtures {
        org_id,
        branch_id,
        product_id: product_id.to_string(),
        variant_id: variant_id.to_string(),
        location_id: location_id.to_string(),
        bin_id: bin_id.to_string(),
    }
}

fn create_second_branch_and_location(conn: &Connection, org_id: &str) -> (String, String) {
    let branch = crate::branch::create_branch(
        conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id.to_string(),
            name: "Branch Two".to_string(),
            address: Some("Branch Two Address".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch two created");

    let loc_id = "loc_branch_two";
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Branch Two Warehouse', 'LOC-B2', 'warehouse', 1)",
        [loc_id, &branch.id],
    )
    .expect("branch two location created");

    (branch.id, loc_id.to_string())
}

// =========================================================================
// 1. REASON DIRECTIONALITY & VALIDATION TESTS
// =========================================================================

#[test]
fn test_opening_balance_positive_accepted_and_balances_updated() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    let input = PostMovementInput {
        branch_id: f.branch_id.clone(),
        product_id: f.product_id.clone(),
        variant_id: Some(f.variant_id.clone()),
        location_id: f.location_id.clone(),
        bin_id: Some(f.bin_id.clone()),
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: StockMovementReason::OpeningBalance,
        source_type: Some("manual".into()),
        source_id: None,
        user_id: None,
        idempotency_key: None,
    };

    let movement = StockLedgerService::post_movement(&mut conn, &input).expect("post movement");

    assert_eq!(movement.quantity_delta_milli, 5000);
    assert_eq!(movement.quantity_before_milli, Some(0));
    assert_eq!(movement.quantity_after_milli, Some(5000));
    assert_eq!(movement.reason, StockMovementReason::OpeningBalance);

    let balance = StockLedgerService::get_balance(&conn, &f.branch_id, &f.product_id, Some(&f.variant_id))
        .expect("get balance");
    assert_eq!(balance.aggregate_quantity_milli, 5000);
    assert_eq!(balance.allocated_spatial_milli, 5000);
    assert_eq!(balance.unallocated_milli, 0);
}

#[test]
fn test_opening_balance_zero_and_negative_rejected() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // Delta 0 rejected
    let input_zero = PostMovementInput {
        branch_id: f.branch_id.clone(),
        product_id: f.product_id.clone(),
        variant_id: Some(f.variant_id.clone()),
        location_id: f.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 0,
        reason: StockMovementReason::OpeningBalance,
        source_type: None,
        source_id: None,
        user_id: None,
        idempotency_key: None,
    };
    let err = StockLedgerService::post_movement(&mut conn, &input_zero).unwrap_err();
    assert!(matches!(err, StockLedgerError::InvalidQuantity(_)));

    // Negative delta rejected for opening_balance
    let input_neg = PostMovementInput {
        quantity_delta_milli: -1000,
        ..input_zero
    };
    let err = StockLedgerService::post_movement(&mut conn, &input_neg).unwrap_err();
    assert!(matches!(err, StockLedgerError::InvalidQuantity(_)));

    // No rows written
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_adjustment_positive_and_negative_within_balance() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // Initial intake of 10,000 milli
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 10000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Positive adjustment +3,000
    let m1 = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 3000,
            reason: StockMovementReason::Adjustment,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();
    assert_eq!(m1.quantity_before_milli, Some(10000));
    assert_eq!(m1.quantity_after_milli, Some(13000));

    // Negative adjustment -4,000
    let m2 = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: -4000,
            reason: StockMovementReason::Adjustment,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();
    assert_eq!(m2.quantity_before_milli, Some(13000));
    assert_eq!(m2.quantity_after_milli, Some(9000));

    let balance = StockLedgerService::get_balance(&conn, &f.branch_id, &f.product_id, Some(&f.variant_id)).unwrap();
    assert_eq!(balance.aggregate_quantity_milli, 9000);
    assert_eq!(balance.allocated_spatial_milli, 9000);
    assert_eq!(balance.unallocated_milli, 0);
}

#[test]
fn test_adjustment_negative_underflow_rejected() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // Initial 5,000 milli
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 5000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Deduct 6,000 milli -> underflow
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: -6000,
            reason: StockMovementReason::Adjustment,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, StockLedgerError::InsufficientStock { .. }));

    // Stock unchanged
    let bal = StockLedgerService::get_balance(&conn, &f.branch_id, &f.product_id, None).unwrap();
    assert_eq!(bal.aggregate_quantity_milli, 5000);
}

#[test]
fn test_damage_negative_accepted_positive_rejected() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 10000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Damage positive delta rejected
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 1000,
            reason: StockMovementReason::Damage,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, StockLedgerError::InvalidQuantity(_)));

    // Damage negative delta accepted
    let m = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: -2000,
            reason: StockMovementReason::Damage,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();
    assert_eq!(m.quantity_after_milli, Some(8000));
}

#[test]
fn test_loss_negative_accepted_positive_rejected() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 5000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Loss positive delta rejected
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 500,
            reason: StockMovementReason::Loss,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, StockLedgerError::InvalidQuantity(_)));

    // Loss negative delta accepted
    let m = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: -1000,
            reason: StockMovementReason::Loss,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();
    assert_eq!(m.quantity_after_milli, Some(4000));
}

#[test]
fn test_unsupported_movement_reason_rejected() {
    assert!(StockMovementReason::from_str("transfer").is_err());
    assert!(StockMovementReason::from_str("reconciliation").is_err());
    assert!(StockMovementReason::from_str("purchase").is_err());
    assert!(StockMovementReason::from_str("return").is_err());
    assert!(StockMovementReason::from_str("sale").is_err());
    assert!(StockMovementReason::from_str("").is_err());
    assert!(StockMovementReason::from_str("arbitrary").is_err());

    assert_eq!(
        StockMovementReason::from_str("opening_balance").unwrap(),
        StockMovementReason::OpeningBalance
    );
    assert_eq!(
        StockMovementReason::from_str("adjustment").unwrap(),
        StockMovementReason::Adjustment
    );
    assert_eq!(
        StockMovementReason::from_str("damage").unwrap(),
        StockMovementReason::Damage
    );
    assert_eq!(
        StockMovementReason::from_str("loss").unwrap(),
        StockMovementReason::Loss
    );
}

// =========================================================================
// 2. ATOMIC ROLLBACK & AUDIT ACCURACY TESTS
// =========================================================================

#[test]
fn test_atomic_rollback_on_failed_mutation() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // Initial stock: 5,000 milli
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 5000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: Some("init-key".into()),
        },
    )
    .unwrap();

    let movements_before: i64 = conn.query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0)).unwrap();
    let keys_before: i64 = conn.query_row("SELECT COUNT(*) FROM idempotency_keys", [], |r| r.get(0)).unwrap();

    // Now issue a movement that will fail due to insufficient stock (-10,000)
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: -10000,
            reason: StockMovementReason::Adjustment,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: Some("failed-key".into()),
        },
    )
    .unwrap_err();

    assert!(matches!(err, StockLedgerError::InsufficientStock { .. }));

    // Verify complete rollback: no new stock movement, no new idempotency key
    let movements_after: i64 = conn.query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0)).unwrap();
    let keys_after: i64 = conn.query_row("SELECT COUNT(*) FROM idempotency_keys", [], |r| r.get(0)).unwrap();
    assert_eq!(movements_after, movements_before);
    assert_eq!(keys_after, keys_before);

    let failed_key_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM idempotency_keys WHERE key = 'failed-key')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!failed_key_exists, "Failed mutation must not persist an idempotency key");

    // Inventory and spatial balances remain untouched at 5,000
    let bal = StockLedgerService::get_balance(&conn, &f.branch_id, &f.product_id, None).unwrap();
    assert_eq!(bal.aggregate_quantity_milli, 5000);
    assert_eq!(bal.allocated_spatial_milli, 5000);
}

#[test]
fn test_movement_row_matches_exact_before_delta_after_fields() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    let m1 = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 7500,
            reason: StockMovementReason::OpeningBalance,
            source_type: Some("goods_receipt".into()),
            source_id: Some("GR-9988".into()),
            user_id: Some("usr_auditor".into()),
            idempotency_key: None,
        },
    )
    .unwrap();

    assert_eq!(m1.quantity_before_milli, Some(0));
    assert_eq!(m1.quantity_delta_milli, 7500);
    assert_eq!(m1.quantity_after_milli, Some(7500));

    // Verify in database directly
    let (delta_milli, delta_real, before_milli, before_real, after_milli, after_real, reason, src_type, src_id, loc_id, bin_id, usr_id): (
        i64, f64, Option<i64>, Option<f64>, Option<i64>, Option<f64>, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>
    ) = conn.query_row(
        "SELECT quantity_delta_milli, quantity_delta, quantity_before_milli, quantity_before,
                quantity_after_milli, quantity_after, reason, source_type, source_id,
                location_id, bin_id, user_id
         FROM stock_movements WHERE id = ?1",
        [&m1.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?)),
    ).unwrap();

    assert_eq!(delta_milli, 7500);
    assert!((delta_real - 7.5).abs() < 1e-6);
    assert_eq!(before_milli, Some(0));
    assert_eq!(before_real, Some(0.0));
    assert_eq!(after_milli, Some(7500));
    assert!((after_real.unwrap() - 7.5).abs() < 1e-6);
    assert_eq!(reason, "opening_balance");
    assert_eq!(src_type.as_deref(), Some("goods_receipt"));
    assert_eq!(src_id.as_deref(), Some("GR-9988"));
    assert_eq!(loc_id.as_deref(), Some(f.location_id.as_str()));
    assert_eq!(bin_id.as_deref(), Some(f.bin_id.as_str()));
    assert_eq!(usr_id.as_deref(), Some("usr_auditor"));
}

// =========================================================================
// 3. SPATIAL BALANCES & UNALLOCATED CALCULATIONS
// =========================================================================

#[test]
fn test_spatial_balance_and_unallocated_balance_calculation() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // Create second location in same branch
    let loc2_id = "loc_ledger_02";
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, location_type, is_active)
         VALUES (?1, ?2, 'Secondary Storage', 'LOC-02', 'shelf', 1)",
        [loc2_id, &f.branch_id],
    )
    .unwrap();

    // Post to Loc 1: 8,000 milli
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 8000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Post to Loc 2: 5,000 milli
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: loc2_id.to_string(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 5000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    let balance = StockLedgerService::get_balance(&conn, &f.branch_id, &f.product_id, None).unwrap();
    assert_eq!(balance.aggregate_quantity_milli, 13000);
    assert_eq!(balance.allocated_spatial_milli, 13000);
    assert_eq!(balance.unallocated_milli, 0);

    // List location inventory
    let loc_inv = StockLedgerService::list_location_inventory(
        &conn,
        &LocationInventoryFilter {
            branch_id: f.branch_id.clone(),
            product_id: Some(f.product_id.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(loc_inv.len(), 2);
}

// =========================================================================
// 4. BRANCH & VARIANT ISOLATION TESTS
// =========================================================================

#[test]
fn test_branch_isolation_enforcement() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);
    let (_branch_two_id, loc_branch_two) = create_second_branch_and_location(&conn, &f.org_id);

    // Movement in Branch A specifying a location from Branch B -> rejected BranchMismatch
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: loc_branch_two,
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 1000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, StockLedgerError::BranchMismatch(_)));
}

#[test]
fn test_variant_consistency_enforcement() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // Create a second product
    let prod2_id = "prod_other_02";
    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active) VALUES (?1, 'Product Two', 15.0, 1)",
        [prod2_id],
    )
    .unwrap();

    // Create variant belonging to prod2
    let var_prod2_id = "var_belonging_to_prod2";
    conn.execute(
        "INSERT INTO product_variants (id, product_id, sku, is_active) VALUES (?1, ?2, 'SKU-P2-01', 1)",
        [var_prod2_id, prod2_id],
    )
    .unwrap();

    // Post movement for prod1 using variant belonging to prod2 -> rejected VariantMismatch
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(var_prod2_id.to_string()),
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 1000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, StockLedgerError::VariantMismatch(_)));
}

// =========================================================================
// 5. BATCH CONSISTENCY & LIFECYCLE TESTS
// =========================================================================

#[test]
fn test_batch_consistency_and_lifecycle() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // 1. Create batch via F2.07 metadata (quantity = 0, starts active)
    let batch = create_batch(
        &conn,
        CreateBatchInput {
            product_id: f.product_id.clone(),
            branch_id: f.branch_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            batch_number: "BATCH-LOT-77".to_string(),
            quantity_milli: 0,
            cost_price_minor: Some(1200),
            manufactured_date: None,
            expiry_date: None,
        },
    )
    .unwrap();
    assert_eq!(batch.status, BatchStatus::Active);
    assert_eq!(batch.quantity_milli, 0);

    // 2. Positive intake via StockLedgerService (+10,000 milli)
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: Some(batch.id.clone()),
            serial_id: None,
            quantity_delta_milli: 10000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    let b_row: (i64, String) = conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            [&batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(b_row.0, 10000);
    assert_eq!(b_row.1, "active");

    // 3. Partial deduction (-4,000 milli): remains active
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: Some(batch.id.clone()),
            serial_id: None,
            quantity_delta_milli: -4000,
            reason: StockMovementReason::Adjustment,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    let b_row: (i64, String) = conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            [&batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(b_row.0, 6000);
    assert_eq!(b_row.1, "active");

    // 4. Deduction to zero (-6,000 milli): transitions to depleted
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: Some(batch.id.clone()),
            serial_id: None,
            quantity_delta_milli: -6000,
            reason: StockMovementReason::Adjustment,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    let b_row: (i64, String) = conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            [&batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(b_row.0, 0);
    assert_eq!(b_row.1, "depleted");

    // 5. Depleted is terminal: further positive intake is rejected
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: Some(batch.id.clone()),
            serial_id: None,
            quantity_delta_milli: 1000,
            reason: StockMovementReason::Adjustment,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, StockLedgerError::InvalidBatch(_)));
}

#[test]
fn test_quarantined_and_recalled_batch_preservation_on_stock_deduction() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // Create batch & intake 5,000 milli while active
    let batch = create_batch(
        &conn,
        CreateBatchInput {
            product_id: f.product_id.clone(),
            branch_id: f.branch_id.clone(),
            variant_id: None,
            batch_number: "BATCH-QUARANTINE-01".to_string(),
            quantity_milli: 0,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: None,
        },
    )
    .unwrap();

    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: Some(batch.id.clone()),
            serial_id: None,
            quantity_delta_milli: 5000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Now transition batch to quarantined via F2.07 lifecycle
    update_batch_status(
        &conn,
        &UpdateBatchStatusInput {
            batch_id: batch.id.clone(),
            status: BatchStatus::Quarantined,
        },
    )
    .unwrap();

    // 1. Positive intake on quarantined batch must be rejected
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: Some(batch.id.clone()),
            serial_id: None,
            quantity_delta_milli: 1000,
            reason: StockMovementReason::Adjustment,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, StockLedgerError::InvalidBatch(_)));

    // 2. Deduction on quarantined batch (e.g. damaged goods written off) is allowed
    // and PRESERVES quarantined status while quantity remains > 0
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: Some(batch.id.clone()),
            serial_id: None,
            quantity_delta_milli: -2000,
            reason: StockMovementReason::Damage,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    let (qty, status): (i64, String) = conn
        .query_row(
            "SELECT quantity_milli, status FROM product_batches WHERE id = ?1",
            [&batch.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(qty, 3000);
    assert_eq!(status, "quarantined"); // Status preserved! Not reactivated to active!
}

// =========================================================================
// 6. SERIAL RESTRICTIONS, ATTRIBUTION & FIREWALL TESTS
// =========================================================================

#[test]
fn test_serial_consistency_exact_delta_and_physical_location() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    // 1. Create serial: registers as reserved with location_id = NULL
    let serial = create_serial_instance(
        &conn,
        CreateSerialInstanceInput {
            product_id: f.product_id.clone(),
            branch_id: f.branch_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            serial_number: Some("SN-TEST-88001".to_string()),
            imei: None,
            asset_tag: None,
            cost_price_minor: Some(15000),
        },
    )
    .unwrap();

    assert_eq!(serial.status, "reserved");
    assert_eq!(serial.location_id, None);
    assert_eq!(serial.bin_id, None);

    // 2. Serial intake with non-1000 delta (+2000) rejected
    let err = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: None,
            serial_id: Some(serial.id.clone()),
            quantity_delta_milli: 2000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, StockLedgerError::InvalidQuantity(_)));

    // 3. Serial intake with exactly +1000 milli succeeds
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: None,
            serial_id: Some(serial.id.clone()),
            quantity_delta_milli: 1000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Verify serial updated to in_stock with location_id and bin_id
    let s_row: (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            [&serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(s_row.0, "in_stock");
    assert_eq!(s_row.1.as_deref(), Some(f.location_id.as_str()));
    assert_eq!(s_row.2.as_deref(), Some(f.bin_id.as_str()));

    // 4. Outbound damage movement: transitions to defective, clears location & bin
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: Some(f.variant_id.clone()),
            location_id: f.location_id.clone(),
            bin_id: Some(f.bin_id.clone()),
            batch_id: None,
            serial_id: Some(serial.id.clone()),
            quantity_delta_milli: -1000,
            reason: StockMovementReason::Damage,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    let s_row2: (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = ?1",
            [&serial.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(s_row2.0, "defective");
    assert_eq!(s_row2.1, None);
    assert_eq!(s_row2.2, None);
}

#[test]
fn test_serial_lifecycle_firewall_blocks_direct_status_bypass() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    let serial = create_serial_instance(
        &conn,
        CreateSerialInstanceInput {
            product_id: f.product_id.clone(),
            branch_id: f.branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-FIREWALL-01".to_string()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Intake serial into stock via ledger
    StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: Some(serial.id.clone()),
            quantity_delta_milli: 1000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Now attempt direct status transition out of in_stock via update_serial_status
    // This MUST be blocked by the firewall to ensure stock ledger integrity
    let err = update_serial_status(
        &conn,
        &UpdateSerialStatusInput {
            serial_id: serial.id.clone(),
            status: "defective".to_string(),
        },
    )
    .unwrap_err();

    assert!(matches!(err, crate::serial::SerialError::InvalidStatusTransition(_)));
}

// =========================================================================
// 7. IDEMPOTENCY & IMMUTABILITY TESTS
// =========================================================================

#[test]
fn test_idempotent_replay_with_same_request_hash() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    let input = PostMovementInput {
        branch_id: f.branch_id.clone(),
        product_id: f.product_id.clone(),
        variant_id: None,
        location_id: f.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 4000,
        reason: StockMovementReason::OpeningBalance,
        source_type: None,
        source_id: None,
        user_id: None,
        idempotency_key: Some("test-idem-001".to_string()),
    };

    // First call
    let m1 = StockLedgerService::post_movement(&mut conn, &input).unwrap();

    // Second call with same key and same parameters
    let m2 = StockLedgerService::post_movement(&mut conn, &input).unwrap();

    assert_eq!(m1.id, m2.id);
    assert_eq!(m1.quantity_delta_milli, m2.quantity_delta_milli);

    // Exactly one movement row in DB
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 1);

    // Inventory only increased once by 4,000
    let bal = StockLedgerService::get_balance(&conn, &f.branch_id, &f.product_id, None).unwrap();
    assert_eq!(bal.aggregate_quantity_milli, 4000);
}

#[test]
fn test_idempotency_conflict_with_different_request_hash() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    let input1 = PostMovementInput {
        branch_id: f.branch_id.clone(),
        product_id: f.product_id.clone(),
        variant_id: None,
        location_id: f.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 5000,
        reason: StockMovementReason::OpeningBalance,
        source_type: None,
        source_id: None,
        user_id: None,
        idempotency_key: Some("conflict-key-01".to_string()),
    };
    StockLedgerService::post_movement(&mut conn, &input1).unwrap();

    // Same idempotency key, but different quantity (3000 instead of 5000)
    let input2 = PostMovementInput {
        quantity_delta_milli: 3000,
        ..input1
    };
    let err = StockLedgerService::post_movement(&mut conn, &input2).unwrap_err();

    assert!(matches!(err, StockLedgerError::IdempotencyConflict(_)));

    // Balance remains 5000
    let bal = StockLedgerService::get_balance(&conn, &f.branch_id, &f.product_id, None).unwrap();
    assert_eq!(bal.aggregate_quantity_milli, 5000);
}

#[test]
fn test_immutable_stock_movement_update_and_delete_rejection() {
    let mut conn = setup_test_db();
    let f = setup_fixtures(&conn);

    let m = StockLedgerService::post_movement(
        &mut conn,
        &PostMovementInput {
            branch_id: f.branch_id.clone(),
            product_id: f.product_id.clone(),
            variant_id: None,
            location_id: f.location_id.clone(),
            bin_id: None,
            batch_id: None,
            serial_id: None,
            quantity_delta_milli: 3000,
            reason: StockMovementReason::OpeningBalance,
            source_type: None,
            source_id: None,
            user_id: None,
            idempotency_key: None,
        },
    )
    .unwrap();

    // Direct UPDATE on stock_movements must be aborted by trigger
    let update_res = conn.execute(
        "UPDATE stock_movements SET quantity_delta_milli = 99999 WHERE id = ?1",
        [&m.id],
    );
    assert!(update_res.is_err(), "Trigger prevent_stock_movements_update must block UPDATE");

    // Direct DELETE on stock_movements must be aborted by trigger
    let delete_res = conn.execute(
        "DELETE FROM stock_movements WHERE id = ?1",
        [&m.id],
    );
    assert!(delete_res.is_err(), "Trigger prevent_stock_movements_delete must block DELETE");
}

// =========================================================================
// 8. LEGACY PRE-020 STOCK & MIGRATION COMPATIBILITY
// =========================================================================

#[test]
fn test_legacy_pre_020_stock_remains_untouched_and_unallocated() {
    let conn = setup_test_db_up_to("019_locations_bins");
    let (org_id, branch_id) = create_test_org_and_branch(&conn);

    let legacy_prod_id = "prod_legacy_01";
    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active) VALUES (?1, 'Legacy Product', 5.0, 1)",
        [legacy_prod_id],
    )
    .unwrap();

    // Seed legacy pre-020 inventory (15,000 milli, 15.0 base units)
    conn.execute(
        "INSERT INTO inventory (id, branch_id, product_id, quantity, quantity_milli, updated_at)
         VALUES ('inv_legacy_01', ?1, ?2, 15.0, 15000, datetime('now'))",
        [&branch_id, legacy_prod_id],
    )
    .unwrap();

    // Apply migration 020
    apply_migrations_up_to(&conn, "020_stock_ledger_spatial");

    // Verify:
    // 1. location_inventory has 0 rows for this product (no synthetic locations invented)
    let loc_inv_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM location_inventory WHERE product_id = ?1",
            [legacy_prod_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(loc_inv_count, 0);

    // 2. StockLedgerService reports unallocated = 15,000 milli
    let balance = StockLedgerService::get_balance(&conn, &branch_id, legacy_prod_id, None).unwrap();
    assert_eq!(balance.aggregate_quantity_milli, 15000);
    assert_eq!(balance.allocated_spatial_milli, 0);
    assert_eq!(balance.unallocated_milli, 15000);
}

// =========================================================================
// 9. SERIALIZED CONCURRENCY WITH ARC<MUTEX<CONNECTION>>
// =========================================================================

/// NOTE ON CONCURRENCY GUARANTEES:
/// This test validates serialized application access and idempotency behavior using
/// Arc<Mutex<Connection>>. SQLite in-memory mode operates on a single connection.
/// This test demonstrates that concurrent application threads attempting duplicate
/// requests with the same idempotency key are safely serialized by application locking
/// and result in identical idempotent returns without duplicate database mutations.
/// It does NOT claim or prove OS-level multi-process SQLite WAL file-lock race resolution.
#[test]
fn test_serialized_application_concurrency_with_arc_mutex() {
    let conn = setup_test_db();
    let f = setup_fixtures(&conn);

    let shared_conn = Arc::new(Mutex::new(conn));
    let mut handles = Vec::new();

    let input = PostMovementInput {
        branch_id: f.branch_id.clone(),
        product_id: f.product_id.clone(),
        variant_id: None,
        location_id: f.location_id.clone(),
        bin_id: None,
        batch_id: None,
        serial_id: None,
        quantity_delta_milli: 10000,
        reason: StockMovementReason::OpeningBalance,
        source_type: None,
        source_id: None,
        user_id: None,
        idempotency_key: Some("concurrent-idem-key-99".to_string()),
    };

    // Spawn 4 threads attempting to submit the same idempotent request concurrently
    for _ in 0..4 {
        let conn_clone = Arc::clone(&shared_conn);
        let input_clone = input.clone();
        handles.push(thread::spawn(move || {
            let mut conn_guard = conn_clone.lock().unwrap();
            StockLedgerService::post_movement(&mut conn_guard, &input_clone)
        }));
    }

    let mut movement_ids = Vec::new();
    for h in handles {
        let res = h.join().unwrap().expect("thread post movement");
        movement_ids.push(res.id);
    }

    // All threads must have received the exact same movement ID (idempotent result replay)
    let first_id = &movement_ids[0];
    for id in &movement_ids {
        assert_eq!(id, first_id);
    }

    let conn_guard = shared_conn.lock().unwrap();
    let movement_count: i64 = conn_guard
        .query_row("SELECT COUNT(*) FROM stock_movements", [], |r| r.get(0))
        .unwrap();
    assert_eq!(movement_count, 1);

    let bal = StockLedgerService::get_balance(&conn_guard, &f.branch_id, &f.product_id, None).unwrap();
    assert_eq!(bal.aggregate_quantity_milli, 10000);
}
