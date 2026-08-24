// Supabase Auth adapter for online account identity.
// F1.04 — Supabase Auth adapter

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupabaseAuthConfig {
    pub url: String,
    pub anon_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnlineIdentity {
    pub id: String,
    pub email: String,
    pub created_at: Option<String>,
    pub last_sign_in_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnlineSession {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
    pub user: OnlineIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignInInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthError {
    InvalidCredentials(String),
    Network(String),
    Unconfigured(String),
    SessionExpired(String),
    InvalidResponse(String),
    Validation(String),
    SecurityViolation(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials(msg) => write!(f, "Invalid credentials: {msg}"),
            AuthError::Network(msg) => write!(f, "Network error: {msg}"),
            AuthError::Unconfigured(msg) => write!(f, "Configuration error: {msg}"),
            AuthError::SessionExpired(msg) => write!(f, "Session expired: {msg}"),
            AuthError::InvalidResponse(msg) => write!(f, "Invalid auth response: {msg}"),
            AuthError::Validation(msg) => write!(f, "Validation error: {msg}"),
            AuthError::SecurityViolation(msg) => write!(f, "Security violation: {msg}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Validates that configuration contains only public client parameters and no private secrets.
pub fn validate_config(config: &SupabaseAuthConfig) -> Result<(), AuthError> {
    let trimmed_url = config.url.trim();
    if trimmed_url.is_empty() {
        return Err(AuthError::Unconfigured(
            "Supabase URL cannot be empty".into(),
        ));
    }

    if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://") {
        return Err(AuthError::Unconfigured(
            "Supabase URL must start with http:// or https://".into(),
        ));
    }

    let trimmed_key = config.anon_key.trim();
    if trimmed_key.is_empty() {
        return Err(AuthError::Unconfigured(
            "Supabase publishable key cannot be empty".into(),
        ));
    }

    // Security guardrail: verify that service_role or admin secret keys are never accepted in the client
    if trimmed_key.to_lowercase().contains("service_role")
        || trimmed_key.to_lowercase().contains("secret")
    {
        return Err(AuthError::SecurityViolation(
            "Privileged service-role key or admin secret must never be used in client configuration"
                .into(),
        ));
    }

    Ok(())
}

/// Validates sign-in credentials before transmission.
pub fn validate_credentials(email: &str, password: &str) -> Result<(String, String), AuthError> {
    let trimmed_email = email.trim();
    if trimmed_email.is_empty() {
        return Err(AuthError::Validation("Email cannot be empty".into()));
    }
    if !trimmed_email.contains('@') || trimmed_email.len() < 3 {
        return Err(AuthError::Validation("Invalid email format".into()));
    }

    if password.is_empty() {
        return Err(AuthError::Validation("Password cannot be empty".into()));
    }

    Ok((trimmed_email.to_string(), password.to_string()))
}

// Internal raw Supabase auth response types for deserialization
#[derive(Debug, Deserialize)]
struct RawSupabaseUser {
    id: String,
    email: Option<String>,
    created_at: Option<String>,
    last_sign_in_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSupabaseTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<i64>,
    expires_in: Option<i64>,
    token_type: Option<String>,
    user: Option<RawSupabaseUser>,
}

#[derive(Debug, Deserialize)]
struct RawSupabaseErrorResponse {
    error: Option<String>,
    error_description: Option<String>,
    message: Option<String>,
    msg: Option<String>,
}

/// Maps raw Supabase token response into clean OnlineSession domain model.
pub fn parse_token_response(json_str: &str) -> Result<OnlineSession, AuthError> {
    let raw: RawSupabaseTokenResponse = serde_json::from_str(json_str).map_err(|e| {
        AuthError::InvalidResponse(format!("Failed to parse authentication response: {e}"))
    })?;

    let user = raw
        .user
        .ok_or_else(|| AuthError::InvalidResponse("User data missing in auth response".into()))?;

    Ok(OnlineSession {
        access_token: raw.access_token,
        refresh_token: raw.refresh_token,
        expires_at: raw.expires_at,
        expires_in: raw.expires_in,
        token_type: raw.token_type,
        user: OnlineIdentity {
            id: user.id,
            email: user.email.unwrap_or_default(),
            created_at: user.created_at,
            last_sign_in_at: user.last_sign_in_at,
        },
    })
}

/// Maps Supabase error response JSON into typed domain AuthError without leaking secrets.
pub fn parse_error_response(status_code: u16, json_str: &str) -> AuthError {
    let error_body: Result<RawSupabaseErrorResponse, _> = serde_json::from_str(json_str);

    let err_msg = match error_body {
        Ok(err) => err
            .error_description
            .or(err.message)
            .or(err.msg)
            .or(err.error)
            .unwrap_or_else(|| "Authentication failed".into()),
        Err(_) => "Authentication failed".into(),
    };

    let lower = err_msg.to_lowercase();
    if lower.contains("invalid login credentials")
        || lower.contains("invalid_grant")
        || lower.contains("invalid credentials")
        || status_code == 400
    {
        AuthError::InvalidCredentials("Invalid email or password".into())
    } else if lower.contains("jwt expired")
        || lower.contains("session expired")
        || status_code == 401
    {
        AuthError::SessionExpired("Session has expired. Please sign in again.".into())
    } else {
        AuthError::InvalidCredentials("Authentication failed".into())
    }
}
