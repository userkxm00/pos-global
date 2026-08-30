// Comprehensive unit, repository, invariant, and migration tests for F2.03 Barcode Management.

use crate::auth::middleware::{require_permission, require_session, AuthMiddlewareError};
use crate::barcode::{
    add_product_barcode, calculate_gs1_check_digit, detect_symbology, generate_internal_ean13,
    get_barcode_by_id, get_product_by_barcode, list_product_barcodes, reassign_product_barcode,
    reconcile_catalog_barcode_mirrors, remove_product_barcode, set_primary_barcode,
    validate_barcode_symbology, verify_catalog_barcode_integrity, verify_gs1_check_digit,
    AddBarcodeRequest, BarcodeError, BarcodeSymbology,
};
use crate::permission::Permission;
use crate::product::{create_product, get_product, CreateProductInput};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db, setup_test_db_up_to,
};
use crate::user::session::create_local_session;
use rusqlite::params;

fn make_test_product(conn: &rusqlite::Connection, name: &str, barcode: Option<&str>) -> String {
    let input = CreateProductInput {
        name: name.to_string(),
        description: None,
        category_id: None,
        sku: None,
        barcode: barcode.map(ToString::to_string),
        product_type: None,
        base_price_minor: 1000,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    let product = create_product(conn, input).expect("product created");
    product.id
}

// =========================================================================
// 1. GS1 MODULO-10 & SYMBOLOGY TESTS
// =========================================================================

#[test]
fn test_gs1_modulo10_calculation_and_validation() {
    // EAN-13 examples:
    // 613123456789 -> check digit 3
    let check1 = calculate_gs1_check_digit("613123456789").expect("calc check digit");
    assert_eq!(check1, 3);
    assert!(verify_gs1_check_digit("6131234567893").is_ok());

    // 400638133393 -> check digit 1
    let check2 = calculate_gs1_check_digit("400638133393").expect("calc check digit");
    assert_eq!(check2, 1);
    assert!(verify_gs1_check_digit("4006381333931").is_ok());

    // EAN-8 example:
    // 9638507 -> check digit 4
    let check3 = calculate_gs1_check_digit("9638507").expect("calc check digit");
    assert_eq!(check3, 4);
    assert!(verify_gs1_check_digit("96385074").is_ok());

    // UPC-A example:
    // 01234567890 -> check digit 5
    let check4 = calculate_gs1_check_digit("01234567890").expect("calc check digit");
    assert_eq!(check4, 5);
    assert!(verify_gs1_check_digit("012345678905").is_ok());

    // Invalid check digits
    let err = verify_gs1_check_digit("6131234567899").unwrap_err();
    assert!(matches!(
        err,
        BarcodeError::InvalidCheckDigit {
            expected: 3,
            actual: 9
        }
    ));

    // Multibyte and invalid character handling
    let utf8_err = verify_gs1_check_digit("61312345678é").unwrap_err();
    assert!(matches!(utf8_err, BarcodeError::Validation(_)));
}

#[test]
fn test_leading_zero_preservation_across_all_symbologies() {
    let leading_zero_ean = "0123456789012";
    let check = calculate_gs1_check_digit(&leading_zero_ean[..12]).expect("calc check");
    let full_ean = format!("{}{check}", &leading_zero_ean[..12]);
    assert!(full_ean.starts_with('0'));

    let validated = validate_barcode_symbology(&full_ean, BarcodeSymbology::Ean13).expect("valid");
    assert_eq!(validated, full_ean);
    assert!(validated.starts_with('0'));
}

#[test]
fn test_symbology_validation_and_detection() {
    assert_eq!(detect_symbology("4006381333931"), BarcodeSymbology::Ean13);
    assert_eq!(detect_symbology("96385074"), BarcodeSymbology::Ean8);
    assert_eq!(detect_symbology("012345678905"), BarcodeSymbology::UpcA);
    assert_eq!(detect_symbology("CODE-39-OK"), BarcodeSymbology::Code39);
    assert_eq!(
        detect_symbology("Code128_Valid!"),
        BarcodeSymbology::Code128
    );

    assert!(validate_barcode_symbology("4006381333931", BarcodeSymbology::Ean13).is_ok());
    assert!(validate_barcode_symbology("96385074", BarcodeSymbology::Ean8).is_ok());
    assert!(validate_barcode_symbology("012345678905", BarcodeSymbology::UpcA).is_ok());
    assert!(validate_barcode_symbology("CODE-39-OK", BarcodeSymbology::Code39).is_ok());
    assert!(validate_barcode_symbology("Custom-BC-1234", BarcodeSymbology::Custom).is_ok());

    // Reject non-numeric in EAN-13
    assert!(validate_barcode_symbology("400638133393A", BarcodeSymbology::Ean13).is_err());
    // Reject invalid length
    assert!(validate_barcode_symbology("4006381333", BarcodeSymbology::Ean13).is_err());
}

// =========================================================================
// 2. DATABASE INVARIANTS & REPOSITORY TESTS
// =========================================================================

#[test]
fn test_database_blocks_second_active_primary_for_same_product() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Single Primary Item", None);

    // 1. Insert first active primary barcode
    conn.execute(
        "INSERT INTO product_barcodes (id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at)
         VALUES ('bc-1', ?1, '4006381333931', 'EAN13', 1, 1, datetime('now'), datetime('now'))",
        params![pid],
    )
    .expect("first primary inserted");

    // 2. Attempting to insert a second active primary for the same product must fail with SQLite constraint violation
    let res = conn.execute(
        "INSERT INTO product_barcodes (id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at)
         VALUES ('bc-2', ?1, '012345678905', 'UPCA', 1, 1, datetime('now'), datetime('now'))",
        params![pid],
    );

    assert!(
        res.is_err(),
        "Database partial unique index must block second active primary row for same product"
    );
}

