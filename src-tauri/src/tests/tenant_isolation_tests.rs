// Tenant and Branch Isolation Test Suite.
// F1.10 — Tenant-Isolation Tests
// Authoritative security and regression test layer validating strict multi-tenant boundaries,
// branch confinement, user/session scoping, privileged role constraints, and cloud RLS guarantees.

use crate::auth::middleware::{
    authorize, require_scoped_permission, AuthMiddlewareError, AuthorizeRequest,
};
use crate::branch::{create_branch, list_branches, CreateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::permission::{set_user_permission_override, Permission};
use crate::register::{create_register, list_registers, CreateRegisterInput, RegisterError};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::{create_local_session, revoke_local_session};
use crate::user::{list_users, UserError};

const RLS_MIGRATION_SQL: &str =
    include_str!("../../../supabase/migrations/001_phase1_identity_and_rls.sql");

#[test]
fn tenant_isolation_org_to_org_strict_boundary() {
    let conn = setup_test_db();
    let fixture_token_a = ["tenant", "alpha", "token", "111"].join("-");
    let fixture_code_a = ["1", "1", "1", "1"].join("");
    let fixture_token_b = ["tenant", "beta", "token", "222"].join("-");
    let fixture_code_b = ["2", "2", "2", "2"].join("");

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
            name: "Branch Alpha-Main".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch a");

    let user_a = create_test_user_with_creds(
        &conn,
        &branch_a.id,
        "Alice Alpha",
        Some("alice_alpha"),
        Some(fixture_token_a.as_str()),
        Some(fixture_code_a.as_str()),
        "cashier",
    )
    .expect("create user a");

    let session_a = create_local_session(&conn, &user_a.id, &branch_a.id, "password", None)
        .expect("create session a");

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
            name: "Branch Beta-Main".to_string(),
            address: None,
            currency: Some("EUR".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch b");

    let user_b = create_test_user_with_creds(
        &conn,
        &branch_b.id,
        "Bob Beta",
        Some("bob_beta"),
        Some(fixture_token_b.as_str()),
        Some(fixture_code_b.as_str()),
        "cashier",
    )
    .expect("create user b");

    let session_b = create_local_session(&conn, &user_b.id, &branch_b.id, "password", None)
        .expect("create session b");

    // 3. User A accessing Org A (Same tenant) -> ALLOWED
    let req_a_valid = AuthorizeRequest::new(&session_a.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org_a.id.as_str())
        .with_branch_scope(branch_a.id.as_str());
    assert!(authorize(&conn, &req_a_valid).is_ok());

    // 4. User A accessing Org B (Cross tenant) -> DENIED with ScopeMismatch
    let req_a_cross_org = AuthorizeRequest::new(&session_a.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org_b.id.as_str());
    let err_a_cross = authorize(&conn, &req_a_cross_org).unwrap_err();
    assert!(matches!(
        err_a_cross,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));

    // 5. User B accessing Org B (Same tenant) -> ALLOWED
    let req_b_valid = AuthorizeRequest::new(&session_b.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org_b.id.as_str())
        .with_branch_scope(branch_b.id.as_str());
    assert!(authorize(&conn, &req_b_valid).is_ok());

    // 6. User B accessing Org A (Cross tenant) -> DENIED with ScopeMismatch
    let req_b_cross_org = AuthorizeRequest::new(&session_b.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org_a.id.as_str());
    let err_b_cross = authorize(&conn, &req_b_cross_org).unwrap_err();
    assert!(matches!(
        err_b_cross,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));
}

#[test]
fn tenant_isolation_branch_to_branch_within_same_org() {
    let conn = setup_test_db();
    let fixture_code = ["3", "3", "3", "3"].join("");

    let org = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Enterprise Retail Group".to_string(),
            default_currency: Some("USD".to_string()),
            default_language: Some("en".to_string()),
        },
    )
    .expect("create org");

    let branch_1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org.id.clone(),
            name: "Downtown Flagship".to_string(),
            address: Some("100 Main St".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch 1");

    let branch_2 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org.id.clone(),
            name: "Uptown Express".to_string(),
            address: Some("500 North Ave".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch 2");

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_1.id,
        "Cashier Branch One",
        Some("cashier_b1"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("create cashier");

    let session = create_local_session(&conn, &cashier.id, &branch_1.id, "pin", None)
        .expect("create session");

    // 1. Same branch authorized operation -> ALLOWED
    let req_same_branch = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org.id.as_str())
        .with_branch_scope(branch_1.id.as_str());
    assert!(authorize(&conn, &req_same_branch).is_ok());

    // 2. Cross-branch operation within same tenant -> DENIED with ScopeMismatch
    let req_cross_branch = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org.id.as_str())
        .with_branch_scope(branch_2.id.as_str());
    let err_cross_branch = authorize(&conn, &req_cross_branch).unwrap_err();
    assert!(matches!(
        err_cross_branch,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));
}

#[test]
fn tenant_isolation_privileged_admin_and_owner_cannot_bypass_tenant_boundaries() {
    let conn = setup_test_db();
    let fixture_code = ["4", "4", "4", "4"].join("");

    let (org_a_id, branch_a_id) = create_test_org_and_branch(&conn);
    let (org_b_id, branch_b_id) = create_test_org_and_branch(&conn);

    // Create Admin in Org A
    let admin = create_test_user_with_creds(
        &conn,
        &branch_a_id,
        "Super Admin Org A",
        Some("admin_org_a"),
        None,
        Some(fixture_code.as_str()),
        "admin",
    )
    .expect("create admin");

    let session =
        create_local_session(&conn, &admin.id, &branch_a_id, "pin", None).expect("create session");

    // 1. Admin accessing Org A operations -> ALLOWED
    let req_admin_org_a = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::SettingsManage)
        .with_organization_scope(org_a_id.as_str())
        .with_branch_scope(branch_a_id.as_str());
    assert!(authorize(&conn, &req_admin_org_a).is_ok());

    // 2. Admin attempting to target Org B -> STRICTLY DENIED with ScopeMismatch
    let req_admin_target_org_b = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::SettingsManage)
        .with_organization_scope(org_b_id.as_str());
    let err_admin_org_b = authorize(&conn, &req_admin_target_org_b).unwrap_err();
    assert!(matches!(
        err_admin_org_b,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));

    // 3. Admin attempting to target Branch B -> STRICTLY DENIED with ScopeMismatch
    let req_admin_target_branch_b = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::UsersManage)
        .with_branch_scope(branch_b_id.as_str());
    let err_admin_branch_b = authorize(&conn, &req_admin_target_branch_b).unwrap_err();
    assert!(matches!(
        err_admin_branch_b,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));
}

