// Unit, repository, exact-money, and authorization tests for F2.01 Product CRUD.

use crate::auth::middleware::{
    require_permission, require_scoped_permission, require_session, AuthMiddlewareError,
    AuthorizeRequest,
};
use crate::branch::{create_branch, CreateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::permission::Permission;
use crate::product::{
    create_product, delete_product, get_catalog_organization_id, get_product, list_products,
    minor_to_real, real_to_minor, update_product, validate_barcode, validate_base_price_minor,
    validate_cost_price_minor, validate_name, validate_product_type, CreateProductInput,
    ProductError, ProductFilter, UpdateProductInput, MAX_SAFE_MINOR_UNITS,
};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::{create_local_session, revoke_local_session};
use rusqlite::params;

fn make_product_fixture(name: &str, price_minor: i64, barcode: Option<&str>) -> CreateProductInput {
    CreateProductInput {
        name: name.to_string(),
        description: None,
        category_id: None,
        sku: None,
        barcode: barcode.map(ToString::to_string),
        product_type: None,
        base_price_minor: price_minor,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    }
}

// =========================================================================
// 1. VALIDATION UNIT TESTS
// =========================================================================

#[test]
fn test_validate_name_trims_and_accepts_valid() {
    let result = validate_name("  Espresso Roast 250g  ").expect("valid name");
    assert_eq!(result, "Espresso Roast 250g");
}

#[test]
fn test_validate_name_accepts_multibyte_unicode_up_to_255_chars() {
    let arabic_name = "قهوة عربية أصيلة درجة أولى مع الهيل والزعفران";
    assert_eq!(arabic_name.chars().count(), 45);
    assert!(arabic_name.len() > 45);
    let result = validate_name(arabic_name).expect("multibyte unicode name accepted");
    assert_eq!(result, arabic_name);

    let exact_255_unicode: String = "ق".repeat(255);
    assert_eq!(exact_255_unicode.chars().count(), 255);
    assert_eq!(exact_255_unicode.len(), 510);
    let result_255 = validate_name(&exact_255_unicode).expect("255 unicode chars accepted");
    assert_eq!(result_255, exact_255_unicode);
}

#[test]
fn test_validate_name_rejects_empty_and_whitespace() {
    let err_empty = validate_name("").unwrap_err();
    assert!(matches!(err_empty, ProductError::Validation(_)));

    let err_ws = validate_name("    ").unwrap_err();
    assert!(matches!(err_ws, ProductError::Validation(_)));
}

#[test]
fn test_validate_name_rejects_too_long() {
    let long_name = "A".repeat(256);
    let err = validate_name(&long_name).unwrap_err();
    assert!(matches!(err, ProductError::Validation(_)));

    let long_unicode = "ق".repeat(256);
    let err_unicode = validate_name(&long_unicode).unwrap_err();
    assert!(matches!(err_unicode, ProductError::Validation(_)));
}

#[test]
fn test_validate_base_price_minor_accepts_zero_and_positive() {
    assert_eq!(validate_base_price_minor(0).expect("zero price"), 0);
    assert_eq!(
        validate_base_price_minor(25000).expect("positive price"),
        25000
    );
    assert_eq!(
        validate_base_price_minor(MAX_SAFE_MINOR_UNITS).expect("max safe price"),
        MAX_SAFE_MINOR_UNITS
    );
}

#[test]
fn test_validate_base_price_minor_rejects_negative_and_lossy_overflow() {
    let err_neg = validate_base_price_minor(-1).unwrap_err();
    assert!(matches!(err_neg, ProductError::Validation(_)));

    let err_overflow = validate_base_price_minor(MAX_SAFE_MINOR_UNITS + 1).unwrap_err();
    assert!(matches!(err_overflow, ProductError::Validation(_)));

    let lossy_val = 4_503_599_627_370_495_i64; // 2^52 - 1 (demonstrated float round-trip loss)
    let err_lossy = validate_base_price_minor(lossy_val).unwrap_err();
    assert!(matches!(err_lossy, ProductError::Validation(_)));
}

#[test]
fn test_validate_cost_price_minor_accepts_none_zero_positive() {
    assert!(validate_cost_price_minor(None)
        .expect("none cost")
        .is_none());
    assert_eq!(
        validate_cost_price_minor(Some(0)).expect("zero cost"),
        Some(0)
    );
    assert_eq!(
        validate_cost_price_minor(Some(12500)).expect("positive cost"),
        Some(12500)
    );
    assert_eq!(
        validate_cost_price_minor(Some(MAX_SAFE_MINOR_UNITS)).expect("max safe cost"),
        Some(MAX_SAFE_MINOR_UNITS)
    );
}

#[test]
fn test_validate_cost_price_minor_rejects_negative_and_overflow() {
    let err_neg = validate_cost_price_minor(Some(-500)).unwrap_err();
    assert!(matches!(err_neg, ProductError::Validation(_)));

    let err_overflow = validate_cost_price_minor(Some(MAX_SAFE_MINOR_UNITS + 1)).unwrap_err();
    assert!(matches!(err_overflow, ProductError::Validation(_)));
}

#[test]
fn test_validate_barcode_normalizes_empty_and_trims() {
    assert!(validate_barcode(None).is_none());
    assert!(validate_barcode(Some("")).is_none());
    assert!(validate_barcode(Some("   ")).is_none());
    assert_eq!(
        validate_barcode(Some("  123456789  ")),
        Some("123456789".to_string())
    );
}

#[test]
fn test_validate_product_type_accepts_valid_types_and_defaults() {
    assert_eq!(validate_product_type(None).expect("default"), "simple");
    assert_eq!(
        validate_product_type(Some("")).expect("empty defaults"),
        "simple"
    );
    assert_eq!(
        validate_product_type(Some("simple")).expect("simple"),
        "simple"
    );
    assert_eq!(
        validate_product_type(Some("variable")).expect("variable"),
        "variable"
    );
    assert_eq!(
        validate_product_type(Some("weighted")).expect("weighted"),
        "weighted"
    );
}

#[test]
fn test_validate_product_type_rejects_invalid() {
    let err = validate_product_type(Some("invalid_type")).unwrap_err();
    assert!(matches!(err, ProductError::Validation(_)));
}

// =========================================================================
// 2. MONEY CONVERSION & ROUND-TRIP TESTS
// =========================================================================

#[test]
fn test_money_exact_round_trip_conversions() {
    let test_cases: &[(i64, f64)] = &[
        (0, 0.0),
        (1, 0.01),
        (99, 0.99),
        (100, 1.0),
        (25000, 250.0),
        (12345678, 123456.78),
        (
            MAX_SAFE_MINOR_UNITS - 1,
            (MAX_SAFE_MINOR_UNITS - 1) as f64 / 100.0,
        ),
        (MAX_SAFE_MINOR_UNITS, MAX_SAFE_MINOR_UNITS as f64 / 100.0),
    ];

    for &(minor, real) in test_cases {
        assert_eq!(minor_to_real(minor), real);
        assert_eq!(real_to_minor(real), minor);
    }
}

#[test]
fn test_product_price_full_persistence_roundtrip_at_bounds() {
    let conn = setup_test_db();
    let created = create_product(
        &conn,
        CreateProductInput {
            name: "High Value Asset".to_string(),
            description: None,
            category_id: None,
            sku: None,
            barcode: Some("BC-BOUND-001".to_string()),
            product_type: None,
            base_price_minor: MAX_SAFE_MINOR_UNITS,
            cost_price_minor: Some(MAX_SAFE_MINOR_UNITS - 100),
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("product created at maximum boundary");

    assert_eq!(created.base_price_minor, MAX_SAFE_MINOR_UNITS);
    assert_eq!(created.cost_price_minor, Some(MAX_SAFE_MINOR_UNITS - 100));

    let fetched = get_product(&conn, &created.id)
        .expect("query succeeds")
        .expect("product found");

    assert_eq!(
        fetched.base_price_minor, MAX_SAFE_MINOR_UNITS,
        "write -> SQLite REAL -> read must return exact base_price_minor without 1-unit deviation"
    );
    assert_eq!(
        fetched.cost_price_minor,
        Some(MAX_SAFE_MINOR_UNITS - 100),
        "write -> SQLite REAL -> read must return exact cost_price_minor without 1-unit deviation"
    );
}

// =========================================================================
// 3. REPOSITORY CRUD & DATABASE TESTS
// =========================================================================

#[test]
fn test_create_and_get_product_by_id() {
    let conn = setup_test_db();

    let input = CreateProductInput {
        name: "Artisan Coffee Beans".to_string(),
        description: Some("Single origin Ethiopian roast".to_string()),
        category_id: None,
        sku: None,
        barcode: Some("6131234567890".to_string()),
        product_type: Some("simple".to_string()),
        base_price_minor: 18500,       // 185.00
        cost_price_minor: Some(11000), // 110.00
        unit_type: Some("bag".to_string()),
        requires_expiry: Some(true),
        requires_serial: Some(false),
        warranty_months: Some(12),
        custom_attributes: Some("{\"roast\":\"medium\"}".to_string()),
    };

    let created = create_product(&conn, input).expect("product created");
    assert!(!created.id.is_empty());
    assert_eq!(created.name, "Artisan Coffee Beans");
    assert_eq!(
        created.description.as_deref(),
        Some("Single origin Ethiopian roast")
    );
    assert_eq!(created.barcode.as_deref(), Some("6131234567890"));
    assert_eq!(created.base_price_minor, 18500);
    assert_eq!(created.cost_price_minor, Some(11000));
    assert_eq!(created.unit_type.as_deref(), Some("bag"));
    assert!(created.requires_expiry);
    assert!(!created.requires_serial);
    assert_eq!(created.warranty_months, Some(12));
    assert!(created.is_active);

    let fetched = get_product(&conn, &created.id)
        .expect("get_product query succeeds")
        .expect("product found");
    assert_eq!(created, fetched);
}

#[test]
fn test_get_product_by_barcode() {
    let conn = setup_test_db();
    let input = make_product_fixture("Green Tea Box", 450, Some("BARCODE-TEA-001"));
    let created = create_product(&conn, input).expect("product created");

    let (by_barcode, _bc) = crate::barcode::get_product_by_barcode(&conn, "BARCODE-TEA-001")
        .expect("lookup succeeds")
        .expect("found by barcode");
    assert_eq!(by_barcode.id, created.id);

    let nonexistent = crate::barcode::get_product_by_barcode(&conn, "NONEXISTENT-BARCODE")
        .expect("lookup succeeds");
    assert!(nonexistent.is_none());
}

#[test]
fn test_duplicate_barcode_rejected() {
    let conn = setup_test_db();
    let input1 = make_product_fixture("Product A", 1000, Some("UNIQUE-BARCODE-99"));
    create_product(&conn, input1).expect("product A created");

    let input2 = make_product_fixture("Product B", 2000, Some("UNIQUE-BARCODE-99"));
    let err = create_product(&conn, input2).unwrap_err();
    assert!(matches!(err, ProductError::DuplicateBarcode(_)));
}

#[test]
fn test_archived_barcode_reuse_succeeds_in_f203() {
    let conn = setup_test_db();
    let input1 = make_product_fixture("Archived Product", 1000, Some("BC-ARCHIVE-DUP"));
    let created = create_product(&conn, input1).expect("product created");

    delete_product(&conn, &created.id).expect("soft delete succeeds");

    // In F2.03, soft-deleting frees the barcode mirror and archives it, so reusing it on an active product succeeds
    let input2 = make_product_fixture("New Attempt Same BC", 2000, Some("BC-ARCHIVE-DUP"));
    let created2 = create_product(&conn, input2).expect("product 2 created with reused barcode");
    assert_eq!(created2.barcode.as_deref(), Some("BC-ARCHIVE-DUP"));
}

#[test]
fn test_multiple_products_with_null_barcode_allowed() {
    let conn = setup_test_db();
    let p1 = create_product(&conn, make_product_fixture("Fresh Bread", 150, None))
        .expect("product 1 created");
    let p2 = create_product(&conn, make_product_fixture("Fresh Croissant", 250, None))
        .expect("product 2 created");

    assert_ne!(p1.id, p2.id);
    assert!(p1.barcode.is_none());
    assert!(p2.barcode.is_none());
}

#[test]
fn test_update_product_success() {
    let conn = setup_test_db();
    let created = create_product(
        &conn,
        make_product_fixture("Original Name", 5000, Some("BC-ORIGINAL")),
    )
    .expect("created");

    let updated = update_product(
        &conn,
        UpdateProductInput {
            id: created.id.clone(),
            name: "Updated Name".to_string(),
            description: Some("New description".to_string()),
            category_id: None,
            sku: None,
            barcode: Some("BC-UPDATED".to_string()),
            product_type: "simple".to_string(),
            base_price_minor: 6500,
            cost_price_minor: Some(4000),
            unit_type: Some("pcs".to_string()),
            requires_expiry: true,
            requires_serial: false,
            warranty_months: Some(6),
            custom_attributes: None,
            is_active: true,
        },
    )
    .expect("updated");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.description.as_deref(), Some("New description"));
    assert_eq!(updated.barcode.as_deref(), Some("BC-UPDATED"));
    assert_eq!(updated.base_price_minor, 6500);
    assert_eq!(updated.cost_price_minor, Some(4000));
    assert_eq!(updated.unit_type.as_deref(), Some("pcs"));
    assert!(updated.requires_expiry);
    assert_eq!(updated.warranty_months, Some(6));
}

#[test]
fn test_update_product_nonexistent_returns_not_found() {
    let conn = setup_test_db();
    let err = update_product(
        &conn,
        UpdateProductInput {
            id: "nonexistent-prod-id".to_string(),
            name: "Does Not Exist".to_string(),
            description: None,
            category_id: None,
            sku: None,
            barcode: None,
            product_type: "simple".to_string(),
            base_price_minor: 100,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: false,
            requires_serial: false,
            warranty_months: None,
            custom_attributes: None,
            is_active: true,
        },
    )
    .unwrap_err();

    assert!(matches!(err, ProductError::NotFound(_)));
}

#[test]
fn test_soft_delete_product_sets_is_active_zero_and_preserves_row() {
    let conn = setup_test_db();
    let created = create_product(
        &conn,
        make_product_fixture("Archivable Item", 1200, Some("BC-ARCHIVE-01")),
    )
    .expect("created");

    assert!(created.is_active);
    delete_product(&conn, &created.id).expect("soft delete succeeds");

    let fetched = get_product(&conn, &created.id)
        .expect("get_product succeeds")
        .expect("product still exists in DB");

    assert!(
        !fetched.is_active,
        "product must be marked inactive (is_active = false)"
    );
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "Archivable Item");

    delete_product(&conn, &created.id).expect("idempotent delete succeeds");
}

#[test]
fn test_soft_delete_preserves_foreign_key_references() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let product = create_product(
        &conn,
        make_product_fixture("Sold Item", 2000, Some("BC-SOLD-01")),
    )
    .expect("product created");

    conn.execute(
        "INSERT INTO inventory (id, branch_id, product_id, quantity, low_stock_threshold)
         VALUES ('inv-ref-1', ?1, ?2, 10.0, 2.0)",
        params![branch_id, product.id],
    )
    .expect("inventory row inserted");

    delete_product(&conn, &product.id).expect("soft delete succeeds");

    let inv_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM inventory WHERE product_id = ?1)",
            [&product.id],
            |r| r.get(0),
        )
        .expect("query succeeds");
    assert!(
        inv_exists,
        "inventory FK reference remains completely valid"
    );
}