#[test]
fn test_database_blocks_primary_flag_on_inactive_barcode() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Inactive Test Item", None);

    // Attempting to insert an inactive barcode with is_primary = 1 must violate CHECK constraint
    let res = conn.execute(
        "INSERT INTO product_barcodes (id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at)
         VALUES ('bc-bad-check', ?1, '96385074', 'EAN8', 1, 0, datetime('now'), datetime('now'))",
        params![pid],
    );

    assert!(
        res.is_err(),
        "Database CHECK constraint must forbid is_primary = 1 when is_active = 0"
    );
}

#[test]
fn test_multi_barcode_alias_registration_and_primary_promotion() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Multi-Barcode Item", None);

    // 1. Add first barcode as primary
    let bc1 = add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: pid.clone(),
            barcode: "4006381333931".to_string(),
            symbology: Some(BarcodeSymbology::Ean13),
            is_primary: Some(true),
        },
    )
    .expect("bc1 added as primary");
    assert!(bc1.is_primary);

    let prod1 = get_product(&conn, &pid).unwrap().unwrap();
    assert_eq!(prod1.barcode.as_deref(), Some("4006381333931"));

    // 2. Add second barcode as secondary alias
    let bc2 = add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: pid.clone(),
            barcode: "96385074".to_string(),
            symbology: Some(BarcodeSymbology::Ean8),
            is_primary: Some(false),
        },
    )
    .expect("bc2 added as secondary");
    assert!(!bc2.is_primary);

    // Legacy mirror remains bc1
    let prod2 = get_product(&conn, &pid).unwrap().unwrap();
    assert_eq!(prod2.barcode.as_deref(), Some("4006381333931"));

    // 3. Promote bc2 to primary
    let promoted = set_primary_barcode(&conn, &pid, &bc2.id).expect("bc2 promoted");
    assert!(promoted.is_primary);

    // bc1 is now demoted
    let bc1_updated = get_barcode_by_id(&conn, &bc1.id).unwrap().unwrap();
    assert!(!bc1_updated.is_primary);

    // Legacy mirror is updated to bc2
    let prod3 = get_product(&conn, &pid).unwrap().unwrap();
    assert_eq!(prod3.barcode.as_deref(), Some("96385074"));
}

