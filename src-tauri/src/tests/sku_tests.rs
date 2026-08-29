// Comprehensive unit, sequence allocation, and uniqueness tests for F2.03 SKU Management.

use crate::barcode::{generate_next_sku, validate_sku, BarcodeError};
use crate::product::{create_product, delete_product, CreateProductInput, ProductError};
use crate::tests::test_helpers::setup_test_db;

fn make_test_product_with_sku(
    conn: &rusqlite::Connection,
    name: &str,
    sku: Option<&str>,
) -> Result<crate::product::Product, ProductError> {
    let input = CreateProductInput {
        name: name.to_string(),
        description: None,
        category_id: None,
        sku: sku.map(ToString::to_string),
        barcode: None,
        product_type: None,
        base_price_minor: 1000,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    create_product(conn, input)
}

// =========================================================================
// 1. SKU FORMAT VALIDATION TESTS
// =========================================================================

#[test]
fn test_sku_validation_rules_and_normalization() {
    // Valid standard SKUs
    assert_eq!(validate_sku("SKU-1001").unwrap(), "SKU-1001");
    assert_eq!(validate_sku("  elec_item.01  ").unwrap(), "ELEC_ITEM.01");
    assert_eq!(validate_sku("ABC-123_456.789").unwrap(), "ABC-123_456.789");

    // Reject too short (< 3 chars)
    let err_short = validate_sku("AB").unwrap_err();
    assert!(matches!(err_short, BarcodeError::Validation(_)));

    // Reject empty / whitespace
    assert!(validate_sku("").is_err());
    assert!(validate_sku("   ").is_err());

    // Reject invalid characters ($ & % # @)
    let err_char = validate_sku("SKU#123$").unwrap_err();
    assert!(matches!(err_char, BarcodeError::Validation(_)));
}

// =========================================================================
// 2. ATOMIC SKU SEQUENCE GENERATION TESTS
// =========================================================================

#[test]
fn test_atomic_sku_generator_increments_and_formats() {
    let conn = setup_test_db();

    let sku1 = generate_next_sku(&conn, Some("ELEC")).expect("sku 1");
    assert_eq!(sku1, "ELEC-000001");

    let sku2 = generate_next_sku(&conn, Some("ELEC")).expect("sku 2");
    assert_eq!(sku2, "ELEC-000002");

    let sku3 = generate_next_sku(&conn, None).expect("sku 3 default prefix");
    assert_eq!(sku3, "SKU-000001");
}

#[test]
fn test_atomic_sku_generator_skips_manual_collisions() {
    let conn = setup_test_db();

    // 1. Manually create a product that occupies "ELEC-000001"
    make_test_product_with_sku(&conn, "Manual Item 1", Some("ELEC-000001"))
        .expect("manual product created");

    // 2. Automated generator must detect the collision, increment the sequence, and return "ELEC-000002"
    let auto_sku = generate_next_sku(&conn, Some("ELEC")).expect("auto sku generated");
    assert_eq!(auto_sku, "ELEC-000002");
}

// =========================================================================
// 3. DATABASE ACTIVE UNIQUENESS & ARCHIVED REUSE TESTS
// =========================================================================

#[test]
fn test_active_sku_uniqueness_enforced() {
    let conn = setup_test_db();

    make_test_product_with_sku(&conn, "Item A", Some("UNIQUE-SKU-99")).expect("product A created");

    let res = make_test_product_with_sku(&conn, "Item B", Some("UNIQUE-SKU-99"));
    assert!(
        matches!(res, Err(ProductError::DuplicateSku(_))),
        "Active duplicate SKU must be rejected"
    );
}

#[test]
fn test_archived_sku_reuse_allowed() {
    let conn = setup_test_db();

    // 1. Create and archive product with SKU
    let prod1 = make_test_product_with_sku(&conn, "Original Item", Some("REUSABLE-SKU-01"))
        .expect("prod 1 created");
    delete_product(&conn, &prod1.id).expect("prod 1 archived");

    // 2. Creating a new active product with the same SKU must succeed
    let prod2 = make_test_product_with_sku(&conn, "New Reused Item", Some("REUSABLE-SKU-01"))
        .expect("prod 2 created with reused SKU");
    assert_eq!(prod2.sku.as_deref(), Some("REUSABLE-SKU-01"));
}
