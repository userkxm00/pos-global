// Comprehensive unit, integration, migration, and contract tests for F2.07 Batches, Expiry & FEFO.
// ADR-0009: Orthogonal capabilities, nullable expiry dates, exact integer milli quantities, and read-only FEFO planning.

use crate::batch::*;
use crate::commands::batch::*;
use crate::product::{create_product, CreateProductInput};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db, setup_test_db_up_to,
};
use crate::user::session::create_local_session;
use crate::variant::{create_variant, CreateVariantInput};
use rusqlite::{params, Connection};

fn make_test_product(conn: &Connection, name: &str, requires_expiry: bool) -> String {
    let p = create_product(
        conn,
        CreateProductInput {
            name: name.to_string(),
            description: None,
            category_id: None,
            sku: None,
            barcode: None,
            product_type: Some("simple".to_string()),
            base_price_minor: 1000,
            cost_price_minor: None,
            unit_type: Some("piece".to_string()),
            requires_expiry: Some(requires_expiry),
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("create test product");
    p.id
}

fn add_product_capability(conn: &Connection, product_id: &str, cap_code: &str) {
    conn.execute(
        "INSERT OR REPLACE INTO product_capabilities (product_id, capability_id, enabled)
         SELECT ?1, id, 1 FROM capabilities WHERE code = ?2",
        params![product_id, cap_code],
    )
    .expect("add product capability");
}

fn make_test_variant(conn: &Connection, product_id: &str, sku: &str) -> String {
    let v = create_variant(
        conn,
        &CreateVariantInput {
            product_id: product_id.to_string(),
            sku: Some(sku.to_string()),
            barcode: None,
            price_override_minor: None,
            cost_override_minor: None,
            attribute_value_ids: vec![],
        },
    )
    .expect("create test variant");
    v.id
}

// =========================================================================
// 1. MIGRATION TESTS
// =========================================================================

#[test]
fn test_migration_016_fresh_application() {
    let conn = setup_test_db();

    // Verify product_batches schema
    let col_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('product_batches')
             WHERE name IN ('quantity_milli', 'cost_price_minor', 'variant_id', 'status', 'manufactured_date', 'expiry_date')",
            [],
            |row| row.get(0),
        )
        .expect("query pragma_table_info");
    assert_eq!(col_count, 6);

    // Verify legacy quantity REAL is removed
    let legacy_qty_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('product_batches') WHERE name = 'quantity'",
            [],
            |row| {
                let count: i64 = row.get(0)?;
                Ok(count > 0)
            },
        )
        .expect("check legacy quantity");
    assert!(
        !legacy_qty_exists,
        "Legacy quantity REAL must be dropped in migration 016"
    );

    // Verify expiry_date is nullable in schema
    let expiry_notnull: bool = conn
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('product_batches') WHERE name = 'expiry_date'",
            [],
            |row| {
                let notnull: i64 = row.get(0)?;
                Ok(notnull != 0)
            },
        )
        .expect("check expiry_date notnull");
    assert!(
        !expiry_notnull,
        "expiry_date must be nullable after migration 016"
    );
}

#[test]
fn test_migration_upgrade_015_to_016_exact_legacy_data() {
    let conn = setup_test_db_up_to("015_weighted_products");
    let (org_id, branch_id) = create_test_org_and_branch(&conn);
    let _ = org_id;
    let product_id = make_test_product(&conn, "Legacy Batch Product", true);

    // Pre-insert exact legacy batches into 001 product_batches
    conn.execute(
        "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity, expiry_date, received_at)
         VALUES
         ('b1', ?1, ?2, 'BATCH-1000', 1.000, '2027-01-01', '2026-01-01 10:00:00'),
         ('b2', ?1, ?2, 'BATCH-1250', 1.250, '2027-02-01', '2026-01-02 10:00:00'),
         ('b3', ?1, ?2, 'BATCH-1001', 1.001, '2027-03-01', '2026-01-03 10:00:00'),
         ('b4', ?1, ?2, 'BATCH-1234', 1.234, '2027-04-01', '2026-01-04 10:00:00')",
        params![product_id, branch_id],
    )
    .expect("insert legacy batches");

    // Apply migration 016
    let sql_016 = include_str!("../db/migrations/016_batches_and_expiry.sql");
    conn.execute_batch(sql_016).expect("apply migration 016");

    // Verify data conversion
    let b1: (i64, String) = conn
        .query_row(
            "SELECT quantity_milli, expiry_date FROM product_batches WHERE id = 'b1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("query b1");
    assert_eq!(b1.0, 1000);
    assert_eq!(b1.1, "2027-01-01");

    let b2_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM product_batches WHERE id = 'b2'",
            [],
            |r| r.get(0),
        )
        .expect("query b2");
    assert_eq!(b2_qty, 1250);

    let b3_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM product_batches WHERE id = 'b3'",
            [],
            |r| r.get(0),
        )
        .expect("query b3");
    assert_eq!(b3_qty, 1001);

    let b4_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM product_batches WHERE id = 'b4'",
            [],
            |r| r.get(0),
        )
        .expect("query b4");
    assert_eq!(b4_qty, 1234);
}

