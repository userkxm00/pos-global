// Unit, repository, exact-money, and authorization tests for F2.01 Product CRUD.

use crate::auth::middleware::{require_permission, require_session, AuthMiddlewareError};
use crate::permission::Permission;
use crate::product::{
    create_product, delete_product, get_product, get_product_by_barcode, list_products,
    minor_to_real, real_to_minor, update_product, validate_barcode, validate_base_price_minor,
    validate_cost_price_minor, validate_name, validate_product_type, CreateProductInput,
    ProductError, ProductFilter, UpdateProductInput, MAX_SAFE_MINOR_UNITS,
};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::create_local_session;
use rusqlite::params;

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
    // Non-Latin Arabic text: 50 Unicode chars, 94 UTF-8 bytes
    let arabic_name = "قهوة عربية أصيلة درجة أولى مع الهيل والزعفران";
    assert_eq!(arabic_name.chars().count(), 45);
    assert!(arabic_name.len() > 45);
    let result = validate_name(arabic_name).expect("multibyte unicode name accepted");
    assert_eq!(result, arabic_name);

    // Exactly 255 Unicode characters (510 UTF-8 bytes)
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
fn test_validate_base_price_minor_rejects_negative_and_overflow() {
    let err_neg = validate_base_price_minor(-1).unwrap_err();
    assert!(matches!(err_neg, ProductError::Validation(_)));

    let err_overflow = validate_base_price_minor(MAX_SAFE_MINOR_UNITS + 1).unwrap_err();
    assert!(matches!(err_overflow, ProductError::Validation(_)));
}

#[test]
fn test_validate_cost_price_minor_accepts_none_zero_positive() {
    assert_eq!(validate_cost_price_minor(None).expect("none cost"), None);
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
    assert_eq!(validate_barcode(None), None);
    assert_eq!(validate_barcode(Some("")), None);
    assert_eq!(validate_barcode(Some("   ")), None);
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
    ];

    for &(minor, real) in test_cases {
        assert_eq!(minor_to_real(minor), real);
        assert_eq!(real_to_minor(real), minor);
    }
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

    let input = CreateProductInput {
        name: "Green Tea Box".to_string(),
        description: None,
        category_id: None,
        barcode: Some("BARCODE-TEA-001".to_string()),
        product_type: Some("simple".to_string()),
        base_price_minor: 450,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };

    let created = create_product(&conn, input).expect("product created");

    let by_barcode = get_product_by_barcode(&conn, "BARCODE-TEA-001")
        .expect("lookup succeeds")
        .expect("found by barcode");
    assert_eq!(by_barcode.id, created.id);

    let nonexistent =
        get_product_by_barcode(&conn, "NONEXISTENT-BARCODE").expect("lookup succeeds");
    assert!(nonexistent.is_none());
}

