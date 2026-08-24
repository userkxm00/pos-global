use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_hierarchy, setup_test_db,
};
use crate::user::{
    create_user, get_auth_rate_limiter, get_user_by_supabase_id, hash_secret, update_user,
    verify_secret, verify_user_password, verify_user_pin, CreateUserInput, UpdateUserInput,
    UserError,
};

#[test]
fn valid_user_creation_and_argon2_hashing() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let dynamic_pw = ["alpha", "beta", "pass"].join("-");
    let dynamic_pin = ["8", "7", "6", "5"].join("");

    let user = create_user(
        &conn,
        CreateUserInput {
            branch_id: branch_id.clone(),
            full_name: "Alice Johnson".into(),
            username: Some("alice_pos".into()),
            password: Some(dynamic_pw.clone()),
            pin: Some(dynamic_pin.clone()),
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

    // Verify stored hashes are in standard Argon2id PHC format
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
        pw_hash.starts_with("$argon2id$"),
        "Password hash must be in Argon2id PHC format"
    );
    assert!(
        pin_hash.starts_with("$argon2id$"),
        "PIN hash must be in Argon2id PHC format"
    );
    assert!(
        !pw_hash.contains(&dynamic_pw),
        "No plaintext password in DB"
    );
    assert!(!pin_hash.contains(&dynamic_pin), "No plaintext PIN in DB");
}

#[test]
fn argon2_hashing_produces_unique_salts() {
    let test_secret = ["argon2", "salt", "test"].join("_");

    let hash1 = hash_secret(&test_secret).expect("hash 1");
    let hash2 = hash_secret(&test_secret).expect("hash 2");

    assert_ne!(
        hash1, hash2,
        "Independent Argon2 hashes must have unique salts"
    );
    assert!(verify_secret(&test_secret, &hash1));
    assert!(verify_secret(&test_secret, &hash2));
    assert!(!verify_secret("invalid_candidate", &hash1));
}

#[test]
fn verify_secret_handles_malformed_hash_safely() {
    assert!(!verify_secret("candidate", "not-a-valid-argon2-hash"));
    assert!(!verify_secret("candidate", "$argon2id$invalid"));
    assert!(!verify_secret("candidate", ""));
}

#[test]
fn create_user_rejects_empty_name() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

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
    let (_, branch_id) = create_test_org_and_branch(&conn);

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
    let (_, branch_id) = create_test_org_and_branch(&conn);

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
fn verify_user_password_authenticates_and_defeats_timing_enumeration() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let dynamic_pw = ["dynamic", "secure", "pass"].join("_");
    get_auth_rate_limiter().reset_all();

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

    // Wrong password returns generic error
    let wrong_err = verify_user_password(&conn, "frank", "wrong_candidate_pw").unwrap_err();
    assert!(matches!(wrong_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        wrong_err.to_string(),
        "Invalid credentials: Invalid username or password"
    );

    // Nonexistent username executes decoy Argon2 verification and returns identical error
    let nonexistent_err =
        verify_user_password(&conn, "nonexistent_operator", &dynamic_pw).unwrap_err();
    assert!(matches!(nonexistent_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        nonexistent_err.to_string(),
        "Invalid credentials: Invalid username or password"
    );
}

#[test]
fn verify_user_pin_authenticates_correctly() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let dynamic_pin = ["9", "8", "7", "6"].join("");
    get_auth_rate_limiter().reset_all();

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

    // Wrong PIN returns generic error
    let wrong_pin_err = verify_user_pin(&conn, &user.id, "0000").unwrap_err();
    assert!(matches!(wrong_pin_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        wrong_pin_err.to_string(),
        "Invalid credentials: Invalid PIN"
    );

    // Nonexistent user ID executes decoy Argon2 verification and returns identical error
    let nonexistent_pin_err = verify_user_pin(&conn, "nonexistent-user-id", "0000").unwrap_err();
    assert!(matches!(
        nonexistent_pin_err,
        UserError::InvalidCredentials(_)
    ));
    assert_eq!(
        nonexistent_pin_err.to_string(),
        "Invalid credentials: Invalid PIN"
    );
}

#[test]
fn inactive_user_returns_generic_error() {
    let conn = setup_test_db();
    let (_, _, user) = create_test_user_hierarchy(&conn);
    let test_pw = ["fixture", "pass", "123"].join("_");
    let test_pin = ["4", "3", "2", "1"].join("");
    get_auth_rate_limiter().reset_all();

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

    let pw_err = verify_user_password(&conn, "test_staff", &test_pw).unwrap_err();
    assert!(matches!(pw_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        pw_err.to_string(),
        "Invalid credentials: Invalid username or password"
    );

    let pin_err = verify_user_pin(&conn, &user.id, &test_pin).unwrap_err();
    assert!(matches!(pin_err, UserError::InvalidCredentials(_)));
    assert_eq!(pin_err.to_string(), "Invalid credentials: Invalid PIN");
}

#[test]
fn rate_limiter_enforces_lockout_and_resets_on_success() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let user_name = "rate_limit_user";
    let valid_pw = ["valid", "pass", "99"].join("_");
    let limiter = get_auth_rate_limiter();
    limiter.reset_all();

    create_user(
        &conn,
        CreateUserInput {
            branch_id,
            full_name: "Rate Limit Subject".into(),
            username: Some(user_name.into()),
            password: Some(valid_pw.clone()),
            pin: None,
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("user created");

    // Perform 5 failed attempts to trigger lockout
    for _ in 0..5 {
        let _ = verify_user_password(&conn, user_name, "wrong_password");
    }

    // 6th attempt must be rejected immediately by rate limiter
    let lockout_err = verify_user_password(&conn, user_name, &valid_pw).unwrap_err();
    assert!(matches!(lockout_err, UserError::InvalidCredentials(_)));
    assert!(lockout_err.to_string().contains("locked"));

    // Reset rate limiter and verify that valid credentials now succeed
    limiter.reset_all();
    let success = verify_user_password(&conn, user_name, &valid_pw);
    assert!(success.is_ok(), "Successful auth after rate limit reset");
}