#[test]
fn test_reassign_barcode_between_products() {
    let conn = setup_test_db();
    let p1 = make_test_product(&conn, "Product 1", None);
    let p2 = make_test_product(&conn, "Product 2", None);

    let bc = add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: p1.clone(),
            barcode: "4006381333931".to_string(),
            symbology: Some(BarcodeSymbology::Ean13),
            is_primary: Some(true),
        },
    )
    .expect("bc added to p1");

    // Reassign barcode from p1 to p2 as primary
    let reassigned = reassign_product_barcode(&conn, &bc.id, &p2, true).expect("reassigned");
    assert_eq!(reassigned.product_id, p2);
    assert!(reassigned.is_primary);

    // p1 legacy mirror is cleared
    let p1_fetched = get_product(&conn, &p1).unwrap().unwrap();
    assert_eq!(p1_fetched.barcode, None);

    // p2 legacy mirror is set
    let p2_fetched = get_product(&conn, &p2).unwrap().unwrap();
    assert_eq!(p2_fetched.barcode.as_deref(), Some("4006381333931"));
}

#[test]
fn test_remove_barcode_updates_mirror_and_promotes_next_active() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Removal Test Item", None);

    let bc1 = add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: pid.clone(),
            barcode: "4006381333931".to_string(),
            symbology: Some(BarcodeSymbology::Ean13),
            is_primary: Some(true),
        },
    )
    .expect("bc1 added");

    let bc2 = add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: pid.clone(),
            barcode: "96385074".to_string(),
            symbology: Some(BarcodeSymbology::Ean8),
            is_primary: Some(false),
        },
    )
    .expect("bc2 added");

    // Remove bc1 (which was primary)
    remove_product_barcode(&conn, &bc1.id).expect("bc1 removed");

    // bc2 is automatically promoted to primary and legacy mirror is updated
    let bc2_updated = get_barcode_by_id(&conn, &bc2.id).unwrap().unwrap();
    assert!(bc2_updated.is_primary);

    let prod = get_product(&conn, &pid).unwrap().unwrap();
    assert_eq!(prod.barcode.as_deref(), Some("96385074"));

    // Remove bc2 as well
    remove_product_barcode(&conn, &bc2.id).expect("bc2 removed");
    let prod_empty = get_product(&conn, &pid).unwrap().unwrap();
    assert_eq!(prod_empty.barcode, None);
}

// =========================================================================
// 3. READ-ONLY LOOKUP & EXPLICIT RECONCILIATION TESTS
// =========================================================================

#[test]
fn test_get_product_by_barcode_does_not_mutate_database() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Lookup Test Item", None);

    add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: pid.clone(),
            barcode: "4006381333931".to_string(),
            symbology: Some(BarcodeSymbology::Ean13),
            is_primary: Some(true),
        },
    )
    .expect("barcode added");

    // Lookup
    let result = get_product_by_barcode(&conn, "4006381333931")
        .expect("lookup succeeds")
        .expect("found");

    assert_eq!(result.0.id, pid);
    assert!(result.1.is_some());
    assert_eq!(result.1.unwrap().barcode, "4006381333931");
}

#[test]
fn test_verify_and_reconcile_catalog_barcode_mirrors() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Desynced Item", None);

    let bc = add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: pid.clone(),
            barcode: "4006381333931".to_string(),
            symbology: Some(BarcodeSymbology::Ean13),
            is_primary: Some(true),
        },
    )
    .expect("barcode added");

    // Manually desync legacy products.barcode to simulate an anomaly
    conn.execute(
        "UPDATE products SET barcode = 'STALE-BARCODE' WHERE id = ?1",
        params![pid],
    )
    .unwrap();

    // 1. Diagnostic verification detects mismatch without modifying anything
    let mismatches = verify_catalog_barcode_integrity(&conn).expect("audit succeeds");
    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].product_id, pid);
    assert_eq!(
        mismatches[0].legacy_mirror.as_deref(),
        Some("STALE-BARCODE")
    );
    assert_eq!(
        mismatches[0].canonical_primary.as_deref(),
        Some("4006381333931")
    );

    // Verify it was NOT mutated by the audit
    let prod_stale = get_product(&conn, &pid).unwrap().unwrap();
    assert_eq!(prod_stale.barcode.as_deref(), Some("STALE-BARCODE"));

    // 2. Explicit reconciliation command repairs the discrepancy
    let repaired = reconcile_catalog_barcode_mirrors(&conn).expect("reconcile succeeds");
    assert_eq!(repaired, 1);

    let prod_fixed = get_product(&conn, &pid).unwrap().unwrap();
    assert_eq!(prod_fixed.barcode.as_deref(), Some("4006381333931"));

    let empty_audit = verify_catalog_barcode_integrity(&conn).expect("audit succeeds");
    assert!(empty_audit.is_empty());
}

