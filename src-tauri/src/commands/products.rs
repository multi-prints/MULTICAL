use tauri::State;

use crate::db::Database;
use crate::models::{IdArg, *};

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
pub fn get_product(db: State<'_, Database>, id: IdArg) -> Result<Option<Product>, String> {
    let id = id.0;
    db.get_product(id)
}

#[tauri::command]
pub fn add_product(db: State<'_, Database>, product: NewProduct) -> Result<Product, String> {
    db.add_product(product)
}

#[tauri::command]
pub fn update_product(
    db: State<'_, Database>,
    id: IdArg,
    updates: ProductUpdate,
) -> Result<SuccessResponse, String> {
    let id = id.0;

    db.update_product(id, updates)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

/// Relative stock change (e.g. +5 or -2). Safe for concurrent multi-PC use.
#[tauri::command]
pub fn adjust_product_stock(
    db: State<'_, Database>,
    id: IdArg,
    delta: i64,
) -> Result<SuccessResponse, String> {
    let id = id.0;

    db.adjust_product_stock(id, delta)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn delete_product(db: State<'_, Database>, id: IdArg) -> Result<SuccessResponse, String> {
    let id = id.0;
    db.delete_product(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}
