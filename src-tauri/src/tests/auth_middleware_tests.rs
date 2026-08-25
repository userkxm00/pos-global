use crate::auth::middleware::{
    authorize, require_all_permissions, require_any_permission, require_permission,
    require_scoped_permission, require_session, AuthMiddlewareError, AuthorizeRequest,
};
use crate::branch::{create_branch, CreateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::permission::{set_user_permission_override, Permission};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::{create_local_session, revoke_local_session};
use rusqlite::params;

#[test]
fn valid_authenticated_session_and_authorized_permission_allows_access() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier One",
        Some("cashier1"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    let session =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("create session");

    // Cashier possesses SalesCreate
    let ctx = require_permission(&conn, &session.id, Permission::SalesCreate)
        .expect("authorization should succeed");

    assert_eq!(ctx.user_id, cashier.id);
    assert_eq!(ctx.role, "cashier");
    assert_eq!(ctx.branch_id, branch_id);
}

#[test]
fn missing_or_nonexistent_session_fails_closed() {
    let conn = setup_test_db();

    // 1. Empty session ID
    let err = require_permission(&conn, "", Permission::SalesCreate).unwrap_err();
    assert!(matches!(err, AuthMiddlewareError::Unauthenticated(_)));

    // 2. Whitespace session ID
    let err_ws = require_permission(&conn, "   ", Permission::SalesCreate).unwrap_err();
    assert!(matches!(err_ws, AuthMiddlewareError::Unauthenticated(_)));

    // 3. Nonexistent session ID
    let err_nonexistent =
        require_permission(&conn, "nonexistent-session-id", Permission::SalesCreate).unwrap_err();
    assert!(matches!(
        err_nonexistent,
        AuthMiddlewareError::Unauthenticated(_)
    ));
}

#[test]
fn revoked_expired_and_inactive_sessions_are_denied() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Test User",
        Some("test_user_lifecycle"),
        None,
        None,
        "admin",
    )
    .expect("create user");

    let session =
        create_local_session(&conn, &user.id, &branch_id, "pin", None).expect("create session");

    // 1. Revoked session is denied
    revoke_local_session(&conn, &session.id).expect("revoke session");
    let err_revoked = require_session(&conn, &session.id).unwrap_err();
    assert!(matches!(
        err_revoked,
        AuthMiddlewareError::SessionRevoked(_)
    ));

    // 2. Expired session is denied
    let expired_session =
        create_local_session(&conn, &user.id, &branch_id, "pin", Some(-1)).expect("create expired");
    let err_expired = require_session(&conn, &expired_session.id).unwrap_err();
    assert!(matches!(
        err_expired,
        AuthMiddlewareError::SessionExpired(_)
    ));

    // 3. Inactive user session is denied
    let active_session =
        create_local_session(&conn, &user.id, &branch_id, "pin", None).expect("active session");
    conn.execute(
        "UPDATE users SET is_active = 0 WHERE id = ?1",
        params![user.id],
    )
    .expect("deactivate user");

    let err_inactive_user = require_session(&conn, &active_session.id).unwrap_err();
    assert!(matches!(
        err_inactive_user,
        AuthMiddlewareError::Unauthenticated(_)
    ));

    // 4. Inactive branch session is denied
    conn.execute(
        "UPDATE users SET is_active = 1 WHERE id = ?1",
        params![user.id],
    )
    .expect("reactivate user");
    conn.execute(
        "UPDATE branches SET is_active = 0 WHERE id = ?1",
        params![branch_id],
    )
    .expect("deactivate branch");

    let err_inactive_branch = require_session(&conn, &active_session.id).unwrap_err();
    assert!(matches!(
        err_inactive_branch,
        AuthMiddlewareError::Unauthenticated(_)
    ));
}

