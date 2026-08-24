// All Tauri commands exposed to the React UI live under this boundary.
// The frontend must not access SQLite/files directly.

pub mod auth;
pub mod branch;
pub mod inventory;
pub mod licence;
pub mod organization;
pub mod register;
pub mod sales;
