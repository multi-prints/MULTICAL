#![allow(dead_code)]

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

// ==================== Tauri Invoke via window.tauriInvoke ====================
// Uses js_sys + JsFuture for reliable Promise handling

async fn tauri_invoke_inner<T: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &impl Serialize,
) -> Result<T, String> {
    let args_json = serde_json::to_string(args).map_err(|e| e.to_string())?;

    // Get window.tauriInvoke as a JS function
    let global = js_sys::global();
    let fn_val = js_sys::Reflect::get(&global, &JsValue::from_str("tauriInvoke"))
        .map_err(|_| "window.tauriInvoke not found".to_string())?;
    let f: js_sys::Function = fn_val
        .dyn_into()
        .map_err(|_| "window.tauriInvoke is not a function".to_string())?;

    // Call the function — returns a Promise
    let prom = f
        .call2(
            &JsValue::UNDEFINED,
            &JsValue::from_str(cmd),
            &JsValue::from_str(&args_json),
        )
        .map_err(|e| format!("tauriInvoke call failed: {:?}", e))?;
    let promise: js_sys::Promise = prom
        .dyn_into()
        .map_err(|_| "tauriInvoke did not return a Promise".to_string())?;

    // Await the Promise
    let result = JsFuture::from(promise).await.map_err(|e| {
        e.as_string()
            .unwrap_or_else(|| format!("IPC rejected for '{}'", cmd))
    })?;

    let raw_str = result
        .as_string()
        .ok_or_else(|| format!("IPC response not a string for '{}'", cmd))?;

    // Parse the wrapper: { ok: ... } or { err: ... }
    let envelope: serde_json::Value = serde_json::from_str(&raw_str).map_err(|e| {
        format!(
            "Failed to parse IPC response for '{}': {} - raw: {}",
            cmd,
            e,
            &raw_str[..raw_str.len().min(200)]
        )
    })?;
    if let Some(err) = envelope.get("err").and_then(|v| v.as_str()) {
        return Err(err.to_string());
    }
    let ok_val = envelope
        .get("ok")
        .ok_or_else(|| format!("No 'ok' field in IPC response for '{}': {}", cmd, raw_str))?;
    serde_json::from_value(ok_val.clone()).map_err(|e| {
        format!(
            "Deserialize error for '{}': {} - raw: {}",
            cmd,
            e,
            &raw_str[..raw_str.len().min(200)]
        )
    })
}

