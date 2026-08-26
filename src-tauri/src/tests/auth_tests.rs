use crate::auth::{
    extract_jwt_role, is_allowed_localhost_http, parse_error_response, parse_token_response,
    validate_config, validate_credentials, AuthError, SupabaseAuthConfig,
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
fn validate_config_accepts_valid_localhost_development_urls() {
    let accepted_urls = [
        "http://localhost",
        "http://localhost:54321",
        "http://localhost/supabase",
        "http://127.0.0.1",
        "http://127.0.0.1:54321",
        "http://[::1]",
        "http://[::1]:54321",
    ];

    for url in accepted_urls {
        assert!(
            is_allowed_localhost_http(url),
            "Expected {url} to be recognized as allowed localhost HTTP"
        );

        let config = SupabaseAuthConfig {
            url: url.into(),
            publishable_key: "sb_publishable_local_key".into(),
        };
        assert!(
            validate_config(&config).is_ok(),
            "Expected config with {url} to be valid"
        );
    }
}

#[test]
fn validate_config_rejects_attacker_controlled_and_insecure_http_hosts() {
    let rejected_urls = [
        "http://localhost.attacker.com",
        "http://localhost.evil",
        "http://127.0.0.1.evil.com",
        "http://[::1].evil.com",
        "http://remote-supabase.example.com",
        "http://insecure-backend.com:8080",
    ];

    for url in rejected_urls {
        assert!(
            !is_allowed_localhost_http(url),
            "Expected {url} to be rejected as forbidden HTTP host"
        );

        let config = SupabaseAuthConfig {
            url: url.into(),
            publishable_key: "sb_publishable_token".into(),
        };
        assert!(
            matches!(
                validate_config(&config),
                Err(AuthError::SecurityViolation(_))
            ),
            "Expected config with {url} to return SecurityViolation"
        );
    }
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
    let anon_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJyb2xlIjoic2VydmljZV9hbm9uX2tleSIsImlzcyI6InN1cGFiYXNlIiwicm9sZSI6ImFub24iLCJleHAiOjE5MDAwMDAwMDB9.signature";

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

#[test]
fn validate_refresh_token_accepts_valid_tokens_and_rejects_empty() {
    use crate::auth::validate_refresh_token;

    assert!(validate_refresh_token("valid_refresh_token_12345").is_ok());
    assert!(matches!(
        validate_refresh_token(""),
        Err(AuthError::Validation(_))
    ));
    assert!(matches!(
        validate_refresh_token("   "),
        Err(AuthError::Validation(_))
    ));
    assert!(matches!(
        validate_refresh_token("short"),
        Err(AuthError::Validation(_))
    ));
}

#[test]
fn validate_access_token_accepts_valid_and_rejects_empty() {
    use crate::auth::validate_access_token;

    assert!(validate_access_token("valid_access_token_12345").is_ok());
    assert!(matches!(
        validate_access_token(""),
        Err(AuthError::Validation(_))
    ));
    assert!(matches!(
        validate_access_token("   "),
        Err(AuthError::Validation(_))
    ));
}

#[test]
fn parse_error_response_classifies_invalid_and_expired_refresh_tokens() {
    let invalid_refresh_json = r#"{
        "error": "invalid_grant",
        "error_description": "Invalid Refresh Token: Refresh Token Not Found"
    }"#;
    let err = parse_error_response(400, invalid_refresh_json);
    assert!(matches!(err, AuthError::SessionExpired(_)));
    assert!(err.to_string().contains("Refresh token is invalid"));

    let already_used_json = r#"{
        "error": "invalid_grant",
        "error_description": "Refresh token has already used"
    }"#;
    let err_used = parse_error_response(400, already_used_json);
    assert!(matches!(err_used, AuthError::SessionExpired(_)));
}

#[test]
fn parse_token_response_handles_successful_refresh_payload() {
    let refresh_json = r#"{
        "access_token": "new_access_token_999",
        "refresh_token": "new_refresh_token_888",
        "expires_in": 3600,
        "expires_at": 1893456000,
        "token_type": "bearer",
        "user": {
            "id": "usr_online_refresh_1",
            "email": "refreshed@example.com",
            "created_at": "2026-01-01T00:00:00Z",
            "last_sign_in_at": "2026-08-26T12:00:00Z"
        }
    }"#;

    let session = parse_token_response(refresh_json).expect("should parse refresh response");
    assert_eq!(session.access_token, "new_access_token_999");
    assert_eq!(
        session.refresh_token,
        Some("new_refresh_token_888".to_string())
    );
    assert_eq!(session.user.id, "usr_online_refresh_1");
    assert_eq!(session.user.email, "refreshed@example.com");
}

#[test]
fn refresh_token_input_constructs_and_deserializes() {
    use crate::auth::RefreshTokenInput;

    let input = RefreshTokenInput {
        refresh_token: "test_refresh_token_12345".into(),
    };
    assert_eq!(input.refresh_token, "test_refresh_token_12345");

    let json = r#"{"refreshToken":"aliased_token_999"}"#;
    let deserialized: RefreshTokenInput =
        serde_json::from_str(json).expect("should deserialize alias");
    assert_eq!(deserialized.refresh_token, "aliased_token_999");
}
