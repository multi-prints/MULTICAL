use tauri::State;

use crate::db::Database;
use crate::models::*;

#[tauri::command]
pub fn get_all_printing_materials(
    db: State<'_, Database>,
) -> Result<Vec<PrintingMaterial>, String> {
    db.get_all_printing_materials()
}

#[tauri::command]
pub fn get_printing_material(
    db: State<'_, Database>,
    id: i64,
) -> Result<Option<PrintingMaterial>, String> {
    db.get_printing_material(id)
}

#[tauri::command]
pub fn add_printing_material(
    db: State<'_, Database>,
    material: NewPrintingMaterial,
) -> Result<PrintingMaterial, String> {
    db.add_printing_material(material)
}

#[tauri::command]
pub fn update_printing_material(
    db: State<'_, Database>,
    id: i64,
    updates: PrintingMaterialUpdate,
) -> Result<SuccessResponse, String> {
    db.update_printing_material(id, updates)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

/// Atomically add rolls to a printing material (and metres via metres_per_roll).
#[tauri::command]
pub fn add_printing_material_rolls(
    db: State<'_, Database>,
    id: i64,
    rolls: i64,
) -> Result<SuccessResponse, String> {
    db.add_printing_material_rolls(id, rolls)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn delete_printing_material(
    db: State<'_, Database>,
    id: i64,
) -> Result<SuccessResponse, String> {
    db.delete_printing_material(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}
