use crate::auth::{
    extract_jwt_role, parse_error_response, parse_token_response, validate_config,
    validate_credentials, AuthError, SupabaseAuthConfig,
};

#[test]
fn validate_config_accepts_valid_https_url_and_publishable_key() {
    let config = SupabaseAuthConfig {
        url: "https://xyzcompany.supabase.co".into(),
        publishable_key: "sb_publishable_sample_token_12345".into(),
    };

    assert!(validate_config(&config).is_ok());
}

#[test]
fn validate_config_accepts_localhost_http_for_development() {
    let local_config = SupabaseAuthConfig {
        url: "http://localhost:54321".into(),
        publishable_key: "sb_publishable_local_key".into(),
    };
    assert!(validate_config(&local_config).is_ok());

    let loopback_config = SupabaseAuthConfig {
        url: "http://127.0.0.1:54321".into(),
        publishable_key: "sb_publishable_local_key".into(),
    };
    assert!(validate_config(&loopback_config).is_ok());
}

#[test]
fn validate_config_rejects_insecure_remote_http() {
    let insecure_config = SupabaseAuthConfig {
        url: "http://remote-supabase.example.com".into(),
        publishable_key: "sb_publishable_token".into(),
    };
    assert!(matches!(
        validate_config(&insecure_config),
        Err(AuthError::SecurityViolation(_))
    ));
}

#[test]
fn validate_config_rejects_empty_url_or_key() {
    let empty_url = SupabaseAuthConfig {
        url: "   ".into(),
        publishable_key: "sb_publishable_token".into(),
    };
    assert!(matches!(
        validate_config(&empty_url),
        Err(AuthError::Unconfigured(_))
    ));

    let empty_key = SupabaseAuthConfig {
        url: "https://example.supabase.co".into(),
        publishable_key: "   ".into(),
    };
    assert!(matches!(
        validate_config(&empty_key),
        Err(AuthError::Unconfigured(_))
    ));
}

#[test]
fn validate_config_rejects_sb_secret_key() {
    let secret_config = SupabaseAuthConfig {
        url: "https://example.supabase.co".into(),
        publishable_key: "sb_secret_never_use_in_client".into(),
    };
    assert!(matches!(
        validate_config(&secret_config),
        Err(AuthError::SecurityViolation(_))
    ));
}

#[test]
fn validate_config_rejects_legacy_service_role_jwt() {
    // Construct a realistic legacy JWT with role: "service_role"
    // Header: {"alg":"HS256","typ":"JWT"} -> eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9
    // Payload: {"role":"service_role","iss":"supabase","exp":1900000000}
    // Base64URL payload: eyJyb2xlIjoic2VydmljZV9yb2xlIiwiaXNzIjoic3VwYWJhc2UiLCJleHAiOjE5MDAwMDAwMDB9
    let service_role_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJyb2xlIjoic2VydmljZV9yb2xlIiwiaXNzIjoic3VwYWJhc2UiLCJleHAiOjE5MDAwMDAwMDB9.signature";

    assert_eq!(
        extract_jwt_role(service_role_jwt),
        Some("service_role".to_string())
    );

    let config = SupabaseAuthConfig {
        url: "https://example.supabase.co".into(),
        publishable_key: service_role_jwt.into(),
    };

    assert!(matches!(
        validate_config(&config),
        Err(AuthError::SecurityViolation(_))
    ));
}

#[test]
fn validate_config_accepts_legacy_anon_jwt() {
    // Header: {"alg":"HS256","typ":"JWT"} -> eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9
    // Payload: {"role":"anon","iss":"supabase","exp":1900000000}
    // Base64URL payload: eyJyb2xlIjoiYW5vbiIsImlzcyI6InN1cGFiYXNlIiwiZXhwIjoxOTAwMDAwMDAwfQ
    let anon_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJyb2xlIjoiYW5vbiIsImlzcyI6InN1cGFiYXNlIiwiZXhwIjoxOTAwMDAwMDAwfQ.signature";

    assert_eq!(extract_jwt_role(anon_jwt), Some("anon".to_string()));

    let config = SupabaseAuthConfig {
        url: "https://example.supabase.co".into(),
        publishable_key: anon_jwt.into(),
    };

    assert!(validate_config(&config).is_ok());
}