#[test]
fn test_list_products_filtering_and_query_search() {
    let conn = setup_test_db();
    let p1 = create_product(
        &conn,
        make_product_fixture("Espresso Beans", 1500, Some("BC-ESP-1")),
    )
    .expect("p1 created");
    let p2 = create_product(
        &conn,
        make_product_fixture("Latte Syrup Vanilla", 800, Some("BC-LAT-2")),
    )
    .expect("p2 created");

    delete_product(&conn, &p2.id).expect("p2 archived");

    let active_only = list_products(
        &conn,
        &ProductFilter {
            is_active: Some(true),
            ..Default::default()
        },
    )
    .expect("list active");
    assert_eq!(active_only.len(), 1);
    assert_eq!(active_only[0].id, p1.id);

    let all = list_products(&conn, &ProductFilter::default()).expect("list all");
    assert_eq!(all.len(), 2);

    let search_latte = list_products(
        &conn,
        &ProductFilter {
            query: Some("latte".to_string()),
            ..Default::default()
        },
    )
    .expect("search latte");
    assert_eq!(search_latte.len(), 1);
    assert_eq!(search_latte[0].id, p2.id);

    let search_barcode = list_products(
        &conn,
        &ProductFilter {
            query: Some("BC-ESP-1".to_string()),
            ..Default::default()
        },
    )
    .expect("search barcode");
    assert_eq!(search_barcode.len(), 1);
    assert_eq!(search_barcode[0].id, p1.id);
}

