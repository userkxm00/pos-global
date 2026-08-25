// Authentication & Authorization Integration Test Suite.
// F1.09 — Auth Integration Tests
// Cross-module integration across Supabase identity boundary, local user/session,
// roles/permissions, authorization middleware, and multi-tenant context.

use crate::auth::middleware::{
    authorize, require_all_permissions, require_any_permission, require_permission,
    require_scoped_permission, require_session, AuthMiddlewareError, AuthorizeRequest,
};
use crate::branch::{create_branch, update_branch, CreateBranchInput, UpdateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::permission::{
    get_effective_user_permissions, set_user_permission_override, Permission, PERMISSION_CATALOG,
};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::{
    create_local_session, revoke_local_session, validate_local_session, SessionValidationError,
};
use crate::user::{
    create_user, update_user, verify_user_password, verify_user_pin, CreateUserInput,
    UpdateUserInput, UserError,
};

#[test]
fn e2e_valid_password_login_to_middleware_authorized_operation() {
    let conn = setup_test_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn);
    let fixture_token = ["integration", "auth", "token", "101"].join("-");
    let fixture_code = ["1", "2", "3", "4"].join("");

    // 1. Provision user
    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Alice Cashier",
        Some("alice_cashier"),
        Some(fixture_token.as_str()),
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("user created");

    // 2. Verify password login
    let verified_user = verify_user_password(&conn, "alice_cashier", fixture_token.as_str())
        .expect("password verification succeeds");
    assert_eq!(verified_user.id, user.id);

    // 3. Create local session
    let session = create_local_session(&conn, &user.id, &branch_id, "password", None)
        .expect("session created");

    // 4. Validate session state
    let session_ctx =
        validate_local_session(&conn, &session.id).expect("session must be valid and active");
    assert_eq!(session_ctx.user_id, user.id);
    assert_eq!(session_ctx.branch_id, branch_id);
    assert_eq!(
        session_ctx.organization_id.as_deref(),
        Some(org_id.as_str())
    );
    assert_eq!(session_ctx.role, "cashier");

    // 5. Middleware authorization for standard cashier permission (sales.create)
    let auth_ctx = require_permission(&conn, &session.id, Permission::SalesCreate)
        .expect("cashier should be authorized for sales.create");

    assert_eq!(auth_ctx.user_id, user.id);
    assert_eq!(auth_ctx.branch_id, branch_id);
    assert_eq!(auth_ctx.organization_id.as_deref(), Some(org_id.as_str()));
    assert_eq!(auth_ctx.role, "cashier");
}