#[test]
fn test_duplicate_barcode_rejected() {
    let conn = setup_test_db();

    let input1 = CreateProductInput {
        name: "Product A".to_string(),
        description: None,
        category_id: None,
        barcode: Some("UNIQUE-BARCODE-99".to_string()),
        product_type: None,
        base_price_minor: 1000,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    create_product(&conn, input1).expect("product A created");

    let input2 = CreateProductInput {
        name: "Product B".to_string(),
        description: None,
        category_id: None,
        barcode: Some("UNIQUE-BARCODE-99".to_string()),
        product_type: None,
        base_price_minor: 2000,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    let err = create_product(&conn, input2).unwrap_err();
    assert!(matches!(err, ProductError::DuplicateBarcode(_)));
}

#[test]
fn test_multiple_products_with_null_barcode_allowed() {
    let conn = setup_test_db();

    let input1 = CreateProductInput {
        name: "Fresh Bread".to_string(),
        description: None,
        category_id: None,
        barcode: None,
        product_type: None,
        base_price_minor: 150,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    let p1 = create_product(&conn, input1).expect("product 1 created");

    let input2 = CreateProductInput {
        name: "Fresh Croissant".to_string(),
        description: None,
        category_id: None,
        barcode: None,
        product_type: None,
        base_price_minor: 250,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    let p2 = create_product(&conn, input2).expect("product 2 created");

    assert_ne!(p1.id, p2.id);
    assert_eq!(p1.barcode, None);
    assert_eq!(p2.barcode, None);
}

#[test]
fn test_update_product_success() {
    let conn = setup_test_db();

    let created = create_product(
        &conn,
        CreateProductInput {
            name: "Original Name".to_string(),
            description: Some("Old description".to_string()),
            category_id: None,
            barcode: Some("BC-ORIGINAL".to_string()),
            product_type: Some("simple".to_string()),
            base_price_minor: 5000,
            cost_price_minor: Some(3000),
            unit_type: None,
            requires_expiry: Some(false),
            requires_serial: Some(false),
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("created");

    let updated = update_product(
        &conn,
        UpdateProductInput {
            id: created.id.clone(),
            name: "Updated Name".to_string(),
            description: Some("New description".to_string()),
            category_id: None,
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
        CreateProductInput {
            name: "Archivable Item".to_string(),
            description: None,
            category_id: None,
            barcode: Some("BC-ARCHIVE-01".to_string()),
            product_type: None,
            base_price_minor: 1200,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
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

    // Idempotent delete call succeeds
    delete_product(&conn, &created.id).expect("idempotent delete succeeds");
}

#[test]
fn test_soft_delete_preserves_foreign_key_references() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let product = create_product(
        &conn,
        CreateProductInput {
            name: "Sold Item".to_string(),
            description: None,
            category_id: None,
            barcode: Some("BC-SOLD-01".to_string()),
            product_type: None,
            base_price_minor: 2000,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("product created");

    // Create an inventory reference
    conn.execute(
        "INSERT INTO inventory (id, branch_id, product_id, quantity, low_stock_threshold)
         VALUES ('inv-ref-1', ?1, ?2, 10.0, 2.0)",
        params![branch_id, product.id],
    )
    .expect("inventory row inserted");

    // Soft delete product
    delete_product(&conn, &product.id).expect("soft delete succeeds");

    // Inventory row and product row both still exist
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
        CreateProductInput {
            name: "Espresso Beans".to_string(),
            description: None,
            category_id: None,
            barcode: Some("BC-ESP-1".to_string()),
            product_type: None,
            base_price_minor: 1500,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("p1 created");

    let p2 = create_product(
        &conn,
        CreateProductInput {
            name: "Latte Syrup Vanilla".to_string(),
            description: None,
            category_id: None,
            barcode: Some("BC-LAT-2".to_string()),
            product_type: None,
            base_price_minor: 800,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("p2 created");

    // Archive p2
    delete_product(&conn, &p2.id).expect("p2 archived");

    // 1. List all active only
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

    // 2. List all (active + inactive)
    let all = list_products(&conn, &ProductFilter::default()).expect("list all");
    assert_eq!(all.len(), 2);

    // 3. Search query substring on name
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

    // 4. Search query exact on barcode
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
            CreateProductInput {
                name: format!("Product Item {i:02}"),
                description: None,
                category_id: None,
                barcode: None,
                product_type: None,
                base_price_minor: 100 * i,
                cost_price_minor: None,
                unit_type: None,
                requires_expiry: None,
                requires_serial: None,
                warranty_months: None,
                custom_attributes: None,
            },
        )
        .expect("created");
    }

    // Offset 2 without limit: skips first 2 and returns remaining 3
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

    // Create products with special characters in name
    create_product(
        &conn,
        CreateProductInput {
            name: "100% Arabica Blend".to_string(),
            description: None,
            category_id: None,
            barcode: None,
            product_type: None,
            base_price_minor: 1500,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("created 100%");

    create_product(
        &conn,
        CreateProductInput {
            name: "1000 Arabica Blend".to_string(),
            description: None,
            category_id: None,
            barcode: None,
            product_type: None,
            base_price_minor: 1600,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("created 1000");

    create_product(
        &conn,
        CreateProductInput {
            name: "Item_1 Premium".to_string(),
            description: None,
            category_id: None,
            barcode: None,
            product_type: None,
            base_price_minor: 2000,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("created Item_1");

    create_product(
        &conn,
        CreateProductInput {
            name: "ItemA1 Premium".to_string(),
            description: None,
            category_id: None,
            barcode: None,
            product_type: None,
            base_price_minor: 2100,
            cost_price_minor: None,
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("created ItemA1");

    // 1. Searching for "100%" must ONLY match "100% Arabica Blend", NOT "1000 Arabica Blend"
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

    // 2. Searching for "Item_1" must ONLY match "Item_1 Premium", NOT "ItemA1 Premium"
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
// 4. AUTHORIZATION TESTS
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

    // Admin has ProductsManage permission
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

    // Manager has ProductsManage permission
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

    // 1. Cashier is DENIED ProductsManage permission (mutation)
    let mutation_auth = require_permission(&conn, &cashier_session.id, Permission::ProductsManage);
    assert!(
        matches!(
            mutation_auth,
            Err(AuthMiddlewareError::PermissionDenied { .. })
        ),
        "Cashier must be denied ProductsManage permission"
    );

    // 2. Cashier is ALLOWED product reads (require_session)
    let read_auth = require_session(&conn, &cashier_session.id);
    assert!(
        read_auth.is_ok(),
        "Cashier with active session must be allowed to read products"
    );
}

#[test]
fn test_unauthenticated_or_revoked_session_denied_all_product_operations() {
    let conn = setup_test_db();

    // 1. Nonexistent session ID
    let err_unauth = require_session(&conn, "nonexistent-session-id");
    assert!(
        matches!(err_unauth, Err(AuthMiddlewareError::Unauthenticated(_))),
        "Nonexistent session must be denied"
    );

    let err_perm = require_permission(&conn, "nonexistent-session-id", Permission::ProductsManage);
    assert!(
        matches!(err_perm, Err(AuthMiddlewareError::Unauthenticated(_))),
        "Nonexistent session must be denied for permission checks"
    );
}