#[test]
fn test_list_products_offset_without_limit() {
    let conn = setup_test_db();

    for i in 1..=5 {
        create_product(
            &conn,
            make_product_fixture(&format!("Product Item {i:02}"), 100 * i, None),
        )
        .expect("created");
    }

    let skipped = list_products(
        &conn,
        &ProductFilter {
            offset: Some(2),
            ..Default::default()
        },
    )
    .expect("offset query succeeds");

    assert_eq!(skipped.len(), 3);
    assert_eq!(skipped[0].name, "Product Item 03");
    assert_eq!(skipped[1].name, "Product Item 04");
    assert_eq!(skipped[2].name, "Product Item 05");
}

#[test]
fn test_list_products_literal_like_wildcard_escaping() {
    let conn = setup_test_db();
    create_product(
        &conn,
        make_product_fixture("100% Arabica Blend", 1500, None),
    )
    .expect("created 100%");
    create_product(
        &conn,
        make_product_fixture("1000 Arabica Blend", 1600, None),
    )
    .expect("created 1000");
    create_product(&conn, make_product_fixture("Item_1 Premium", 2000, None))
        .expect("created Item_1");
    create_product(&conn, make_product_fixture("ItemA1 Premium", 2100, None))
        .expect("created ItemA1");

    let percent_results = list_products(
        &conn,
        &ProductFilter {
            query: Some("100%".to_string()),
            ..Default::default()
        },
    )
    .expect("search with %");
    assert_eq!(percent_results.len(), 1);
    assert_eq!(percent_results[0].name, "100% Arabica Blend");

    let underscore_results = list_products(
        &conn,
        &ProductFilter {
            query: Some("Item_1".to_string()),
            ..Default::default()
        },
    )
    .expect("search with _");
    assert_eq!(underscore_results.len(), 1);
    assert_eq!(underscore_results[0].name, "Item_1 Premium");
}

