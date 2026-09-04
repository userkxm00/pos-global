// Comprehensive unit, integration, migration, and contract tests for F2.09 Warranty.
// Implements ADR-0011: Lightweight core, canonical date contract, coverage evaluation, and branch tenancy.

use crate::commands::warranty::{
    calculate_warranty_expiration_impl, evaluate_warranty_coverage_impl,
    get_instance_warranty_impl, register_instance_warranty_impl, RegisterInstanceWarrantyRequest,
};
use crate::product::{create_product, CreateProductInput};
use crate::serial::{create_serial_instance, CreateSerialInput};
use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::create_local_session;
use crate::warranty::*;
use rusqlite::{params, Connection};

// =========================================================================
// TEST FIXTURES & HELPERS
// =========================================================================

fn make_test_product(
    conn: &Connection,
    name: &str,
    requires_serial: bool,
    warranty_months: Option<i32>,
) -> String {
    let p = create_product(
        conn,
        CreateProductInput {
            name: name.to_string(),
            description: None,
            category_id: None,
            sku: None,
            barcode: None,
            product_type: Some("simple".to_string()),
            base_price_minor: 99900,
            cost_price_minor: Some(50000),
            unit_type: Some("piece".to_string()),
            requires_expiry: None,
            requires_serial: Some(requires_serial),
            warranty_months,
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

fn make_test_serial_instance(
    conn: &Connection,
    product_id: &str,
    branch_id: &str,
    serial_number: Option<&str>,
    imei: Option<&str>,
    asset_tag: Option<&str>,
) -> String {
    let input = CreateSerialInput {
        product_id: product_id.to_string(),
        branch_id: branch_id.to_string(),
        variant_id: None,
        serial_number: serial_number.map(String::from),
        imei: imei.map(String::from),
        asset_tag: asset_tag.map(String::from),
        cost_price_minor: None,
    };
    let inst = create_serial_instance(conn, &input).expect("create serial instance");
    inst.id
}

// =========================================================================
// A. PRODUCT WARRANTY VALIDATION TESTS
// =========================================================================

#[test]
fn test_product_warranty_months_validation() {
    assert!(validate_warranty_months(None).is_ok());
    assert!(validate_warranty_months(Some(0)).is_ok());
    assert!(validate_warranty_months(Some(12)).is_ok());
    assert!(validate_warranty_months(Some(24)).is_ok());

    let err = validate_warranty_months(Some(-1)).unwrap_err();
    assert!(err.to_string().contains("cannot be negative"));

    // Verify create_product rejects negative warranty_months
    let conn = setup_test_db();
    let res = create_product(
        &conn,
        CreateProductInput {
            name: "Negative Warranty Product".to_string(),
            description: None,
            category_id: None,
            sku: None,
            barcode: None,
            product_type: Some("simple".to_string()),
            base_price_minor: 1000,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: Some(-3),
            custom_attributes: None,
        },
    );
    assert!(res.is_err());
}

// =========================================================================
// B. CAPABILITY & ELIGIBILITY TESTS
// =========================================================================

#[test]
fn test_is_warranty_tracked_capability_and_duration() {
    let conn = setup_test_db();

    // 1. Product with positive warranty_months
    let p1 = make_test_product(&conn, "Laptop Pro", true, Some(12));
    assert!(is_warranty_tracked(&conn, &p1).unwrap());

    // 2. Product with 0 warranty_months and no capability
    let p2 = make_test_product(&conn, "Coffee Mug", false, Some(0));
    assert!(!is_warranty_tracked(&conn, &p2).unwrap());

    // 3. Product with NULL warranty_months and no capability
    let p3 = make_test_product(&conn, "Paper Clips", false, None);
    assert!(!is_warranty_tracked(&conn, &p3).unwrap());

    // 4. Product with NULL warranty_months but explicit WARRANTY capability
    let p4 = make_test_product(&conn, "Custom Hardware", true, None);
    add_product_capability(&conn, &p4, "WARRANTY");
    assert!(is_warranty_tracked(&conn, &p4).unwrap());

    // 5. Nonexistent product errors
    assert!(is_warranty_tracked(&conn, "nonexistent-id").is_err());
}

// =========================================================================
// C. DATE PARSING & CANONICAL NORMALIZATION TESTS
// =========================================================================

#[test]
fn test_parse_canonical_date_valid() {
    assert_eq!(parse_canonical_date("2026-09-03").unwrap(), (2026, 9, 3));
    assert_eq!(parse_canonical_date("1970-01-01").unwrap(), (1970, 1, 1));
    assert_eq!(parse_canonical_date("2028-02-29").unwrap(), (2028, 2, 29));
}

#[test]
fn test_parse_canonical_date_invalid() {
    assert!(parse_canonical_date("2026/09/03").is_err());
    assert!(parse_canonical_date("2026-9-3").is_err());
    assert!(parse_canonical_date("2026-13-01").is_err());
    assert!(parse_canonical_date("2026-00-01").is_err());
    assert!(parse_canonical_date("2026-04-31").is_err()); // April has 30 days
    assert!(parse_canonical_date("2026-02-29").is_err()); // 2026 not a leap year
    assert!(parse_canonical_date("1969-12-31").is_err()); // Before 1970
    assert!(parse_canonical_date("not-a-date").is_err());
}

#[test]
fn test_normalize_to_canonical_date() {
    // Pure YYYY-MM-DD
    assert_eq!(
        normalize_to_canonical_date("2026-09-03").unwrap(),
        "2026-09-03"
    );

    // ISO timestamp with T
    assert_eq!(
        normalize_to_canonical_date("2026-09-03T14:30:00Z").unwrap(),
        "2026-09-03"
    );

    // Timestamp with space
    assert_eq!(
        normalize_to_canonical_date("2026-09-03 00:00:00").unwrap(),
        "2026-09-03"
    );

    // Malformed timestamp delimiter
    assert!(normalize_to_canonical_date("2026-09-03X12:00:00").is_err());

    // Timestamp with positive timezone offset normalized to UTC date
    assert_eq!(
        normalize_to_canonical_date("2026-09-03T00:30:00+02:00").unwrap(),
        "2026-09-02"
    );

    // Malformed suffix rejected
    assert!(normalize_to_canonical_date("2026-09-03Tgarbage").is_err());

    // Multibyte characters at boundary handled gracefully without panic
    assert!(normalize_to_canonical_date("2026-09-03🚀extra").is_err());

    // Too short
    assert!(normalize_to_canonical_date("2026-09").is_err());
}

// =========================================================================
// D. EXPIRATION CALCULATION & MONTH-END CLAMPING TESTS
// =========================================================================

#[test]
fn test_calculate_warranty_expiration_cases() {
    // Normal month addition
    assert_eq!(
        calculate_warranty_expiration("2026-03-15", 6).unwrap(),
        "2026-09-15"
    );

    // January 31 clamped to February 28 (non-leap year 2026)
    assert_eq!(
        calculate_warranty_expiration("2026-01-31", 1).unwrap(),
        "2026-02-28"
    );

    // January 31 clamped to February 29 (leap year 2028)
    assert_eq!(
        calculate_warranty_expiration("2028-01-31", 1).unwrap(),
        "2028-02-29"
    );

    // May 31 clamped to June 30
    assert_eq!(
        calculate_warranty_expiration("2026-05-31", 1).unwrap(),
        "2026-06-30"
    );

    // Year rollover across December
    assert_eq!(
        calculate_warranty_expiration("2026-11-15", 3).unwrap(),
        "2027-02-15"
    );

    // Multi-year duration (24 months)
    assert_eq!(
        calculate_warranty_expiration("2026-01-15", 24).unwrap(),
        "2028-01-15"
    );

    // Zero duration rejected
    assert!(calculate_warranty_expiration("2026-01-15", 0).is_err());

    // Out of range year (overflow past 9999) rejected
    let oob_err = calculate_warranty_expiration("9999-12-01", 2).unwrap_err();
    assert!(oob_err.to_string().contains("outside the supported range"));
}

// =========================================================================
// E. COVERAGE EVALUATION SEMANTICS TESTS
// =========================================================================

#[test]
fn test_evaluate_warranty_coverage_semantics() {
    let expiry = "2026-09-15";

    // 1. Before expiration: Active, days_remaining > 0
    let cov_before = evaluate_warranty_coverage(Some(expiry), Some("2026-09-10"), true).unwrap();
    match cov_before {
        WarrantyCoverageStatus::Active {
            expiry_date,
            days_remaining,
        } => {
            assert_eq!(expiry_date, expiry);
            assert_eq!(days_remaining, 5);
        }
        _ => panic!("expected Active status"),
    }

    // 2. Exactly on expiration date: Active, days_remaining == 0 (ADR-0011 Invariant!)
    let cov_exact = evaluate_warranty_coverage(Some(expiry), Some("2026-09-15"), true).unwrap();
    match cov_exact {
        WarrantyCoverageStatus::Active {
            expiry_date,
            days_remaining,
        } => {
            assert_eq!(expiry_date, expiry);
            assert_eq!(days_remaining, 0);
        }
        _ => panic!("expected Active status on expiration day"),
    }

    // 3. After expiration: Expired, days_elapsed > 0
    let cov_after = evaluate_warranty_coverage(Some(expiry), Some("2026-09-18"), true).unwrap();
    match cov_after {
        WarrantyCoverageStatus::Expired {
            expiry_date,
            days_elapsed,
        } => {
            assert_eq!(expiry_date, expiry);
            assert_eq!(days_elapsed, 3);
        }
        _ => panic!("expected Expired status"),
    }

    // 4. Tracked product without registered expiration date: NotRegistered
    let cov_not_reg = evaluate_warranty_coverage(None, Some("2026-09-15"), true).unwrap();
    assert_eq!(cov_not_reg, WarrantyCoverageStatus::NotRegistered);

    // 5. Untracked product without registered expiration date: NotCovered
    let cov_not_cov = evaluate_warranty_coverage(None, Some("2026-09-15"), false).unwrap();
    assert_eq!(cov_not_cov, WarrantyCoverageStatus::NotCovered);
}

#[test]
fn test_days_between_calculation() {
    assert_eq!(days_between("2026-01-01", "2026-01-02").unwrap(), 1);
    assert_eq!(days_between("2026-01-01", "2026-01-01").unwrap(), 0);
    assert_eq!(days_between("2026-01-02", "2026-01-01").unwrap(), -1);
    assert_eq!(days_between("2026-02-28", "2026-03-01").unwrap(), 1);
    assert_eq!(days_between("2028-02-28", "2028-03-01").unwrap(), 2); // Leap year 2028 has Feb 29
}

// =========================================================================
// F. SERIALIZED INSTANCE REGISTRATION TESTS
// =========================================================================

#[test]
fn test_register_instance_warranty_product_default_and_explicit() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let product_id = make_test_product(&conn, "Smartphone 15", true, Some(12));
    let serial_id = make_test_serial_instance(
        &conn,
        &product_id,
        &branch_id,
        Some("SN-PHONE-001"),
        Some("864508041234565"),
        None,
    );

    // 1. Register with product default duration (12 months from 2026-01-01)
    let reg1 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: serial_id.clone(),
            start_date: Some("2026-01-01".to_string()),
            duration_months: None,
            warranty_expires_at: None,
        },
    )
    .unwrap();

    assert_eq!(reg1.warranty_expires_at.as_deref(), Some("2027-01-01"));

    // 2. Register with explicit duration override (24 months from 2026-01-01)
    let reg2 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: serial_id.clone(),
            start_date: Some("2026-01-01".to_string()),
            duration_months: Some(24),
            warranty_expires_at: None,
        },
    )
    .unwrap();

    assert_eq!(reg2.warranty_expires_at.as_deref(), Some("2028-01-01"));

    // 3. Register with direct expiration date
    let reg3 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: serial_id.clone(),
            start_date: None,
            duration_months: None,
            warranty_expires_at: Some("2029-06-30".to_string()),
        },
    )
    .unwrap();

    assert_eq!(reg3.warranty_expires_at.as_deref(), Some("2029-06-30"));
}

