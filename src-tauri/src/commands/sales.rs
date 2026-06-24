use tauri::State;

use crate::db::Database;
use crate::models::*;

#[tauri::command]
pub fn get_all_sales(db: State<'_, Database>) -> Result<Vec<Sale>, String> {
    db.get_all_sales()
}

#[tauri::command]
pub fn get_sales_page(
    db: State<'_, Database>,
    query: SalesPageQuery,
) -> Result<SalesPageData, String> {
    db.get_sales_page(query)
}

#[tauri::command]
pub fn get_today_sales(db: State<'_, Database>) -> Result<Vec<Sale>, String> {
    db.get_today_sales()
}

#[tauri::command]
pub fn add_sale(db: State<'_, Database>, sale: NewSale) -> Result<Sale, String> {
    db.add_sale(sale)
}

#[tauri::command]
pub fn get_today_total_sales(db: State<'_, Database>) -> Result<f64, String> {
    db.get_today_total_sales()
}

#[tauri::command]
pub fn update_sale(
    db: State<'_, Database>,
    id: i64,
    updates: SaleUpdate,
) -> Result<SuccessResponse, String> {
    db.update_sale(id, updates)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn delete_sale(db: State<'_, Database>, id: i64) -> Result<SuccessResponse, String> {
    db.delete_sale(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}