// =========================================================================
// 4. AUTHORIZATION & TENANT ISOLATION TESTS
// =========================================================================

#[test]
fn test_admin_and_manager_authorized_to_mutate_products() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let admin = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Admin User",
        Some("admin_prod_auth"),
        None,
        None,
        "admin",
    )
    .expect("admin created");

    let admin_session =
        create_local_session(&conn, &admin.id, &branch_id, "pin", None).expect("admin session");

    let admin_auth = require_permission(&conn, &admin_session.id, Permission::ProductsManage);
    assert!(
        admin_auth.is_ok(),
        "Admin must be authorized for ProductsManage"
    );

    let manager = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Manager User",
        Some("manager_prod_auth"),
        None,
        None,
        "manager",
    )
    .expect("manager created");

    let manager_session =
        create_local_session(&conn, &manager.id, &branch_id, "pin", None).expect("manager session");

    let manager_auth = require_permission(&conn, &manager_session.id, Permission::ProductsManage);
    assert!(
        manager_auth.is_ok(),
        "Manager must be authorized for ProductsManage"
    );
}

#[test]
fn test_cashier_denied_product_mutation_but_allowed_product_reads() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier User",
        Some("cashier_prod_auth"),
        None,
        None,
        "cashier",
    )
    .expect("cashier created");

    let cashier_session =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("cashier session");

    let mutation_auth = require_permission(&conn, &cashier_session.id, Permission::ProductsManage);
    assert!(
        matches!(
            mutation_auth,
            Err(AuthMiddlewareError::PermissionDenied { .. })
        ),
        "Cashier must be denied ProductsManage permission"
    );

    let read_auth = require_session(&conn, &cashier_session.id);
    assert!(
        read_auth.is_ok(),
        "Cashier with active session must be allowed to read products"
    );
}

