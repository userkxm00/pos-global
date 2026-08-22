// Authentication command boundary.
// Security-sensitive authentication belongs in the auth/domain layer.
// These commands are intentionally explicit stubs until the approved
// Supabase/local-session authentication contract is implemented.

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

#[tauri::command]
pub fn login(_username: String, _password: String) -> Result<LoginResult, String> {
    Err("Authentication service is not implemented yet".into())
}

#[tauri::command]
pub fn verify_pin(_user_id: String, _pin: String) -> Result<bool, String> {
    Err("PIN verification service is not implemented yet".into())
}