// =========================================================================
// 4. INTERNAL EAN-13 GENERATION & AUTHORIZATION TESTS
// =========================================================================

#[test]
fn test_internal_ean13_generator_validity_and_retry() {
    let conn = setup_test_db();

    let ean1 = generate_internal_ean13(&conn, Some("200")).expect("gen ean1");
    assert_eq!(ean1.len(), 13);
    assert!(ean1.starts_with("200"));
    assert!(verify_gs1_check_digit(&ean1).is_ok());

    let ean2 = generate_internal_ean13(&conn, Some("200")).expect("gen ean2");
    assert_ne!(ean1, ean2);
    assert!(ean2.starts_with("200"));
    assert!(verify_gs1_check_digit(&ean2).is_ok());

    // Rejection of out-of-range prefix
    let err = generate_internal_ean13(&conn, Some("100")).unwrap_err();
    assert!(matches!(err, BarcodeError::Validation(_)));
}

#[test]
fn test_barcode_and_sku_mutations_require_products_manage() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier User",
        Some("cashier_bc_auth"),
        None,
        None,
        "cashier",
    )
    .expect("cashier created");

    let cashier_session =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("session");

    let perm_res = require_permission(&conn, &cashier_session.id, Permission::ProductsManage);
    assert!(
        matches!(perm_res, Err(AuthMiddlewareError::PermissionDenied { .. })),
        "Cashier must be denied ProductsManage for barcode mutations"
    );

    let read_res = require_session(&conn, &cashier_session.id);
    assert!(
        read_res.is_ok(),
        "Cashier must be allowed to read barcodes and catalog"
    );
}

#[test]
fn test_archived_barcode_reuse_and_soft_delete_lifecycle() {
    let conn = setup_test_db();
    let p1_id = make_test_product(&conn, "Product A", Some("REUSE-BARCODE-123"));

    // Verify p1 has barcode registered as primary
    let p1 = get_product(&conn, &p1_id).unwrap().unwrap();
    assert_eq!(p1.barcode.as_deref(), Some("REUSE-BARCODE-123"));

    let p1_barcodes = list_product_barcodes(&conn, &p1_id, false).unwrap();
    assert_eq!(p1_barcodes.len(), 1);
    assert_eq!(p1_barcodes[0].barcode, "REUSE-BARCODE-123");

    // Soft delete Product A
    crate::product::delete_product(&conn, &p1_id).expect("delete p1");

    // Verify p1 legacy mirror is cleared and barcodes deactivated
    let p1_deleted = get_product(&conn, &p1_id).unwrap().unwrap();
    assert!(!p1_deleted.is_active);
    assert_eq!(p1_deleted.barcode, None);

    let p1_active_bcs = list_product_barcodes(&conn, &p1_id, false).unwrap();
    assert!(p1_active_bcs.is_empty());

    let p1_all_bcs = list_product_barcodes(&conn, &p1_id, true).unwrap();
    assert_eq!(p1_all_bcs.len(), 1);
    assert!(!p1_all_bcs[0].is_active);

    // Product B can now reuse "REUSE-BARCODE-123" without UNIQUE constraint collision
    let p2_id = make_test_product(&conn, "Product B", Some("REUSE-BARCODE-123"));
    let p2 = get_product(&conn, &p2_id).unwrap().unwrap();
    assert_eq!(p2.barcode.as_deref(), Some("REUSE-BARCODE-123"));

    // get_product_by_barcode resolves directly to Product B
    let found = get_product_by_barcode(&conn, "REUSE-BARCODE-123")
        .unwrap()
        .unwrap();
    assert_eq!(found.0.id, p2_id);
}

