use tauri::State;

use crate::db::Database;
use crate::models::*;

#[tauri::command]
pub fn get_all_service_transactions(
    db: State<'_, Database>,
) -> Result<Vec<ServiceTransaction>, String> {
    db.get_all_service_transactions()
}

#[tauri::command]
pub fn get_printing_page(
    db: State<'_, Database>,
    query: PrintingPageQuery,
) -> Result<PrintingPageData, String> {
    db.get_printing_page(query)
}

#[tauri::command]
pub fn get_today_service_transactions(
    db: State<'_, Database>,
) -> Result<Vec<ServiceTransaction>, String> {
    db.get_today_service_transactions()
}

#[tauri::command]
pub fn add_service_transaction(
    db: State<'_, Database>,
    transaction: NewServiceTransaction,
) -> Result<ServiceTransaction, String> {
    db.add_service_transaction(transaction)
}

#[tauri::command]
pub fn get_today_total_service_earnings(db: State<'_, Database>) -> Result<f64, String> {
    db.get_today_total_service_earnings()
}

#[tauri::command]
pub fn get_total_service_earnings(db: State<'_, Database>) -> Result<f64, String> {
    db.get_total_service_earnings()
}

#[tauri::command]
pub fn update_service_transaction(
    db: State<'_, Database>,
    id: i64,
    updates: ServiceTransactionUpdate,
) -> Result<SuccessResponse, String> {
    db.update_service_transaction(id, updates)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn delete_service_transaction(
    db: State<'_, Database>,
    id: i64,
) -> Result<SuccessResponse, String> {
    db.delete_service_transaction(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}