#[test]
fn test_migration_upgrade_fails_on_inexact_fractional_precision() {
    let conn = setup_test_db_up_to("015_weighted_products");
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Inexact Batch Product", true);

    // Pre-insert inexact fractional quantity (4 decimals: 1.2345)
    conn.execute(
        "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity, expiry_date, received_at)
         VALUES ('bad1', ?1, ?2, 'BATCH-BAD', 1.2345, '2027-01-01', '2026-01-01 10:00:00')",
        params![product_id, branch_id],
    )
    .expect("insert inexact batch");

    // Migration 016 must fail closed
    let sql_016 = include_str!("../db/migrations/016_batches_and_expiry.sql");
    let result = conn.execute_batch(sql_016);
    assert!(
        result.is_err(),
        "Migration 016 must abort on inexact fractional quantity"
    );
}

#[test]
fn test_migration_upgrade_fails_on_negative_legacy_quantity() {
    let conn = setup_test_db_up_to("015_weighted_products");
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Negative Batch Product", true);

    conn.execute(
        "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity, expiry_date, received_at)
         VALUES ('neg1', ?1, ?2, 'BATCH-NEG', -0.5, '2027-01-01', '2026-01-01 10:00:00')",
        params![product_id, branch_id],
    )
    .expect("insert negative batch");

    let sql_016 = include_str!("../db/migrations/016_batches_and_expiry.sql");
    let result = conn.execute_batch(sql_016);
    assert!(
        result.is_err(),
        "Migration 016 must abort on negative quantity"
    );
}

#[test]
fn test_migration_nullable_expiry_after_rebuild() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Tile Batch", false);
    add_product_capability(&conn, &product_id, "BATCH");

    // Insert lot with NULL expiry_date
    let res = conn.execute(
        "INSERT INTO product_batches (product_id, branch_id, batch_number, quantity_milli, expiry_date)
         VALUES (?1, ?2, 'LOT-TILE-01', 50000, NULL)",
        params![product_id, branch_id],
    );
    assert!(
        res.is_ok(),
        "Rebuilt table must accept NULL expiry_date for non-perishables"
    );
}

// =========================================================================
// 2. BATCH ELIGIBILITY & ORTHOGONAL CAPABILITY TESTS
// =========================================================================

#[test]
fn test_batch_creation_requires_expiry_product() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Milk", true);

    // requires_expiry = true requires expiry_date
    let err = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "MILK-01".into(),
            quantity_milli: 10000,
            cost_price_minor: Some(150),
            manufactured_date: None,
            expiry_date: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, BatchError::Validation(msg) if msg.contains("Expiry date is mandatory")));

    // Succeeds when expiry provided
    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id,
            variant_id: None,
            batch_number: "MILK-01".into(),
            quantity_milli: 10000,
            cost_price_minor: Some(150),
            manufactured_date: None,
            expiry_date: Some("2027-06-01".into()),
        },
    )
    .expect("create batch with expiry");
    assert_eq!(batch.batch_number, "MILK-01");
    assert_eq!(batch.expiry_date.as_deref(), Some("2027-06-01"));
}

#[test]
fn test_batch_creation_batch_only_product_allows_null_expiry() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Denim Dye Lot", false);
    add_product_capability(&conn, &product_id, "BATCH");

    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id,
            variant_id: None,
            batch_number: "DYE-BLUE-44".into(),
            quantity_milli: 25000,
            cost_price_minor: Some(1200),
            manufactured_date: None,
            expiry_date: None,
        },
    )
    .expect("create non-perishable batch");
    assert_eq!(batch.batch_number, "DYE-BLUE-44");
    assert_eq!(batch.expiry_date, None);
}

#[test]
fn test_batch_creation_ineligible_product_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    // Product with no batch capability and requires_expiry = false
    let product_id = make_test_product(&conn, "Book", false);

    let err = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id,
            variant_id: None,
            batch_number: "BOOK-PRINT-1".into(),
            quantity_milli: 1000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, BatchError::IneligibleProduct(_)));
}