#[test]
fn test_validation_errors_on_empty_and_whitespace_inputs() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Validation Item", None);

    assert!(list_product_barcodes(&conn, "   ", false).is_err());
    assert_eq!(get_barcode_by_id(&conn, "   ").unwrap(), None);
    assert!(remove_product_barcode(&conn, "   ").is_err());
    assert!(set_primary_barcode(&conn, &pid, "   ").is_err());
    assert!(set_primary_barcode(&conn, "   ", "bc-1").is_err());
    assert!(reassign_product_barcode(&conn, "   ", &pid, false).is_err());
    assert!(reassign_product_barcode(&conn, "bc-1", "   ", false).is_err());
    assert!(add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: "   ".into(),
            barcode: "12345".into(),
            symbology: None,
            is_primary: None,
        }
    )
    .is_err());
    assert!(add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: pid,
            barcode: "   ".into(),
            symbology: None,
            is_primary: None,
        }
    )
    .is_err());
}

#[test]
fn test_real_011_to_012_migration_transition() {
    // 1. Initialize test database exactly at migration 011 (prior to Migration 012)
    let conn = setup_test_db_up_to("011_categories_brands_manufacturers");

    let pre_migrations_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("query migrations ledger");
    assert_eq!(pre_migrations_count, 11);

    let pre_barcodes_table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'product_barcodes'",
            [],
            |row| row.get(0),
        )
        .expect("table check");
    assert_eq!(pre_barcodes_table_exists, 0);

    let pre_sku_seq_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'sku_sequences'",
            [],
            |row| row.get(0),
        )
        .expect("table check");
    assert_eq!(pre_sku_seq_exists, 0);

    // 2. Seed realistic legacy `products` data BEFORE Migration 012
    let prod_a = uuid::Uuid::new_v4().to_string();
    let prod_b = uuid::Uuid::new_v4().to_string();
    let prod_c = uuid::Uuid::new_v4().to_string();
    let prod_d1 = uuid::Uuid::new_v4().to_string();
    let prod_d2 = uuid::Uuid::new_v4().to_string();
    let prod_e = uuid::Uuid::new_v4().to_string();

    // A. Active product with normal valid barcode
    conn.execute(
        "INSERT INTO products (id, name, barcode, is_active, base_price, created_at, updated_at)
         VALUES (?1, 'Active Normal Product', '4006381333931', 1, 1000, '2026-01-01 10:00:00', '2026-01-01 10:00:00')",
        params![prod_a],
    )
    .expect("seed prod_a");

    // B. Active product with surrounding whitespace in legacy barcode
    conn.execute(
        "INSERT INTO products (id, name, barcode, is_active, base_price, created_at, updated_at)
         VALUES (?1, 'Active Whitespace Product', '   6131234567893   ', 1, 1500, '2026-01-02 10:00:00', '2026-01-02 10:00:00')",
        params![prod_b],
    )
    .expect("seed prod_b");

    // C. Inactive (soft-deleted) product with legacy barcode
    conn.execute(
        "INSERT INTO products (id, name, barcode, is_active, base_price, created_at, updated_at)
         VALUES (?1, 'Inactive Legacy Product', '96385074', 0, 2000, '2026-01-03 10:00:00', '2026-01-03 10:00:00')",
        params![prod_c],
    )
    .expect("seed prod_c");

    // D. Normalized collision: earlier created active product vs later created active product
    conn.execute(
        "INSERT INTO products (id, name, barcode, is_active, base_price, created_at, updated_at)
         VALUES (?1, 'Collision Winner (Earlier)', 'COLLIDE-123', 1, 500, '2026-01-04 09:00:00', '2026-01-04 09:00:00')",
        params![prod_d1],
    )
    .expect("seed prod_d1");

    conn.execute(
        "INSERT INTO products (id, name, barcode, is_active, base_price, created_at, updated_at)
         VALUES (?1, 'Collision Loser (Later)', 'collide-123', 1, 600, '2026-01-04 10:00:00', '2026-01-04 10:00:00')",
        params![prod_d2],
    )
    .expect("seed prod_d2");

    // E. Active product with NULL barcode
    conn.execute(
        "INSERT INTO products (id, name, barcode, is_active, base_price, created_at, updated_at)
         VALUES (?1, 'Active Null Barcode Product', NULL, 1, 800, '2026-01-05 10:00:00', '2026-01-05 10:00:00')",
        params![prod_e],
    )
    .expect("seed prod_e");

    // 3. Apply Migration 012 using the REAL production migration runner
    crate::db::init_database(&conn).expect("migration runner applies Migration 012 cleanly");

    // Verify migration ledger state
    let post_migrations_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("query migrations ledger");
    assert_eq!(post_migrations_count, 12);

    // Verify migration repeatability (re-running init_database is a clean no-op)
    crate::db::init_database(&conn).expect("repeatable migration init must succeed");
    let repeat_migrations_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("query migrations ledger");
    assert_eq!(repeat_migrations_count, 12);

    // 4. Verify new schema elements created by Migration 012
    for table in ["sku_sequences", "product_barcodes"] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("table existence check");
        assert_eq!(exists, 1, "expected table {table} to exist");
    }

    for index in [
        "idx_products_sku_active",
        "idx_product_barcodes_product",
        "idx_product_barcodes_lookup",
        "idx_product_barcodes_unique_active",
        "idx_product_barcodes_one_active_primary",
    ] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [index],
                |row| row.get(0),
            )
            .expect("index existence check");
        assert_eq!(exists, 1, "expected index {index} to exist");
    }

    // Verify products.sku column exists
    let sku_col_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('products') WHERE name = 'sku'",
            [],
            |row| row.get(0),
        )
        .expect("sku column check");
    assert_eq!(sku_col_exists, 1);

    // 5. Verify actual post-migration data states

    // A. Active Normal Product: backfilled to canonical registry and mirror preserved
    let prod_a_db = get_product(&conn, &prod_a).unwrap().unwrap();
    assert_eq!(prod_a_db.barcode.as_deref(), Some("4006381333931"));
    let prod_a_bcs = list_product_barcodes(&conn, &prod_a, false).unwrap();
    assert_eq!(prod_a_bcs.len(), 1);
    assert_eq!(prod_a_bcs[0].barcode, "4006381333931");
    assert!(prod_a_bcs[0].is_primary);
    assert!(prod_a_bcs[0].is_active);
    assert_eq!(prod_a_bcs[0].symbology, BarcodeSymbology::Unknown);

    // B. Active Whitespace Product: surrounding whitespace trimmed in both registry and mirror
    let prod_b_db = get_product(&conn, &prod_b).unwrap().unwrap();
    assert_eq!(prod_b_db.barcode.as_deref(), Some("6131234567893"));
    let prod_b_bcs = list_product_barcodes(&conn, &prod_b, false).unwrap();
    assert_eq!(prod_b_bcs.len(), 1);
    assert_eq!(prod_b_bcs[0].barcode, "6131234567893");
    assert!(prod_b_bcs[0].is_primary);
    assert!(prod_b_bcs[0].is_active);

    // C. Inactive Product: legacy mirror cleared from products; archived as inactive/non-primary in registry
    let prod_c_db = get_product(&conn, &prod_c).unwrap().unwrap();
    assert_eq!(prod_c_db.barcode, None);
    let prod_c_active_bcs = list_product_barcodes(&conn, &prod_c, false).unwrap();
    assert_eq!(prod_c_active_bcs.len(), 0);
    let prod_c_all_bcs = list_product_barcodes(&conn, &prod_c, true).unwrap();
    assert_eq!(prod_c_all_bcs.len(), 1);
    assert_eq!(prod_c_all_bcs[0].barcode, "96385074");
    assert!(!prod_c_all_bcs[0].is_primary);
    assert!(!prod_c_all_bcs[0].is_active);

    // D. Normalized Collision: earlier record retains canonical registry & mirror; later colliding record cleared
    let prod_d1_db = get_product(&conn, &prod_d1).unwrap().unwrap();
    assert_eq!(prod_d1_db.barcode.as_deref(), Some("COLLIDE-123"));
    let prod_d1_bcs = list_product_barcodes(&conn, &prod_d1, false).unwrap();
    assert_eq!(prod_d1_bcs.len(), 1);
    assert_eq!(prod_d1_bcs[0].barcode, "COLLIDE-123");
    assert!(prod_d1_bcs[0].is_primary);
    assert!(prod_d1_bcs[0].is_active);

    let prod_d2_db = get_product(&conn, &prod_d2).unwrap().unwrap();
    assert_eq!(
        prod_d2_db.barcode, None,
        "colliding later product mirror must be cleared"
    );
    let prod_d2_bcs = list_product_barcodes(&conn, &prod_d2, true).unwrap();
    assert_eq!(
        prod_d2_bcs.len(),
        0,
        "colliding later record has no duplicate active registry entry"
    );

    // E. Active Null Barcode Product: remains NULL with no registry entries
    let prod_e_db = get_product(&conn, &prod_e).unwrap().unwrap();
    assert_eq!(prod_e_db.barcode, None);
    let prod_e_bcs = list_product_barcodes(&conn, &prod_e, true).unwrap();
    assert_eq!(prod_e_bcs.len(), 0);

    // 6. Verify repository lookup operations against migrated database
    let (found_a, bc_a) = get_product_by_barcode(&conn, "4006381333931")
        .unwrap()
        .expect("lookup prod_a");
    assert_eq!(found_a.id, prod_a);
    assert_eq!(bc_a.unwrap().barcode, "4006381333931");

    let (found_b, bc_b) = get_product_by_barcode(&conn, "6131234567893")
        .unwrap()
        .expect("lookup prod_b");
    assert_eq!(found_b.id, prod_b);
    assert_eq!(bc_b.unwrap().barcode, "6131234567893");

    let (found_d1, bc_d1) = get_product_by_barcode(&conn, "collide-123")
        .unwrap()
        .expect("case-insensitive lookup prod_d1");
    assert_eq!(found_d1.id, prod_d1);
    assert_eq!(bc_d1.unwrap().barcode, "COLLIDE-123");
}

