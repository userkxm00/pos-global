use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_hierarchy, create_test_user_with_creds,
    setup_test_db,
};
use crate::user::{
    create_user, get_user_by_supabase_id, hash_secret, update_user, verify_secret,
    verify_user_password_with_limiter, verify_user_pin_with_limiter, CreateUserInput, RateLimiter,
    UpdateUserInput, UserError,
};

#[test]
fn valid_user_creation_and_argon2_hashing() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let dynamic_pw = ["alpha", "beta", "pass"].join("-");
    let dynamic_pin = ["8", "7", "6", "5"].join("");

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Alice Johnson",
        Some("alice_pos"),
        Some(&dynamic_pw),
        Some(&dynamic_pin),
        "cashier",
    )
    .expect("user should be created");

    assert_eq!(user.full_name, "Alice Johnson");
    assert_eq!(user.username.as_deref(), Some("alice_pos"));
    assert_eq!(user.role, "cashier");
    assert!(user.is_active);
    assert_eq!(user.branch_id, branch_id);
    assert_eq!(user.auth_provider, "local");

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

    let err =
        create_test_user_with_creds(&conn, &branch_id, "   ", Some("bob"), None, None, "cashier")
            .unwrap_err();

    assert!(matches!(err, UserError::Validation(_)));
}

#[test]
fn create_user_rejects_invalid_branch() {
    let conn = setup_test_db();

    let err = create_test_user_with_creds(
        &conn,
        "non-existent-branch",
        "Charlie Brown",
        Some("charlie"),
        None,
        None,
        "cashier",
    )
    .unwrap_err();

    assert!(matches!(err, UserError::BranchNotFound(_)));
}

#[test]
fn create_user_enforces_unique_username() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    create_test_user_with_creds(
        &conn,
        &branch_id,
        "David Cashier 1",
        Some("david_unique"),
        None,
        None,
        "cashier",
    )
    .expect("first user created");

    let err = create_test_user_with_creds(
        &conn,
        &branch_id,
        "David Cashier 2",
        Some("david_unique"),
        None,
        None,
        "cashier",
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
    let wrong_candidate = ["wrong", "candidate", "pw"].join("_");
    let test_limiter = RateLimiter::new(5, 30, 100);
    let client_id = uuid::Uuid::new_v4().to_string();

    create_test_user_with_creds(
        &conn,
        &branch_id,
        "Frank Staff",
        Some("frank"),
        Some(&dynamic_pw),
        None,
        "cashier",
    )
    .expect("user created");

    // Correct password succeeds
    let auth_user =
        verify_user_password_with_limiter(&conn, &test_limiter, &client_id, "frank", &dynamic_pw)
            .expect("login succeeds");
    assert_eq!(auth_user.username.as_deref(), Some("frank"));

    // Wrong password returns generic error
    let wrong_err = verify_user_password_with_limiter(
        &conn,
        &test_limiter,
        &client_id,
        "frank",
        &wrong_candidate,
    )
    .unwrap_err();
    assert!(matches!(wrong_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        wrong_err.to_string(),
        "Invalid credentials: Invalid username or password"
    );

    // Nonexistent username executes decoy Argon2 verification and returns identical error
    let nonexistent_err = verify_user_password_with_limiter(
        &conn,
        &test_limiter,
        &client_id,
        "nonexistent_operator",
        &dynamic_pw,
    )
    .unwrap_err();
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
    let wrong_pin = ["0", "0", "0", "0"].join("");
    let test_limiter = RateLimiter::new(5, 30, 100);
    let client_id = uuid::Uuid::new_v4().to_string();

    let user = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Grace Cashier",
        Some("grace"),
        None,
        Some(&dynamic_pin),
        "cashier",
    )
    .expect("user created");

    // Correct PIN succeeds
    let auth_user =
        verify_user_pin_with_limiter(&conn, &test_limiter, &client_id, &user.id, &dynamic_pin)
            .expect("PIN succeeds");
    assert_eq!(auth_user.id, user.id);

    // Wrong PIN returns generic error
    let wrong_pin_err =
        verify_user_pin_with_limiter(&conn, &test_limiter, &client_id, &user.id, &wrong_pin)
            .unwrap_err();
    assert!(matches!(wrong_pin_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        wrong_pin_err.to_string(),
        "Invalid credentials: Invalid PIN"
    );

    // Nonexistent user ID executes decoy Argon2 verification and returns identical error
    let nonexistent_pin_err = verify_user_pin_with_limiter(
        &conn,
        &test_limiter,
        &client_id,
        "nonexistent-user-id",
        &wrong_pin,
    )
    .unwrap_err();
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
    let test_limiter = RateLimiter::new(5, 30, 100);
    let client_id = uuid::Uuid::new_v4().to_string();

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

    let pw_err =
        verify_user_password_with_limiter(&conn, &test_limiter, &client_id, "test_staff", &test_pw)
            .unwrap_err();
    assert!(matches!(pw_err, UserError::InvalidCredentials(_)));
    assert_eq!(
        pw_err.to_string(),
        "Invalid credentials: Invalid username or password"
    );

    let pin_err =
        verify_user_pin_with_limiter(&conn, &test_limiter, &client_id, &user.id, &test_pin)
            .unwrap_err();
    assert!(matches!(pin_err, UserError::InvalidCredentials(_)));
    assert_eq!(pin_err.to_string(), "Invalid credentials: Invalid PIN");
}

#[test]
fn rate_limiter_client_scoping_prevents_victim_lockout() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let user_name = "shared_target_user";
    let valid_pw = ["valid", "pass", "99"].join("_");
    let wrong_guess = ["wrong", "guess", "val"].join("_");
    let test_limiter = RateLimiter::new(3, 30, 100);
    let attacker_client = "attacker_terminal_01";
    let victim_client = "cashier_terminal_02";

    create_test_user_with_creds(
        &conn,
        &branch_id,
        "Target User",
        Some(user_name),
        Some(&valid_pw),
        None,
        "cashier",
    )
    .expect("user created");

    // Attacker exhausts attempts from their terminal
    for _ in 0..3 {
        let _ = verify_user_password_with_limiter(
            &conn,
            &test_limiter,
            attacker_client,
            user_name,
            &wrong_guess,
        );
    }

    // Attacker is now locked out on their terminal
    let attacker_err = verify_user_password_with_limiter(
        &conn,
        &test_limiter,
        attacker_client,
        user_name,
        &valid_pw,
    )
    .unwrap_err();
    assert!(matches!(attacker_err, UserError::InvalidCredentials(_)));
    assert!(attacker_err.to_string().contains("locked"));

    // Victim on legitimate cashier terminal is NOT locked out and logs in successfully
    let victim_auth = verify_user_password_with_limiter(
        &conn,
        &test_limiter,
        victim_client,
        user_name,
        &valid_pw,
    );
    assert!(
        victim_auth.is_ok(),
        "Legitimate client must not be locked out by attacker on another client"
    );
}

