use tauri::State;

use crate::db::Database;
use crate::models::{DashboardChartPoint, DashboardSummary};

#[tauri::command]
pub fn get_dashboard_summary(db: State<'_, Database>) -> Result<DashboardSummary, String> {
    db.get_dashboard_summary()
}

#[tauri::command]
pub fn get_dashboard_chart(
    db: State<'_, Database>,
    period: String,
) -> Result<Vec<DashboardChartPoint>, String> {
    db.get_dashboard_chart(&period)
}