#[test]
fn test_primary_barcode_reassignment_ordering_preserves_unique_active_primary_invariant() {
    let conn = setup_test_db();
    let p1_id = make_test_product(&conn, "Product 1", Some("P1-PRIMARY"));
    let _p1_sec = add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: p1_id.clone(),
            barcode: "P1-SECONDARY".into(),
            symbology: None,
            is_primary: Some(false),
        },
    )
    .expect("p1 secondary added");

    let p2_id = make_test_product(&conn, "Product 2", None);

    let p1_primary_row = get_product_by_barcode(&conn, "P1-PRIMARY")
        .unwrap()
        .unwrap()
        .1
        .unwrap();

    let reassigned = reassign_product_barcode(&conn, &p1_primary_row.id, &p2_id, true)
        .expect("reassignment of active primary must succeed without constraint collision");

    assert_eq!(reassigned.product_id, p2_id);
    assert!(reassigned.is_primary);

    let p1_fetched = get_product(&conn, &p1_id).unwrap().unwrap();
    assert_eq!(p1_fetched.barcode.as_deref(), Some("P1-SECONDARY"));

    let p2_fetched = get_product(&conn, &p2_id).unwrap().unwrap();
    assert_eq!(p2_fetched.barcode.as_deref(), Some("P1-PRIMARY"));
}

