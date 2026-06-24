use tauri::State;

use crate::db::Database;
use crate::models::*;

#[tauri::command]
pub fn get_all_products(db: State<'_, Database>) -> Result<Vec<Product>, String> {
    db.get_all_products()
}

#[tauri::command]
pub fn get_products_page(
    db: State<'_, Database>,
    query: ProductsPageQuery,
) -> Result<ProductsPageData, String> {
    db.get_products_page(query)
}

#[tauri::command]
pub fn get_product(db: State<'_, Database>, id: i64) -> Result<Option<Product>, String> {
    db.get_product(id)
}

#[tauri::command]
pub fn add_product(db: State<'_, Database>, product: NewProduct) -> Result<Product, String> {
    db.add_product(product)
}

#[tauri::command]
pub fn update_product(
    db: State<'_, Database>,
    id: i64,
    updates: ProductUpdate,
) -> Result<SuccessResponse, String> {
    db.update_product(id, updates)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn delete_product(db: State<'_, Database>, id: i64) -> Result<SuccessResponse, String> {
    db.delete_product(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}
