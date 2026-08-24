use crate::permission::{
    check_role_all_permissions, check_role_any_permission, check_role_permission,
    evaluate_user_permission, get_effective_user_permissions, list_user_permission_overrides,
    remove_user_permission_override, set_user_permission_override, validate_scope, Permission,
    PermissionError, Role,
};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};

#[test]
fn permission_catalog_exact_matching_and_fail_closed() {
    // Verify all 17 authoritative catalog permissions parse exactly
    let all_codes = [
        "sales.create",
        "sales.refund",
        "sales.void",
        "inventory.adjust",
        "inventory.transfer",
        "products.manage",
        "purchases.manage",
        "customers.manage",
        "debts.manage",
        "cash.open",
        "cash.close",
        "cash.adjust",
        "reports.view",
        "reports.export",
        "users.manage",
        "settings.manage",
        "license.manage",
    ];

    assert_eq!(Permission::ALL.len(), 17);
    for code in all_codes {
        let perm = Permission::parse(code).unwrap_or_else(|| panic!("failed to parse {code}"));
        assert_eq!(perm.as_str(), code);
        assert_eq!(perm.to_string(), code);
    }

    // Fail-closed verification: case, whitespace, prefixes, and unknown strings
    assert_eq!(Permission::parse("sales.Create"), None);
    assert_eq!(Permission::parse("SALES.CREATE"), None);
    assert_eq!(Permission::parse(" sales.create "), None);
    assert_eq!(Permission::parse("sales."), None);
    assert_eq!(Permission::parse("sales.create.extra"), None);
    assert_eq!(Permission::parse("sales.*"), None);
    assert_eq!(Permission::parse(""), None);
    assert_eq!(Permission::parse("unknown_permission"), None);
}

#[test]
fn role_exact_matching_and_fail_closed() {
    assert_eq!(Role::parse("admin"), Some(Role::Admin));
    assert_eq!(Role::parse("manager"), Some(Role::Manager));
    assert_eq!(Role::parse("cashier"), Some(Role::Cashier));

    assert_eq!(Role::Admin.as_str(), "admin");
    assert_eq!(Role::Manager.as_str(), "manager");
    assert_eq!(Role::Cashier.as_str(), "cashier");

    // Fail-closed verification
    assert_eq!(Role::parse("Admin"), None);
    assert_eq!(Role::parse("MANAGER"), None);
    assert_eq!(Role::parse(" cashier "), None);
    assert_eq!(Role::parse("superuser"), None);
    assert_eq!(Role::parse("root"), None);
    assert_eq!(Role::parse("guest"), None);
    assert_eq!(Role::parse(""), None);
}

#[test]
fn built_in_role_permission_mappings() {
    // Admin has all 17 permissions
    for perm in Permission::ALL {
        assert!(
            Role::Admin.has_default_permission(*perm),
            "Admin must have {perm}"
        );
        assert!(check_role_permission("admin", perm.as_str()));
    }

    // Manager has 15 permissions (all except users.manage and license.manage)
    assert!(Role::Manager.has_default_permission(Permission::SalesCreate));
    assert!(Role::Manager.has_default_permission(Permission::SalesRefund));
    assert!(Role::Manager.has_default_permission(Permission::SalesVoid));
    assert!(Role::Manager.has_default_permission(Permission::InventoryAdjust));
    assert!(Role::Manager.has_default_permission(Permission::InventoryTransfer));
    assert!(Role::Manager.has_default_permission(Permission::ProductsManage));
    assert!(Role::Manager.has_default_permission(Permission::PurchasesManage));
    assert!(Role::Manager.has_default_permission(Permission::CustomersManage));
    assert!(Role::Manager.has_default_permission(Permission::DebtsManage));
    assert!(Role::Manager.has_default_permission(Permission::CashOpen));
    assert!(Role::Manager.has_default_permission(Permission::CashClose));
    assert!(Role::Manager.has_default_permission(Permission::CashAdjust));
    assert!(Role::Manager.has_default_permission(Permission::ReportsView));
    assert!(Role::Manager.has_default_permission(Permission::ReportsExport));
    assert!(Role::Manager.has_default_permission(Permission::SettingsManage));
    assert!(!Role::Manager.has_default_permission(Permission::UsersManage));
    assert!(!Role::Manager.has_default_permission(Permission::LicenseManage));

    // Cashier has exactly 5 permissions
    assert!(Role::Cashier.has_default_permission(Permission::SalesCreate));
    assert!(Role::Cashier.has_default_permission(Permission::CustomersManage));
    assert!(Role::Cashier.has_default_permission(Permission::ReportsView));
    assert!(Role::Cashier.has_default_permission(Permission::CashOpen));
    assert!(Role::Cashier.has_default_permission(Permission::CashClose));

    // Privileged permissions strictly denied to Cashier
    assert!(!Role::Cashier.has_default_permission(Permission::SalesRefund));
    assert!(!Role::Cashier.has_default_permission(Permission::SalesVoid));
    assert!(!Role::Cashier.has_default_permission(Permission::InventoryAdjust));
    assert!(!Role::Cashier.has_default_permission(Permission::InventoryTransfer));
    assert!(!Role::Cashier.has_default_permission(Permission::ProductsManage));
    assert!(!Role::Cashier.has_default_permission(Permission::PurchasesManage));
    assert!(!Role::Cashier.has_default_permission(Permission::DebtsManage));
    assert!(!Role::Cashier.has_default_permission(Permission::CashAdjust));
    assert!(!Role::Cashier.has_default_permission(Permission::ReportsExport));
    assert!(!Role::Cashier.has_default_permission(Permission::UsersManage));
    assert!(!Role::Cashier.has_default_permission(Permission::SettingsManage));
    assert!(!Role::Cashier.has_default_permission(Permission::LicenseManage));

    // Unknown or malformed roles fail closed
    assert!(!check_role_permission("intruder", "sales.create"));
    assert!(!check_role_permission("", "sales.create"));
    assert!(!check_role_permission("cashier ", "sales.create"));
    assert!(!check_role_permission("admin", "unknown.action"));
}