#[test]
fn test_register_instance_warranty_tri_identifier_models() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let product_id = make_test_product(&conn, "Universal Asset", true, Some(6));

    // Case 1: Serial-only
    let s1 = make_test_serial_instance(
        &conn,
        &product_id,
        &branch_id,
        Some("SN-ONLY-01"),
        None,
        None,
    );
    let r1 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: s1,
            start_date: Some("2026-03-01".to_string()),
            duration_months: None,
            warranty_expires_at: None,
        },
    )
    .unwrap();
    assert_eq!(r1.warranty_expires_at.as_deref(), Some("2026-09-01"));

    // Case 2: IMEI-only
    let s2 = make_test_serial_instance(
        &conn,
        &product_id,
        &branch_id,
        None,
        Some("358721098765430"),
        None,
    );
    let r2 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: s2,
            start_date: Some("2026-03-01".to_string()),
            duration_months: None,
            warranty_expires_at: None,
        },
    )
    .unwrap();
    assert_eq!(r2.warranty_expires_at.as_deref(), Some("2026-09-01"));

    // Case 3: Asset-tag-only
    let s3 = make_test_serial_instance(
        &conn,
        &product_id,
        &branch_id,
        None,
        None,
        Some("ASSET-TAG-99"),
    );
    let r3 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: s3,
            start_date: Some("2026-03-01".to_string()),
            duration_months: None,
            warranty_expires_at: None,
        },
    )
    .unwrap();
    assert_eq!(r3.warranty_expires_at.as_deref(), Some("2026-09-01"));
}

