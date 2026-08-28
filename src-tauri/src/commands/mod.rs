// All Tauri commands exposed to the React UI live under this boundary.
// The frontend must not access SQLite/files directly.

pub mod auth;
pub mod branch;
pub mod brand;
pub mod category;
pub mod inventory;
pub mod licence;
pub mod manufacturer;
pub mod organization;
pub mod product;
pub mod register;
pub mod sales;
