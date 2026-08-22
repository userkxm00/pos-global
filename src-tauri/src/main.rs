// Tauri entry point. Keep application orchestration here; domain logic belongs
// in commands/services/db modules.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod licence;
#[cfg(test)]
mod tests;

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
            app.manage(db::DbState(db));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_state,
            commands::auth::login,
            commands::auth::verify_pin,
            commands::licence::activate_licence,
            commands::licence::check_licence_status,
            commands::sales::create_sale,
            commands::sales::get_sales_report,
            commands::inventory::list_products,
            commands::inventory::upsert_product,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
