// Licence command boundary. Cryptographic verification and activation stay
// isolated in licence/mod.rs and must follow the approved licensing contract.

use crate::licence::{self, LicenceStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivationResult {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub async fn activate_licence(_licence_key: String) -> Result<ActivationResult, String> {
    let _ = &licence::validate_and_activate;
    Err("License activation is not implemented yet; security design gate required".into())
}

#[tauri::command]
pub async fn check_licence_status() -> Result<LicenceStatus, String> {
    let _ = &licence::check_local_status;
    Err("License status verification is not implemented yet".into())
}