#[test]
fn unauthorized_role_and_unknown_role_are_denied() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier Role",
        Some("cashier_role_test"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    let session =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("create session");

    // 1. Cashier attempting administrative action
    let err_unauthorized =
        require_permission(&conn, &session.id, Permission::UsersManage).unwrap_err();
    assert!(matches!(
        err_unauthorized,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    // 2. Cashier attempting void
    let err_void = require_permission(&conn, &session.id, Permission::SalesVoid).unwrap_err();
    assert!(matches!(
        err_void,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    // 3. User with unknown role fails closed
    conn.execute(
        "UPDATE users SET role = 'intruder' WHERE id = ?1",
        params![cashier.id],
    )
    .expect("tamper role");

    let err_unknown_role =
        require_permission(&conn, &session.id, Permission::SalesCreate).unwrap_err();
    assert!(matches!(
        err_unknown_role,
        AuthMiddlewareError::PermissionDenied { .. }
    ));
}

#[test]
fn explicit_user_overrides_obey_precedence_in_middleware() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    // 1. Cashier with explicit allow override for InventoryAdjust
    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Overridden Cashier",
        Some("overridden_cashier"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    let session_cashier =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("create session");

    // Baseline: Denied
    assert!(require_permission(&conn, &session_cashier.id, Permission::InventoryAdjust).is_err());

    // Grant user override
    set_user_permission_override(&conn, &cashier.id, Permission::InventoryAdjust, "allow")
        .expect("set allow");

    // Now allowed through middleware
    assert!(require_permission(&conn, &session_cashier.id, Permission::InventoryAdjust).is_ok());

    // 2. Admin with explicit deny override for SalesVoid (Deny precedence)
    let admin = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Restricted Admin",
        Some("restricted_admin"),
        None,
        None,
        "admin",
    )
    .expect("create admin");

    let session_admin =
        create_local_session(&conn, &admin.id, &branch_id, "pin", None).expect("create session");

    // Baseline: Allowed
    assert!(require_permission(&conn, &session_admin.id, Permission::SalesVoid).is_ok());

    // Set user deny override
    set_user_permission_override(&conn, &admin.id, Permission::SalesVoid, "deny")
        .expect("set deny");

    // Now denied through middleware even though role is admin
    let err_admin_deny =
        require_permission(&conn, &session_admin.id, Permission::SalesVoid).unwrap_err();
    assert!(matches!(
        err_admin_deny,
        AuthMiddlewareError::PermissionDenied { .. }
    ));
}

#[test]
fn scope_isolation_enforces_organization_and_branch_boundaries() {
    let conn = setup_test_db();
    let (org_a, branch_a) = create_test_org_and_branch(&conn);

    let org_b = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Org B".to_string(),
            default_currency: Some("USD".to_string()),
            default_language: Some("en".to_string()),
        },
    )
    .expect("create org b");

    let branch_b = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_b.id.clone(),
            name: "Branch B".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch b");

    let admin = create_test_user_with_creds(
        &conn,
        &branch_a,
        "Admin User",
        Some("scoped_admin"),
        None,
        None,
        "admin",
    )
    .expect("create admin");

    let session =
        create_local_session(&conn, &admin.id, &branch_a, "pin", None).expect("create session");

    // 1. Matching org and branch succeeds
    assert!(require_scoped_permission(
        &conn,
        &session.id,
        Permission::SalesCreate,
        Some(&org_a),
        Some(&branch_a)
    )
    .is_ok());

    // 2. Mismatched organization fails closed (even for Admin)
    let err_org = require_scoped_permission(
        &conn,
        &session.id,
        Permission::SalesCreate,
        Some(&org_b.id),
        Some(&branch_a),
    )
    .unwrap_err();
    assert!(matches!(err_org, AuthMiddlewareError::ScopeMismatch { .. }));

    // 3. Mismatched branch fails closed (even for Admin)
    let err_branch = require_scoped_permission(
        &conn,
        &session.id,
        Permission::SalesCreate,
        Some(&org_a),
        Some(&branch_b.id),
    )
    .unwrap_err();
    assert!(matches!(
        err_branch,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));
}

#[test]
fn multi_permission_all_and_any_semantics() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier Multi",
        Some("cashier_multi"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    let session_cashier =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("create session");

    // 1. ALL requirements: Cashier has SalesCreate but lacks InventoryAdjust -> DENY
    let req_all_mixed = [Permission::SalesCreate, Permission::InventoryAdjust];
    let err_all = require_all_permissions(&conn, &session_cashier.id, &req_all_mixed).unwrap_err();
    assert!(matches!(
        err_all,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    // Cashier has both SalesCreate and ReportsView -> ALLOW
    let req_all_allowed = [Permission::SalesCreate, Permission::ReportsView];
    assert!(require_all_permissions(&conn, &session_cashier.id, &req_all_allowed).is_ok());

    // Empty list fails closed
    assert!(require_all_permissions(&conn, &session_cashier.id, &[]).is_err());

    // 2. ANY requirements: Cashier has at least one (SalesCreate) -> ALLOW
    assert!(require_any_permission(&conn, &session_cashier.id, &req_all_mixed).is_ok());

    // Cashier has neither SalesVoid nor InventoryAdjust -> DENY
    let req_any_denied = [Permission::SalesVoid, Permission::InventoryAdjust];
    let err_any = require_any_permission(&conn, &session_cashier.id, &req_any_denied).unwrap_err();
    assert!(matches!(
        err_any,
        AuthMiddlewareError::PermissionDenied { .. }
    ));

    // Empty list fails closed
    assert!(require_any_permission(&conn, &session_cashier.id, &[]).is_err());
}

#[test]
fn authorization_errors_are_deterministic_and_do_not_leak_secrets() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Secret Test Cashier",
        Some("secret_cashier"),
        Some("SensitivePassword123!"),
        Some("9876"),
        "cashier",
    )
    .expect("create cashier");

    let session =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("create session");

    let err = require_permission(&conn, &session.id, Permission::UsersManage).unwrap_err();
    let err_msg = err.to_string();

    // Verify error contains domain role/permission info but NO secrets
    assert!(err_msg.contains("cashier"));
    assert!(err_msg.contains("users.manage"));
    assert!(!err_msg.contains("SensitivePassword"));
    assert!(!err_msg.contains("9876"));
}

#[test]
fn repeated_authorization_checks_behave_consistently() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Idempotent Cashier",
        Some("idempotent_cashier"),
        None,
        None,
        "cashier",
    )
    .expect("create cashier");

    let session =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("create session");

    for _ in 0..5 {
        assert!(require_permission(&conn, &session.id, Permission::SalesCreate).is_ok());
        assert!(require_permission(&conn, &session.id, Permission::UsersManage).is_err());
    }
}

#[test]
fn full_end_to_end_authorization_pipeline_integration() {
    let conn = setup_test_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn);

    let manager = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Store Manager",
        Some("store_manager"),
        None,
        None,
        "manager",
    )
    .expect("create manager");

    let session = create_local_session(&conn, &manager.id, &branch_id, "password", Some(4))
        .expect("create session");

    // Declarative builder authorization request
    let auth_result = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::ProductsManage)
        .with_all_permissions(&[Permission::InventoryAdjust, Permission::ReportsView])
        .with_organization_scope(&org_id)
        .with_branch_scope(&branch_id)
        .execute(&conn);

    assert!(auth_result.is_ok());
    let ctx = auth_result.unwrap();
    assert_eq!(ctx.user_id, manager.id);
    assert_eq!(ctx.role, "manager");
    assert_eq!(ctx.organization_id.as_deref(), Some(org_id.as_str()));
    assert_eq!(ctx.branch_id, branch_id);
}