#[test]
fn test_register_instance_warranty_validation_errors() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    // Product without warranty capability or duration
    let p_untracked = make_test_product(&conn, "Plain Item", true, None);
    let s_untracked = make_test_serial_instance(
        &conn,
        &p_untracked,
        &branch_id,
        Some("SN-UNTRACKED"),
        None,
        None,
    );

    let err1 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: s_untracked,
            start_date: Some("2026-01-01".to_string()),
            duration_months: Some(12),
            warranty_expires_at: None,
        },
    )
    .unwrap_err();
    assert!(err1.to_string().contains("not eligible for warranty"));

    // Product with capability but no default duration, without explicit duration
    let p_cap = make_test_product(&conn, "Custom Device", true, None);
    add_product_capability(&conn, &p_cap, "WARRANTY");
    let s_cap = make_test_serial_instance(&conn, &p_cap, &branch_id, Some("SN-CAP"), None, None);

    let err2 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: s_cap,
            start_date: Some("2026-01-01".to_string()),
            duration_months: None,
            warranty_expires_at: None,
        },
    )
    .unwrap_err();
    assert!(err2
        .to_string()
        .contains("no default warranty duration; explicit duration_months"));

    // Nonexistent serial instance
    let err3 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: "nonexistent-id".to_string(),
            start_date: Some("2026-01-01".to_string()),
            duration_months: Some(12),
            warranty_expires_at: None,
        },
    )
    .unwrap_err();
    assert!(err3.to_string().contains("not found"));

    // Malformed direct expiration date
    let p_valid = make_test_product(&conn, "Valid Phone", true, Some(12));
    let s_valid =
        make_test_serial_instance(&conn, &p_valid, &branch_id, Some("SN-VALID"), None, None);
    let err4 = register_instance_warranty(
        &conn,
        &RegisterWarrantyInput {
            serial_number_id: s_valid,
            start_date: None,
            duration_months: None,
            warranty_expires_at: Some("invalid-date-format".to_string()),
        },
    )
    .unwrap_err();
    assert!(
        err4.to_string().contains("Date parse error") || err4.to_string().contains("Invalid date")
    );
}

