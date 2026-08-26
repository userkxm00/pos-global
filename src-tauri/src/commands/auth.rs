// Authentication command boundary.
// Security-sensitive authentication belongs in the auth/domain layer.
// F1.04 — Supabase Auth adapter & F1.05 — Local user/session model

use crate::auth::{
    parse_error_response, parse_token_response, validate_config, validate_credentials, AuthError,
    OnlineSession, RefreshTokenInput, SignInInput, SupabaseAuthConfig,
};
use crate::db::DbState;
use crate::user;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthState {
    pub authenticated: bool,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub branch_id: Option<String>,
    pub role: Option<String>,
    pub organization_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginResult {
    pub success: bool,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub role: Option<String>,
    pub branch_id: Option<String>,
}

/// Status query. Validates an active local session against SQLite database state.
#[tauri::command]
pub fn auth_state(
    state: tauri::State<DbState>,
    session_id: Option<String>,
) -> Result<AuthState, String> {
    let sid = match session_id {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return Ok(AuthState {
                authenticated: false,
                session_id: None,
                user_id: None,
                branch_id: None,
                role: None,
                organization_id: None,
            });
        }
    };

    let conn = state
        .0
        .lock()
        .map_err(|e| format!("Database lock failed: {e}"))?;

    match user::session::validate_local_session(&conn, &sid) {
        Ok(ctx) => Ok(AuthState {
            authenticated: true,
            session_id: Some(ctx.session_id),
            user_id: Some(ctx.user_id),
            branch_id: Some(ctx.branch_id),
            role: Some(ctx.role),
            organization_id: ctx.organization_id,
        }),
        Err(_) => Ok(AuthState {
            authenticated: false,
            session_id: None,
            user_id: None,
            branch_id: None,
            role: None,
            organization_id: None,
        }),
    }
}

/// Performs online account authentication against Supabase Auth.
#[tauri::command]
pub async fn online_login(
    config: SupabaseAuthConfig,
    credentials: SignInInput,
) -> Result<OnlineSession, String> {
    validate_config(&config).map_err(|e| e.to_string())?;
    let (email, password) = validate_credentials(&credentials.email, &credentials.password)
        .map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let base_url = config.url.trim_end_matches('/');
    let token_url = format!("{base_url}/auth/v1/token?grant_type=password");

    let payload = serde_json::json!({
        "email": email,
        "password": password,
    });

    let res = client
        .post(&token_url)
        .header("apikey", &config.publishable_key)
        .header(
            "Authorization",
            format!("Bearer {}", config.publishable_key),
        )
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            AuthError::Network(format!(
                "Unable to reach Supabase authentication service: {e}"
            ))
            .to_string()
        })?;

    let status = res.status().as_u16();
    let body_text = res
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    if status == 200 {
        parse_token_response(&body_text).map_err(|e| e.to_string())
    } else {
        Err(parse_error_response(status, &body_text).to_string())
    }
}

/// Performs online session token refresh against Supabase Auth.
#[tauri::command]
pub async fn refresh_online_session(
    config: SupabaseAuthConfig,
    input: RefreshTokenInput,
) -> Result<OnlineSession, String> {
    validate_config(&config).map_err(|e| e.to_string())?;
    let token =
        crate::auth::validate_refresh_token(&input.refresh_token).map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let base_url = config.url.trim_end_matches('/');
    let token_url = format!("{base_url}/auth/v1/token?grant_type=refresh_token");

    let payload = serde_json::json!({
        "refresh_token": token,
    });

    let res = client
        .post(&token_url)
        .header("apikey", &config.publishable_key)
        .header(
            "Authorization",
            format!("Bearer {}", config.publishable_key),
        )
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            AuthError::Network(format!(
                "Unable to reach Supabase authentication service: {e}"
            ))
            .to_string()
        })?;

    let status = res.status().as_u16();
    let body_text = res
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    if status == 200 {
        parse_token_response(&body_text).map_err(|e| e.to_string())
    } else {
        Err(parse_error_response(status, &body_text).to_string())
    }
}

/// Explicit online account logout and token revocation on Supabase Auth.
#[tauri::command]
pub async fn online_logout(config: SupabaseAuthConfig, access_token: String) -> Result<(), String> {
    validate_config(&config).map_err(|e| e.to_string())?;
    let token = crate::auth::validate_access_token(&access_token).map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let base_url = config.url.trim_end_matches('/');
    let logout_url = format!("{base_url}/auth/v1/logout");

    let res = client
        .post(&logout_url)
        .header("apikey", &config.publishable_key)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| {
            AuthError::Network(format!(
                "Unable to reach Supabase authentication service: {e}"
            ))
            .to_string()
        })?;

    let status = res.status().as_u16();
    if status == 200 || status == 204 {
        Ok(())
    } else {
        let body_text = res.text().await.unwrap_or_default();
        Err(parse_error_response(status, &body_text).to_string())
    }
}

/// Local POS user login with username and password.
#[tauri::command]
pub fn login(
    state: tauri::State<DbState>,
    username: String,
    password: String,
) -> Result<LoginResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("Database lock failed: {e}"))?;

    let user =
        user::verify_user_password(&conn, &username, &password).map_err(|e| e.to_string())?;

    let session =
        user::session::create_local_session(&conn, &user.id, &user.branch_id, "password", None)
            .map_err(|e| e.to_string())?;

    Ok(LoginResult {
        success: true,
        session_id: Some(session.id),
        user_id: Some(user.id),
        role: Some(user.role),
        branch_id: Some(user.branch_id),
    })
}

/// Fast cashier PIN verification for POS terminal operations.
#[tauri::command]
pub fn verify_pin(
    state: tauri::State<DbState>,
    user_id: String,
    pin: String,
) -> Result<LoginResult, String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("Database lock failed: {e}"))?;

    let user = user::verify_user_pin(&conn, &user_id, &pin).map_err(|e| e.to_string())?;

    let session =
        user::session::create_local_session(&conn, &user.id, &user.branch_id, "pin", None)
            .map_err(|e| e.to_string())?;

    Ok(LoginResult {
        success: true,
        session_id: Some(session.id),
        user_id: Some(user.id),
        role: Some(user.role),
        branch_id: Some(user.branch_id),
    })
}

/// Explicit logout / session revocation.
#[tauri::command]
pub fn logout(state: tauri::State<DbState>, session_id: String) -> Result<(), String> {
    let conn = state
        .0
        .lock()
        .map_err(|e| format!("Database lock failed: {e}"))?;

    user::session::revoke_local_session(&conn, &session_id).map_err(|e| e.to_string())
}
