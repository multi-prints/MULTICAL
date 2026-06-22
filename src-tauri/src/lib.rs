// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod auth;
mod commands;
mod db;
mod models;

use auth::AuthManager;
use db::Database;
use models::{LocalStorageData, SuccessResponse};
use tauri::{Manager, State};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let auth_manager = AuthManager::new();

    tauri::Builder::default()
        .setup(|app| {
            // Get app data directory for database
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            let db_path = app_data_dir.join("multiprints.db");

            // Initialize database
            let database = Database::new(db_path)
                .expect("Failed to initialize database");

            // Initialize default users
            auth_manager.init_default_users(&database);

            // Store database and auth as managed state
            app.manage(database);
            app.manage(auth_manager);

            println!("MULTIPRINTS Tauri app initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth commands
            commands::auth_cmds::login,
            commands::auth_cmds::logout,
            commands::auth_cmds::validate_session,
            commands::auth_cmds::get_session,
            commands::auth_cmds::add_user,
            commands::auth_cmds::update_password,
            commands::auth_cmds::update_username,
            commands::auth_cmds::get_all_users,
            commands::auth_cmds::delete_user,
            // Product commands
            commands::products::get_all_products,
            commands::products::get_product,
            commands::products::add_product,
            commands::products::update_product,
            commands::products::delete_product,
            // Stock commands
            commands::stock::get_all_stock,
            commands::stock::get_stock,
            commands::stock::get_stock_by_color_size_type,
            commands::stock::add_stock,
            commands::stock::update_stock,
            commands::stock::delete_stock,
            // Sales commands
            commands::sales::get_all_sales,
            commands::sales::get_today_sales,
            commands::sales::add_sale,
            commands::sales::get_today_total_sales,
            commands::sales::update_sale,
            commands::sales::delete_sale,
            // Debt commands
            commands::debts::get_all_debts,
            commands::debts::get_pending_debts,
            commands::debts::add_debt,
            commands::debts::update_debt,
            commands::debts::get_debt_by_sale_id,
            commands::debts::get_debt_by_transaction_id,
            commands::debts::mark_debt_paid,
            commands::debts::delete_debt,
            commands::debts::get_total_outstanding,
            commands::debts::get_paid_this_month,
            commands::debts::get_overdue_debts,
            commands::debts::add_debt_payment,
            commands::debts::get_debt_payments,
            commands::debts::delete_debt_payment,
            // Service commands
            commands::services::get_all_services,
            commands::services::get_active_services,
            commands::services::get_service,
            commands::services::add_service,
            commands::services::update_service,
            commands::services::delete_service,
            // Transaction commands
            commands::transactions::get_all_service_transactions,
            commands::transactions::get_today_service_transactions,
            commands::transactions::add_service_transaction,
            commands::transactions::get_today_total_service_earnings,
            commands::transactions::get_total_service_earnings,
            commands::transactions::update_service_transaction,
            commands::transactions::delete_service_transaction,
            // Material commands
            commands::materials::get_all_printing_materials,
            commands::materials::get_printing_material,
            commands::materials::add_printing_material,
            commands::materials::update_printing_material,
            commands::materials::delete_printing_material,
            // App info
            get_app_version,
            get_platform,
            // Data management
            clear_all_data,
            migrate_from_localstorage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ==================== App Info Commands ====================

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

// ==================== Data Management Commands ====================

#[tauri::command]
fn clear_all_data(db: State<'_, Database>) -> Result<SuccessResponse, String> {
    db.clear_all_data()?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: Some("All data cleared".to_string()),
    })
}

#[tauri::command]
fn migrate_from_localstorage(
    db: State<'_, Database>,
    data: LocalStorageData,
) -> Result<SuccessResponse, String> {
    db.migrate_from_localstorage(data)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: Some("Migration completed".to_string()),
    })
}
