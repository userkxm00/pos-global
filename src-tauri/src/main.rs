// Tauri entry point. Keep application orchestration here; domain logic belongs
// in commands/services/db modules.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod branch;
mod commands;
mod db;
mod licence;
mod organization;
mod permission;
mod register;
#[cfg(test)]
mod tests;
mod user;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("application data directory must be available");
            std::fs::create_dir_all(&app_data_dir)
                .expect("application data directory must be creatable");
            let db_path = app_data_dir.join("pos-global.sqlite");
            let db =
                db::open_database(&db_path).expect("local database initialization must succeed");
            app.manage(db::DbState(db.into()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_state,
            commands::auth::login,
            commands::auth::verify_pin,
            commands::auth::logout,
            commands::auth::online_login,
            commands::licence::activate_licence,
            commands::licence::check_licence_status,
            commands::sales::create_sale,
            commands::sales::get_sales_report,
            commands::inventory::list_products,
            commands::inventory::upsert_product,
            commands::organization::create_organization,
            commands::organization::get_organization,
            commands::organization::update_organization,
            commands::organization::list_organizations,
            commands::branch::create_branch,
            commands::branch::get_branch,
            commands::branch::update_branch,
            commands::branch::list_branches,
            commands::register::create_register,
            commands::register::get_register,
            commands::register::update_register,
            commands::register::list_registers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