// ==================== Models ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub product_type: String,
    pub color: Option<String>,
    pub size: Option<String>,
    pub selling_price: f64,
    pub stock: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductsPageQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductsPageData {
    pub items: Vec<Product>,
    pub total_count: i64,
    pub total_stock_units: i64,
    pub life_saver_stock: i64,
    pub chevron_stock: i64,
    pub stripes_stock: i64,
    pub stock_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProduct {
    pub name: String,
    pub product_type: String,
    pub color: Option<String>,
    pub size: Option<String>,
    pub selling_price: f64,
    pub stock: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selling_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockItem {
    pub id: i64,
    pub color: String,
    pub size: String,
    pub sticker_type: String,
    pub rolls: i64,
    pub metres_per_roll: f64,
    pub total_metres: f64,
    pub metres_used: f64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockPageQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockPageData {
    pub items: Vec<StockItem>,
    pub total_count: i64,
    pub total_rolls: i64,
    pub total_metres: f64,
    pub remaining_metres: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewStockItem {
    pub color: String,
    #[serde(default = "default_size")]
    pub size: String,
    #[serde(default = "default_sticker_type")]
    pub sticker_type: String,
    pub rolls: i64,
    pub metres_per_roll: Option<f64>,
    pub total_metres: Option<f64>,
    #[serde(default)]
    pub metres_used: f64,
    pub custom_metres_per_roll: Option<f64>,
}

fn default_size() -> String {
    "1".into()
}
fn default_sticker_type() -> String {
    "colored".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sale {
    pub id: i64,
    pub r#type: String,
    pub product_id: Option<i64>,
    pub stock_id: Option<i64>,
    pub product_name: Option<String>,
    pub product_type: Option<String>,
    pub sticker_type: Option<String>,
    pub quantity: Option<String>,
    pub amount: f64,
    pub payment_method: String,
    pub customer_name: String,
    pub is_debt: i64,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesPageQuery {
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesPageData {
    pub items: Vec<Sale>,
    pub total_count: i64,
    pub today_total: f64,
    pub all_revenue: f64,
    pub product_sales_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSale {
    pub r#type: String,
    pub product_id: Option<i64>,
    pub stock_id: Option<i64>,
    pub product_name: Option<String>,
    pub product_type: Option<String>,
    pub sticker_type: Option<String>,
    pub quantity: Option<String>,
    pub amount: f64,
    #[serde(default = "default_payment")]
    pub payment_method: String,
    #[serde(default = "default_customer")]
    pub customer_name: String,
    #[serde(default)]
    pub is_debt: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_quantity: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_metres_used: Option<f64>,
}

fn default_payment() -> String {
    "cash".into()
}
fn default_customer() -> String {
    "Walk-in".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Debt {
    pub id: i64,
    pub customer_name: String,
    pub phone: Option<String>,
    pub amount: f64,
    pub paid_amount: f64,
    pub remaining_amount: f64,
    pub due_date: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub sale_id: Option<i64>,
    pub service_transaction_id: Option<i64>,
    pub paid_at: Option<String>,
    pub last_payment_at: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtsPageQuery {
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtsPageData {
    pub items: Vec<Debt>,
    pub total_count: i64,
    pub total_outstanding: f64,
    pub paid_this_month: f64,
    pub overdue_count: i64,
    pub all_debts: Vec<Debt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDebt {
    pub customer_name: String,
    pub phone: Option<String>,
    pub amount: f64,
    pub paid_amount: Option<f64>,
    pub remaining_amount: Option<f64>,
    pub due_date: Option<String>,
    pub description: Option<String>,
    pub sale_id: Option<i64>,
    pub service_transaction_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtPayment {
    pub id: i64,
    pub debt_id: i64,
    pub amount: f64,
    pub payment_method: String,
    pub notes: Option<String>,
    pub payment_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDebtPayment {
    pub debt_id: i64,
    pub amount: f64,
    #[serde(default = "default_payment")]
    pub payment_method: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub unit: Option<String>,
    pub uses_stock: i64,
    pub is_active: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewService {
    pub name: String,
    pub description: Option<String>,
    pub price: Option<f64>,
    pub unit: Option<String>,
    #[serde(default = "default_one")]
    pub is_active: i64,
}

fn default_one() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTransaction {
    pub id: i64,
    pub service_id: Option<i64>,
    pub service_name: String,
    pub quantity: f64,
    pub price: f64,
    pub amount: f64,
    pub payment_method: String,
    pub customer_name: String,
    pub notes: Option<String>,
    pub stock_id: Option<i64>,
    pub stock_metres_used: f64,
    pub material_size: Option<String>,
    pub material_type: Option<String>,
    pub printing_material_id: Option<i64>,
    pub is_debt: i64,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintingPageQuery {
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintingPageData {
    pub items: Vec<ServiceTransaction>,
    pub total_count: i64,
    pub today_earnings: f64,
    pub total_jobs_count: i64,
    pub material_used: f64,
    pub total_revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewServiceTransaction {
    pub service_id: Option<i64>,
    pub service_name: String,
    #[serde(default = "default_f64_one")]
    pub quantity: f64,
    pub price: Option<f64>,
    pub amount: Option<f64>,
    #[serde(default = "default_payment")]
    pub payment_method: String,
    #[serde(default = "default_customer")]
    pub customer_name: String,
    pub notes: Option<String>,
    pub stock_id: Option<i64>,
    #[serde(default)]
    pub stock_metres_used: f64,
    pub material_size: Option<String>,
    pub material_type: Option<String>,
    pub printing_material_id: Option<i64>,
    #[serde(default)]
    pub is_debt: i64,
}

fn default_f64_one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintingMaterial {
    pub id: i64,
    pub name: String,
    pub material_type: String,
    pub width: f64,
    pub rolls: i64,
    pub metres_per_roll: f64,
    pub total_metres: f64,
    pub metres_used: f64,
    pub color: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPrintingMaterial {
    pub name: String,
    pub material_type: String,
    #[serde(default)]
    pub width: f64,
    pub rolls: i64,
    #[serde(default = "default_50")]
    pub metres_per_roll: f64,
    pub total_metres: Option<f64>,
    #[serde(default)]
    pub metres_used: f64,
    pub color: Option<String>,
}

fn default_50() -> f64 {
    50.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub user: Option<UserInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserInfo {
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub error: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub available: bool,
    pub version: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub success: bool,
    pub session: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardRecentTransaction {
    pub name: String,
    pub date: String,
    pub amount: f64,
    pub is_debt: bool,
    pub type_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardActivityItem {
    pub item_type: String,
    pub text: String,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTopProduct {
    pub product_id: Option<i64>,
    pub name: String,
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardChartPoint {
    pub label: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total_revenue: f64,
    pub today_sales_count: i64,
    pub today_revenue: f64,
    pub outstanding_debts: f64,
    pub pending_debts_count: i64,
    pub recent_transactions: Vec<DashboardRecentTransaction>,
    pub activity_items: Vec<DashboardActivityItem>,
    pub top_products: Vec<DashboardTopProduct>,
}

// ==================== API Functions ====================

macro_rules! api_fn {
    ($name:ident, $cmd:literal, $ret:ty) => {
        pub async fn $name() -> Result<$ret, String> {
            tauri_invoke_inner($cmd, &serde_json::json!({})).await
        }
    };
    ($name:ident, $cmd:literal, $arg_name:ident: $arg_ty:ty, $ret:ty) => {
        pub async fn $name($arg_name: $arg_ty) -> Result<$ret, String> {
            tauri_invoke_inner($cmd, &serde_json::json!({ stringify!($arg_name): $arg_name })).await
        }
    };
}

api_fn!(
    get_dashboard_summary,
    "get_dashboard_summary",
    DashboardSummary
);
api_fn!(get_all_products, "get_all_products", Vec<Product>);
api_fn!(get_product, "get_product", id: i64, Option<Product>);
api_fn!(add_product, "add_product", product: &NewProduct, Product);
api_fn!(delete_product, "delete_product", id: i64, SuccessResponse);
api_fn!(get_all_stock, "get_all_stock", Vec<StockItem>);
api_fn!(get_stock, "get_stock", id: i64, Option<StockItem>);
api_fn!(add_stock, "add_stock", item: &NewStockItem, StockItem);
api_fn!(delete_stock, "delete_stock", id: i64, SuccessResponse);
api_fn!(get_all_sales, "get_all_sales", Vec<Sale>);
api_fn!(get_today_sales, "get_today_sales", Vec<Sale>);
api_fn!(add_sale, "add_sale", sale: &NewSale, Sale);
api_fn!(get_today_total_sales, "get_today_total_sales", f64);
api_fn!(delete_sale, "delete_sale", id: i64, SuccessResponse);
api_fn!(get_all_debts, "get_all_debts", Vec<Debt>);
api_fn!(get_pending_debts, "get_pending_debts", Vec<Debt>);
api_fn!(add_debt, "add_debt", debt: &NewDebt, Debt);
api_fn!(mark_debt_paid, "mark_debt_paid", id: i64, SuccessResponse);
api_fn!(delete_debt, "delete_debt", id: i64, SuccessResponse);
api_fn!(get_total_outstanding, "get_total_outstanding", f64);
api_fn!(get_paid_this_month, "get_paid_this_month", f64);
api_fn!(get_overdue_debts, "get_overdue_debts", Vec<Debt>);
api_fn!(add_debt_payment, "add_debt_payment", payment: &NewDebtPayment, DebtPayment);
pub async fn get_debt_payments(debt_id: i64) -> Result<Vec<DebtPayment>, String> {
    tauri_invoke_inner(
        "get_debt_payments",
        &serde_json::json!({ "debtId": debt_id }),
    )
    .await
}
api_fn!(delete_debt_payment, "delete_debt_payment", id: i64, SuccessResponse);
api_fn!(get_all_services, "get_all_services", Vec<Service>);
api_fn!(get_active_services, "get_active_services", Vec<Service>);
api_fn!(add_service, "add_service", service: &NewService, Service);
api_fn!(delete_service, "delete_service", id: i64, SuccessResponse);
api_fn!(
    get_all_service_transactions,
    "get_all_service_transactions",
    Vec<ServiceTransaction>
);
api_fn!(
    get_today_service_transactions,
    "get_today_service_transactions",
    Vec<ServiceTransaction>
);
api_fn!(add_service_transaction, "add_service_transaction", transaction: &NewServiceTransaction, ServiceTransaction);
api_fn!(
    get_today_total_service_earnings,
    "get_today_total_service_earnings",
    f64
);
api_fn!(
    get_total_service_earnings,
    "get_total_service_earnings",
    f64
);
api_fn!(delete_service_transaction, "delete_service_transaction", id: i64, SuccessResponse);
api_fn!(
    get_all_printing_materials,
    "get_all_printing_materials",
    Vec<PrintingMaterial>
);
api_fn!(get_printing_material, "get_printing_material", id: i64, Option<PrintingMaterial>);
api_fn!(add_printing_material, "add_printing_material", material: &NewPrintingMaterial, PrintingMaterial);
api_fn!(delete_printing_material, "delete_printing_material", id: i64, SuccessResponse);
api_fn!(get_all_users, "get_all_users", Vec<User>);
api_fn!(delete_user, "delete_user", username: String, SuccessResponse);
api_fn!(clear_all_data, "clear_all_data", SuccessResponse);
api_fn!(get_app_version, "get_app_version", String);
api_fn!(get_platform, "get_platform", String);
api_fn!(check_for_update, "check_for_update", UpdateResult);
api_fn!(
    check_and_install_update,
    "check_and_install_update",
    UpdateResult
);
api_fn!(uninstall_app, "uninstall_app", SuccessResponse);

pub async fn get_dashboard_chart(period: &str) -> Result<Vec<DashboardChartPoint>, String> {
    tauri_invoke_inner(
        "get_dashboard_chart",
        &serde_json::json!({ "period": period }),
    )
    .await
}

pub async fn login(username: &str, password: &str) -> Result<LoginResponse, String> {
    tauri_invoke_inner(
        "login",
        &serde_json::json!({ "username": username, "password": password }),
    )
    .await
}

pub async fn validate_session(token: &str) -> Result<bool, String> {
    tauri_invoke_inner("validate_session", &serde_json::json!({ "token": token })).await
}

pub async fn logout(token: &str) -> Result<SuccessResponse, String> {
    tauri_invoke_inner("logout", &serde_json::json!({ "token": token })).await
}

pub async fn get_session(token: &str) -> Result<SessionResponse, String> {
    tauri_invoke_inner("get_session", &serde_json::json!({ "token": token })).await
}

pub async fn add_user(
    username: &str,
    password: &str,
    role: &str,
) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "add_user",
        &serde_json::json!({ "username": username, "password": password, "role": role }),
    )
    .await
}

pub async fn get_products_page(query: &ProductsPageQuery) -> Result<ProductsPageData, String> {
    tauri_invoke_inner("get_products_page", &serde_json::json!({ "query": query })).await
}

pub async fn update_product(id: i64, updates: &ProductUpdate) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "update_product",
        &serde_json::json!({ "id": id, "updates": updates }),
    )
    .await
}

pub async fn get_stock_by_color_size_type(
    color: &str,
    size: &str,
    sticker_type: &str,
) -> Result<Option<StockItem>, String> {
    tauri_invoke_inner(
        "get_stock_by_color_size_type",
        &serde_json::json!({ "color": color, "size": size, "stickerType": sticker_type }),
    )
    .await
}

pub async fn get_stock_page(query: &StockPageQuery) -> Result<StockPageData, String> {
    tauri_invoke_inner("get_stock_page", &serde_json::json!({ "query": query })).await
}

pub async fn update_stock(id: i64, updates: &serde_json::Value) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "update_stock",
        &serde_json::json!({ "id": id, "updates": updates }),
    )
    .await
}

pub async fn get_sales_page(query: &SalesPageQuery) -> Result<SalesPageData, String> {
    tauri_invoke_inner("get_sales_page", &serde_json::json!({ "query": query })).await
}

pub async fn update_sale(id: i64, updates: &serde_json::Value) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "update_sale",
        &serde_json::json!({ "id": id, "updates": updates }),
    )
    .await
}

pub async fn get_debts_page(query: &DebtsPageQuery) -> Result<DebtsPageData, String> {
    tauri_invoke_inner("get_debts_page", &serde_json::json!({ "query": query })).await
}

pub async fn update_debt(id: i64, updates: &serde_json::Value) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "update_debt",
        &serde_json::json!({ "id": id, "updates": updates }),
    )
    .await
}

pub async fn get_debt_by_sale_id(sale_id: i64) -> Result<Option<Debt>, String> {
    tauri_invoke_inner(
        "get_debt_by_sale_id",
        &serde_json::json!({ "saleId": sale_id }),
    )
    .await
}

pub async fn get_debt_by_transaction_id(transaction_id: i64) -> Result<Option<Debt>, String> {
    tauri_invoke_inner(
        "get_debt_by_transaction_id",
        &serde_json::json!({ "transactionId": transaction_id }),
    )
    .await
}

pub async fn update_service(
    id: i64,
    updates: &serde_json::Value,
) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "update_service",
        &serde_json::json!({ "id": id, "updates": updates }),
    )
    .await
}

pub async fn get_service(id: i64) -> Result<Option<Service>, String> {
    tauri_invoke_inner("get_service", &serde_json::json!({ "id": id })).await
}

pub async fn get_printing_page(query: &PrintingPageQuery) -> Result<PrintingPageData, String> {
    tauri_invoke_inner("get_printing_page", &serde_json::json!({ "query": query })).await
}

pub async fn update_service_transaction(
    id: i64,
    updates: &serde_json::Value,
) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "update_service_transaction",
        &serde_json::json!({ "id": id, "updates": updates }),
    )
    .await
}

pub async fn update_printing_material(
    id: i64,
    updates: &serde_json::Value,
) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "update_printing_material",
        &serde_json::json!({ "id": id, "updates": updates }),
    )
    .await
}

pub async fn update_password(
    username: &str,
    old_password: &str,
    new_password: &str,
) -> Result<SuccessResponse, String> {
    tauri_invoke_inner("update_password", &serde_json::json!({ "username": username, "oldPassword": old_password, "newPassword": new_password })).await
}

pub async fn update_username(
    old_username: &str,
    new_username: &str,
) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "update_username",
        &serde_json::json!({ "oldUsername": old_username, "newUsername": new_username }),
    )
    .await
}

pub async fn migrate_from_localstorage(
    data: &serde_json::Value,
) -> Result<SuccessResponse, String> {
    tauri_invoke_inner(
        "migrate_from_localstorage",
        &serde_json::json!({ "data": data }),
    )
    .await
}
