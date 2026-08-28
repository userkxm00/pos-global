use crate::permission::{
    check_role_all_permissions, check_role_any_permission, check_role_permission,
    evaluate_user_permission, get_effective_user_permissions, grant_role_permission,
    list_role_permissions, list_user_permission_overrides, reconcile_role_permissions,
    remove_user_permission_override, revoke_role_permission, set_user_permission_override,
    validate_role_catalog_integrity, validate_scope, Permission, PermissionError, Role,
    PERMISSION_CATALOG, ROLE_CATALOG,
};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use std::collections::HashSet;

#[test]
fn permission_catalog_is_exhaustive_and_bijection() {
    assert_eq!(
        PERMISSION_CATALOG.len(),
        Permission::ALL.len(),
        "Catalog length must match Permission::ALL"
    );

    let mut catalog_permissions = HashSet::new();
    let mut catalog_codes = HashSet::new();

    for entry in PERMISSION_CATALOG {
        assert!(
            catalog_permissions.insert(entry.permission),
            "Duplicate permission enum in catalog: {:?}",
            entry.permission
        );
        assert!(
            catalog_codes.insert(entry.code),
            "Duplicate code in catalog: {}",
            entry.code
        );
        assert!(!entry.code.is_empty(), "Code must not be empty");
        assert!(
            !entry.description.is_empty(),
            "Description must not be empty"
        );
    }

    for permission in Permission::ALL {
        assert!(
            catalog_permissions.contains(permission),
            "Permission {:?} missing from PERMISSION_CATALOG",
            permission
        );
        assert!(
            !permission.as_str().is_empty(),
            "as_str() must not be empty for {:?}",
            permission
        );
        assert!(
            !permission.description().is_empty(),
            "description() must not be empty for {:?}",
            permission
        );
        assert_eq!(
            Permission::parse(permission.as_str()),
            Some(*permission),
            "Parsing roundtrip must succeed for {:?}",
            permission
        );
    }
}

#[test]
fn role_catalog_is_exhaustive_and_bijection() {
    assert_eq!(
        ROLE_CATALOG.len(),
        Role::ALL.len(),
        "Role catalog length must match Role::ALL"
    );

    let mut catalog_roles = HashSet::new();
    let mut catalog_codes = HashSet::new();

    for entry in ROLE_CATALOG {
        assert!(
            catalog_roles.insert(entry.role),
            "Duplicate role enum in catalog: {:?}",
            entry.role
        );
        assert!(
            catalog_codes.insert(entry.code),
            "Duplicate code in role catalog: {}",
            entry.code
        );
        assert!(!entry.code.is_empty(), "Role code must not be empty");
    }

    for role in Role::ALL {
        assert!(
            catalog_roles.contains(role),
            "Role {:?} missing from ROLE_CATALOG",
            role
        );
        assert!(
            !role.as_str().is_empty(),
            "as_str() must not be empty for {:?}",
            role
        );
        assert!(
            !role.default_permissions().is_empty(),
            "default_permissions() must not be empty for {:?}",
            role
        );
        assert_eq!(
            Role::parse(role.as_str()),
            Some(*role),
            "Parsing roundtrip must succeed for {:?}",
            role
        );
    }
}

