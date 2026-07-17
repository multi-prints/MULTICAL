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
    let _ = dotenvy::dotenv();

    let auth_manager = AuthManager::new();

    tauri::Builder::default()
        .setup(|app| {
            // Get app data directory for database
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            let db_path = app_data_dir.join("multiprints.db");

            // Initialize database (local open is fast; Turso sync runs in background)
            let database = Database::new(db_path).expect("Failed to initialize database");

            // Initialize default users
            auth_manager.init_default_users(&database);

            // Pull/push Turso after the window can show — do not block setup on the network
            database.spawn_background_sync();

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
            // Dashboard commands
            commands::dashboard::get_dashboard_summary,
            commands::dashboard::get_dashboard_chart,
            // Product commands
            commands::products::get_all_products,
            commands::products::get_products_page,
            commands::products::get_product,
            commands::products::add_product,
            commands::products::update_product,
            commands::products::adjust_product_stock,
            commands::products::delete_product,
            // Stock commands
            commands::stock::get_all_stock,
            commands::stock::get_stock_page,
            commands::stock::get_stock,
            commands::stock::get_stock_by_color_size_type,
            commands::stock::add_stock,
            commands::stock::update_stock,
            commands::stock::add_stock_rolls,
            commands::stock::delete_stock,
            // Sales commands
            commands::sales::get_all_sales,
            commands::sales::get_sales_page,
            commands::sales::get_today_sales,
            commands::sales::add_sale,
            commands::sales::get_today_total_sales,
            commands::sales::update_sale,
            commands::sales::delete_sale,
            // Debt commands
            commands::debts::get_all_debts,
            commands::debts::get_debts_page,
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
            commands::transactions::get_printing_page,
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
            commands::materials::add_printing_material_rolls,
            commands::materials::delete_printing_material,
            // App info
            get_app_version,
            get_platform,
            commands::updates::check_for_update,
            commands::updates::check_and_install_update,
            // Data management
            clear_all_data,
            migrate_from_localstorage,
            uninstall_app,
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

#[tauri::command]
fn uninstall_app(app: tauri::AppHandle) -> Result<SuccessResponse, String> {
    // Use compile-time #[cfg] so Unix-only APIs are not type-checked on Windows.
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // NSIS installs uninstall.exe next to the app binary
        let exe_dir = std::env::current_exe()
            .map_err(|e| format!("Cannot locate app binary: {e}"))?
            .parent()
            .ok_or("Invalid exe path")?
            .to_path_buf();

        let uninstaller = exe_dir.join("uninstall.exe");
        if !uninstaller.exists() {
            return Err("Uninstaller not found. Uninstall via Settings > Apps.".into());
        }

        Command::new(&uninstaller)
            .arg("/S")
            .spawn()
            .map_err(|e| format!("Failed to start uninstaller: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        // Cannot dpkg -r while app is running (files locked).
        // Write a detached helper script that waits for this process to exit,
        // then runs pkexec dpkg -r, then deletes itself.
        let helper = r#"#!/bin/bash
set -e
APP_PID="$1"
if [ -n "$APP_PID" ] && kill -0 "$APP_PID" 2>/dev/null; then
    tail --pid="$APP_PID" -f /dev/null
fi
pkexec dpkg -r multiprints
rm -f "$0"
"#;

        let pid = std::process::id();
        let script_path = std::env::temp_dir().join("multiprints-uninstall.sh");
        std::fs::write(&script_path, helper).map_err(|e| e.to_string())?;

        let mut perms = std::fs::metadata(&script_path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).map_err(|e| e.to_string())?;

        // Spawn detached — survives parent exit
        Command::new(&script_path)
            .arg(pid.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start uninstall: {e}"))?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        return Err("Uninstall is not supported on this platform".into());
    }

    app.exit(0);
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: Some("Uninstalling…".to_string()),
    })
}
