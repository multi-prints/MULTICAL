use serde::{Deserialize, Serialize};

// ==================== Product ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ==================== Stock ====================

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

// ==================== Sale ====================

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
pub struct NewSale {
    pub r#type: String,
    pub product_id: Option<i64>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleUpdate {
    pub r#type: Option<String>,
    pub amount: Option<f64>,
    pub payment_method: Option<String>,
    pub customer_name: Option<String>,
    pub is_debt: Option<i64>,
}

// ==================== Debt ====================

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
    #[serde(default = "default_payment_method")]
    pub payment_method: String,
    pub notes: Option<String>,
}

// ==================== Service ====================

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
pub struct NewServiceTransaction {
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
    pub stock_id: Option<i64>,
    #[serde(default)]
    pub stock_metres_used: f64,
    pub material_size: Option<String>,
    pub material_type: Option<String>,
    pub printing_material_id: Option<i64>,
    #[serde(default)]
    pub is_debt: i64,
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
    pub stock_id: Option<i64>,
    pub stock_metres_used: Option<f64>,
    pub material_size: Option<String>,
    pub material_type: Option<String>,
    pub is_debt: Option<i64>,
}

// ==================== Printing Material ====================

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
    pub id: i64,
    pub username: String,
    pub role: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRow {
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

// ==================== Migration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageData {
    pub products: Option<Vec<serde_json::Value>>,
    pub stock: Option<Vec<serde_json::Value>>,
    pub sales: Option<Vec<serde_json::Value>>,
    pub debts: Option<Vec<serde_json::Value>>,
}