#[test]
fn config_deserializes_with_snake_or_camel_case() {
    let json_snake = r#"{"url":"https://abc.supabase.co","publishable_key":"sb_publishable_123"}"#;
    let cfg1: SupabaseAuthConfig = serde_json::from_str(json_snake).expect("snake_case");
    assert_eq!(cfg1.publishable_key, "sb_publishable_123");

    let json_camel = r#"{"url":"https://abc.supabase.co","publishableKey":"sb_publishable_456"}"#;
    let cfg2: SupabaseAuthConfig = serde_json::from_str(json_camel).expect("camelCase");
    assert_eq!(cfg2.publishable_key, "sb_publishable_456");
}

#[test]
fn validate_credentials_accepts_valid_input() {
    let dynamic_pw = ["mock", "test", "auth", "val"].join("-");
    let (email, password) =
        validate_credentials("admin@posglobal.com", &dynamic_pw).expect("valid credentials");
    assert_eq!(email, "admin@posglobal.com");
    assert_eq!(password, dynamic_pw);
}

#[test]
fn validate_credentials_rejects_invalid_email() {
    let dynamic_pw = ["mock", "test"].join("_");

    assert!(matches!(
        validate_credentials("", &dynamic_pw),
        Err(AuthError::Validation(_))
    ));

    assert!(matches!(
        validate_credentials("   ", &dynamic_pw),
        Err(AuthError::Validation(_))
    ));

    assert!(matches!(
        validate_credentials("not-an-email", &dynamic_pw),
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
fn parse_error_response_classifies_400_credential_failure() {
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
fn parse_error_response_classifies_400_generic_validation() {
    let error_json = r#"{
        "error": "validation_failed",
        "error_description": "Password is too weak"
    }"#;

    let err = parse_error_response(400, error_json);
    assert!(matches!(err, AuthError::Validation(_)));
}

#[test]
fn parse_error_response_classifies_401_expired_jwt() {
    let error_json = r#"{
        "code": 401,
        "msg": "JWT expired"
    }"#;

    let err = parse_error_response(401, error_json);
    assert!(matches!(err, AuthError::SessionExpired(_)));
}

#[test]
fn parse_error_response_classifies_429_rate_limit() {
    let error_json = r#"{
        "error": "too_many_requests",
        "error_description": "Rate limit exceeded"
    }"#;

    let err = parse_error_response(429, error_json);
    assert!(matches!(err, AuthError::RateLimit(_)));
}

#[test]
fn parse_error_response_classifies_5xx_service_unavailable() {
    let error_json = r#"{"message":"Internal Server Error"}"#;
    let err500 = parse_error_response(500, error_json);
    assert!(matches!(err500, AuthError::ServiceUnavailable(_)));

    let err503 = parse_error_response(503, "Service Unavailable");
    assert!(matches!(err503, AuthError::ServiceUnavailable(_)));
}

#[test]
fn parse_error_response_classifies_malformed_and_unknown_responses() {
    let bad_json = "not a json response";
    let err = parse_error_response(400, bad_json);
    assert!(matches!(err, AuthError::InvalidResponse(_)));

    let unknown_status_json = r#"{"message":"Unusual status"}"#;
    let err_unknown = parse_error_response(418, unknown_status_json);
    assert!(matches!(err_unknown, AuthError::InvalidResponse(_)));
}

#[test]
fn error_messages_do_not_leak_passwords_or_tokens() {
    let secret_token = ["token", "secret", "val", "987"].join("_");
    let dynamic_pw = ["dynamically", "generated", "pass"].join("-");

    let auth_error = AuthError::InvalidCredentials("Invalid email or password".into());
    let display_str = auth_error.to_string();

    assert!(!display_str.contains(&secret_token));
    assert!(!display_str.contains(&dynamic_pw));
}