#[test]
fn multi_permission_check_semantics() {
    // All required permissions check
    let cashier_req = [Permission::SalesCreate, Permission::ReportsView];
    assert!(check_role_all_permissions("cashier", &cashier_req));

    let cashier_mixed = [Permission::SalesCreate, Permission::InventoryAdjust];
    assert!(!check_role_all_permissions("cashier", &cashier_mixed));

    // Any required permission check
    assert!(check_role_any_permission("cashier", &cashier_mixed));

    let cashier_none = [Permission::InventoryAdjust, Permission::LicenseManage];
    assert!(!check_role_any_permission("cashier", &cashier_none));

    // Empty requirements fail closed
    assert!(!check_role_all_permissions("admin", &[]));
    assert!(!check_role_any_permission("admin", &[]));

    // Unknown roles fail closed
    assert!(!check_role_all_permissions("unknown", &cashier_req));
    assert!(!check_role_any_permission("unknown", &cashier_req));
}

#[test]
fn scope_validation_enforces_tenant_and_branch_boundaries() {
    let org_a = "org-111";
    let org_b = "org-222";
    let branch_a = "branch-aaa";
    let branch_b = "branch-bbb";

    // Matching org and branch succeeds
    assert!(validate_scope(Some(org_a), branch_a, Some(org_a), Some(branch_a)).is_ok());
    assert!(validate_scope(Some(org_a), branch_a, None, Some(branch_a)).is_ok());
    assert!(validate_scope(Some(org_a), branch_a, Some(org_a), None).is_ok());

    // Mismatched org fails closed
    let org_err = validate_scope(Some(org_a), branch_a, Some(org_b), Some(branch_a)).unwrap_err();
    assert!(matches!(org_err, PermissionError::ScopeMismatch { .. }));

    // Mismatched branch fails closed
    let branch_err =
        validate_scope(Some(org_a), branch_a, Some(org_a), Some(branch_b)).unwrap_err();
    assert!(matches!(branch_err, PermissionError::ScopeMismatch { .. }));

    // Session lacking org when target org expected fails closed
    let no_org_err = validate_scope(None, branch_a, Some(org_a), Some(branch_a)).unwrap_err();
    assert!(matches!(no_org_err, PermissionError::ScopeMismatch { .. }));
}

#[test]
fn database_user_override_allows_granular_elevation_and_restriction() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier With Override",
        Some("cashier_override"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    // 1. Initial baseline: Cashier lacks inventory.adjust
    assert!(!evaluate_user_permission(
        &conn,
        &cashier.id,
        &cashier.role,
        Permission::InventoryAdjust
    )
    .expect("eval baseline"));

    // 2. Grant allow override for inventory.adjust
    set_user_permission_override(&conn, &cashier.id, Permission::InventoryAdjust, "allow")
        .expect("set allow override");

    assert!(evaluate_user_permission(
        &conn,
        &cashier.id,
        &cashier.role,
        Permission::InventoryAdjust
    )
    .expect("eval after allow override"));

    let overrides = list_user_permission_overrides(&conn, &cashier.id).expect("list overrides");
    assert_eq!(overrides.len(), 1);
    assert_eq!(overrides[0], (Permission::InventoryAdjust, "allow".into()));

    // 3. Admin user with explicit deny override (Deny takes highest precedence)
    let admin = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Restricted Admin",
        Some("admin_restricted"),
        None,
        None,
        "admin",
    )
    .expect("create admin");

    // Baseline: Admin has sales.void
    assert!(
        evaluate_user_permission(&conn, &admin.id, &admin.role, Permission::SalesVoid)
            .expect("eval admin baseline")
    );

    // Set explicit deny override for sales.void
    set_user_permission_override(&conn, &admin.id, Permission::SalesVoid, "deny")
        .expect("set deny override");

    // Deny override blocks sales.void even for admin
    assert!(
        !evaluate_user_permission(&conn, &admin.id, &admin.role, Permission::SalesVoid)
            .expect("eval admin after deny override")
    );

    // 4. Revoking override restores base role inheritance
    remove_user_permission_override(&conn, &admin.id, Permission::SalesVoid)
        .expect("remove override");

    assert!(
        evaluate_user_permission(&conn, &admin.id, &admin.role, Permission::SalesVoid)
            .expect("eval admin after remove override")
    );
}

#[test]
fn effective_user_permissions_computes_complete_catalog_subset() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Standard Cashier",
        Some("std_cashier"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    let effective =
        get_effective_user_permissions(&conn, &cashier.id, "cashier").expect("get effective");

    // Baseline Cashier has 5 permissions
    assert_eq!(effective.len(), 5);
    assert!(effective.contains(&Permission::SalesCreate));
    assert!(effective.contains(&Permission::CustomersManage));
    assert!(effective.contains(&Permission::ReportsView));
    assert!(effective.contains(&Permission::CashOpen));
    assert!(effective.contains(&Permission::CashClose));
    assert!(!effective.contains(&Permission::SalesVoid));

    // Grant override
    set_user_permission_override(&conn, &cashier.id, Permission::SalesVoid, "allow")
        .expect("grant void");

    let updated_effective =
        get_effective_user_permissions(&conn, &cashier.id, "cashier").expect("get updated");
    assert_eq!(updated_effective.len(), 6);
    assert!(updated_effective.contains(&Permission::SalesVoid));
}