// =========================================================================
// 3. DATE VALIDATION & INTEGRITY TESTS
// =========================================================================

#[test]
fn test_batch_creation_invalid_date_formats() {
    assert!(validate_iso_calendar_date("2026/12/31").is_err());
    assert!(validate_iso_calendar_date("2026-13-01").is_err());
    assert!(validate_iso_calendar_date("2026-04-31").is_err()); // April has 30 days
    assert!(validate_iso_calendar_date("2025-02-29").is_err()); // 2025 is not leap
    assert!(validate_iso_calendar_date("not-a-date").is_err());
    assert!(validate_iso_calendar_date("2026-1-1").is_err());
}

#[test]
fn test_batch_creation_valid_leap_year_date() {
    assert!(validate_iso_calendar_date("2024-02-29").is_ok()); // 2024 is leap year
    assert!(validate_iso_calendar_date("2028-02-29").is_ok()); // 2028 is leap year
}

#[test]
fn test_batch_creation_manufactured_date_after_expiry_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Yogurt", true);

    let err = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id,
            variant_id: None,
            batch_number: "YOG-01".into(),
            quantity_milli: 5000,
            cost_price_minor: None,
            manufactured_date: Some("2027-05-10".into()),
            expiry_date: Some("2027-05-01".into()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, BatchError::Validation(msg) if msg.contains("strictly before expiry")));
}

// =========================================================================
// 4. BATCH NUMBER & QUANTITY BOUNDS
// =========================================================================

#[test]
fn test_batch_number_normalization_and_bounds() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Bread", true);

    // Empty batch number
    let err_empty = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "   ".into(),
            quantity_milli: 1000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .unwrap_err();
    assert!(matches!(err_empty, BatchError::Validation(msg) if msg.contains("empty")));

    // Exceeds 100 chars
    let long_num = "A".repeat(101);
    let err_long = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: long_num,
            quantity_milli: 1000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .unwrap_err();
    assert!(matches!(err_long, BatchError::Validation(msg) if msg.contains("maximum length")));

    // Negative quantity rejected
    let err_qty = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id,
            variant_id: None,
            batch_number: "BREAD-01".into(),
            quantity_milli: -500,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .unwrap_err();
    assert!(matches!(err_qty, BatchError::Validation(msg) if msg.contains("negative")));
}

// =========================================================================
// 5. UNIQUENESS & PARTIAL INDEX TESTS
// =========================================================================

#[test]
fn test_batch_duplicate_number_case_insensitive_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Cheese", true);

    create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "LOT-CHEESE-99".into(),
            quantity_milli: 1000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .expect("first batch created");

    // Case insensitive duplicate in same branch/product
    let err = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id,
            variant_id: None,
            batch_number: "lot-cheese-99".into(),
            quantity_milli: 2000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-02-01".into()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, BatchError::DuplicateBatchNumber(_)));
}

#[test]
fn test_batch_same_number_different_branch_allowed() {
    let conn = setup_test_db();
    let (org_id, branch_1) = create_test_org_and_branch(&conn);
    let branch_2 = crate::branch::create_branch(
        &conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id,
            name: "Branch 2".into(),
            code: Some("BR2".into()),
            address: None,
            phone: None,
            is_active: true,
        },
    )
    .expect("create branch 2")
    .id;

    let product_id = make_test_product(&conn, "Butter", true);

    let b1 = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_1,
            variant_id: None,
            batch_number: "BUTTER-A1".into(),
            quantity_milli: 5000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    );
    assert!(b1.is_ok());

    let b2 = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id: branch_2,
            variant_id: None,
            batch_number: "BUTTER-A1".into(),
            quantity_milli: 5000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    );
    assert!(
        b2.is_ok(),
        "Same batch number in different branches must be allowed"
    );
}

// =========================================================================
// 6. VARIANT INTEGRITY TESTS
// =========================================================================

#[test]
fn test_batch_creation_variant_mismatch_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_a = make_test_product(&conn, "Product A", true);
    let product_b = make_test_product(&conn, "Product B", true);

    let variant_b = make_test_variant(&conn, &product_b, "SKU-VAR-B");

    // Attempt to assign Variant of Product B to a batch for Product A
    let err = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_a,
            branch_id,
            variant_id: Some(variant_b),
            batch_number: "BATCH-MISMATCH".into(),
            quantity_milli: 1000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, BatchError::Validation(msg) if msg.contains("belongs to product")));
}

