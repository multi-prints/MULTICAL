use tauri::State;

use crate::db::Database;
use crate::models::{IdArg, *};

#[tauri::command]
pub fn get_all_debts(db: State<'_, Database>) -> Result<Vec<Debt>, String> {
    db.get_all_debts()
}

#[tauri::command]
pub fn get_debts_page(
    db: State<'_, Database>,
    query: DebtsPageQuery,
) -> Result<DebtsPageData, String> {
    db.get_debts_page(query)
}

#[tauri::command]
pub fn get_pending_debts(db: State<'_, Database>) -> Result<Vec<Debt>, String> {
    db.get_pending_debts()
}

#[tauri::command]
pub fn add_debt(db: State<'_, Database>, debt: NewDebt) -> Result<Debt, String> {
    db.add_debt(debt)
}

#[tauri::command]
pub fn update_debt(
    db: State<'_, Database>,
    id: IdArg,
    updates: DebtUpdate,
) -> Result<SuccessResponse, String> {
    let id = id.0;

    db.update_debt(id, updates)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn get_debt_by_sale_id(
    db: State<'_, Database>,
    sale_id: IdArg,
) -> Result<Option<Debt>, String> {
    let sale_id = sale_id.0;
    db.get_debt_by_sale_id(sale_id)
}

#[tauri::command]
pub fn get_debt_by_transaction_id(
    db: State<'_, Database>,
    transaction_id: IdArg,
) -> Result<Option<Debt>, String> {
    let transaction_id = transaction_id.0;

    db.get_debt_by_transaction_id(transaction_id)
}

#[tauri::command]
pub fn mark_debt_paid(db: State<'_, Database>, id: IdArg) -> Result<SuccessResponse, String> {
    let id = id.0;
    db.mark_debt_paid(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn delete_debt(db: State<'_, Database>, id: IdArg) -> Result<SuccessResponse, String> {
    let id = id.0;
    db.delete_debt(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}

#[tauri::command]
pub fn get_total_outstanding(db: State<'_, Database>) -> Result<f64, String> {
    db.get_total_outstanding()
}

#[tauri::command]
pub fn get_paid_this_month(db: State<'_, Database>) -> Result<f64, String> {
    db.get_paid_this_month()
}

#[tauri::command]
pub fn get_overdue_debts(db: State<'_, Database>) -> Result<Vec<Debt>, String> {
    db.get_overdue_debts()
}

// Debt Payments

#[tauri::command]
pub fn add_debt_payment(
    db: State<'_, Database>,
    payment: NewDebtPayment,
) -> Result<DebtPayment, String> {
    db.add_debt_payment(payment)
}

#[tauri::command]
pub fn get_debt_payments(
    db: State<'_, Database>,
    debt_id: IdArg,
) -> Result<Vec<DebtPayment>, String> {
    let debt_id = debt_id.0;

    db.get_debt_payments(debt_id)
}

#[tauri::command]
pub fn delete_debt_payment(db: State<'_, Database>, id: IdArg) -> Result<SuccessResponse, String> {
    let id = id.0;
    db.delete_debt_payment(id)?;
    Ok(SuccessResponse {
        success: true,
        error: None,
        message: None,
    })
}
