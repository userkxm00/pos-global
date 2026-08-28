use crate::auth::middleware::{require_permission, AuthMiddlewareError};
use crate::brand::{
    create_brand, delete_brand, get_brand, list_brands, update_brand, validate_name,
    validate_website, BrandError, BrandFilter, CreateBrandInput, UpdateBrandInput,
};
use crate::permission::Permission;
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::create_local_session;

fn make_brand_fixture(name: &str) -> CreateBrandInput {
    CreateBrandInput {
        name: name.to_string(),
        description: None,
        website: None,
    }
}

// =========================================================================
// 1. VALIDATION TESTS
// =========================================================================

#[test]
fn test_validate_brand_name_trims_and_accepts_valid() {
    let result = validate_name("  Acme Supplies  ").expect("valid name");
    assert_eq!(result, "Acme Supplies");
}

#[test]
fn test_validate_brand_name_accepts_multibyte_unicode_up_to_255_chars() {
    let unicode_name = "ماركة النخبة العالمية للملابس والأقمشة";
    let result = validate_name(unicode_name).expect("multibyte unicode accepted");
    assert_eq!(result, unicode_name);

    let exact_255_unicode: String = "ن".repeat(255);
    let result_255 = validate_name(&exact_255_unicode).expect("exact 255 accepted");
    assert_eq!(result_255, exact_255_unicode);
}

#[test]
fn test_validate_brand_name_rejects_empty_and_too_long() {
    assert!(matches!(validate_name(""), Err(BrandError::Validation(_))));
    assert!(matches!(
        validate_name("   \t  "),
        Err(BrandError::Validation(_))
    ));
    let too_long = "x".repeat(256);
    assert!(matches!(
        validate_name(&too_long),
        Err(BrandError::Validation(_))
    ));
}

#[test]
fn test_validate_brand_website() {
    assert_eq!(validate_website(None).expect("none"), None);
    assert_eq!(validate_website(Some("   ")).expect("empty"), None);
    assert_eq!(
        validate_website(Some("  https://acme.com  ")).expect("valid"),
        Some("https://acme.com".to_string())
    );
    assert_eq!(
        validate_website(Some("www.acme.com")).expect("domain"),
        Some("www.acme.com".to_string())
    );

    // Whitespace inside URL rejected
    assert!(matches!(
        validate_website(Some("https://ac me.com")),
        Err(BrandError::Validation(_))
    ));

    // Malformed URLs without valid host rejected
    assert!(matches!(
        validate_website(Some("http://")),
        Err(BrandError::Validation(_))
    ));
    assert!(matches!(
        validate_website(Some("https://")),
        Err(BrandError::Validation(_))
    ));
    assert!(matches!(
        validate_website(Some(".invalid")),
        Err(BrandError::Validation(_))
    ));
    assert!(matches!(
        validate_website(Some("invalid.")),
        Err(BrandError::Validation(_))
    ));
    assert!(matches!(
        validate_website(Some("https://invalid..com")),
        Err(BrandError::Validation(_))
    ));
}

// =========================================================================
// 2. REPOSITORY & CRUD TESTS
// =========================================================================

#[test]
fn test_brand_crud_lifecycle() {
    let conn = setup_test_db();
    let input = CreateBrandInput {
        name: "Logitech".to_string(),
        description: Some("Computer peripherals".to_string()),
        website: Some("https://logitech.com".to_string()),
    };

    let created = create_brand(&conn, input).expect("created");
    assert_eq!(created.name, "Logitech");
    assert_eq!(created.description.as_deref(), Some("Computer peripherals"));
    assert_eq!(created.website.as_deref(), Some("https://logitech.com"));
    assert!(created.is_active);

    let fetched = get_brand(&conn, &created.id)
        .expect("query")
        .expect("found");
    assert_eq!(created, fetched);

    let updated = update_brand(
        &conn,
        UpdateBrandInput {
            id: created.id.clone(),
            name: "Logitech International".to_string(),
            description: Some("Peripherals & Software".to_string()),
            website: Some("https://www.logitech.com".to_string()),
            is_active: true,
        },
    )
    .expect("updated");
    assert_eq!(updated.name, "Logitech International");
    assert_eq!(
        updated.description.as_deref(),
        Some("Peripherals & Software")
    );

    delete_brand(&conn, &created.id).expect("deleted");
    let after_delete = get_brand(&conn, &created.id)
        .expect("query")
        .expect("found");
    assert!(!after_delete.is_active);
}

