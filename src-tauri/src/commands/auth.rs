use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuthState {
    pub authenticated: bool,
    pub user_id: Option<String>,
    pub branch_id: Option<String>,
}

/// UI must treat this as a status query only. Privileged operations must
/// perform permission checks again in the Rust/domain layer.
#[tauri::command]
pub fn auth_state() -> AuthState {
    AuthState {
        authenticated: false,
        user_id: None,
        branch_id: None,
    }
}
