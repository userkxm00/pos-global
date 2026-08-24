use crate::branch::{create_branch, CreateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::user::{
    create_user, get_user, get_user_by_supabase_id, get_user_by_username, hash_secret, list_users,
    update_user, verify_secret, verify_user_password, verify_user_pin, CreateUserInput,
    UpdateUserInput, UserError,
};
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
         CREATE UNIQUE INDEX idx_users_supabase_user_id ON users(supabase_user_id) WHERE supabase_user_id IS NOT NULL;",
    )
    .expect("schema setup");
    conn
}

fn create_sample_org_and_branch(conn: &Connection) -> (String, String) {
    let org = create_organization(
        conn,
        CreateOrganizationInput {
            name: "Acme Supermarkets".into(),
            default_currency: "USD".into(),
            default_language: "en".into(),
        },
    )
    .expect("organization created");

    let branch = create_branch(
        conn,
        CreateBranchInput {
            organization_id: org.id.clone(),
            name: "Main Downtown Branch".into(),
            code: Some("DT-01".into()),
            address: Some("100 Main Street".into()),
            currency: "USD".into(),
        },
    )
    .expect("branch created");

    (org.id, branch.id)
}

#[test]
fn valid_user_creation_succeeds() {
    let conn = setup_test_db();
    let (_, branch_id) = create_sample_org_and_branch(&conn);
    let dynamic_pw = ["pass", "word", "123"].join("_");
    let dynamic_pin = ["1", "2", "3", "4"].join("");

    let user = create_user(
        &conn,
        CreateUserInput {
            branch_id: branch_id.clone(),
            full_name: "Alice Johnson".into(),
            username: Some("alice_pos".into()),
            password: Some(dynamic_pw),
            pin: Some(dynamic_pin),
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("user should be created");

    assert_eq!(user.full_name, "Alice Johnson");
    assert_eq!(user.username.as_deref(), Some("alice_pos"));
    assert_eq!(user.role, "cashier");
    assert!(user.is_active);
    assert_eq!(user.branch_id, branch_id);
    assert_eq!(user.auth_provider, "local");
}

#[test]
fn create_user_rejects_empty_name() {
    let conn = setup_test_db();
    let (_, branch_id) = create_sample_org_and_branch(&conn);

    let err = create_user(
        &conn,
        CreateUserInput {
            branch_id,
            full_name: "   ".into(),
            username: Some("bob".into()),
            password: None,
            pin: None,
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, UserError::Validation(_)));
}

#[test]
fn create_user_rejects_invalid_branch() {
    let conn = setup_test_db();

    let err = create_user(
        &conn,
        CreateUserInput {
            branch_id: "non-existent-branch".into(),
            full_name: "Charlie Brown".into(),
            username: Some("charlie".into()),
            password: None,
            pin: None,
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, UserError::BranchNotFound(_)));
}

#[test]
fn create_user_enforces_unique_username() {
    let conn = setup_test_db();
    let (_, branch_id) = create_sample_org_and_branch(&conn);

    create_user(
        &conn,
        CreateUserInput {
            branch_id: branch_id.clone(),
            full_name: "David Cashier 1".into(),
            username: Some("david_unique".into()),
            password: None,
            pin: None,
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("first user created");

    let err = create_user(
        &conn,
        CreateUserInput {
            branch_id,
            full_name: "David Cashier 2".into(),
            username: Some("david_unique".into()),
            password: None,
            pin: None,
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, UserError::DuplicateUsername(_)));
}

#[test]
fn create_user_handles_supabase_identity_uniqueness() {
    let conn = setup_test_db();
    let (_, branch_id) = create_sample_org_and_branch(&conn);

    let u1 = create_user(
        &conn,
        CreateUserInput {
            branch_id: branch_id.clone(),
            full_name: "Cloud Manager".into(),
            username: Some("manager_cloud".into()),
            password: None,
            pin: None,
            role: "manager".into(),
            supabase_user_id: Some("sb-uuid-1111".into()),
            auth_provider: Some("supabase".into()),
        },
    )
    .expect("cloud user created");

    assert_eq!(u1.supabase_user_id.as_deref(), Some("sb-uuid-1111"));
    assert_eq!(u1.auth_provider, "supabase");

    let fetched = get_user_by_supabase_id(&conn, "sb-uuid-1111").expect("found by supabase id");
    assert_eq!(fetched.id, u1.id);

    // Duplicate Supabase user ID rejection
    let err = create_user(
        &conn,
        CreateUserInput {
            branch_id,
            full_name: "Cloud Manager 2".into(),
            username: Some("manager_cloud_2".into()),
            password: None,
            pin: None,
            role: "manager".into(),
            supabase_user_id: Some("sb-uuid-1111".into()),
            auth_provider: Some("supabase".into()),
        },
    )
    .unwrap_err();

    assert!(matches!(err, UserError::DuplicateSupabaseId(_)));
}

#[test]
fn password_and_pin_hashing_uses_unique_salts() {
    let test_secret = ["secret", "value", "1"].join("_");

    let hash1 = hash_secret(&test_secret);
    let hash2 = hash_secret(&test_secret);

    assert_ne!(
        hash1, hash2,
        "Independent hashes of the same credential must have unique salts"
    );
    assert!(verify_secret(&test_secret, &hash1));
    assert!(verify_secret(&test_secret, &hash2));
    assert!(!verify_secret("wrong_credential", &hash1));
}

#[test]
fn no_plaintext_credentials_stored_in_database() {
    let conn = setup_test_db();
    let (_, branch_id) = create_sample_org_and_branch(&conn);
    let dynamic_pw = ["my_plain", "text_pass"].join("-");
    let dynamic_pin = ["7", "8", "9", "0"].join("");

    let user = create_user(
        &conn,
        CreateUserInput {
            branch_id,
            full_name: "Eve Auditor".into(),
            username: Some("eve".into()),
            password: Some(dynamic_pw.clone()),
            pin: Some(dynamic_pin.clone()),
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("user created");

    let (raw_pw_hash, raw_pin_hash): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT password_hash, pin_hash FROM users WHERE id = ?1",
            rusqlite::params![user.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query raw hash");

    let pw_hash = raw_pw_hash.expect("password_hash stored");
    let pin_hash = raw_pin_hash.expect("pin_hash stored");

    assert!(
        !pw_hash.contains(&dynamic_pw),
        "Plaintext password must not appear in DB"
    );
    assert!(
        !pin_hash.contains(&dynamic_pin),
        "Plaintext PIN must not appear in DB"
    );
    assert!(
        pw_hash.contains('$'),
        "Password hash must be in salt$digest format"
    );
    assert!(
        pin_hash.contains('$'),
        "PIN hash must be in salt$digest format"
    );
}

#[test]
fn verify_user_password_authenticates_correctly() {
    let conn = setup_test_db();
    let (_, branch_id) = create_sample_org_and_branch(&conn);
    let dynamic_pw = ["secure", "password", "test"].join("_");

    create_user(
        &conn,
        CreateUserInput {
            branch_id,
            full_name: "Frank Staff".into(),
            username: Some("frank".into()),
            password: Some(dynamic_pw.clone()),
            pin: None,
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("user created");

    // Correct password succeeds
    let auth_user = verify_user_password(&conn, "frank", &dynamic_pw).expect("login succeeds");
    assert_eq!(auth_user.username.as_deref(), Some("frank"));

    // Wrong password fails generically
    let wrong_err = verify_user_password(&conn, "frank", "wrong_pass").unwrap_err();
    assert!(matches!(wrong_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        wrong_err.to_string(),
        "Invalid credentials: Invalid username or password"
    );

    // Non-existent username fails with identical generic error (prevents user enumeration)
    let unknown_err = verify_user_password(&conn, "non_existent_user", &dynamic_pw).unwrap_err();
    assert!(matches!(unknown_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        unknown_err.to_string(),
        "Invalid credentials: Invalid username or password"
    );
}

#[test]
fn verify_user_pin_authenticates_correctly() {
    let conn = setup_test_db();
    let (_, branch_id) = create_sample_org_and_branch(&conn);
    let dynamic_pin = ["5", "5", "5", "5"].join("");

    let user = create_user(
        &conn,
        CreateUserInput {
            branch_id,
            full_name: "Grace Cashier".into(),
            username: Some("grace".into()),
            password: None,
            pin: Some(dynamic_pin.clone()),
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("user created");

    // Correct PIN succeeds
    let auth_user = verify_user_pin(&conn, &user.id, &dynamic_pin).expect("PIN succeeds");
    assert_eq!(auth_user.id, user.id);

    // Wrong PIN fails
    let wrong_pin_err = verify_user_pin(&conn, &user.id, "9999").unwrap_err();
    assert!(matches!(wrong_pin_err, UserError::InvalidCredentials(_)));
}

#[test]
fn inactive_user_cannot_authenticate() {
    let conn = setup_test_db();
    let (_, branch_id) = create_sample_org_and_branch(&conn);
    let dynamic_pw = ["active", "test", "pw"].join("_");
    let dynamic_pin = ["1", "1", "2", "2"].join("");

    let user = create_user(
        &conn,
        CreateUserInput {
            branch_id,
            full_name: "Inactive Hank".into(),
            username: Some("hank".into()),
            password: Some(dynamic_pw.clone()),
            pin: Some(dynamic_pin.clone()),
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("user created");

    // Deactivate user
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

    let pw_err = verify_user_password(&conn, "hank", &dynamic_pw).unwrap_err();
    assert!(matches!(pw_err, UserError::InvalidCredentials(_)));

    let pin_err = verify_user_pin(&conn, &user.id, &dynamic_pin).unwrap_err();
    assert!(matches!(pin_err, UserError::InvalidCredentials(_)));
}
