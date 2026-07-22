#![allow(dead_code)]

use crate::remote;
use leptos::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// After a successful write (local or remote), bump the live-refresh epoch.
fn touch_live_data() {
    remote::notify_data_changed();
}

/// Keep entity/FK ids exact across the JS IPC hop (JS numbers are only safe to 2^53-1).
mod lossless_i64 {
    use super::*;

    pub fn serialize<S: Serializer>(value: &i64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = i64;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an i64 as a string or integer")
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<i64, E> {
                Ok(v)
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<i64, E> {
                i64::try_from(v).map_err(E::custom)
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<i64, E> {
                v.parse().map_err(E::custom)
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<i64, E> {
                v.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_any(V)
    }
}

mod lossless_opt_i64 {
    use super::*;

    pub fn serialize<S: Serializer>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => serializer.serialize_some(&v.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<i64>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Opt {
            Str(String),
            I64(i64),
            U64(u64),
        }
        match Option::<Opt>::deserialize(deserializer)? {
            None => Ok(None),
            Some(Opt::Str(s)) if s.is_empty() => Ok(None),
            Some(Opt::Str(s)) => s.parse().map(Some).map_err(serde::de::Error::custom),
            Some(Opt::I64(v)) => Ok(Some(v)),
            Some(Opt::U64(v)) => i64::try_from(v).map(Some).map_err(serde::de::Error::custom),
        }
    }
}

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
    #[serde(with = "lossless_i64")]
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
    #[serde(with = "lossless_i64")]
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
    #[serde(with = "lossless_i64")]
    pub id: i64,
    pub r#type: String,
    #[serde(default, with = "lossless_opt_i64")]
    pub product_id: Option<i64>,
    #[serde(default, with = "lossless_opt_i64")]
    pub stock_id: Option<i64>,
    pub product_name: Option<String>,
    pub product_type: Option<String>,
    pub sticker_type: Option<String>,
    pub quantity: Option<String>,
    pub amount: f64,
    /// Cash collected so far (full amount when not debt).
    #[serde(default)]
    pub amount_paid: f64,
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
    #[serde(default, with = "lossless_opt_i64")]
    pub product_id: Option<i64>,
    #[serde(default, with = "lossless_opt_i64")]
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
    #[serde(with = "lossless_i64")]
    pub id: i64,
    pub customer_name: String,
    pub phone: Option<String>,
    pub amount: f64,
    pub paid_amount: f64,
    pub remaining_amount: f64,
    pub due_date: Option<String>,
    pub description: Option<String>,
    pub status: String,
    #[serde(default, with = "lossless_opt_i64")]
    pub sale_id: Option<i64>,
    #[serde(default, with = "lossless_opt_i64")]
    pub service_transaction_id: Option<i64>,
    pub paid_at: Option<String>,
    pub last_payment_at: Option<String>,
    pub created_at: Option<String>,
    #[serde(default)]
    pub source_label: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_detail: Option<String>,
    #[serde(default)]
    pub source_sale_type: Option<String>,
    #[serde(default)]
    pub source_product_type: Option<String>,
    #[serde(default)]
    pub source_color: Option<String>,
    #[serde(default)]
    pub source_sticker_type: Option<String>,
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
    #[serde(default, with = "lossless_opt_i64")]
    pub sale_id: Option<i64>,
    #[serde(default, with = "lossless_opt_i64")]
    pub service_transaction_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtPayment {
    #[serde(with = "lossless_i64")]
    pub id: i64,
    #[serde(with = "lossless_i64")]
    pub debt_id: i64,
    pub amount: f64,
    pub payment_method: String,
    pub notes: Option<String>,
    pub payment_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDebtPayment {
    #[serde(with = "lossless_i64")]
    pub debt_id: i64,
    pub amount: f64,
    #[serde(default = "default_payment")]
    pub payment_method: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    #[serde(with = "lossless_i64")]
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
    #[serde(with = "lossless_i64")]
    pub id: i64,
    #[serde(default, with = "lossless_opt_i64")]
    pub service_id: Option<i64>,
    pub service_name: String,
    pub quantity: f64,
    pub price: f64,
    pub amount: f64,
    #[serde(default)]
    pub amount_paid: f64,
    pub payment_method: String,
    pub customer_name: String,
    pub notes: Option<String>,
    #[serde(default, with = "lossless_opt_i64")]
    pub stock_id: Option<i64>,
    pub stock_metres_used: f64,
    pub material_size: Option<String>,
    pub material_type: Option<String>,
    #[serde(default, with = "lossless_opt_i64")]
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
    #[serde(default, with = "lossless_opt_i64")]
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
    #[serde(default, with = "lossless_opt_i64")]
    pub stock_id: Option<i64>,
    #[serde(default)]
    pub stock_metres_used: f64,
    pub material_size: Option<String>,
    pub material_type: Option<String>,
    #[serde(default, with = "lossless_opt_i64")]
    pub printing_material_id: Option<i64>,
    #[serde(default)]
    pub is_debt: i64,
}

fn default_f64_one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintingMaterial {
    #[serde(with = "lossless_i64")]
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
    #[serde(with = "lossless_i64")]
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
    #[serde(default = "default_true")]
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

fn default_true() -> bool {
    true
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
    #[serde(default, with = "lossless_opt_i64")]
    pub product_id: Option<i64>,
    pub name: String,
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardChartPoint {
    pub label: String,
    /// Revenue (product sales + printing services) for the bucket.
    pub amount: f64,
    /// Number of sales + printing jobs in the bucket.
    #[serde(default)]
    pub sales_count: f64,
    /// Remaining amount of pending debts created in the bucket.
    #[serde(default)]
    pub debt_amount: f64,
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
    ($name:ident, $cmd:literal, $arg_name:ident: i64, $ret:ty) => {
        pub async fn $name($arg_name: i64) -> Result<$ret, String> {
            // Send ids as strings — JS Number cannot hold full i64 distributed ids.
            tauri_invoke_inner(
                $cmd,
                &serde_json::json!({ stringify!($arg_name): $arg_name.to_string() }),
            )
            .await
        }
    };
    ($name:ident, $cmd:literal, $arg_name:ident: $arg_ty:ty, $ret:ty) => {
        pub async fn $name($arg_name: $arg_ty) -> Result<$ret, String> {
            tauri_invoke_inner($cmd, &serde_json::json!({ stringify!($arg_name): $arg_name })).await
        }
    };
}

pub async fn get_dashboard_summary() -> Result<DashboardSummary, String> {
    remote::prefer_remote_then_local(
        "get_dashboard_summary",
        async { remote::get_json("/v1/dashboard/summary").await },
        async { tauri_invoke_inner("get_dashboard_summary", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_all_products() -> Result<Vec<Product>, String> {
    remote::prefer_remote_then_local(
        "get_all_products",
        async {
            let page: ProductsPageData = remote::get_json("/v1/products?per_page=500").await?;
            Ok(page.items)
        },
        async { tauri_invoke_inner("get_all_products", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_product(id: i64) -> Result<Option<Product>, String> {
    remote::prefer_remote_then_local(
        "get_product",
        async {
            match remote::get_json::<Product>(&format!("/v1/products/{id}")).await {
                Ok(p) => Ok(Some(p)),
                Err(e) if e.contains("404") || e.contains("Not found") => Ok(None),
                Err(e) => Err(e),
            }
        },
        async {
            tauri_invoke_inner("get_product", &serde_json::json!({ "id": id.to_string() })).await
        },
    )
    .await
}

pub async fn add_product(product: &NewProduct) -> Result<Product, String> {
    let r = remote::prefer_remote_then_local(
        "add_product",
        async { remote::post_json::<_, Product>("/v1/products", product).await },
        async {
            tauri_invoke_inner("add_product", &serde_json::json!({ "product": product })).await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn delete_product(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_product",
        async { remote::delete_json::<SuccessResponse>(&format!("/v1/products/{id}")).await },
        async {
            tauri_invoke_inner(
                "delete_product",
                &serde_json::json!({ "id": id.to_string() }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_all_stock() -> Result<Vec<StockItem>, String> {
    remote::prefer_remote_then_local(
        "get_all_stock",
        async {
            let page: StockPageData = remote::get_json("/v1/stock?per_page=500").await?;
            Ok(page.items)
        },
        async { tauri_invoke_inner("get_all_stock", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_stock(id: i64) -> Result<Option<StockItem>, String> {
    remote::prefer_remote_then_local(
        "get_stock",
        async {
            let page: StockPageData = remote::get_json("/v1/stock?per_page=500").await?;
            Ok(page.items.into_iter().find(|s| s.id == id))
        },
        async {
            tauri_invoke_inner("get_stock", &serde_json::json!({ "id": id.to_string() })).await
        },
    )
    .await
}

pub async fn add_stock(item: &NewStockItem) -> Result<StockItem, String> {
    let r = remote::prefer_remote_then_local(
        "add_stock",
        async {
            let body = serde_json::json!({
                "color": item.color,
                "size": item.size,
                "sticker_type": item.sticker_type,
                "rolls": item.rolls,
                "metres_per_roll": item.metres_per_roll.unwrap_or(50.0),
            });
            remote::post_json::<_, StockItem>("/v1/stock", &body).await
        },
        async { tauri_invoke_inner("add_stock", &serde_json::json!({ "item": item })).await },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn delete_stock(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_stock",
        async { remote::delete_json::<SuccessResponse>(&format!("/v1/stock/{id}")).await },
        async {
            tauri_invoke_inner("delete_stock", &serde_json::json!({ "id": id.to_string() })).await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_all_sales() -> Result<Vec<Sale>, String> {
    remote::prefer_remote_then_local(
        "get_all_sales",
        async {
            let page: SalesPageData = remote::get_json("/v1/sales?per_page=500").await?;
            Ok(page.items)
        },
        async { tauri_invoke_inner("get_all_sales", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_today_sales() -> Result<Vec<Sale>, String> {
    remote::prefer_remote_then_local(
        "get_today_sales",
        async {
            let page: SalesPageData = remote::get_json("/v1/sales?per_page=200").await?;
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            Ok(page
                .items
                .into_iter()
                .filter(|s| {
                    s.timestamp
                        .as_ref()
                        .map(|ts| ts.starts_with(&today) || ts.contains(&today))
                        .unwrap_or(false)
                })
                .collect())
        },
        async { tauri_invoke_inner("get_today_sales", &serde_json::json!({})).await },
    )
    .await
}

pub async fn add_sale(sale: &NewSale) -> Result<Sale, String> {
    let r = remote::prefer_remote_then_local(
        "add_sale",
        async {
            let body = serde_json::json!({
                "type": sale.r#type,
                "product_id": sale.product_id,
                "stock_id": sale.stock_id,
                "product_name": sale.product_name,
                "product_type": sale.product_type,
                "sticker_type": sale.sticker_type,
                "quantity": sale.quantity,
                "amount": sale.amount,
                "payment_method": sale.payment_method,
                "customer_name": sale.customer_name,
                "is_debt": sale.is_debt,
                "stock_metres_used": sale.stock_metres_used,
            });
            remote::post_json::<_, Sale>("/v1/sales", &body).await
        },
        async { tauri_invoke_inner("add_sale", &serde_json::json!({ "sale": sale })).await },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

api_fn!(get_today_total_sales, "get_today_total_sales", f64);

pub async fn delete_sale(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_sale",
        async { remote::delete_json::<SuccessResponse>(&format!("/v1/sales/{id}")).await },
        async {
            tauri_invoke_inner("delete_sale", &serde_json::json!({ "id": id.to_string() })).await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}
#[derive(Debug, Clone, Deserialize)]
struct ItemsDebt {
    items: Vec<Debt>,
}
#[derive(Debug, Clone, Deserialize)]
struct ItemsPayment {
    items: Vec<DebtPayment>,
}
#[derive(Debug, Clone, Deserialize)]
struct ItemsService {
    items: Vec<Service>,
}
#[derive(Debug, Clone, Deserialize)]
struct ItemsUser {
    items: Vec<User>,
}
#[derive(Debug, Clone, Deserialize)]
struct MetricsDebt {
    total_outstanding: f64,
    paid_this_month: f64,
    #[serde(default)]
    overdue_count: i64,
    #[serde(default)]
    overdue_total: f64,
}

/// Worker notification feed (`GET /v1/notifications` or `/v1/debts/overdue`).
#[derive(Debug, Clone, Deserialize)]
struct NotificationsFeed {
    items: Vec<Debt>,
    #[serde(default)]
    overdue_count: i64,
    #[serde(default)]
    overdue_total: f64,
}

pub async fn get_all_debts() -> Result<Vec<Debt>, String> {
    remote::prefer_remote_then_local(
        "get_all_debts",
        async {
            let p: DebtsPageData = remote::get_json("/v1/debts?per_page=500").await?;
            Ok(p.all_debts)
        },
        async { tauri_invoke_inner("get_all_debts", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_pending_debts() -> Result<Vec<Debt>, String> {
    remote::prefer_remote_then_local(
        "get_pending_debts",
        async {
            let r: ItemsDebt = remote::get_json("/v1/debts/pending").await?;
            Ok(r.items)
        },
        async { tauri_invoke_inner("get_pending_debts", &serde_json::json!({})).await },
    )
    .await
}

pub async fn add_debt(debt: &NewDebt) -> Result<Debt, String> {
    let r = remote::prefer_remote_then_local(
        "add_debt",
        async { remote::post_json::<_, Debt>("/v1/debts", debt).await },
        async { tauri_invoke_inner("add_debt", &serde_json::json!({ "debt": debt })).await },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn mark_debt_paid(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "mark_debt_paid",
        async {
            remote::post_json::<_, SuccessResponse>(
                &format!("/v1/debts/{id}/mark-paid"),
                &serde_json::json!({}),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "mark_debt_paid",
                &serde_json::json!({ "id": id.to_string() }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn delete_debt(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_debt",
        async { remote::delete_json::<SuccessResponse>(&format!("/v1/debts/{id}")).await },
        async {
            tauri_invoke_inner("delete_debt", &serde_json::json!({ "id": id.to_string() })).await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_total_outstanding() -> Result<f64, String> {
    remote::prefer_remote_then_local(
        "get_total_outstanding",
        async {
            let m: MetricsDebt = remote::get_json("/v1/debts/metrics").await?;
            Ok(m.total_outstanding)
        },
        async { tauri_invoke_inner("get_total_outstanding", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_paid_this_month() -> Result<f64, String> {
    remote::prefer_remote_then_local(
        "get_paid_this_month",
        async {
            let m: MetricsDebt = remote::get_json("/v1/debts/metrics").await?;
            Ok(m.paid_this_month)
        },
        async { tauri_invoke_inner("get_paid_this_month", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_overdue_debts() -> Result<Vec<Debt>, String> {
    remote::prefer_remote_then_local(
        "get_overdue_debts",
        async {
            // Prefer dedicated notifications feed; fall back to debts/overdue.
            if let Ok(r) = remote::get_json::<NotificationsFeed>("/v1/notifications").await {
                return Ok(r.items);
            }
            let r: NotificationsFeed = remote::get_json("/v1/debts/overdue").await?;
            Ok(r.items)
        },
        async { tauri_invoke_inner("get_overdue_debts", &serde_json::json!({})).await },
    )
    .await
}

pub async fn add_debt_payment(payment: &NewDebtPayment) -> Result<DebtPayment, String> {
    let r = remote::prefer_remote_then_local(
        "add_debt_payment",
        async {
            remote::post_json::<_, DebtPayment>(
                &format!("/v1/debts/{}/payments", payment.debt_id),
                &serde_json::json!({
                    "amount": payment.amount,
                    "payment_method": payment.payment_method,
                    "notes": payment.notes,
                }),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "add_debt_payment",
                &serde_json::json!({ "payment": payment }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_debt_payments(debt_id: i64) -> Result<Vec<DebtPayment>, String> {
    remote::prefer_remote_then_local(
        "get_debt_payments",
        async {
            let r: ItemsPayment =
                remote::get_json(&format!("/v1/debts/{debt_id}/payments")).await?;
            Ok(r.items)
        },
        async {
            tauri_invoke_inner(
                "get_debt_payments",
                &serde_json::json!({ "debtId": debt_id.to_string() }),
            )
            .await
        },
    )
    .await
}

pub async fn delete_debt_payment(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_debt_payment",
        async { remote::delete_json::<SuccessResponse>(&format!("/v1/debts/payments/{id}")).await },
        async {
            tauri_invoke_inner(
                "delete_debt_payment",
                &serde_json::json!({ "id": id.to_string() }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_all_services() -> Result<Vec<Service>, String> {
    remote::prefer_remote_then_local(
        "get_all_services",
        async {
            let r: ItemsService = remote::get_json("/v1/services").await?;
            Ok(r.items)
        },
        async { tauri_invoke_inner("get_all_services", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_active_services() -> Result<Vec<Service>, String> {
    remote::prefer_remote_then_local(
        "get_active_services",
        async {
            let r: ItemsService = remote::get_json("/v1/services?active=1").await?;
            Ok(r.items)
        },
        async { tauri_invoke_inner("get_active_services", &serde_json::json!({})).await },
    )
    .await
}

pub async fn add_service(service: &NewService) -> Result<Service, String> {
    let r = remote::prefer_remote_then_local(
        "add_service",
        async { remote::post_json::<_, Service>("/v1/services", service).await },
        async {
            tauri_invoke_inner("add_service", &serde_json::json!({ "service": service })).await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn delete_service(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_service",
        async { remote::delete_json::<SuccessResponse>(&format!("/v1/services/{id}")).await },
        async {
            tauri_invoke_inner(
                "delete_service",
                &serde_json::json!({ "id": id.to_string() }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_all_service_transactions() -> Result<Vec<ServiceTransaction>, String> {
    remote::prefer_remote_then_local(
        "get_all_service_transactions",
        async {
            let p: PrintingPageData = remote::get_json("/v1/printing/jobs?per_page=500").await?;
            Ok(p.items)
        },
        async { tauri_invoke_inner("get_all_service_transactions", &serde_json::json!({})).await },
    )
    .await
}

pub async fn get_today_service_transactions() -> Result<Vec<ServiceTransaction>, String> {
    // Filter client-side from remote page; local uses dedicated command.
    remote::prefer_remote_then_local(
        "get_today_service_transactions",
        async {
            let p: PrintingPageData = remote::get_json("/v1/printing/jobs?per_page=200").await?;
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            Ok(p.items
                .into_iter()
                .filter(|t| {
                    t.timestamp
                        .as_ref()
                        .map(|ts| ts.starts_with(&today))
                        .unwrap_or(false)
                })
                .collect())
        },
        async {
            tauri_invoke_inner("get_today_service_transactions", &serde_json::json!({})).await
        },
    )
    .await
}
pub async fn add_service_transaction(
    transaction: &NewServiceTransaction,
) -> Result<ServiceTransaction, String> {
    let r = remote::prefer_remote_then_local(
        "add_service_transaction",
        async {
            let body = serde_json::json!({
                "service_name": transaction.service_name,
                "quantity": transaction.quantity,
                "price": transaction.price,
                "amount": transaction.amount,
                "payment_method": transaction.payment_method,
                "customer_name": transaction.customer_name,
                "notes": transaction.notes,
                "printing_material_id": transaction.printing_material_id,
                "stock_metres_used": transaction.stock_metres_used,
                "material_size": transaction.material_size,
                "material_type": transaction.material_type,
                "is_debt": transaction.is_debt,
            });
            remote::post_json::<_, ServiceTransaction>("/v1/printing/jobs", &body).await
        },
        async {
            tauri_invoke_inner(
                "add_service_transaction",
                &serde_json::json!({ "transaction": transaction }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}
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
pub async fn delete_service_transaction(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_service_transaction",
        async { remote::delete_json::<SuccessResponse>(&format!("/v1/printing/jobs/{id}")).await },
        async {
            tauri_invoke_inner(
                "delete_service_transaction",
                &serde_json::json!({ "id": id.to_string() }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

#[derive(Debug, Clone, Deserialize)]
struct MaterialsListResponse {
    items: Vec<PrintingMaterial>,
}

pub async fn get_all_printing_materials() -> Result<Vec<PrintingMaterial>, String> {
    remote::prefer_remote_then_local(
        "get_all_printing_materials",
        async {
            let r: MaterialsListResponse = remote::get_json("/v1/materials").await?;
            Ok(r.items)
        },
        async { tauri_invoke_inner("get_all_printing_materials", &serde_json::json!({})).await },
    )
    .await
}

api_fn!(get_printing_material, "get_printing_material", id: i64, Option<PrintingMaterial>);

pub async fn add_printing_material(
    material: &NewPrintingMaterial,
) -> Result<PrintingMaterial, String> {
    let r = remote::prefer_remote_then_local(
        "add_printing_material",
        async {
            let body = serde_json::json!({
                "name": material.name,
                "material_type": material.material_type,
                "width": material.width,
                "rolls": material.rolls,
                "metres_per_roll": material.metres_per_roll,
                "color": material.color,
            });
            remote::post_json::<_, PrintingMaterial>("/v1/materials", &body).await
        },
        async {
            tauri_invoke_inner(
                "add_printing_material",
                &serde_json::json!({ "material": material }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn delete_printing_material(id: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_printing_material",
        async { remote::delete_json::<SuccessResponse>(&format!("/v1/materials/{id}")).await },
        async {
            tauri_invoke_inner(
                "delete_printing_material",
                &serde_json::json!({ "id": id.to_string() }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}
pub async fn get_all_users() -> Result<Vec<User>, String> {
    remote::prefer_remote_then_local(
        "get_all_users",
        async {
            let r: ItemsUser = remote::get_json("/v1/users").await?;
            Ok(r.items)
        },
        async { tauri_invoke_inner("get_all_users", &serde_json::json!({})).await },
    )
    .await
}

pub async fn delete_user(username: String) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "delete_user",
        async {
            let enc = urlencoding_lite(&username);
            remote::delete_json::<SuccessResponse>(&format!("/v1/users/{enc}")).await
        },
        async {
            tauri_invoke_inner("delete_user", &serde_json::json!({ "username": username })).await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

// Device/local only — not multi-PC business data
api_fn!(clear_all_data, "clear_all_data", SuccessResponse);
api_fn!(export_database, "export_database", SuccessResponse);
api_fn!(import_database, "import_database", SuccessResponse);
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
    remote::prefer_remote_then_local(
        "get_dashboard_chart",
        async {
            let q = urlencoding_lite(period);
            remote::get_json(&format!("/v1/dashboard/chart?period={q}")).await
        },
        async {
            tauri_invoke_inner(
                "get_dashboard_chart",
                &serde_json::json!({ "period": period }),
            )
            .await
        },
    )
    .await
}

pub async fn login(username: &str, password: &str) -> Result<LoginResponse, String> {
    remote::prefer_remote_then_local(
        "login",
        async {
            remote::post_json::<_, LoginResponse>(
                "/v1/auth/login",
                &serde_json::json!({ "username": username, "password": password }),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "login",
                &serde_json::json!({ "username": username, "password": password }),
            )
            .await
        },
    )
    .await
}

pub async fn validate_session(token: &str) -> Result<bool, String> {
    remote::prefer_remote_then_local(
        "validate_session",
        async {
            remote::post_json::<_, bool>(
                "/v1/auth/validate",
                &serde_json::json!({ "token": token }),
            )
            .await
        },
        async {
            tauri_invoke_inner("validate_session", &serde_json::json!({ "token": token })).await
        },
    )
    .await
}

pub async fn logout(token: &str) -> Result<SuccessResponse, String> {
    remote::prefer_remote_then_local(
        "logout",
        async {
            remote::post_json::<_, SuccessResponse>(
                "/v1/auth/logout",
                &serde_json::json!({ "token": token }),
            )
            .await
        },
        async { tauri_invoke_inner("logout", &serde_json::json!({ "token": token })).await },
    )
    .await
}

pub async fn get_session(token: &str) -> Result<SessionResponse, String> {
    remote::prefer_remote_then_local(
        "get_session",
        async {
            remote::post_json::<_, SessionResponse>(
                "/v1/auth/session",
                &serde_json::json!({ "token": token }),
            )
            .await
        },
        async { tauri_invoke_inner("get_session", &serde_json::json!({ "token": token })).await },
    )
    .await
}

pub async fn add_user(
    username: &str,
    password: &str,
    role: &str,
) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "add_user",
        async {
            remote::post_json::<_, SuccessResponse>(
                "/v1/users",
                &serde_json::json!({
                    "username": username,
                    "password": password,
                    "role": role
                }),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "add_user",
                &serde_json::json!({ "username": username, "password": password, "role": role }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_products_page(query: &ProductsPageQuery) -> Result<ProductsPageData, String> {
    remote::prefer_remote_then_local(
        "get_products_page",
        async {
            let page = query.page.unwrap_or(1);
            let per = query.per_page.unwrap_or(50);
            remote::get_json(&format!("/v1/products?page={page}&per_page={per}")).await
        },
        async {
            tauri_invoke_inner("get_products_page", &serde_json::json!({ "query": query })).await
        },
    )
    .await
}

pub async fn update_product(id: i64, updates: &ProductUpdate) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "update_product",
        async {
            remote::patch_json::<_, SuccessResponse>(&format!("/v1/products/{id}"), updates).await
        },
        async {
            tauri_invoke_inner(
                "update_product",
                &serde_json::json!({ "id": id, "updates": updates }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

/// Relative product stock change — safe when multiple PCs update the same item.
pub async fn adjust_product_stock(id: i64, delta: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "adjust_product_stock",
        async {
            remote::post_json::<_, SuccessResponse>(
                &format!("/v1/products/{id}/adjust-stock"),
                &serde_json::json!({ "delta": delta }),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "adjust_product_stock",
                &serde_json::json!({ "id": id, "delta": delta }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
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
    remote::prefer_remote_then_local(
        "get_stock_page",
        async {
            let page = query.page.unwrap_or(1);
            let per = query.per_page.unwrap_or(50);
            remote::get_json(&format!("/v1/stock?page={page}&per_page={per}")).await
        },
        async {
            tauri_invoke_inner("get_stock_page", &serde_json::json!({ "query": query })).await
        },
    )
    .await
}

pub async fn update_stock(id: i64, updates: &serde_json::Value) -> Result<SuccessResponse, String> {
    // Stock updates still local-only on Worker (no PATCH yet); keep Tauri path.
    let r = tauri_invoke_inner(
        "update_stock",
        &serde_json::json!({ "id": id, "updates": updates }),
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

/// Atomically add rolls to a stock item (safe across concurrent PCs).
pub async fn add_stock_rolls(id: i64, rolls: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "add_stock_rolls",
        async {
            remote::post_json::<_, SuccessResponse>(
                &format!("/v1/stock/{id}/add-rolls"),
                &serde_json::json!({ "rolls": rolls }),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "add_stock_rolls",
                &serde_json::json!({ "id": id, "rolls": rolls }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_sales_page(query: &SalesPageQuery) -> Result<SalesPageData, String> {
    remote::prefer_remote_then_local(
        "get_sales_page",
        async {
            let page = query.page.unwrap_or(1);
            let per = query.per_page.unwrap_or(50);
            let search = query.search.clone().unwrap_or_default();
            let q = urlencoding_lite(&search);
            let raw: serde_json::Value =
                remote::get_json(&format!("/v1/sales?page={page}&per_page={per}&search={q}"))
                    .await?;
            // Worker returns a subset of metrics; fill the rest for UI compatibility.
            Ok(SalesPageData {
                items: serde_json::from_value(raw.get("items").cloned().unwrap_or_default())
                    .unwrap_or_default(),
                total_count: raw.get("total_count").and_then(|v| v.as_i64()).unwrap_or(0),
                today_total: raw
                    .get("today_total")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                all_revenue: raw
                    .get("all_revenue")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                product_sales_count: raw
                    .get("product_sales_count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
            })
        },
        async {
            tauri_invoke_inner("get_sales_page", &serde_json::json!({ "query": query })).await
        },
    )
    .await
}

fn urlencoding_lite(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "%20".into(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

pub async fn update_sale(id: i64, updates: &serde_json::Value) -> Result<SuccessResponse, String> {
    // When remote is on, never fall back to local Tauri for this path — local
    // update_sale after convert was freezing Windows while the write had already
    // succeeded on Turso.
    if remote::is_enabled() {
        let r = remote::patch_json::<_, SuccessResponse>(&format!("/v1/sales/{id}"), updates).await;
        if r.is_ok() {
            touch_live_data();
        }
        return r;
    }
    tauri_invoke_inner(
        "update_sale",
        &serde_json::json!({ "id": id.to_string(), "updates": updates }),
    )
    .await
}

pub async fn get_debts_page(query: &DebtsPageQuery) -> Result<DebtsPageData, String> {
    remote::prefer_remote_then_local(
        "get_debts_page",
        async {
            let page = query.page.unwrap_or(1);
            let per = query.per_page.unwrap_or(50);
            let search = query.search.clone().unwrap_or_default();
            let sort = query.sort_by.clone().unwrap_or_else(|| "newest".into());
            let q = urlencoding_lite(&search);
            let s = urlencoding_lite(&sort);
            remote::get_json(&format!(
                "/v1/debts?page={page}&per_page={per}&search={q}&sort_by={s}"
            ))
            .await
        },
        async {
            tauri_invoke_inner("get_debts_page", &serde_json::json!({ "query": query })).await
        },
    )
    .await
}

pub async fn update_debt(id: i64, updates: &serde_json::Value) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "update_debt",
        async {
            remote::patch_json::<_, SuccessResponse>(&format!("/v1/debts/{id}"), updates).await
        },
        async {
            tauri_invoke_inner(
                "update_debt",
                &serde_json::json!({ "id": id, "updates": updates }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_debt_by_sale_id(sale_id: i64) -> Result<Option<Debt>, String> {
    remote::prefer_remote_then_local(
        "get_debt_by_sale_id",
        async {
            let v: Option<Debt> = remote::get_json(&format!("/v1/debts/by-sale/{sale_id}")).await?;
            Ok(v)
        },
        async {
            tauri_invoke_inner(
                "get_debt_by_sale_id",
                &serde_json::json!({ "saleId": sale_id }),
            )
            .await
        },
    )
    .await
}

pub async fn get_debt_by_transaction_id(transaction_id: i64) -> Result<Option<Debt>, String> {
    remote::prefer_remote_then_local(
        "get_debt_by_transaction_id",
        async {
            let v: Option<Debt> =
                remote::get_json(&format!("/v1/debts/by-transaction/{transaction_id}")).await?;
            Ok(v)
        },
        async {
            tauri_invoke_inner(
                "get_debt_by_transaction_id",
                &serde_json::json!({ "transactionId": transaction_id }),
            )
            .await
        },
    )
    .await
}

pub async fn update_service(
    id: i64,
    updates: &serde_json::Value,
) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "update_service",
        async {
            remote::patch_json::<_, SuccessResponse>(&format!("/v1/services/{id}"), updates).await
        },
        async {
            tauri_invoke_inner(
                "update_service",
                &serde_json::json!({ "id": id, "updates": updates }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn get_service(id: i64) -> Result<Option<Service>, String> {
    remote::prefer_remote_then_local(
        "get_service",
        async {
            match remote::get_json::<Service>(&format!("/v1/services/{id}")).await {
                Ok(s) => Ok(Some(s)),
                Err(e) if e.contains("null") || e.contains("404") => Ok(None),
                Err(e) => Err(e),
            }
        },
        async { tauri_invoke_inner("get_service", &serde_json::json!({ "id": id })).await },
    )
    .await
}

pub async fn get_printing_page(query: &PrintingPageQuery) -> Result<PrintingPageData, String> {
    remote::prefer_remote_then_local(
        "get_printing_page",
        async {
            let page = query.page.unwrap_or(1);
            let per = query.per_page.unwrap_or(50);
            let search = query.search.clone().unwrap_or_default();
            let q = urlencoding_lite(&search);
            remote::get_json(&format!(
                "/v1/printing/jobs?page={page}&per_page={per}&search={q}"
            ))
            .await
        },
        async {
            tauri_invoke_inner("get_printing_page", &serde_json::json!({ "query": query })).await
        },
    )
    .await
}

pub async fn update_service_transaction(
    id: i64,
    updates: &serde_json::Value,
) -> Result<SuccessResponse, String> {
    // Same as update_sale: remote-only when Worker is enabled (no local hang).
    if remote::is_enabled() {
        let r =
            remote::patch_json::<_, SuccessResponse>(&format!("/v1/printing/jobs/{id}"), updates)
                .await;
        if r.is_ok() {
            touch_live_data();
        }
        return r;
    }
    tauri_invoke_inner(
        "update_service_transaction",
        &serde_json::json!({ "id": id.to_string(), "updates": updates }),
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

/// Atomically add rolls to a printing material (safe across concurrent PCs).
pub async fn add_printing_material_rolls(id: i64, rolls: i64) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "add_printing_material_rolls",
        async {
            remote::post_json::<_, SuccessResponse>(
                &format!("/v1/materials/{id}/add-rolls"),
                &serde_json::json!({ "rolls": rolls }),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "add_printing_material_rolls",
                &serde_json::json!({ "id": id, "rolls": rolls }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn update_password(
    username: &str,
    old_password: &str,
    new_password: &str,
) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "update_password",
        async {
            remote::post_json::<_, SuccessResponse>(
                "/v1/users/update-password",
                &serde_json::json!({
                    "username": username,
                    "old_password": old_password,
                    "new_password": new_password
                }),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "update_password",
                &serde_json::json!({
                    "username": username,
                    "oldPassword": old_password,
                    "newPassword": new_password
                }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
}

pub async fn update_username(
    old_username: &str,
    new_username: &str,
) -> Result<SuccessResponse, String> {
    let r = remote::prefer_remote_then_local(
        "update_username",
        async {
            remote::post_json::<_, SuccessResponse>(
                "/v1/users/update-username",
                &serde_json::json!({
                    "old_username": old_username,
                    "new_username": new_username
                }),
            )
            .await
        },
        async {
            tauri_invoke_inner(
                "update_username",
                &serde_json::json!({
                    "oldUsername": old_username,
                    "newUsername": new_username
                }),
            )
            .await
        },
    )
    .await;
    if r.is_ok() {
        touch_live_data();
    }
    r
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