#[test]
fn test_product_organization_isolation_enforced() {
    let conn = setup_test_db();

    // 1. Setup Tenant Alpha
    let org_a = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Organization Alpha".to_string(),
            default_currency: Some("USD".to_string()),
            default_language: Some("en".to_string()),
        },
    )
    .expect("create org a");

    let branch_a = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id.clone(),
            name: "Branch Alpha".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch a");

    let admin_a = create_test_user_with_creds(
        &conn,
        &branch_a.id,
        "Admin Alpha",
        Some("admin_alpha"),
        None,
        None,
        "admin",
    )
    .expect("create admin a");

    let session_a = create_local_session(&conn, &admin_a.id, &branch_a.id, "pin", None)
        .expect("session a created");

    // 2. Setup Tenant Beta
    let org_b = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Organization Beta".to_string(),
            default_currency: Some("EUR".to_string()),
            default_language: Some("de".to_string()),
        },
    )
    .expect("create org b");

    let branch_b = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_b.id.clone(),
            name: "Branch Beta".to_string(),
            address: None,
            currency: Some("EUR".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch b");

    let admin_b = create_test_user_with_creds(
        &conn,
        &branch_b.id,
        "Admin Beta",
        Some("admin_beta"),
        None,
        None,
        "admin",
    )
    .expect("create admin b");

    let session_b = create_local_session(&conn, &admin_b.id, &branch_b.id, "pin", None)
        .expect("session b created");

    // 3. Set business settings to Org A
    conn.execute(
        "INSERT INTO business_settings (id, business_name, default_currency, organization_id)
         VALUES ('bs-1', 'Org A Store', 'USD', ?1)
         ON CONFLICT(id) DO UPDATE SET organization_id = ?1",
        params![org_a.id],
    )
    .expect("business settings configured");

    let catalog_org = get_catalog_organization_id(&conn).expect("get catalog org");
    assert_eq!(catalog_org.as_deref(), Some(org_a.id.as_str()));

    // 4. Session A (matching Org A) has valid scoped authorization
    let auth_a = require_scoped_permission(
        &conn,
        &session_a.id,
        Permission::ProductsManage,
        catalog_org.as_deref(),
        None,
    );
    assert!(auth_a.is_ok(), "Org A admin authorized for Org A catalog");

    let read_a = AuthorizeRequest::new(&session_a.id)
        .with_organization_scope(org_a.id.as_str())
        .execute(&conn);
    assert!(
        read_a.is_ok(),
        "Org A admin authorized to read Org A catalog"
    );

    // 5. Session B (cross-tenant Org B) is rejected on scoped mutation and read
    let auth_b = require_scoped_permission(
        &conn,
        &session_b.id,
        Permission::ProductsManage,
        catalog_org.as_deref(),
        None,
    );
    assert!(
        matches!(auth_b, Err(AuthMiddlewareError::ScopeMismatch { .. })),
        "Org B admin must be rejected with ScopeMismatch for Org A catalog"
    );

    let read_b = AuthorizeRequest::new(&session_b.id)
        .with_organization_scope(org_a.id.as_str())
        .execute(&conn);
    assert!(
        matches!(read_b, Err(AuthMiddlewareError::ScopeMismatch { .. })),
        "Org B admin must be rejected with ScopeMismatch for Org A catalog reads"
    );
}

