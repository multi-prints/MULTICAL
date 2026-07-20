use tauri::State;

use crate::db::Database;
use crate::models::{IdArg, *};

#[tauri::command]
pub fn get_all_services(db: State<'_, Database>) -> Result<Vec<Service>, String> {
    db.get_all_services()
}

#[tauri::command]
pub fn get_active_services(db: State<'_, Database>) -> Result<Vec<Service>, String> {
    db.get_active_services()
}

#[tauri::command]
pub fn get_service(db: State<'_, Database>, id: IdArg) -> Result<Option<Service>, String> {
    let id = id.0;
    db.get_service(id)
}

#[tauri::command]
pub fn add_service(db: State<'_, Database>, service: NewService) -> Result<Service, String> {
    db.add_service(service)
}

#[tauri::command]
pub fn update_service(
    db: State<'_, Database>,
    id: IdArg,
    updates: ServiceUpdate,
) -> Result<SuccessResponse, String> {
    let id = id.0;

    db.update_service(id, updates)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn delete_service(db: State<'_, Database>, id: IdArg) -> Result<SuccessResponse, String> {
    let id = id.0;
    db.delete_service(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}