#[test]
fn permission_catalog_exact_matching_and_fail_closed() {
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
        let perm = Permission::parse(code).expect("failed to parse permission code");
        assert_eq!(perm.as_str(), code);
        assert_eq!(perm.to_string(), code);
        assert!(!perm.description().is_empty());
    }

    // Fail-closed verification: case, whitespace, prefixes, and unknown strings
    assert!(Permission::parse("sales.Create").is_none());
    assert!(Permission::parse("SALES.CREATE").is_none());
    assert!(Permission::parse(" sales.create ").is_none());
    assert!(Permission::parse("sales.").is_none());
    assert!(Permission::parse("sales.create.extra").is_none());
    assert!(Permission::parse("sales.*").is_none());
    assert!(Permission::parse("").is_none());
    assert!(Permission::parse("unknown_permission").is_none());
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
    assert!(Role::parse("Admin").is_none());
    assert!(Role::parse("MANAGER").is_none());
    assert!(Role::parse(" cashier ").is_none());
    assert!(Role::parse("superuser").is_none());
    assert!(Role::parse("root").is_none());
    assert!(Role::parse("guest").is_none());
    assert!(Role::parse("").is_none());
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
fn database_role_permission_revocation_is_strictly_authoritative() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Authoritative DB Cashier",
        Some("cashier_db_auth"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    // Baseline: Cashier has sales.create in DB seed
    assert!(
        evaluate_user_permission(&conn, &cashier.id, &cashier.role, Permission::SalesCreate)
            .expect("eval initial")
    );

    // Explicitly revoke sales.create from role cashier in DB
    revoke_role_permission(&conn, Role::Cashier, Permission::SalesCreate)
        .expect("revoke role perm");

    // Crucial security check: Cashier MUST be denied sales.create now!
    // It must NOT silently fall back to code defaults!
    let allowed_after_revoke =
        evaluate_user_permission(&conn, &cashier.id, &cashier.role, Permission::SalesCreate)
            .expect("eval after role perm revocation");

    assert!(
        !allowed_after_revoke,
        "Revoked role permission in DB must remain DENIED and not fall back to code default"
    );

    // Re-grant role permission in DB
    grant_role_permission(&conn, Role::Cashier, Permission::SalesCreate).expect("grant role perm");

    assert!(
        evaluate_user_permission(&conn, &cashier.id, &cashier.role, Permission::SalesCreate)
            .expect("eval after re-grant")
    );
}

#[test]
fn fallback_to_code_defaults_happens_only_when_role_has_zero_db_rows() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Zero Row Cashier",
        Some("zero_cashier"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    // Delete all DB role_permissions for cashier
    conn.execute("DELETE FROM role_permissions WHERE role = 'cashier'", [])
        .expect("delete all cashier role perms");

    // Now cashier has ZERO DB rows -> fallback to code defaults occurs
    assert!(
        evaluate_user_permission(&conn, &cashier.id, &cashier.role, Permission::SalesCreate)
            .expect("eval zero db rows")
    );
}

#[test]
fn list_role_permissions_queries_database_mappings() {
    let conn = setup_test_db();

    let cashier_perms = list_role_permissions(&conn, Role::Cashier).expect("list perms");
    assert_eq!(cashier_perms.len(), 5);
    assert!(cashier_perms.contains(&Permission::SalesCreate));
    assert!(cashier_perms.contains(&Permission::CashOpen));
    assert!(!cashier_perms.contains(&Permission::SalesVoid));
}

#[test]
fn catalog_integrity_and_reconciliation_verifies_seed_and_detects_mismatches() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    // 1. Seeded database matches the compiled catalog with 0 mismatches
    let mismatches = validate_role_catalog_integrity(&conn).expect("validate integrity");
    assert!(
        mismatches.is_empty(),
        "Seeded database must have 0 catalog mismatches"
    );

    // 2. Simulate missing expected permission for Admin in DB
    revoke_role_permission(&conn, Role::Admin, Permission::LicenseManage)
        .expect("simulate missing permission");

    let detected = validate_role_catalog_integrity(&conn).expect("validate after mismatch");
    assert_eq!(detected.len(), 1);
    assert_eq!(detected[0].role, Role::Admin);
    assert_eq!(detected[0].missing_permission, Permission::LicenseManage);

    // 3. Authorization check stays DENIED on the missing row (no silent auto-granting)
    let admin = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Integrity Admin",
        Some("integrity_admin"),
        None,
        None,
        "admin",
    )
    .expect("create admin");

    assert!(
        !evaluate_user_permission(&conn, &admin.id, &admin.role, Permission::LicenseManage)
            .expect("eval missing permission")
    );

    // 4. Safe explicit reconciliation resolves the mismatch and restores catalog defaults
    let reconciled = reconcile_role_permissions(&conn).expect("reconcile");
    assert!(reconciled >= 1, "Must reconcile at least the missing row");

    // 5. Idempotent check: second reconciliation inserts 0 rows
    let second_run = reconcile_role_permissions(&conn).expect("second reconcile");
    assert_eq!(
        second_run, 0,
        "Reconciliation must be idempotent when already consistent"
    );

    let clean = validate_role_catalog_integrity(&conn).expect("validate after reconcile");
    assert!(
        clean.is_empty(),
        "After reconciliation, catalog mismatches must be zero"
    );

    assert!(
        evaluate_user_permission(&conn, &admin.id, &admin.role, Permission::LicenseManage)
            .expect("eval after reconcile")
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
