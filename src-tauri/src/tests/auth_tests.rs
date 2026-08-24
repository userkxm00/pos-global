use crate::auth::{
    parse_error_response, parse_token_response, validate_config, validate_credentials, AuthError,
    SupabaseAuthConfig,
};

#[test]
fn validate_config_accepts_valid_public_configuration() {
    let config = SupabaseAuthConfig {
        url: "https://xyzcompany.supabase.co".into(),
        anon_key: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.anon_key_payload.signature".into(),
    };

    assert!(validate_config(&config).is_ok());
}

#[test]
fn validate_config_rejects_empty_url_or_key() {
    let empty_url = SupabaseAuthConfig {
        url: "   ".into(),
        anon_key: "anon-key-123".into(),
    };
    assert!(matches!(
        validate_config(&empty_url),
        Err(AuthError::Unconfigured(_))
    ));

    let empty_key = SupabaseAuthConfig {
        url: "https://example.supabase.co".into(),
        anon_key: "   ".into(),
    };
    assert!(matches!(
        validate_config(&empty_key),
        Err(AuthError::Unconfigured(_))
    ));
}

#[test]
fn validate_config_rejects_invalid_url_scheme() {
    let invalid_scheme = SupabaseAuthConfig {
        url: "ftp://example.supabase.co".into(),
        anon_key: "anon-key-123".into(),
    };
    assert!(matches!(
        validate_config(&invalid_scheme),
        Err(AuthError::Unconfigured(_))
    ));
}

#[test]
fn validate_config_rejects_service_role_or_admin_secret() {
    let service_role_config = SupabaseAuthConfig {
        url: "https://example.supabase.co".into(),
        anon_key: "eyJhbGciOi...service_role...secret".into(),
    };
    assert!(matches!(
        validate_config(&service_role_config),
        Err(AuthError::SecurityViolation(_))
    ));
}

#[test]
fn validate_credentials_accepts_valid_input() {
    let (email, password) = validate_credentials("admin@posglobal.com", "SecurePassword123!")
        .expect("valid credentials");
    assert_eq!(email, "admin@posglobal.com");
    assert_eq!(password, "SecurePassword123!");
}

#[test]
fn validate_credentials_rejects_invalid_email() {
    assert!(matches!(
        validate_credentials("", "password123"),
        Err(AuthError::Validation(_))
    ));

    assert!(matches!(
        validate_credentials("   ", "password123"),
        Err(AuthError::Validation(_))
    ));

    assert!(matches!(
        validate_credentials("not-an-email", "password123"),
        Err(AuthError::Validation(_))
    ));
}

#[test]
fn validate_credentials_rejects_empty_password() {
    assert!(matches!(
        validate_credentials("user@example.com", ""),
        Err(AuthError::Validation(_))
    ));
}

#[test]
fn parse_token_response_parses_valid_supabase_auth_json() {
    let raw_json = r#"{
        "access_token": "mock-access-token-12345",
        "token_type": "bearer",
        "expires_in": 3600,
        "expires_at": 1700003600,
        "refresh_token": "mock-refresh-token-67890",
        "user": {
            "id": "u-1111-2222-3333",
            "aud": "authenticated",
            "role": "authenticated",
            "email": "owner@acme.com",
            "created_at": "2026-01-01T12:00:00Z",
            "last_sign_in_at": "2026-01-02T08:30:00Z"
        }
    }"#;

    let session = parse_token_response(raw_json).expect("should parse valid auth response");
    assert_eq!(session.access_token, "mock-access-token-12345");
    assert_eq!(
        session.refresh_token,
        Some("mock-refresh-token-67890".to_string())
    );
    assert_eq!(session.expires_in, Some(3600));
    assert_eq!(session.expires_at, Some(1700003600));
    assert_eq!(session.user.id, "u-1111-2222-3333");
    assert_eq!(session.user.email, "owner@acme.com");
    assert_eq!(
        session.user.created_at,
        Some("2026-01-01T12:00:00Z".to_string())
    );
}

#[test]
fn parse_token_response_rejects_malformed_json() {
    let bad_json = "{ invalid_json: true }";
    assert!(matches!(
        parse_token_response(bad_json),
        Err(AuthError::InvalidResponse(_))
    ));
}

#[test]
fn parse_token_response_rejects_missing_user_object() {
    let missing_user = r#"{
        "access_token": "mock-token",
        "token_type": "bearer"
    }"#;
    assert!(matches!(
        parse_token_response(missing_user),
        Err(AuthError::InvalidResponse(_))
    ));
}

#[test]
fn parse_error_response_maps_invalid_credentials() {
    let error_json = r#"{
        "error": "invalid_grant",
        "error_description": "Invalid login credentials"
    }"#;

    let err = parse_error_response(400, error_json);
    assert!(matches!(err, AuthError::InvalidCredentials(_)));
    assert_eq!(
        err.to_string(),
        "Invalid credentials: Invalid email or password"
    );
}

#[test]
fn parse_error_response_maps_expired_jwt() {
    let error_json = r#"{
        "code": 401,
        "msg": "JWT expired"
    }"#;

    let err = parse_error_response(401, error_json);
    assert!(matches!(err, AuthError::SessionExpired(_)));
}

#[test]
fn error_messages_do_not_leak_passwords_or_tokens() {
    let secret_token = "secret_jwt_token_12345";
    let password = "SuperSecretPassword123";

    let auth_error = AuthError::InvalidCredentials("Invalid email or password".into());
    let display_str = auth_error.to_string();

    assert!(!display_str.contains(secret_token));
    assert!(!display_str.contains(password));
}