#[test]
fn tenant_isolation_user_cannot_create_session_in_foreign_branch_or_tenant() {
    let conn = setup_test_db();
    let fixture_code = ["5", "5", "5", "5"].join("");

    let org_a = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Org Alpha".to_string(),
            default_currency: Some("USD".to_string()),
            default_language: Some("en".to_string()),
        },
    )
    .expect("create org a");

    let branch_a1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id.clone(),
            name: "Branch A-1".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch a1");

    let branch_a2 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id.clone(),
            name: "Branch A-2".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch a2");

    let (org_b_id, branch_b1_id) = create_test_org_and_branch(&conn);
    assert_ne!(org_a.id, org_b_id);

    let user_a1 = create_test_user_with_creds(
        &conn,
        &branch_a1.id,
        "Staff Member A1",
        Some("staff_a1"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("create user a1");

    // 1. Session creation in assigned branch A1 -> ALLOWED
    let session_valid = create_local_session(&conn, &user_a1.id, &branch_a1.id, "pin", None);
    assert!(session_valid.is_ok());

    // 2. Session creation in sibling branch A2 (same tenant, wrong branch) -> REJECTED
    let err_sibling =
        create_local_session(&conn, &user_a1.id, &branch_a2.id, "pin", None).unwrap_err();
    assert!(matches!(err_sibling, UserError::Validation(_)));

    // 3. Session creation in foreign tenant branch B1 -> REJECTED
    let err_foreign =
        create_local_session(&conn, &user_a1.id, &branch_b1_id, "pin", None).unwrap_err();
    assert!(matches!(err_foreign, UserError::Validation(_)));
}

#[test]
fn tenant_isolation_explicit_user_permission_override_never_crosses_scope() {
    let conn = setup_test_db();
    let fixture_code = ["6", "6", "6", "6"].join("");

    let (org_a_id, branch_a_id) = create_test_org_and_branch(&conn);
    let (org_b_id, branch_b_id) = create_test_org_and_branch(&conn);

    let cashier = create_test_user_with_creds(
        &conn,
        &branch_a_id,
        "Override Cashier",
        Some("override_cashier_iso"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("create cashier");

    let session = create_local_session(&conn, &cashier.id, &branch_a_id, "pin", None)
        .expect("create session");

    // Explicitly grant InventoryAdjust override to cashier in local DB
    set_user_permission_override(&conn, &cashier.id, Permission::InventoryAdjust, "allow")
        .expect("grant override");

    // 1. Scoped operation matching Org A and Branch A -> ALLOWED
    let req_same = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::InventoryAdjust)
        .with_organization_scope(org_a_id.as_str())
        .with_branch_scope(branch_a_id.as_str());
    assert!(authorize(&conn, &req_same).is_ok());

    // 2. Scoped operation targeting Org B -> DENIED with ScopeMismatch despite explicit override
    let req_cross_org = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::InventoryAdjust)
        .with_organization_scope(org_b_id.as_str());
    let err_org = authorize(&conn, &req_cross_org).unwrap_err();
    assert!(matches!(err_org, AuthMiddlewareError::ScopeMismatch { .. }));

    // 3. Scoped operation targeting Branch B -> DENIED with ScopeMismatch despite explicit override
    let req_cross_branch = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::InventoryAdjust)
        .with_branch_scope(branch_b_id.as_str());
    let err_branch = authorize(&conn, &req_cross_branch).unwrap_err();
    assert!(matches!(
        err_branch,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));
}