// =========================================================================
// 3. UNIQUENESS & ARCHIVED REUSE TESTS
// =========================================================================

#[test]
fn test_duplicate_active_brand_rejected() {
    let conn = setup_test_db();
    create_brand(&conn, make_brand_fixture("Samsung")).expect("first");

    let err = create_brand(&conn, make_brand_fixture("samsung")).unwrap_err();
    assert!(matches!(err, BrandError::DuplicateName(_)));
}

#[test]
fn test_archived_brand_name_can_be_reused() {
    let conn = setup_test_db();
    let b1 = create_brand(&conn, make_brand_fixture("Sony")).expect("first");
    delete_brand(&conn, &b1.id).expect("archived");

    let b2 = create_brand(&conn, make_brand_fixture("Sony")).expect("reused");
    assert_ne!(b1.id, b2.id);
}

#[test]
fn test_reactivate_brand_with_duplicate_conflict_rejected() {
    let conn = setup_test_db();
    let b1 = create_brand(&conn, make_brand_fixture("LG")).expect("b1");
    delete_brand(&conn, &b1.id).expect("b1 archived");

    let b2 = create_brand(&conn, make_brand_fixture("LG")).expect("b2 active");

    let err = update_brand(
        &conn,
        UpdateBrandInput {
            id: b1.id.clone(),
            name: "LG".to_string(),
            description: None,
            website: None,
            is_active: true,
        },
    )
    .unwrap_err();
    assert!(matches!(err, BrandError::DuplicateName(_)));
}

// =========================================================================
// 4. LIST & FILTER TESTS
// =========================================================================

#[test]
fn test_list_brands_search_and_active_filters() {
    let conn = setup_test_db();
    create_brand(&conn, make_brand_fixture("Apple")).expect("apple");
    create_brand(&conn, make_brand_fixture("Asus")).expect("asus");
    let acer = create_brand(&conn, make_brand_fixture("Acer")).expect("acer");
    delete_brand(&conn, &acer.id).expect("acer archived");

    // Active only
    let active = list_brands(
        &conn,
        &BrandFilter {
            query: None,
            is_active: Some(true),
        },
    )
    .expect("active");
    assert_eq!(active.len(), 2);

    // Search query with prefix
    let a_query = list_brands(
        &conn,
        &BrandFilter {
            query: Some("As".into()),
            is_active: Some(true),
        },
    )
    .expect("query");
    assert_eq!(a_query.len(), 1);
    assert_eq!(a_query[0].name, "Asus");
}

// =========================================================================
// 5. AUTHORIZATION TESTS
// =========================================================================

#[test]
fn test_brand_authorization() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    let admin =
        create_test_user_with_creds(&conn, &branch_id, "Admin User", None, None, None, "admin")
            .expect("admin");
    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier User",
        None,
        None,
        None,
        "cashier",
    )
    .expect("cashier");

    let admin_sess = create_local_session(&conn, &admin.id, &branch_id, None, 8).expect("sess");
    let cashier_sess = create_local_session(&conn, &cashier.id, &branch_id, None, 8).expect("sess");

    assert!(require_permission(&conn, &admin_sess.id, Permission::ProductsManage).is_ok());
    let err = require_permission(&conn, &cashier_sess.id, Permission::ProductsManage).unwrap_err();
    assert!(matches!(err, AuthMiddlewareError::PermissionDenied(_)));
}
