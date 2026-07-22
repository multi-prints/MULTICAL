use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Serialize entity / FK ids as JSON strings so the webview (JS IEEE-754 numbers)
/// cannot round them. Distributed i64 ids exceed Number.MAX_SAFE_INTEGER.
pub mod lossless_i64 {
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

pub mod lossless_opt_i64 {
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

/// Command-argument id that accepts JSON string or number (lossless with strings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdArg(pub i64);

impl From<IdArg> for i64 {
    fn from(v: IdArg) -> Self {
        v.0
    }
}

impl<'de> Deserialize<'de> for IdArg {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        lossless_i64::deserialize(deserializer).map(IdArg)
    }
}

impl Serialize for IdArg {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        lossless_i64::serialize(&self.0, serializer)
    }
}

// ==================== Product ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub name: Option<String>,
    pub product_type: Option<String>,
    pub color: Option<String>,
    pub size: Option<String>,
    pub selling_price: Option<f64>,
    pub stock: Option<i64>,
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

// ==================== Stock ====================

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
    "1".to_string()
}

fn default_sticker_type() -> String {
    "colored".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockUpdate {
    pub color: Option<String>,
    pub size: Option<String>,
    pub sticker_type: Option<String>,
    pub rolls: Option<i64>,
    pub metres_per_roll: Option<f64>,
    pub total_metres: Option<f64>,
    pub metres_used: Option<f64>,
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

// ==================== Sale ====================

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
    /// Cash collected so far: full `amount` when not a debt; debt `paid_amount` when converted.
    #[serde(default)]
    pub amount_paid: f64,
    pub payment_method: String,
    pub customer_name: String,
    pub is_debt: i64,
    pub timestamp: Option<String>,
    /// Logged-in username who recorded the sale (employee / admin).
    #[serde(default)]
    pub created_by: Option<String>,
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
    #[serde(default = "default_payment_method")]
    pub payment_method: String,
    #[serde(default = "default_customer_name")]
    pub customer_name: String,
    #[serde(default)]
    pub is_debt: i64,
    #[serde(default)]
    pub product_quantity: Option<i64>,
    #[serde(default)]
    pub stock_metres_used: Option<f64>,
    /// Username of the logged-in staff who recorded this sale.
    #[serde(default)]
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleUpdate {
    pub r#type: Option<String>,
    pub amount: Option<f64>,
    pub payment_method: Option<String>,
    pub customer_name: Option<String>,
    pub is_debt: Option<i64>,
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

// ==================== Debt ====================

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
    /// Human label for linked sale product / printing job / manual note.
    #[serde(default)]
    pub source_label: Option<String>,
    /// "sale" | "printing" | "manual"
    #[serde(default)]
    pub source_kind: Option<String>,
    /// Extra context (qty, metres, material, etc.).
    #[serde(default)]
    pub source_detail: Option<String>,
    /// Linked sale type: product | stock | service
    #[serde(default)]
    pub source_sale_type: Option<String>,
    /// life_saver | chevron | stripes
    #[serde(default)]
    pub source_product_type: Option<String>,
    /// Product/stock color key for preview swatches
    #[serde(default)]
    pub source_color: Option<String>,
    /// colored | reflective (stickers)
    #[serde(default)]
    pub source_sticker_type: Option<String>,
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
pub struct DebtUpdate {
    pub customer_name: Option<String>,
    pub phone: Option<String>,
    pub amount: Option<f64>,
    pub paid_amount: Option<f64>,
    pub remaining_amount: Option<f64>,
    pub due_date: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

// ==================== Debt Payment ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtPayment {
    #[serde(with = "lossless_i64")]
    pub id: i64,
    #[serde(with = "lossless_i64", alias = "debtId")]
    pub debt_id: i64,
    pub amount: f64,
    pub payment_method: String,
    pub notes: Option<String>,
    pub payment_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDebtPayment {
    #[serde(with = "lossless_i64", alias = "debtId")]
    pub debt_id: i64,
    pub amount: f64,
    #[serde(default = "default_payment_method")]
    pub payment_method: String,
    pub notes: Option<String>,
}

// ==================== Service ====================

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
pub struct ServiceUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<f64>,
    pub unit: Option<String>,
    pub is_active: Option<i64>,
}

// ==================== Service Transaction ====================

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
    /// Cash collected so far (full amount when not debt; debt payments when converted).
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
    /// Logged-in username who recorded the printing job.
    #[serde(default)]
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewServiceTransaction {
    #[serde(default, with = "lossless_opt_i64")]
    pub service_id: Option<i64>,
    pub service_name: String,
    #[serde(default = "default_quantity")]
    pub quantity: f64,
    pub price: Option<f64>,
    pub amount: Option<f64>,
    #[serde(default = "default_payment_method")]
    pub payment_method: String,
    #[serde(default = "default_customer_name")]
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
    /// Username of the logged-in staff who recorded this job.
    #[serde(default)]
    pub created_by: Option<String>,
}

fn default_quantity() -> f64 {
    1.0
}

fn default_payment_method() -> String {
    "cash".to_string()
}

fn default_customer_name() -> String {
    "Walk-in".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTransactionUpdate {
    pub service_name: Option<String>,
    pub quantity: Option<f64>,
    pub price: Option<f64>,
    pub amount: Option<f64>,
    pub payment_method: Option<String>,
    pub customer_name: Option<String>,
    pub notes: Option<String>,
    #[serde(default, with = "lossless_opt_i64")]
    pub stock_id: Option<i64>,
    pub stock_metres_used: Option<f64>,
    pub material_size: Option<String>,
    pub material_type: Option<String>,
    pub is_debt: Option<i64>,
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

// ==================== Printing Material ====================

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
    #[serde(default = "default_width")]
    pub width: f64,
    pub rolls: i64,
    #[serde(default = "default_metres_per_roll")]
    pub metres_per_roll: f64,
    pub total_metres: Option<f64>,
    #[serde(default)]
    pub metres_used: f64,
    pub color: Option<String>,
}

fn default_width() -> f64 {
    1.0
}

fn default_metres_per_roll() -> f64 {
    50.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintingMaterialUpdate {
    pub name: Option<String>,
    pub material_type: Option<String>,
    pub width: Option<f64>,
    pub rolls: Option<i64>,
    pub metres_per_roll: Option<f64>,
    pub total_metres: Option<f64>,
    pub metres_used: Option<f64>,
    pub color: Option<String>,
}

// ==================== User ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(with = "lossless_i64")]
    pub id: i64,
    pub username: String,
    pub role: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRow {
    #[serde(with = "lossless_i64")]
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub permissions: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ==================== Auth ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    pub version: String,
}

// ==================== Dashboard ====================

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

// ==================== Migration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageData {
    pub products: Option<Vec<serde_json::Value>>,
    pub stock: Option<Vec<serde_json::Value>>,
    pub sales: Option<Vec<serde_json::Value>>,
    pub debts: Option<Vec<serde_json::Value>>,
}

/// Full business-data backup (JSON). Users are intentionally excluded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseBackup {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub app_version: String,
    #[serde(default)]
    pub products: Vec<serde_json::Value>,
    #[serde(default)]
    pub stock: Vec<serde_json::Value>,
    #[serde(default)]
    pub services: Vec<serde_json::Value>,
    #[serde(default)]
    pub printing_materials: Vec<serde_json::Value>,
    #[serde(default)]
    pub sales: Vec<serde_json::Value>,
    #[serde(default)]
    pub service_transactions: Vec<serde_json::Value>,
    #[serde(default)]
    pub debts: Vec<serde_json::Value>,
    #[serde(default)]
    pub debt_payments: Vec<serde_json::Value>,
}

// ==================== Business statement (admin PDF) ====================

/// What to include in a business revenue statement PDF.
/// Values: `"sales"` | `"printing"` | `"both"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessStatementRequest {
    /// `"sales"` | `"printing"` | `"both"`
    pub source: String,
    /// Months counted backward from today: 1–6.
    pub months: u32,
    /// Admin username requesting the statement (shown on the PDF).
    #[serde(default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementPaymentBreakdown {
    pub method: String,
    pub amount: f64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementMonthRow {
    /// e.g. "2026-07"
    pub year_month: String,
    /// e.g. "Jul 2026"
    pub label: String,
    pub sales_revenue: f64,
    pub sales_count: i64,
    pub printing_revenue: f64,
    pub printing_count: i64,
    pub total_revenue: f64,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementSalesSection {
    pub transaction_count: i64,
    pub gross_billed: f64,
    pub cash_collected: f64,
    pub debt_transactions: i64,
    pub debt_billed: f64,
    pub product_sales_count: i64,
    pub stock_sales_count: i64,
    pub payment_methods: Vec<StatementPaymentBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementPrintingSection {
    pub job_count: i64,
    pub gross_billed: f64,
    pub cash_collected: f64,
    pub debt_jobs: i64,
    pub debt_billed: f64,
    pub material_metres_used: f64,
    pub payment_methods: Vec<StatementPaymentBreakdown>,
}

/// Aggregated figures for a bank-ready business revenue statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessStatement {
    pub source: String,
    pub months: u32,
    pub period_start: String,
    pub period_end: String,
    pub generated_at: String,
    pub requested_by: Option<String>,
    pub app_version: String,
    /// Gross billed across selected sources (incl. credit/debt sales).
    pub total_gross_billed: f64,
    /// Cash recognized: non-debt amounts + debt repayments in period for selected sources.
    pub total_cash_collected: f64,
    pub total_transactions: i64,
    pub average_monthly_cash: f64,
    /// Outstanding remaining on debts created in this period (selected sources).
    pub period_outstanding_receivables: f64,
    pub sales: Option<StatementSalesSection>,
    pub printing: Option<StatementPrintingSection>,
    pub monthly: Vec<StatementMonthRow>,
}
