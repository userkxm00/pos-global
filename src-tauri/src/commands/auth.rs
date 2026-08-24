// Authentication command boundary.
// Security-sensitive authentication belongs in the auth/domain layer.
// F1.04 — Supabase Auth adapter

use crate::auth::{
    self, parse_error_response, parse_token_response, validate_config, validate_credentials,
    OnlineSession, SignInInput, SupabaseAuthConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthState {
    pub authenticated: bool,
    pub user_id: Option<String>,
    pub branch_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResult {
    pub success: bool,
    pub user_id: Option<String>,
    pub role: Option<String>,
    pub session_id: Option<String>,
}

/// Status query only. Privileged commands must enforce authorization again
/// in the Rust/domain layer.
#[tauri::command]
pub fn auth_state() -> AuthState {
    AuthState {
        authenticated: false,
        user_id: None,
        branch_id: None,
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
        .header("apikey", &config.anon_key)
        .header("Authorization", format!("Bearer {}", config.anon_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            auth::AuthError::Network(format!(
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

/// Placeholder for local user PIN/password authentication (implemented in F1.05).
#[tauri::command]
pub fn login(_username: String, _password: String) -> Result<LoginResult, String> {
    Err("Local POS login is handled by the local session model in F1.05".into())
}

/// Placeholder for fast cashier PIN verification (implemented in F1.05).
#[tauri::command]
pub fn verify_pin(_user_id: String, _pin: String) -> Result<bool, String> {
    Err("PIN verification is handled by the local session model in F1.05".into())
}
