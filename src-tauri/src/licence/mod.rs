// Licence core boundary.
// This module intentionally contains no fake cryptography. The production
// protocol will use a server-side private signing key and a client-side public
// verification key, with explicit offline/replay/clock-rollback rules.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenceStatus {
    pub is_valid: bool,
    pub expires_at: Option<String>,
    pub days_offline_remaining: i32,
    pub license_id: Option<String>,
    pub device_activation_id: Option<String>,
}

pub fn generate_device_fingerprint() -> Result<String, String> {
    Err("Device fingerprint design is not implemented yet".into())
}

pub async fn validate_and_activate(_licence_key: &str) -> Result<LicenceStatus, String> {
    Err("License protocol is not implemented yet".into())
}

pub fn check_local_status() -> LicenceStatus {
    LicenceStatus {
        is_valid: false,
        expires_at: None,
        days_offline_remaining: 0,
        license_id: None,
        device_activation_id: None,
    }
}

/*
SECURITY DESIGN REQUIREMENTS

1. SHA-256 is a hash, not a digital signature.
2. The desktop application must never contain the private signing key.
3. Local license state must be signed by a server-side private key.
4. The client stores/verifies only the public verification key.
5. Device identity must minimize hardware-identifying data.
6. Activation/revocation limits are enforced server-side.
7. Offline grace has explicit limits.
8. Replay and clock rollback attacks need a defined strategy.
9. License responses need schema/versioning.
10. The protocol must be tested before production release.
*/