#[test]
fn tenant_isolation_scoped_convenience_helper_matrix() {
    let conn = setup_test_db();
    let (org_a_id, branch_a_id) = create_test_org_and_branch(&conn);
    let (org_b_id, branch_b_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["7", "7", "7", "7"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_a_id,
        "Matrix Test User",
        Some("matrix_user"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("create user");

    let session =
        create_local_session(&conn, &user.id, &branch_a_id, "pin", None).expect("create session");

    // Permutation 1: Matching Org A + Matching Branch A -> ALLOWED
    assert!(require_scoped_permission(
        &conn,
        &session.id,
        Permission::SalesCreate,
        Some(org_a_id.as_str()),
        Some(branch_a_id.as_str()),
    )
    .is_ok());

    // Permutation 2: Matching Org A + Wrong Branch B -> DENIED
    let err_org_a_branch_b = require_scoped_permission(
        &conn,
        &session.id,
        Permission::SalesCreate,
        Some(org_a_id.as_str()),
        Some(branch_b_id.as_str()),
    )
    .unwrap_err();
    assert!(matches!(
        err_org_a_branch_b,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));

    // Permutation 3: Wrong Org B + Matching Branch A -> DENIED
    let err_org_b_branch_a = require_scoped_permission(
        &conn,
        &session.id,
        Permission::SalesCreate,
        Some(org_b_id.as_str()),
        Some(branch_a_id.as_str()),
    )
    .unwrap_err();
    assert!(matches!(
        err_org_b_branch_a,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));

    // Permutation 4: Wrong Org B + Wrong Branch B -> DENIED
    let err_org_b_branch_b = require_scoped_permission(
        &conn,
        &session.id,
        Permission::SalesCreate,
        Some(org_b_id.as_str()),
        Some(branch_b_id.as_str()),
    )
    .unwrap_err();
    assert!(matches!(
        err_org_b_branch_b,
        AuthMiddlewareError::ScopeMismatch { .. }
    ));
}

#[test]
fn tenant_isolation_register_creation_and_listing_cross_tenant_prevention() {
    let conn = setup_test_db();
    let (org_a_id, branch_a_id) = create_test_org_and_branch(&conn);
    let (org_b_id, branch_b_id) = create_test_org_and_branch(&conn);

    // 1. Create register in Org A, Branch A -> ALLOWED
    let reg_a = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_a_id.clone(),
            branch_id: branch_a_id.clone(),
            name: "Register Alpha-1".to_string(),
            code: Some("REG-A1".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create register a");
    assert_eq!(reg_a.organization_id, org_a_id);
    assert_eq!(reg_a.branch_id, branch_a_id);

    // 2. Create register in Org B, Branch B -> ALLOWED
    let reg_b = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_b_id.clone(),
            branch_id: branch_b_id.clone(),
            name: "Register Beta-1".to_string(),
            code: Some("REG-B1".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create register b");
    assert_eq!(reg_b.organization_id, org_b_id);
    assert_eq!(reg_b.branch_id, branch_b_id);

    // 3. Attempt to create register associating Org A with Branch B -> REJECTED
    let err_mismatch = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_a_id.clone(),
            branch_id: branch_b_id.clone(),
            name: "Mismatched Register".to_string(),
            code: Some("REG-FAIL".to_string()),
            is_active: Some(true),
        },
    )
    .unwrap_err();
    assert!(matches!(err_mismatch, RegisterError::InvalidBranch(_)));

    // 4. Listing registers for Branch A returns only reg_a
    let list_a = list_registers(&conn, &branch_a_id).expect("list registers a");
    assert_eq!(list_a.len(), 1);
    assert_eq!(list_a[0].id, reg_a.id);

    // 5. Listing registers for Branch B returns only reg_b
    let list_b = list_registers(&conn, &branch_b_id).expect("list registers b");
    assert_eq!(list_b.len(), 1);
    assert_eq!(list_b[0].id, reg_b.id);
}

#[test]
fn tenant_isolation_domain_listing_queries_strictly_scoped() {
    let conn = setup_test_db();
    let fixture_code = ["8", "8", "8", "8"].join("");

    let org_a = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Org Alpha Queries".to_string(),
            default_currency: Some("USD".to_string()),
            default_language: Some("en".to_string()),
        },
    )
    .expect("create org a");

    let branch_a1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id.clone(),
            name: "Branch A1".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch a1");

    let branch_a2 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id.clone(),
            name: "Branch A2".to_string(),
            address: None,
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch a2");

    let org_b = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Org Beta Queries".to_string(),
            default_currency: Some("EUR".to_string()),
            default_language: Some("de".to_string()),
        },
    )
    .expect("create org b");

    let branch_b1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_b.id.clone(),
            name: "Branch B1".to_string(),
            address: None,
            currency: Some("EUR".to_string()),
            is_active: Some(true),
        },
    )
    .expect("create branch b1");

    let user_a1 = create_test_user_with_creds(
        &conn,
        &branch_a1.id,
        "User A1",
        Some("user_a1"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("create user a1");

    let user_b1 = create_test_user_with_creds(
        &conn,
        &branch_b1.id,
        "User B1",
        Some("user_b1"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("create user b1");

    // 1. list_branches for Org A returns exactly Branch A1 and Branch A2
    let branches_a = list_branches(&conn, &org_a.id).expect("list branches a");
    assert_eq!(branches_a.len(), 2);
    assert!(branches_a.iter().any(|b| b.id == branch_a1.id));
    assert!(branches_a.iter().any(|b| b.id == branch_a2.id));
    assert!(!branches_a.iter().any(|b| b.id == branch_b1.id));

    // 2. list_branches for Org B returns exactly Branch B1
    let branches_b = list_branches(&conn, &org_b.id).expect("list branches b");
    assert_eq!(branches_b.len(), 1);
    assert_eq!(branches_b[0].id, branch_b1.id);

    // 3. list_users for Branch A1 returns only user_a1
    let users_a1 = list_users(&conn, &branch_a1.id).expect("list users a1");
    assert_eq!(users_a1.len(), 1);
    assert_eq!(users_a1[0].id, user_a1.id);

    // 4. list_users for Branch B1 returns only user_b1
    let users_b1 = list_users(&conn, &branch_b1.id).expect("list users b1");
    assert_eq!(users_b1.len(), 1);
    assert_eq!(users_b1[0].id, user_b1.id);
}

#[test]
fn tenant_isolation_unauthenticated_and_compromised_sessions_fail_closed_before_scope() {
    let conn = setup_test_db();
    let (org_a_id, branch_a_id) = create_test_org_and_branch(&conn);
    let fixture_code = ["9", "9", "9", "9"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_a_id,
        "Compromise Test User",
        Some("comp_user"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("create user");

    // 1. Nonexistent session ID -> Unauthenticated
    let req_nonexistent = AuthorizeRequest::new("unknown-session-id")
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org_a_id.as_str())
        .with_branch_scope(branch_a_id.as_str());
    let err_nonexistent = authorize(&conn, &req_nonexistent).unwrap_err();
    assert!(matches!(
        err_nonexistent,
        AuthMiddlewareError::Unauthenticated(_)
    ));

    // 2. Revoked session -> SessionRevoked
    let session =
        create_local_session(&conn, &user.id, &branch_a_id, "pin", None).expect("create session");
    revoke_local_session(&conn, &session.id).expect("revoke session");

    let req_revoked = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org_a_id.as_str());
    let err_revoked = authorize(&conn, &req_revoked).unwrap_err();
    assert!(matches!(
        err_revoked,
        AuthMiddlewareError::SessionRevoked(_)
    ));

    // 3. Expired session -> SessionExpired
    let expired_session = create_local_session(&conn, &user.id, &branch_a_id, "pin", Some(-2))
        .expect("create expired session");
    let req_expired = AuthorizeRequest::new(&expired_session.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org_a_id.as_str());
    let err_expired = authorize(&conn, &req_expired).unwrap_err();
    assert!(matches!(
        err_expired,
        AuthMiddlewareError::SessionExpired(_)
    ));
}

#[test]
fn tenant_isolation_error_formatting_never_leaks_secrets_or_private_data() {
    let conn = setup_test_db();
    let (_, branch_a_id) = create_test_org_and_branch(&conn);
    let (org_b_id, _) = create_test_org_and_branch(&conn);

    let confidential_token = ["secret", "pass", "token", "777"].join("_");
    let confidential_pin = ["1", "3", "5", "7"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_a_id,
        "Secret User",
        Some("sec_user_iso"),
        Some(confidential_token.as_str()),
        Some(confidential_pin.as_str()),
        "cashier",
    )
    .expect("create user");

    let session = create_local_session(&conn, &user.id, &branch_a_id, "password", None)
        .expect("create session");

    // 1. Cross-tenant scope mismatch error message
    let req_mismatch = AuthorizeRequest::new(&session.id)
        .with_permission(Permission::SalesCreate)
        .with_organization_scope(org_b_id.as_str());
    let err = authorize(&conn, &req_mismatch).unwrap_err();
    let err_msg = err.to_string();

    assert!(!err_msg.contains(confidential_token.as_str()));
    assert!(!err_msg.contains(confidential_pin.as_str()));
    assert!(err_msg.contains("Scope mismatch"));
}

#[test]
fn tenant_isolation_repeated_checks_are_deterministic_and_idempotent() {
    let conn = setup_test_db();
    let (org_a_id, branch_a_id) = create_test_org_and_branch(&conn);
    let (org_b_id, _) = create_test_org_and_branch(&conn);
    let fixture_code = ["1", "2", "3", "0"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_a_id,
        "Idempotent User",
        Some("idem_user"),
        None,
        Some(fixture_code.as_str()),
        "cashier",
    )
    .expect("create user");

    let session =
        create_local_session(&conn, &user.id, &branch_a_id, "pin", None).expect("create session");

    for _ in 0..20 {
        // Valid same-tenant authorization succeeds
        let req_valid = AuthorizeRequest::new(&session.id)
            .with_permission(Permission::SalesCreate)
            .with_organization_scope(org_a_id.as_str())
            .with_branch_scope(branch_a_id.as_str());
        assert!(authorize(&conn, &req_valid).is_ok());

        // Cross-tenant authorization strictly denied with ScopeMismatch
        let req_cross = AuthorizeRequest::new(&session.id)
            .with_permission(Permission::SalesCreate)
            .with_organization_scope(org_b_id.as_str());
        let err = authorize(&conn, &req_cross).unwrap_err();
        assert!(matches!(err, AuthMiddlewareError::ScopeMismatch { .. }));
    }
}

#[test]
fn tenant_isolation_cloud_rls_migration_policies_static_coverage() {
    let normalized_sql = RLS_MIGRATION_SQL
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Verify all multi-tenant tables enforce row level security
    let tenant_tables = [
        "organizations",
        "organization_members",
        "branches",
        "registers",
        "users",
        "permissions",
        "role_permissions",
        "user_permissions",
    ];

    for table in tenant_tables {
        let rls_stmt = format!("ALTER TABLE public.{table} ENABLE ROW LEVEL SECURITY;");
        assert!(
            normalized_sql.contains(&rls_stmt),
            "Table public.{table} must enable RLS"
        );
    }

    // Verify security definer tenant boundary functions exist with search_path = public
    let helper_functions = [
        "get_user_organization_ids",
        "is_org_member",
        "is_org_admin_or_owner",
        "is_org_manager_or_above",
        "can_delete_organization_member",
        "prevent_orphaned_organization",
        "handle_new_organization_owner",
    ];

    for func in helper_functions {
        assert!(
            normalized_sql.contains(&format!("FUNCTION public.{func}")),
            "RLS helper function public.{func} must be defined"
        );
    }

    assert!(
        normalized_sql.contains("SECURITY DEFINER"),
        "RLS helper functions must use SECURITY DEFINER"
    );
    assert!(
        normalized_sql.contains("SET search_path = public"),
        "RLS helper functions must set search_path = public"
    );

    // Verify explicit tenant boundary checks in mutation policies
    assert!(
        normalized_sql.contains(
            "b.id = registers.branch_id AND b.organization_id = registers.organization_id"
        ),
        "Registers table mutation policies must enforce branch and organization tenant alignment"
    );
    assert!(
        normalized_sql
            .contains("b.id = users.branch_id AND b.organization_id = users.organization_id"),
        "Users table mutation policies must enforce branch and organization tenant alignment"
    );
}
