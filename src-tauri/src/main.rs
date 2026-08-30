// Tauri entry point. Keep application orchestration here; domain logic belongs
// in commands/services/db modules.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod barcode;
mod branch;
mod brand;
mod category;
mod commands;
mod db;
mod licence;
mod manufacturer;
mod organization;
mod permission;
mod product;
mod register;
#[cfg(test)]
mod tests;
mod unit;
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
            commands::auth::refresh_online_session,
            commands::auth::online_logout,
            commands::licence::activate_licence,
            commands::licence::check_licence_status,
            commands::sales::create_sale,
            commands::sales::get_sales_report,
            commands::inventory::list_products,
            commands::inventory::upsert_product,
            commands::product::create_product,
            commands::product::get_product,
            commands::product::update_product,
            commands::product::delete_product,
            commands::product::list_catalog_products,
            commands::barcode::get_product_by_barcode,
            commands::barcode::add_product_barcode,
            commands::barcode::remove_product_barcode,
            commands::barcode::set_primary_barcode,
            commands::barcode::reassign_product_barcode,
            commands::barcode::list_product_barcodes,
            commands::barcode::validate_barcode_string,
            commands::barcode::generate_internal_barcode,
            commands::barcode::generate_product_sku,
            commands::barcode::verify_catalog_barcode_integrity,
            commands::barcode::reconcile_catalog_barcode_mirrors,
            commands::category::create_category,
            commands::category::get_category,
            commands::category::update_category,
            commands::category::delete_category,
            commands::category::list_categories,
            commands::category::get_category_tree,
            commands::brand::create_brand,
            commands::brand::get_brand,
            commands::brand::update_brand,
            commands::brand::delete_brand,
            commands::brand::list_brands,
            commands::manufacturer::create_manufacturer,
            commands::manufacturer::get_manufacturer,
            commands::manufacturer::update_manufacturer,
            commands::manufacturer::delete_manufacturer,
            commands::manufacturer::list_manufacturers,
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
            commands::unit::create_unit,
            commands::unit::get_unit,
            commands::unit::get_unit_by_code,
            commands::unit::list_units,
            commands::unit::update_unit,
            commands::unit::delete_unit,
            commands::unit::create_unit_conversion,
            commands::unit::delete_unit_conversion,
            commands::unit::list_unit_conversions,
            commands::unit::convert_quantity,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