#[test]
fn e2e_valid_pin_login_to_fast_pos_operation() {
    let conn = setup_test_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["9", "8", "7", "6"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Bob FastCashier",
        Some("bob_fast"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("user created");

    // 1. PIN verification
    let verified_user =
        verify_user_pin(&conn, &user.id, fixture_code.as_str()).expect("PIN verification succeeds");
    assert_eq!(verified_user.id, user.id);

    // 2. Create session with PIN auth level
    let session =
        create_local_session(&conn, &user.id, &branch_id, "pin", None).expect("session created");

    // 3. Authorize cashier cash drawer operation
    let auth_ctx = require_permission(&conn, &session.id, Permission::CashOpen)
        .expect("cashier is authorized for cash.open");
    assert_eq!(auth_ctx.user_id, user.id);
    assert_eq!(auth_ctx.organization_id.as_deref(), Some(org_id.as_str()));
}

#[test]
fn e2e_authentication_failure_and_missing_identity_fail_closed() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let valid_token = ["valid", "token", "202"].join("-");
    let invalid_token = ["mismatch", "token", "303"].join("-");
    let valid_code = ["5", "5", "5", "5"].join("");
    let invalid_code = ["9", "9", "9", "9"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Charlie AuthTest",
        Some("charlie_auth"),
        Some(valid_token.as_str()),
        Some(valid_code.as_str()),
        "cashier",
    )
    .expect("user created");

    // 1. Invalid password fails
    let err_pw = verify_user_password(&conn, "charlie_auth", invalid_token.as_str()).unwrap_err();
    assert!(matches!(err_pw, UserError::InvalidCredentials(_)));

    // 2. Invalid PIN fails
    let err_pin = verify_user_pin(&conn, &user.id, invalid_code.as_str()).unwrap_err();
    assert!(matches!(err_pin, UserError::InvalidCredentials(_)));

    // 3. Nonexistent user fails
    let err_nonexistent =
        verify_user_password(&conn, "ghost_user", invalid_token.as_str()).unwrap_err();
    assert!(matches!(err_nonexistent, UserError::InvalidCredentials(_)));

    // 4. Missing / empty / unknown session tokens fail closed
    let empty_session_err = require_session(&conn, "").unwrap_err();
    assert!(matches!(
        empty_session_err,
        AuthMiddlewareError::Unauthenticated(_)
    ));

    let unknown_session_err = require_session(&conn, "unknown-session-token").unwrap_err();
    assert!(matches!(
        unknown_session_err,
        AuthMiddlewareError::Unauthenticated(_)
    ));
}

#[test]
fn e2e_revoked_session_lifecycle_flow() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["1", "1", "2", "2"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "David RevokeTest",
        Some("david_revoke"),
        None,
        Some(fixture_code.as_str()),
        "manager",
    )
    .expect("user created");

    let session =
        create_local_session(&conn, &user.id, &branch_id, "pin", None).expect("session created");

    // Initially authorized
    assert!(require_permission(&conn, &session.id, Permission::InventoryAdjust).is_ok());

    // Revoke session (user logout)
    revoke_local_session(&conn, &session.id).expect("session revoked");

    // Subsequent access fails closed
    let err = require_permission(&conn, &session.id, Permission::InventoryAdjust).unwrap_err();
    assert!(matches!(err, AuthMiddlewareError::SessionRevoked(_)));

    // Re-validating local session fails closed
    let err_val = validate_local_session(&conn, &session.id).unwrap_err();
    assert_eq!(err_val, SessionValidationError::Revoked);
}