// =========================================================================
// G. AUTHORIZATION & TENANT ISOLATION TESTS
// =========================================================================

#[test]
fn test_warranty_ipc_authorization_and_isolation() {
    let conn = setup_test_db();
    let (_, branch_a) = create_test_org_and_branch(&conn);
    let (_, branch_b) = create_test_org_and_branch(&conn);

    let admin = create_test_user_with_creds(
        &conn,
        &branch_a,
        "Warranty Manager",
        Some("war_mgr"),
        Some("pass123"),
        Some("1234"),
        "manager",
    )
    .expect("manager");

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_a,
        "Warranty Cashier",
        Some("war_cashier"),
        Some("pass123"),
        Some("1234"),
        "cashier",
    )
    .expect("cashier");

    let session_admin =
        create_local_session(&conn, &admin.id, &branch_a, "pin", None).expect("admin session");
    let session_cashier =
        create_local_session(&conn, &cashier.id, &branch_a, "pin", None).expect("cashier session");

    let product_id = make_test_product(&conn, "Flagship Phone", true, Some(12));
    let serial_id = make_test_serial_instance(
        &conn,
        &product_id,
        &branch_a,
        Some("SN-AUTH-001"),
        Some("490154203237518"),
        None,
    );

    // 1. Unauthenticated session rejected
    let unauth_err = register_instance_warranty_impl(
        &conn,
        "invalid-session-id",
        &RegisterInstanceWarrantyRequest {
            branch_id: branch_a.clone(),
            serial_number_id: serial_id.clone(),
            start_date: Some("2026-01-01".to_string()),
            duration_months: Some(12),
            warranty_expires_at: None,
        },
    )
    .unwrap_err();
    assert!(unauth_err.contains("Session expired or invalid") || unauth_err.contains("not found"));

    // 2. Cashier without InventoryAdjust permission rejected
    let perm_err = register_instance_warranty_impl(
        &conn,
        &session_cashier.id,
        &RegisterInstanceWarrantyRequest {
            branch_id: branch_a.clone(),
            serial_number_id: serial_id.clone(),
            start_date: Some("2026-01-01".to_string()),
            duration_months: Some(12),
            warranty_expires_at: None,
        },
    )
    .unwrap_err();
    assert!(perm_err.contains("Permission denied") || perm_err.contains("lacks permission"));

    // 3. Authorized admin can register warranty
    let auth_ok = register_instance_warranty_impl(
        &conn,
        &session_admin.id,
        &RegisterInstanceWarrantyRequest {
            branch_id: branch_a.clone(),
            serial_number_id: serial_id.clone(),
            start_date: Some("2026-01-01".to_string()),
            duration_months: Some(12),
            warranty_expires_at: None,
        },
    )
    .unwrap();
    assert_eq!(auth_ok.warranty_expires_at.as_deref(), Some("2027-01-01"));

    // 4. Anti-existence leakage: accessing instance from branch_b using session for branch_a
    let serial_b = make_test_serial_instance(
        &conn,
        &product_id,
        &branch_b,
        Some("SN-BRANCH-B-001"),
        None,
        None,
    );

    // Registration with branch_a attempting to update branch_b instance
    let wrong_branch_reg = register_instance_warranty_impl(
        &conn,
        &session_admin.id,
        &RegisterInstanceWarrantyRequest {
            branch_id: branch_a.clone(),
            serial_number_id: serial_b.clone(),
            start_date: Some("2026-01-01".to_string()),
            duration_months: Some(12),
            warranty_expires_at: None,
        },
    )
    .unwrap_err();
    assert!(wrong_branch_reg.contains("not found or inaccessible for this session"));

    // Query for branch_b instance using branch_a session
    let wrong_branch_get =
        get_instance_warranty_impl(&conn, &session_admin.id, &serial_b, None).unwrap_err();
    assert!(wrong_branch_get.contains("not found or inaccessible for this session"));

    // 5. Querying warranty for instance with authorized session succeeds
    let get_ok =
        get_instance_warranty_impl(&conn, &session_admin.id, &serial_id, Some("2026-06-01"))
            .unwrap();
    assert_eq!(get_ok.warranty_expires_at.as_deref(), Some("2027-01-01"));
    match get_ok.coverage {
        WarrantyCoverageStatus::Active { days_remaining, .. } => {
            assert!(days_remaining > 0);
        }
        _ => panic!("expected Active coverage"),
    }

    // 6. Direct command test for calculate_warranty_expiration_impl
    let exp_cmd =
        calculate_warranty_expiration_impl(&conn, &session_admin.id, "2026-01-15", 12).unwrap();
    assert_eq!(exp_cmd, "2027-01-15");

    // 7. Direct command test for evaluate_warranty_coverage_impl
    let cov_cmd = evaluate_warranty_coverage_impl(
        &conn,
        &session_admin.id,
        Some("2027-01-15"),
        Some("2026-06-01"),
        true,
    )
    .unwrap();
    match cov_cmd {
        WarrantyCoverageStatus::Active { days_remaining, .. } => {
            assert!(days_remaining > 0);
        }
        _ => panic!("expected Active coverage"),
    }
}