// =========================================================================
// 7. STATUS LIFECYCLE TESTS
// =========================================================================

#[test]
fn test_batch_status_lifecycle_valid_and_terminal_transitions() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Vaccine", true);

    let batch = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id,
            variant_id: None,
            batch_number: "VAC-001".into(),
            quantity_milli: 100,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-12-31".into()),
        },
    )
    .expect("create batch");
    assert_eq!(batch.status, BatchStatus::Active);

    // Active -> Quarantined
    let q = update_batch_status(
        &conn,
        &UpdateBatchStatusInput {
            batch_id: batch.id.clone(),
            status: BatchStatus::Quarantined,
        },
    )
    .expect("quarantine");
    assert_eq!(q.status, BatchStatus::Quarantined);

    // Quarantined -> Active
    let a = update_batch_status(
        &conn,
        &UpdateBatchStatusInput {
            batch_id: batch.id.clone(),
            status: BatchStatus::Active,
        },
    )
    .expect("release from quarantine");
    assert_eq!(a.status, BatchStatus::Active);

    // Active -> Depleted
    let d = update_batch_status(
        &conn,
        &UpdateBatchStatusInput {
            batch_id: batch.id.clone(),
            status: BatchStatus::Depleted,
        },
    )
    .expect("deplete");
    assert_eq!(d.status, BatchStatus::Depleted);

    // Depleted cannot transition back to Active (Terminal in F2.07)
    let err_reopen = update_batch_status(
        &conn,
        &UpdateBatchStatusInput {
            batch_id: batch.id.clone(),
            status: BatchStatus::Active,
        },
    )
    .unwrap_err();
    assert!(
        matches!(err_reopen, BatchError::InvalidStatusTransition(msg) if msg.contains("Depleted batches are terminal"))
    );
}

// =========================================================================
// 8. FEFO PLANNING ALGORITHM (DETERMINISTIC READ-ONLY TESTS)
// =========================================================================

#[test]
fn test_fefo_planning_disabled_for_non_fefo_product() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    // Product with requires_expiry = true but NO active FEFO capability
    let product_id = make_test_product(&conn, "Artisan Cheese", true);

    let err = plan_fefo_allocation(&conn, &branch_id, &product_id, None, 1000).unwrap_err();
    assert!(
        matches!(err, BatchError::Validation(msg) if msg.contains("does not have the FEFO capability"))
    );
}

#[test]
fn test_fefo_planning_earliest_expiry_and_multi_batch_split() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Fresh Milk 1L", true);
    add_product_capability(&conn, &product_id, "FEFO");

    // Insert 3 batches with different expiries:
    // Batch 1: Exp 2027-02-15, Qty 2000
    // Batch 2: Exp 2027-01-10, Qty 1500 (Earliest)
    // Batch 3: Exp 2027-03-01, Qty 3000
    create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "BATCH-MID".into(),
            quantity_milli: 2000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-02-15".into()),
        },
    )
    .expect("batch 1");

    create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "BATCH-EARLIEST".into(),
            quantity_milli: 1500,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-10".into()),
        },
    )
    .expect("batch 2");

    create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "BATCH-LATEST".into(),
            quantity_milli: 3000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-03-01".into()),
        },
    )
    .expect("batch 3");

    // Request 2500 milli: should allocate 1500 from BATCH-EARLIEST and 1000 from BATCH-MID
    let plan = plan_fefo_allocation(&conn, &branch_id, &product_id, None, 2500).expect("plan fefo");

    assert_eq!(plan.requested_quantity_milli, 2500);
    assert_eq!(plan.allocated_quantity_milli, 2500);
    assert_eq!(plan.shortfall_quantity_milli, 0);
    assert_eq!(plan.allocations.len(), 2);

    assert_eq!(plan.allocations[0].batch_number, "BATCH-EARLIEST");
    assert_eq!(plan.allocations[0].allocated_quantity_milli, 1500);
    assert_eq!(plan.allocations[0].remaining_batch_quantity_milli, 0);

    assert_eq!(plan.allocations[1].batch_number, "BATCH-MID");
    assert_eq!(plan.allocations[1].allocated_quantity_milli, 1000);
    assert_eq!(plan.allocations[1].remaining_batch_quantity_milli, 1000);
}

