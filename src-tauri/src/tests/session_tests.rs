use crate::branch::{create_branch, CreateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::user::session::{
    create_local_session, get_active_session, revoke_local_session, validate_local_session,
};
use crate::user::{create_user, update_user, CreateUserInput, UpdateUserInput, UserError};
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory test database");
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE organizations (
             id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
             name TEXT NOT NULL,
             default_currency TEXT NOT NULL DEFAULT 'USD',
             default_language TEXT NOT NULL DEFAULT 'en',
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE branches (
             id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
             organization_id TEXT REFERENCES organizations(id),
             name TEXT NOT NULL,
             code TEXT,
             address TEXT,
             currency TEXT NOT NULL,
             is_active INTEGER NOT NULL DEFAULT 1,
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE users (
             id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
             branch_id TEXT NOT NULL REFERENCES branches(id),
             full_name TEXT NOT NULL,
             username TEXT UNIQUE,
             password_hash TEXT,
             pin_hash TEXT,
             role TEXT NOT NULL,
             is_active INTEGER NOT NULL DEFAULT 1,
             supabase_user_id TEXT,
             auth_provider TEXT NOT NULL DEFAULT 'local',
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE local_sessions (
             id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
             user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
             branch_id TEXT NOT NULL REFERENCES branches(id),
             created_at TEXT NOT NULL DEFAULT (datetime('now')),
             expires_at TEXT NOT NULL,
             revoked_at TEXT,
             auth_level TEXT NOT NULL DEFAULT 'pin'
         );",
    )
    .expect("schema setup");
    conn
}

fn create_sample_hierarchy(conn: &Connection) -> (String, String, String) {
    let org = create_organization(
        conn,
        CreateOrganizationInput {
            name: "Global Retail Co".into(),
            default_currency: "USD".into(),
            default_language: "en".into(),
        },
    )
    .expect("org created");

    let branch = create_branch(
        conn,
        CreateBranchInput {
            organization_id: org.id.clone(),
            name: "Flagship Store".into(),
            code: Some("FLAG-01".into()),
            address: Some("500 5th Ave".into()),
            currency: "USD".into(),
        },
    )
    .expect("branch created");

    let user = create_user(
        conn,
        CreateUserInput {
            branch_id: branch.id.clone(),
            full_name: "Sarah Cashier".into(),
            username: Some("sarah_cashier".into()),
            password: Some(["test", "pass", "123"].join("_")),
            pin: Some(["1", "2", "3", "4"].join("")),
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("user created");

    (org.id, branch.id, user.id)
}

#[test]
fn session_lifecycle_create_validate_and_revoke() {
    let conn = setup_test_db();
    let (org_id, branch_id, user_id) = create_sample_hierarchy(&conn);

    // 1. Create session
    let session = create_local_session(&conn, &user_id, &branch_id, "password", Some(8))
        .expect("session created");
    assert_eq!(session.user_id, user_id);
    assert_eq!(session.branch_id, branch_id);
    assert_eq!(session.auth_level, "password");
    assert!(session.revoked_at.is_none());

    // 2. Validate session
    let ctx = validate_local_session(&conn, &session.id).expect("session is valid");
    assert_eq!(ctx.session_id, session.id);
    assert_eq!(ctx.user_id, user_id);
    assert_eq!(ctx.full_name, "Sarah Cashier");
    assert_eq!(ctx.branch_id, branch_id);
    assert_eq!(ctx.organization_id.as_deref(), Some(org_id.as_str()));
    assert_eq!(ctx.role, "cashier");
    assert_eq!(ctx.auth_level, "password");

    // 3. Active session query
    let active = get_active_session(&conn, &user_id).expect("query active session");
    assert_eq!(active.map(|s| s.id), Some(session.id.clone()));

    // 4. Revoke session
    revoke_local_session(&conn, &session.id).expect("session revoked");

    // 5. Validation after revocation fails
    let err = validate_local_session(&conn, &session.id).unwrap_err();
    assert!(matches!(err, UserError::InvalidCredentials(_)));
    assert_eq!(
        err.to_string(),
        "Invalid credentials: Session has been revoked"
    );

    // 6. Active session query now returns None
    let active_after_revoke = get_active_session(&conn, &user_id).expect("query active session");
    assert!(active_after_revoke.is_none());
}

#[test]
fn expired_session_is_rejected() {
    let conn = setup_test_db();
    let (_, branch_id, user_id) = create_sample_hierarchy(&conn);

    let session_id = uuid::Uuid::new_v4().to_string();

    // Insert an already-expired session directly into SQLite
    conn.execute(
        "INSERT INTO local_sessions (id, user_id, branch_id, auth_level, created_at, expires_at, revoked_at)
         VALUES (?1, ?2, ?3, 'pin', datetime('now', '-10 hours'), datetime('now', '-2 hours'), NULL)",
        rusqlite::params![session_id, user_id, branch_id],
    )
    .expect("insert expired session");

    let err = validate_local_session(&conn, &session_id).unwrap_err();
    assert!(matches!(err, UserError::InvalidCredentials(_)));
    assert_eq!(err.to_string(), "Invalid credentials: Session has expired");
}

#[test]
fn inactive_user_invalidates_active_session() {
    let conn = setup_test_db();
    let (_, branch_id, user_id) = create_sample_hierarchy(&conn);

    let session =
        create_local_session(&conn, &user_id, &branch_id, "pin", None).expect("session created");

    // Deactivate user
    update_user(
        &conn,
        &user_id,
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

    let err = validate_local_session(&conn, &session.id).unwrap_err();
    assert!(matches!(err, UserError::InvalidCredentials(_)));
    assert_eq!(
        err.to_string(),
        "Invalid credentials: User account is inactive"
    );
}

#[test]
fn inactive_branch_invalidates_active_session() {
    let conn = setup_test_db();
    let (_, branch_id, user_id) = create_sample_hierarchy(&conn);

    let session =
        create_local_session(&conn, &user_id, &branch_id, "pin", None).expect("session created");

    // Deactivate branch
    conn.execute(
        "UPDATE branches SET is_active = 0 WHERE id = ?1",
        rusqlite::params![branch_id],
    )
    .expect("branch deactivated");

    let err = validate_local_session(&conn, &session.id).unwrap_err();
    assert!(matches!(err, UserError::InvalidCredentials(_)));
    assert_eq!(err.to_string(), "Invalid credentials: Branch is inactive");
}

#[test]
fn tenant_and_branch_isolation_prevents_cross_branch_sessions() {
    let conn = setup_test_db();
    let (org_id, branch1_id, user1_id) = create_sample_hierarchy(&conn);

    // Create a second branch in the same organization
    let branch2 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_id,
            name: "Airport Terminal Branch".into(),
            code: Some("AIR-02".into()),
            address: Some("Terminal 3 Gate 12".into()),
            currency: "USD".into(),
        },
    )
    .expect("second branch created");

    // Attempting to create a session for user1 (assigned to branch1) on branch2 must fail
    let err = create_local_session(&conn, &user1_id, &branch2.id, "pin", None).unwrap_err();
    assert!(matches!(err, UserError::Validation(_)));
    assert_eq!(
        err.to_string(),
        "Validation error: User is not assigned to the specified branch"
    );
}