// =========================================================================
// H. MIGRATION 018 VERIFICATION TESTS
// =========================================================================

#[test]
fn test_migration_018_fresh_and_upgrade() {
    // 1. Fresh database application
    let fresh_conn = setup_test_db();
    let index_count: i64 = fresh_conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_serial_numbers_warranty_expires'",
            [],
            |row| row.get(0),
        )
        .expect("query index");
    assert_eq!(index_count, 1, "Migration 018 partial index must exist");

    // 2. Incremental upgrade over Migration 017
    let conn = Connection::open_in_memory().expect("in-memory connection");
    apply_migrations_up_to(&conn, "017_serial_imei_assets");

    // Insert legacy data with warranty_expires_at before Migration 018
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Legacy Phone", true, Some(12));
    let serial_id = make_test_serial_instance(
        &conn,
        &product_id,
        &branch_id,
        Some("SN-LEGACY-001"),
        None,
        None,
    );

    conn.execute(
        "UPDATE serial_numbers SET warranty_expires_at = '2027-12-31' WHERE id = ?1",
        params![serial_id],
    )
    .expect("update legacy warranty_expires_at");

    // Apply Migration 018
    apply_migrations_up_to(&conn, "018_warranty");

    // Verify index exists
    let index_applied: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_serial_numbers_warranty_expires'",
            [],
            |row| row.get(0),
        )
        .expect("query index");
    assert_eq!(index_applied, 1);

    // Verify legacy data preserved
    let stored_expiry: Option<String> = conn
        .query_row(
            "SELECT warranty_expires_at FROM serial_numbers WHERE id = ?1",
            params![serial_id],
            |row| row.get(0),
        )
        .expect("query stored expiry");
    assert_eq!(stored_expiry.as_deref(), Some("2027-12-31"));
}