#[test]
fn test_unauthenticated_or_revoked_session_denied_all_product_operations() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Test Revocation User",
        Some("test_revocation_user"),
        None,
        None,
        "admin",
    )
    .expect("user created");

    let session =
        create_local_session(&conn, &user.id, &branch_id, "pin", None).expect("session created");

    // 1. Session is initially active and valid
    assert!(require_session(&conn, &session.id).is_ok());
    assert!(require_permission(&conn, &session.id, Permission::ProductsManage).is_ok());

    // 2. Explicitly revoke the session using production revocation function
    revoke_local_session(&conn, &session.id).expect("session revoked");

    // 3. Revoked session is rejected fail-closed
    let err_revoked_session = require_session(&conn, &session.id).unwrap_err();
    assert!(
        matches!(err_revoked_session, AuthMiddlewareError::SessionRevoked(_)),
        "Revoked session must be rejected with SessionRevoked"
    );

    let err_revoked_perm =
        require_permission(&conn, &session.id, Permission::ProductsManage).unwrap_err();
    assert!(
        matches!(err_revoked_perm, AuthMiddlewareError::SessionRevoked(_)),
        "Revoked session must be rejected with SessionRevoked for permission checks"
    );

    // 4. Nonexistent session ID is rejected as Unauthenticated
    let err_unauth = require_session(&conn, "nonexistent-session-id");
    assert!(
        matches!(err_unauth, Err(AuthMiddlewareError::Unauthenticated(_))),
        "Nonexistent session must be denied as Unauthenticated"
    );

    let err_perm = require_permission(&conn, "nonexistent-session-id", Permission::ProductsManage);
    assert!(
        matches!(err_perm, Err(AuthMiddlewareError::Unauthenticated(_))),
        "Nonexistent session must be denied as Unauthenticated for permission checks"
    );
}

