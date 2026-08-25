// Supabase Auth adapter for online account identity.
// F1.04 — Supabase Auth adapter & F1.07 — Rust authorization middleware

pub mod middleware;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupabaseAuthConfig {
    pub url: String,
    #[serde(alias = "publishableKey")]
    pub publishable_key: String,
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
    RateLimit(String),
    Network(String),
    ServiceUnavailable(String),
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
            AuthError::RateLimit(msg) => write!(f, "Rate limit exceeded: {msg}"),
            AuthError::Network(msg) => write!(f, "Network error: {msg}"),
            AuthError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {msg}"),
            AuthError::Unconfigured(msg) => write!(f, "Configuration error: {msg}"),
            AuthError::SessionExpired(msg) => write!(f, "Session expired: {msg}"),
            AuthError::InvalidResponse(msg) => write!(f, "Invalid auth response: {msg}"),
            AuthError::Validation(msg) => write!(f, "Validation error: {msg}"),
            AuthError::SecurityViolation(msg) => write!(f, "Security violation: {msg}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Decodes a base64url string into bytes without external dependencies.
pub fn decode_base64url(s: &str) -> Result<Vec<u8>, &'static str> {
    let mut clean: Vec<u8> = Vec::new();
    for b in s.bytes() {
        match b {
            b'-' => clean.push(b'+'),
            b'_' => clean.push(b'/'),
            b if b.is_ascii_alphanumeric() || b == b'+' || b == b'/' => clean.push(b),
            _ => return Err("invalid base64url character"),
        }
    }
    while !clean.len().is_multiple_of(4) {
        clean.push(b'=');
    }

    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for &b in &clean {
        if b == b'=' {
            break;
        }
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err("invalid base64 byte"),
        };
        buffer = (buffer << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// Inspects a legacy JWT payload to determine the token's role claim.
pub fn extract_jwt_role(jwt: &str) -> Option<String> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let payload_bytes = decode_base64url(parts[1]).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;
    json.get("role")
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
}

/// Checks whether an HTTP URL points strictly to a local development loopback host.
pub fn is_allowed_localhost_http(url: &str) -> bool {
    let rest = match url.strip_prefix("http://") {
        Some(r) => r,
        None => return false,
    };

    let authority = rest.split(['/', '?', '#']).next().unwrap_or("").trim();

    if authority.is_empty() || authority.contains('@') {
        return false;
    }

    let host = if authority.starts_with('[') {
        if let Some(end_bracket) = authority.find(']') {
            let after_bracket = &authority[end_bracket + 1..];
            if !after_bracket.is_empty() && !after_bracket.starts_with(':') {
                return false;
            }
            &authority[..=end_bracket]
        } else {
            return false;
        }
    } else {
        authority.split(':').next().unwrap_or("")
    };

    matches!(host, "localhost" | "127.0.0.1" | "[::1]")
}

/// Validates that configuration contains only public client parameters and no private secrets.
pub fn validate_config(config: &SupabaseAuthConfig) -> Result<(), AuthError> {
    let trimmed_url = config.url.trim();
    if trimmed_url.is_empty() {
        return Err(AuthError::Unconfigured(
            "Supabase URL cannot be empty".into(),
        ));
    }

    // HTTPS is required for all remote/production endpoints to protect credentials in transit.
    // Insecure HTTP is strictly restricted to local development loopback hosts (localhost, 127.0.0.1, [::1]).
    if trimmed_url.starts_with("https://") {
        // Valid secure HTTPS URL
    } else if is_allowed_localhost_http(trimmed_url) {
        // Valid local development URL
    } else if trimmed_url.starts_with("http://") {
        return Err(AuthError::SecurityViolation(
            "Insecure HTTP Supabase URL is forbidden. Production and remote environments must use HTTPS (HTTP is restricted to localhost)".into(),
        ));
    } else {
        return Err(AuthError::Unconfigured(
            "Supabase URL must start with https:// (or http:// for localhost development only)"
                .into(),
        ));
    }

    let trimmed_key = config.publishable_key.trim();
    if trimmed_key.is_empty() {
        return Err(AuthError::Unconfigured(
            "Supabase publishable key cannot be empty".into(),
        ));
    }

    // Security guardrail: Supabase secret keys (sb_secret_*) must never be used in the client
    if trimmed_key.starts_with("sb_secret_") {
        return Err(AuthError::SecurityViolation(
            "Supabase secret key (sb_secret_*) must never be configured in client application"
                .into(),
        ));
    }

    // Security guardrail: Check legacy JWT keys for service_role entitlement
    if let Some(role) = extract_jwt_role(trimmed_key) {
        if role == "service_role" {
            return Err(AuthError::SecurityViolation(
                "Legacy service_role JWT key must never be configured in client application".into(),
            ));
        }
    }

    // Security guardrail: Generic substring safeguard for service-role/secret markers
    if trimmed_key.to_lowercase().contains("service_role")
        || trimmed_key.to_lowercase().contains("secret_key")
    {
        return Err(AuthError::SecurityViolation(
            "Privileged service-role key or secret must never be configured in client application"
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
    if status_code == 429 {
        return AuthError::RateLimit(
            "Too many authentication requests. Please wait a moment and try again.".into(),
        );
    }

    if status_code >= 500 {
        return AuthError::ServiceUnavailable(
            "Supabase authentication service is currently unavailable. Please try again later."
                .into(),
        );
    }

    let error_body: Result<RawSupabaseErrorResponse, _> = serde_json::from_str(json_str);

    let err_msg = match error_body {
        Ok(err) => err
            .error_description
            .or(err.message)
            .or(err.msg)
            .or(err.error)
            .unwrap_or_else(|| "Authentication request failed".into()),
        Err(_) => {
            return AuthError::InvalidResponse(format!(
                "Authentication service returned unparseable error response (HTTP {status_code})"
            ));
        }
    };

    let lower = err_msg.to_lowercase();

    if status_code == 400 {
        if lower.contains("invalid login credentials")
            || lower.contains("invalid_grant")
            || lower.contains("invalid credentials")
            || lower.contains("email not confirmed")
            || lower.contains("user not found")
        {
            AuthError::InvalidCredentials("Invalid email or password".into())
        } else {
            AuthError::Validation(err_msg)
        }
    } else if status_code == 401 {
        if lower.contains("jwt expired")
            || lower.contains("session expired")
            || lower.contains("token expired")
        {
            AuthError::SessionExpired("Session has expired. Please sign in again.".into())
        } else {
            AuthError::InvalidCredentials(
                "Authentication credentials are invalid or expired".into(),
            )
        }
    } else {
        AuthError::InvalidResponse(format!(
            "Unexpected authentication error (HTTP {status_code}): {err_msg}"
        ))
    }
}