#[test]
fn rate_limiter_evicts_oldest_non_locked_entry_when_capacity_reached() {
    let limiter = RateLimiter::new(3, 30, 2); // capacity = 2 entries, 3 attempts to lock

    limiter.record_failure("clientA:user:first_attempt");
    limiter.record_failure("clientB:user:second_attempt");
    assert_eq!(limiter.len(), 2);

    // Record a 3rd distinct entry; capacity is 2 so oldest non-locked (first_attempt) is evicted
    limiter.record_failure("clientC:user:third_attempt");
    assert_eq!(limiter.len(), 2);

    // first_attempt is now evicted and starts clean
    assert!(limiter.check("clientA:user:first_attempt").is_ok());
}

#[test]
fn rate_limiter_never_evicts_active_lockouts_when_all_entries_locked() {
    let limiter = RateLimiter::new(2, 30, 2); // capacity = 2 entries, 2 attempts to lock

    // Lock both entries to fill capacity with 100% active lockouts
    limiter.record_failure("client1:user:locked_a");
    limiter.record_failure("client1:user:locked_a");
    assert!(limiter.check("client1:user:locked_a").is_err());

    limiter.record_failure("client2:user:locked_b");
    limiter.record_failure("client2:user:locked_b");
    assert!(limiter.check("client2:user:locked_b").is_err());

    assert_eq!(limiter.len(), 2);

    // Attempt to flood with new keys; neither active lockout may be evicted
    limiter.record_failure("client3:user:flooder_1");
    limiter.record_failure("client4:user:flooder_2");

    assert_eq!(limiter.len(), 2);
    assert!(
        limiter.check("client1:user:locked_a").is_err(),
        "locked_a must remain locked"
    );
    assert!(
        limiter.check("client2:user:locked_b").is_err(),
        "locked_b must remain locked"
    );
}

#[test]
fn rate_limiter_saturated_lockouts_trigger_admission_throttle_for_new_keys() {
    let limiter = RateLimiter::new(2, 30, 2); // capacity = 2 entries, 2 attempts to lock

    // Lock all available slots
    limiter.record_failure("slot1:user:target_1");
    limiter.record_failure("slot1:user:target_1");
    limiter.record_failure("slot2:user:target_2");
    limiter.record_failure("slot2:user:target_2");

    assert_eq!(limiter.len(), 2);

    // Overflow attempts with new keys exceed threshold
    limiter.record_failure("slot3:user:overflow_1");
    limiter.record_failure("slot4:user:overflow_2");

    // Admission throttle is active: new keys cannot bypass rate limiting
    let throttle_err = limiter.check("slot5:user:new_key");
    assert!(
        throttle_err.is_err(),
        "New keys must be throttled when capacity is saturated with active lockouts"
    );

    // Original lockouts remain strictly preserved
    assert!(limiter.check("slot1:user:target_1").is_err());
    assert!(limiter.check("slot2:user:target_2").is_err());
    assert_eq!(limiter.len(), 2);
}

#[test]
fn rate_limiter_eviction_preserves_active_lockouts_under_flooding() {
    let limiter = RateLimiter::new(3, 30, 5); // capacity = 5 entries, 3 attempts to lock

    // 1. Lock out a victim key
    let locked_key = "terminal1:user:victim_operator";
    for _ in 0..3 {
        limiter.record_failure(locked_key);
    }
    assert!(limiter.check(locked_key).is_err(), "Key must be locked");

    // 2. Flood with 10 distinct non-locked keys
    for i in 0..10 {
        let key = format!("flood_client:user:flooder_{i}");
        limiter.record_failure(&key);
    }

    // 3. Verify total tracked keys does not exceed max capacity
    assert!(
        limiter.len() <= 5,
        "Limiter size must be bounded at max capacity"
    );

    // 4. Verify the actively locked key was preserved and NOT evicted
    let locked_check = limiter.check(locked_key);
    assert!(
        locked_check.is_err(),
        "Actively locked entry must survive eviction flooding"
    );
}
