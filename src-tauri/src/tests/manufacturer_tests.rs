use crate::auth::middleware::{require_permission, AuthMiddlewareError};
use crate::manufacturer::{
    create_manufacturer, delete_manufacturer, get_manufacturer, list_manufacturers,
    update_manufacturer, validate_email, validate_name, validate_phone, validate_website,
    CreateManufacturerInput, ManufacturerError, ManufacturerFilter, UpdateManufacturerInput,
};
use crate::permission::Permission;
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::create_local_session;

fn make_manufacturer_fixture(name: &str) -> CreateManufacturerInput {
    CreateManufacturerInput {
        name: name.to_string(),
        description: None,
        website: None,
        support_phone: None,
        support_email: None,
    }
}

// =========================================================================
// 1. VALIDATION TESTS
// =========================================================================

#[test]
fn test_validate_manufacturer_name_trims_and_accepts_valid() {
    let result = validate_name("  Foxconn Precision  ").expect("valid name");
    assert_eq!(result, "Foxconn Precision");
}

#[test]
fn test_validate_manufacturer_name_accepts_multibyte_unicode_up_to_255_chars() {
    let unicode_name = "شركة الصناعات الدقيقة والتصنيع المتقدم";
    let result = validate_name(unicode_name).expect("multibyte unicode accepted");
    assert_eq!(result, unicode_name);

    let exact_255_unicode: String = "ص".repeat(255);
    let result_255 = validate_name(&exact_255_unicode).expect("exact 255 accepted");
    assert_eq!(result_255, exact_255_unicode);
}