#[test]
fn test_multiple_organizations_without_configured_catalog_org_fails_closed() {
    let conn = setup_test_db();

    // 1. Create Org A and Branch A
    let org_a = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Org Alpha".to_string(),
            default_currency: Some("USD".to_string()),
            default_language: Some("en".to_string()),
        },
    )
    .expect("org a created");

    let branch_a = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id.clone(),
            name: "Branch A".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch a created");

    let user_a = create_test_user_with_creds(
        &conn,
        &branch_a.id,
        "User Alpha",
        Some("user_alpha_multi"),
        None,
        None,
        "admin",
    )
    .expect("user a created");

    let session_a = create_local_session(&conn, &user_a.id, &branch_a.id, "pin", None)
        .expect("session created");

    // 2. Create Org B and Branch B (ambiguous multi-tenant DB with no business_settings configured)
    let org_b = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Org Beta".to_string(),
            default_currency: Some("EUR".to_string()),
            default_language: Some("de".to_string()),
        },
    )
    .expect("org b created");

    create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_b.id,
            name: "Branch B".to_string(),
            address: None,
            currency: Some("EUR".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch b created");

    // 3. get_catalog_organization_id must return Ok(None) due to ambiguous multi-org presence
    let catalog_org = get_catalog_organization_id(&conn).expect("lookup succeeds");
    assert!(catalog_org.is_none());

    // 4. Mutation and read command authorizers must fail closed
    let mutation_err =
        crate::commands::authorize_catalog_mutation(&conn, &session_a.id).unwrap_err();
    assert!(
        mutation_err.contains("no catalog organization configured"),
        "Mutation must fail closed when catalog org is unresolved"
    );

    let read_err = crate::commands::authorize_catalog_read(&conn, &session_a.id).unwrap_err();
    assert!(
        read_err.contains("no catalog organization configured"),
        "Read must fail closed when catalog org is unresolved"
    );
}
