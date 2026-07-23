use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

use crate::db::Database;
use crate::models::{BusinessStatement, BusinessStatementRequest, SuccessResponse};
use crate::statement_pdf;

/// Async so multi-query aggregation does not run on the UI thread.
#[tauri::command]
pub async fn get_business_statement(
    app: tauri::AppHandle,
    request: BusinessStatementRequest,
) -> Result<BusinessStatement, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Database>();
        db.get_business_statement(&request.source, request.months, request.requested_by)
    })
    .await
    .map_err(|e| format!("Statement query task failed: {e}"))?
}

/// Aggregate sales/printing data for the selected period and save a PDF statement.
///
/// **Async on purpose:** Tauri runs *sync* commands on the main/UI thread. A sync
/// command that calls `blocking_save_file` (or builds a PDF) freezes the window
/// ("Not Responding"). Async commands run off the UI thread; the dialog plugin
/// documents `blocking_*` as the correct API in that context.
#[tauri::command]
pub async fn generate_business_statement_pdf(
    app: tauri::AppHandle,
    request: BusinessStatementRequest,
) -> Result<SuccessResponse, String> {
    let source = request.source.trim().to_lowercase();
    if !matches!(source.as_str(), "sales" | "printing" | "both") {
        return Err("source must be 'sales', 'printing', or 'both'".into());
    }
    if !(1..=6).contains(&request.months) {
        return Err("months must be between 1 and 6".into());
    }

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

    // Ask for the save path first so the user sees the dialog immediately.
    // `blocking_save_file` is safe here because this command is async (not on the UI thread).
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

    let months = request.months;
    let requested_by = request.requested_by.clone();
    let app_for_db = app.clone();

    // DB + PDF can be non-trivial; keep them off the async runtime worker too.
    let pdf_bytes = tauri::async_runtime::spawn_blocking(move || {
        let db = app_for_db.state::<Database>();
        let statement = db.get_business_statement(&source, months, requested_by)?;
        statement_pdf::render_business_statement_pdf(&statement)
    })
    .await
    .map_err(|e| format!("Statement generation task failed: {e}"))??;

    std::fs::write(&path, pdf_bytes).map_err(|e| format!("Failed to write PDF: {e}"))?;

    Ok(SuccessResponse {
        success: true,
        error: None,
        message: Some(format!("Statement saved to {}", path.display())),
    })
}