#[test]
fn test_fefo_planning_dynamically_excludes_expired_and_quarantined() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Salad Pack", true);
    add_product_capability(&conn, &product_id, "FEFO");

    // Past expiry batch (derived expired)
    conn.execute(
        "INSERT INTO product_batches (product_id, branch_id, batch_number, quantity_milli, status, expiry_date)
         VALUES (?1, ?2, 'EXPIRED-BATCH', 5000, 'active', '2020-01-01')",
        params![product_id, branch_id],
    )
    .expect("insert expired batch");

    // Quarantined batch
    let q_batch = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "QUARANTINED-BATCH".into(),
            quantity_milli: 5000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .expect("batch");
    update_batch_status(
        &conn,
        &UpdateBatchStatusInput {
            batch_id: q_batch.id,
            status: BatchStatus::Quarantined,
        },
    )
    .expect("quarantine");

    // Active valid batch
    create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "VALID-BATCH".into(),
            quantity_milli: 2000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-05-01".into()),
        },
    )
    .expect("valid batch");

    // Plan for 3000: only VALID-BATCH should be allocated (2000), leaving shortfall 1000
    let plan = plan_fefo_allocation(&conn, &branch_id, &product_id, None, 3000).expect("plan fefo");
    assert_eq!(plan.allocated_quantity_milli, 2000);
    assert_eq!(plan.shortfall_quantity_milli, 1000);
    assert_eq!(plan.allocations.len(), 1);
    assert_eq!(plan.allocations[0].batch_number, "VALID-BATCH");
}

#[test]
fn test_fefo_planning_strictly_read_only_invariant() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Butter 250g", true);
    add_product_capability(&conn, &product_id, "FEFO");

    let b = create_batch(
        &conn,
        &CreateBatchInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            batch_number: "BUTTER-B1".into(),
            quantity_milli: 5000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .expect("create batch");

    // Execute plan
    let _ = plan_fefo_allocation(&conn, &branch_id, &product_id, None, 3000).expect("fefo plan");

    // Re-query batch: quantity_milli must be strictly 5000 (no mutation)
    let current_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM product_batches WHERE id = ?1",
            params![b.id],
            |r| r.get(0),
        )
        .expect("query batch");
    assert_eq!(
        current_qty, 5000,
        "FEFO planning must be 100% read-only with zero balance mutations"
    );
}

// =========================================================================
// 9. AUTHORIZATION & TENANCY BOUNDARY TESTS
// =========================================================================

#[test]
fn test_create_batch_command_authorized_and_unauthenticated() {
    let conn = setup_test_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Organic Honey", true);

    let user = create_test_user_with_creds(&conn, &org_id, &branch_id, "manager", "test_pin");
    let session = create_local_session(&conn, &user.id, &branch_id).expect("session");

    let req = CreateBatchInput {
        product_id: product_id.clone(),
        branch_id: branch_id.clone(),
        variant_id: None,
        batch_number: "HONEY-01".into(),
        quantity_milli: 1000,
        cost_price_minor: None,
        manufactured_date: None,
        expiry_date: Some("2027-01-01".into()),
    };

    // Authenticated manager succeeds
    let b = create_product_batch_impl(&conn, &session.session_id, &req).expect("create batch");
    assert_eq!(b.batch_number, "HONEY-01");

    // Unauthenticated session fails
    let err_auth = create_product_batch_impl(&conn, "nonexistent_session", &req).unwrap_err();
    assert!(err_auth.contains("Authentication required"));
}

#[test]
fn test_get_batch_command_cross_branch_leakage_prevented() {
    let conn = setup_test_db();
    let (org_id, branch_1) = create_test_org_and_branch(&conn);
    let branch_2 = crate::branch::create_branch(
        &conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Branch 2".into(),
            code: Some("BR2".into()),
            address: None,
            phone: None,
            is_active: true,
        },
    )
    .expect("create branch 2")
    .id;

    let product_id = make_test_product(&conn, "Sugar", true);

    // Create batch in branch 2
    let b2 = create_batch(
        &conn,
        &CreateBatchInput {
            product_id,
            branch_id: branch_2.clone(),
            variant_id: None,
            batch_number: "SUGAR-BR2".into(),
            quantity_milli: 1000,
            cost_price_minor: None,
            manufactured_date: None,
            expiry_date: Some("2027-01-01".into()),
        },
    )
    .expect("batch in branch 2");

    // User session scoped strictly to branch 1
    let user = create_test_user_with_creds(&conn, &org_id, &branch_1, "cashier", "test_pin");
    let session = create_local_session(&conn, &user.id, &branch_1).expect("session");

    // Attempting to query batch from branch 2 must fail without existence leakage
    let res = get_product_batch_impl(&conn, &session.session_id, &b2.id);
    assert!(res.is_err(), "Cross-branch batch access must fail closed");
}
