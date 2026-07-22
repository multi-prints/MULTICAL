use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::db::Database;
use crate::models::{BusinessStatement, BusinessStatementRequest, SuccessResponse};
use crate::statement_pdf;

#[tauri::command]
pub fn get_business_statement(
    db: State<'_, Database>,
    request: BusinessStatementRequest,
) -> Result<BusinessStatement, String> {
    db.get_business_statement(&request.source, request.months, request.requested_by)
}

/// Aggregate sales/printing data for the selected period and save a PDF statement.
/// Admin-only in the UI; desktop-only (native save dialog), same pattern as export_database.
#[tauri::command]
pub fn generate_business_statement_pdf(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    request: BusinessStatementRequest,
) -> Result<SuccessResponse, String> {
    let source = request.source.trim().to_lowercase();
    if !matches!(source.as_str(), "sales" | "printing" | "both") {
        return Err("source must be 'sales', 'printing', or 'both'".into());
    }
    if !(1..=6).contains(&request.months) {
        return Err("months must be between 1 and 6".into());
    }

    let statement =
        db.get_business_statement(&source, request.months, request.requested_by.clone())?;

    let pdf_bytes = statement_pdf::render_business_statement_pdf(&statement)?;

    let source_slug = match source.as_str() {
        "sales" => "sales",
        "printing" => "printing",
        _ => "sales-printing",
    };
    let default_name = format!(
        "multiprints-statement-{}-{}mo-{}.pdf",
        source_slug,
        request.months,
        chrono::Local::now().format("%Y-%m-%d")
    );

    let Some(file_path) = app
        .dialog()
        .file()
        .set_file_name(&default_name)
        .add_filter("PDF Document", &["pdf"])
        .set_title("Save business revenue statement")
        .blocking_save_file()
    else {
        return Ok(SuccessResponse {
            success: false,
            error: None,
            message: Some("Export cancelled".into()),
        });
    };

    let path = file_path
        .into_path()
        .map_err(|e| format!("Invalid save path: {e}"))?;

    let path = if path.extension().is_none() {
        path.with_extension("pdf")
    } else {
        path
    };

    std::fs::write(&path, pdf_bytes).map_err(|e| format!("Failed to write PDF: {e}"))?;

    Ok(SuccessResponse {
        success: true,
        error: None,
        message: Some(format!("Statement saved to {}", path.display())),
    })
}