#[test]
fn e2e_expired_session_lifecycle_flow() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["3", "3", "4", "4"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Eve ExpireTest",
        Some("eve_expire"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("user created");

    // Create session that expired in the past (-1 hour)
    let expired_session = create_local_session(&conn, &user.id, &branch_id, "pin", Some(-1))
        .expect("expired session created");

    // Access fails with SessionExpired
    let err = require_session(&conn, &expired_session.id).unwrap_err();
    assert!(matches!(err, AuthMiddlewareError::SessionExpired(_)));

    // Direct local session validation fails with Expired
    let err_val = validate_local_session(&conn, &expired_session.id).unwrap_err();
    assert_eq!(err_val, SessionValidationError::Expired);
}

#[test]
fn e2e_inactive_user_and_inactive_branch_fail_closed() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["4", "4", "5", "5"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Frank InactiveTest",
        Some("frank_inactive"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("user created");

    let session =
        create_local_session(&conn, &user.id, &branch_id, "pin", None).expect("session created");

    // 1. Deactivate user -> access denied
    update_user(
        &conn,
        &user.id,
        UpdateUserInput {
            full_name: None,
            username: None,
            password: None,
            pin: None,
            role: None,
            is_active: Some(false),
            supabase_user_id: None,
        },
    )
    .expect("user deactivated");

    let err_user_inactive = require_session(&conn, &session.id).unwrap_err();
    assert!(matches!(
        err_user_inactive,
        AuthMiddlewareError::Unauthenticated(_)
    ));

    // 2. Reactivate user -> access restored
    update_user(
        &conn,
        &user.id,
        UpdateUserInput {
            full_name: None,
            username: None,
            password: None,
            pin: None,
            role: None,
            is_active: Some(true),
            supabase_user_id: None,
        },
    )
    .expect("user reactivated");
    assert!(require_session(&conn, &session.id).is_ok());

    // 3. Deactivate branch -> access denied
    update_branch(
        &conn,
        UpdateBranchInput {
            id: branch_id.clone(),
            name: "Main Downtown Branch".to_string(),
            address: None,
            currency: "USD".to_string(),
            is_active: false,
        },
    )
    .expect("branch deactivated");

    let err_branch_inactive = require_session(&conn, &session.id).unwrap_err();
    assert!(matches!(
        err_branch_inactive,
        AuthMiddlewareError::Unauthenticated(_)
    ));
}

#[test]
fn e2e_role_privilege_and_insufficient_permission_flow() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["5", "5", "6", "6"].join("");

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Grace Cashier",
        Some("grace_cashier"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("cashier created");

    let session =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("session created");

    // Cashier allowed: sales.create, customers.manage, cash.open, cash.close, reports.view
    assert!(require_permission(&conn, &session.id, Permission::SalesCreate).is_ok());
    assert!(require_permission(&conn, &session.id, Permission::CustomersManage).is_ok());
    assert!(require_permission(&conn, &session.id, Permission::ReportsView).is_ok());

    // Cashier denied: users.manage, license.manage, inventory.adjust, settings.manage
    let err_users = require_permission(&conn, &session.id, Permission::UsersManage).unwrap_err();
    assert!(matches!(
        err_users,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    let err_license =
        require_permission(&conn, &session.id, Permission::LicenseManage).unwrap_err();
    assert!(matches!(
        err_license,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    let err_settings =
        require_permission(&conn, &session.id, Permission::SettingsManage).unwrap_err();
    assert!(matches!(
        err_settings,
        AuthMiddlewareError::PermissionDenied { .. }
    ));
}

#[test]
fn e2e_explicit_user_permission_allow_and_deny_overrides() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["6", "6", "7", "7"].join("");

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Heidi OverrideTest",
        Some("heidi_override"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("cashier created");

    let session =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("session created");

    // Initial state: cashier does NOT have inventory.adjust
    let err_initial =
        require_permission(&conn, &session.id, Permission::InventoryAdjust).unwrap_err();
    assert!(matches!(
        err_initial,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    // 1. Grant explicit allow override
    set_user_permission_override(&conn, &cashier.id, Permission::InventoryAdjust, "allow")
        .expect("set allow override");
    assert!(require_permission(&conn, &session.id, Permission::InventoryAdjust).is_ok());

    // 2. Grant explicit deny override on default role permission (sales.create)
    assert!(require_permission(&conn, &session.id, Permission::SalesCreate).is_ok());
    set_user_permission_override(&conn, &cashier.id, Permission::SalesCreate, "deny")
        .expect("set deny override");
    let err_denied = require_permission(&conn, &session.id, Permission::SalesCreate).unwrap_err();
    assert!(matches!(
        err_denied,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    // 3. Clear deny override (set to allow or reset) -> restores ability
    set_user_permission_override(&conn, &cashier.id, Permission::SalesCreate, "allow")
        .expect("update override to allow");
    assert!(require_permission(&conn, &session.id, Permission::SalesCreate).is_ok());
}

#[test]
fn e2e_multi_tenant_and_cross_branch_context_propagation() {
    let conn = setup_test_db();
    let fixture_code = ["7", "7", "8", "8"].join("");

    // Setup Org A with Branch A1 and Branch A2
    let org_a = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Organization Alpha".to_string(),
            default_currency: Some("USD".to_string()),
            default_language: Some("en".to_string()),
        },
    )
    .expect("org a");

    let branch_a1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id.clone(),
            name: "Branch Alpha-1".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch a1");

    let branch_a2 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id.clone(),
            name: "Branch Alpha-2".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch a2");

    // Setup Org B with Branch B1
    let org_b = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Organization Beta".to_string(),
            default_currency: Some("EUR".to_string()),
            default_language: Some("de".to_string()),
        },
    )
    .expect("org b");

    let branch_b1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_b.id.clone(),
            name: "Branch Beta-1".to_string(),
            address: None,
            currency: Some("EUR".to_string()),
            is_active: Some(true),
        },
    )
    .expect("branch b1");

    // Manager in Org A, Branch A1
    let manager = create_test_user_with_creds(
        &conn,
        &branch_a1.id,
        "Ian Manager",
        Some("ian_mgr"),
        None,
        Some(fixture_code.as_str()),
        "manager",
    )
    .expect("manager created");

    let session = create_local_session(&conn, &manager.id, &branch_a1.id, "pin", None)
        .expect("session created");

    // 1. Same tenant + same branch: Allowed
    let req_ok = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::InventoryAdjust)
        .with_organization_scope(org_a.id.as_str())
        .with_branch_scope(branch_a1.id.as_str());
    assert!(authorize(&conn, &req_ok).is_ok());

    // 2. Same tenant + wrong branch: Denied with ScopeMismatch
    let req_wrong_branch = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::InventoryAdjust)
        .with_organization_scope(org_a.id.as_str())
        .with_branch_scope(branch_a2.id.as_str());
    let err_branch = authorize(&conn, &req_wrong_branch).unwrap_err();
    assert!(matches!(
        err_branch,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));

    // 3. Cross-tenant request (Target Org B): Denied with ScopeMismatch
    let req_cross_tenant = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::InventoryAdjust)
        .with_organization_scope(org_b.id.as_str())
        .with_branch_scope(branch_b1.id.as_str());
    let err_tenant = authorize(&conn, &req_cross_tenant).unwrap_err();
    assert!(matches!(
        err_tenant,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));

    // 4. Convenience scoped helper:
    assert!(require_scoped_permission(
        &conn,
        &session.id,
        Permission::InventoryAdjust,
        Some(org_a.id.as_str()),
        Some(branch_a1.id.as_str()),
    )
    .is_ok());
}

#[test]
fn e2e_supabase_identity_boundary_and_local_user_mapping() {
    let conn = setup_test_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["8", "8", "9", "9"].join("");

    let supabase_uid = uuid::Uuid::new_v4().to_string();

    // User mapped to cloud Supabase identity
    let user = create_user(
        &conn,
        CreateUserInput {
            branch_id: branch_id.clone(),
            full_name: "Jack CloudUser".to_string(),
            username: Some("jack_cloud".to_string()),
            password: None,
            pin: Some(fixture_code),
            role: "admin".to_string(),
            supabase_user_id: Some(supabase_uid.clone()),
            auth_provider: Some("supabase".to_string()),
        },
    )
    .expect("user created");

    assert_eq!(
        user.supabase_user_id.as_deref(),
        Some(supabase_uid.as_str())
    );
    assert_eq!(user.auth_provider, "supabase");

    // Create session via supabase authentication provider
    let session = create_local_session(&conn, &user.id, &branch_id, "supabase", None)
        .expect("session created");

    // Verify session context & permissions
    let ctx = require_permission(&conn, &session.id, Permission::SettingsManage)
        .expect("admin should have settings.manage");
    assert_eq!(ctx.user_id, user.id);
    assert_eq!(ctx.organization_id.as_deref(), Some(org_id.as_str()));
    assert_eq!(ctx.role, "admin");

    // Admin possesses all permissions in catalog
    let all_perms = get_effective_user_permissions(&conn, &user.id, user.role.as_str())
        .expect("get user permissions");
    for entry in PERMISSION_CATALOG {
        assert!(
            all_perms.contains(&entry.permission),
            "Admin must possess permission {}",
            entry.code
        );
    }
}

#[test]
fn e2e_composite_and_any_permission_authorization_evaluations() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["9", "9", "0", "0"].join("");

    let manager = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Karen CompositeMgr",
        Some("karen_mgr"),
        None,
        Some(fixture_code.as_str()),
        "manager",
    )
    .expect("manager created");

    let session =
        create_local_session(&conn, &manager.id, &branch_id, "pin", None).expect("session created");

    // 1. require_all_permissions: Manager has InventoryAdjust and CustomersManage
    assert!(require_all_permissions(
        &conn,
        &session.id,
        &[Permission::InventoryAdjust, Permission::CustomersManage]
    )
    .is_ok());

    // 2. require_all_permissions: Manager lacks UsersManage -> Fails
    let err_all = require_all_permissions(
        &conn,
        &session.id,
        &[Permission::InventoryAdjust, Permission::UsersManage],
    )
    .unwrap_err();
    assert!(matches!(
        err_all,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    // 3. require_any_permission: Manager has at least one (InventoryAdjust) -> Succeeds
    assert!(require_any_permission(
        &conn,
        &session.id,
        &[Permission::UsersManage, Permission::InventoryAdjust]
    )
    .is_ok());

    // 4. require_any_permission: Manager has none (UsersManage, LicenseManage) -> Fails
    let err_any = require_any_permission(
        &conn,
        &session.id,
        &[Permission::UsersManage, Permission::LicenseManage],
    )
    .unwrap_err();
    assert!(matches!(
        err_any,
        AuthMiddlewareError::PermissionDenied { .. }
    ));
}

#[test]
fn e2e_repeated_authentication_and_authorization_is_deterministic() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let fixture_token = ["deterministic", "token", "888"].join("-");
    let fixture_code = ["1", "2", "1", "2"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Leo Deterministic",
        Some("leo_det"),
        Some(fixture_token.as_str()),
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("cashier created");

    for _ in 0..20 {
        let verified =
            verify_user_password(&conn, "leo_det", fixture_token.as_str()).expect("password check");
        assert_eq!(verified.id, user.id);

        let session = create_local_session(&conn, &user.id, &branch_id, "password", None)
            .expect("session created");
        assert!(require_permission(&conn, &session.id, Permission::SalesCreate).is_ok());

        revoke_local_session(&conn, &session.id).expect("session revoked");
        let err_revoked =
            require_permission(&conn, &session.id, Permission::SalesCreate).unwrap_err();
        assert!(matches!(
            err_revoked,
            AuthMiddlewareError::SessionRevoked(_)
        ));
    }
}

#[test]
fn e2e_diagnostic_and_error_safety_no_credential_leakage() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let confidential_val = ["confidential", "value", "777"].join("_");
    let confidential_code = ["7", "8", "9", "0"].join("");
    let mismatch_val = ["mismatch", "value", "999"].join("_");
    let mismatch_code = ["1", "1", "1", "1"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Security User",
        Some("sec_user"),
        Some(confidential_val.as_str()),
        Some(confidential_code.as_str()),
        "cashier",
    )
    .expect("user created");

    // 1. Password verification error formatting
    let err_pw = verify_user_password(&conn, "sec_user", mismatch_val.as_str()).unwrap_err();
    let err_str = err_pw.to_string();
    assert!(!err_str.contains(confidential_val.as_str()));
    assert!(!err_str.contains(mismatch_val.as_str()));

    // 2. PIN verification error formatting
    let err_pin = verify_user_pin(&conn, &user.id, mismatch_code.as_str()).unwrap_err();
    let pin_err_str = err_pin.to_string();
    assert!(!pin_err_str.contains(confidential_code.as_str()));
    assert!(!pin_err_str.contains(mismatch_code.as_str()));

    // 3. Middleware error formatting
    let mw_err = require_session(&conn, "invalid-token-xyz").unwrap_err();
    let mw_err_str = mw_err.to_string();
    assert!(!mw_err_str.contains(confidential_val.as_str()));
    assert!(!mw_err_str.contains(confidential_code.as_str()));
}
