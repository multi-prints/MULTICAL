use tauri::State;

use crate::db::Database;
use crate::models::*;

#[tauri::command]
pub fn get_all_stock(db: State<'_, Database>) -> Result<Vec<StockItem>, String> {
    db.get_all_stock()
}

#[tauri::command]
pub fn get_stock(db: State<'_, Database>, id: i64) -> Result<Option<StockItem>, String> {
    db.get_stock(id)
}

#[tauri::command]
pub fn get_stock_by_color_size_type(
    db: State<'_, Database>,
    color: String,
    size: String,
    sticker_type: String,
) -> Result<Option<StockItem>, String> {
    db.get_stock_by_color_size_type(&color, &size, &sticker_type)
}

#[tauri::command]
pub fn add_stock(db: State<'_, Database>, stock_item: NewStockItem) -> Result<StockItem, String> {
    db.add_stock(stock_item)
}

#[tauri::command]
pub fn update_stock(
    db: State<'_, Database>,
    id: i64,
    updates: StockUpdate,
) -> Result<SuccessResponse, String> {
    db.update_stock(id, updates)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn delete_stock(db: State<'_, Database>, id: i64) -> Result<SuccessResponse, String> {
    db.delete_stock(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}