#[test]
fn test_validate_manufacturer_contacts_international() {
    // Phone validation
    assert_eq!(
        validate_phone(Some(" +1 (800) 555-0199 ")).expect("us phone"),
        Some("+1 (800) 555-0199".to_string())
    );
    assert_eq!(
        validate_phone(Some("+44 20 7946 0912")).expect("uk phone"),
        Some("+44 20 7946 0912".to_string())
    );
    assert_eq!(
        validate_phone(Some("+971.4.123.4567")).expect("uae phone"),
        Some("+971.4.123.4567".to_string())
    );
    assert!(matches!(
        validate_phone(Some("phone-number#abc")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_phone(Some("+")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_phone(Some("---")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_phone(Some("...")),
        Err(ManufacturerError::Validation(_))
    ));

    // Email validation
    assert_eq!(
        validate_email(Some(" support@global-mfr.co.jp ")).expect("intl email"),
        Some("support@global-mfr.co.jp".to_string())
    );
    assert!(matches!(
        validate_email(Some("invalid-email-without-at")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_email(Some("user@domain")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_email(Some("user @domain.com")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_email(Some("a@.b")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_email(Some("a@b..com")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_email(Some("a@.example.com")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_email(Some("a@example.com.")),
        Err(ManufacturerError::Validation(_))
    ));

    // Website validation
    assert!(matches!(
        validate_website(Some("http://")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_website(Some("https://")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_website(Some(".invalid")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_website(Some("invalid.")),
        Err(ManufacturerError::Validation(_))
    ));
    assert!(matches!(
        validate_website(Some("https://invalid..com")),
        Err(ManufacturerError::Validation(_))
    ));
}

// =========================================================================
// 2. REPOSITORY & CRUD TESTS
// =========================================================================

#[test]
fn test_manufacturer_crud_lifecycle() {
    let conn = setup_test_db();
    let input = CreateManufacturerInput {
        name: "Bosch Industrial".to_string(),
        description: Some("Automotive and industrial tools".to_string()),
        website: Some("https://bosch.com".to_string()),
        support_phone: Some("+49 711 8110".to_string()),
        support_email: Some("contact@bosch.com".to_string()),
    };

    let created = create_manufacturer(&conn, input).expect("created");
    assert_eq!(created.name, "Bosch Industrial");
    assert_eq!(created.support_phone.as_deref(), Some("+49 711 8110"));
    assert_eq!(created.support_email.as_deref(), Some("contact@bosch.com"));
    assert!(created.is_active);

    let fetched = get_manufacturer(&conn, &created.id)
        .expect("query")
        .expect("found");
    assert_eq!(created, fetched);

    let updated = update_manufacturer(
        &conn,
        UpdateManufacturerInput {
            id: created.id.clone(),
            name: "Robert Bosch GmbH".to_string(),
            description: created.description,
            website: created.website,
            support_phone: Some("+49 711 8111".to_string()),
            support_email: created.support_email,
            is_active: true,
        },
    )
    .expect("updated");
    assert_eq!(updated.name, "Robert Bosch GmbH");
    assert_eq!(updated.support_phone.as_deref(), Some("+49 711 8111"));

    delete_manufacturer(&conn, &created.id).expect("deleted");
    let after_delete = get_manufacturer(&conn, &created.id)
        .expect("query")
        .expect("found");
    assert!(!after_delete.is_active);
}

// =========================================================================
// 3. UNIQUENESS & ARCHIVED REUSE TESTS
// =========================================================================

#[test]
fn test_duplicate_active_manufacturer_rejected() {
    let conn = setup_test_db();
    create_manufacturer(&conn, make_manufacturer_fixture("Foxconn")).expect("first");

    let err = create_manufacturer(&conn, make_manufacturer_fixture("foxconn")).unwrap_err();
    assert!(matches!(err, ManufacturerError::DuplicateName(_)));
}

#[test]
fn test_archived_manufacturer_name_can_be_reused() {
    let conn = setup_test_db();
    let m1 = create_manufacturer(&conn, make_manufacturer_fixture("Siemens")).expect("first");
    delete_manufacturer(&conn, &m1.id).expect("archived");

    let m2 = create_manufacturer(&conn, make_manufacturer_fixture("Siemens")).expect("reused");
    assert_ne!(m1.id, m2.id);
}

#[test]
fn test_reactivate_manufacturer_with_duplicate_conflict_rejected() {
    let conn = setup_test_db();
    let m1 = create_manufacturer(&conn, make_manufacturer_fixture("Makita")).expect("m1");
    delete_manufacturer(&conn, &m1.id).expect("m1 archived");

    let m2 = create_manufacturer(&conn, make_manufacturer_fixture("Makita")).expect("m2 active");

    let err = update_manufacturer(
        &conn,
        UpdateManufacturerInput {
            id: m1.id.clone(),
            name: "Makita".to_string(),
            description: None,
            website: None,
            support_phone: None,
            support_email: None,
            is_active: true,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ManufacturerError::DuplicateName(_)));
}

// =========================================================================
// 4. LIST & FILTER TESTS
// =========================================================================

#[test]
fn test_list_manufacturers_search_and_active_filters() {
    let conn = setup_test_db();
    create_manufacturer(&conn, make_manufacturer_fixture("Panasonic")).expect("panasonic");
    create_manufacturer(&conn, make_manufacturer_fixture("Philips")).expect("philips");
    let pioneer =
        create_manufacturer(&conn, make_manufacturer_fixture("Pioneer")).expect("pioneer");
    delete_manufacturer(&conn, &pioneer.id).expect("pioneer archived");

    // Active only
    let active = list_manufacturers(
        &conn,
        &ManufacturerFilter {
            query: None,
            is_active: Some(true),
        },
    )
    .expect("active");
    assert_eq!(active.len(), 2);

    // Search query with prefix
    let p_query = list_manufacturers(
        &conn,
        &ManufacturerFilter {
            query: Some("Phil".into()),
            is_active: Some(true),
        },
    )
    .expect("query");
    assert_eq!(p_query.len(), 1);
    assert_eq!(p_query[0].name, "Philips");
}

// =========================================================================
// 5. AUTHORIZATION TESTS
// =========================================================================

#[test]
fn test_manufacturer_authorization() {
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