#[test]
fn test_canonical_barcode_lookup_resolves_active_secondary_barcode() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Barcode User",
        Some("bc_user"),
        None,
        None,
        "cashier",
    );
    let session = create_local_session(&conn, &user.id, &branch_id, "pin", None).expect("session");

    let prod_id = make_test_product(&conn, "Multi-Barcode Item", Some("PRIMARY-999"));

    let secondary = add_product_barcode(
        &conn,
        AddBarcodeRequest {
            product_id: prod_id.clone(),
            barcode: "SECONDARY-ALIAS-888".into(),
            symbology: None,
            is_primary: Some(false),
        },
    )
    .expect("secondary barcode created");
    assert!(!secondary.is_primary);
    assert!(secondary.is_active);

    // Verify canonical service lookup resolves secondary barcode alias
    let (found_prod, found_bc) = get_product_by_barcode(&conn, "SECONDARY-ALIAS-888")
        .unwrap()
        .expect("secondary barcode resolved");

    assert_eq!(found_prod.id, prod_id);
    assert_eq!(found_prod.name, "Multi-Barcode Item");
    let bc = found_bc.expect("associated barcode metadata returned");
    assert_eq!(bc.barcode, "SECONDARY-ALIAS-888");
    assert!(!bc.is_primary);
    assert!(bc.is_active);

    // Verify session authorization for barcode read
    assert!(crate::commands::authorize_catalog_read(&conn, &session.id).is_ok());
}
