use libsql::Builder;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::models::*;

const TURSO_CONFIG_FILE: &str = "turso.json";
const SYNC_DB_FILE: &str = "multiprints-sync.db";
const SYNC_INTERVAL_SECS: u64 = 15;

#[derive(Debug, Clone, Deserialize)]
struct TursoConfig {
    database_url: String,
    auth_token: String,
}

pub struct Database {
    pub conn: Mutex<Connection>,
    sync_db: Option<Arc<libsql::Database>>,
    db_path: PathBuf,
}

fn infer_debt_payment_method(
    conn: &Connection,
    sale_id: Option<i64>,
    service_transaction_id: Option<i64>,
) -> String {
    if let Some(sid) = sale_id {
        if let Ok(method) = conn.query_row(
            "SELECT payment_method FROM sales WHERE id = ?1",
            params![sid],
            |row| row.get::<_, String>(0),
        ) {
            return method;
        }
    }

    if let Some(tid) = service_transaction_id {
        if let Ok(method) = conn.query_row(
            "SELECT payment_method FROM service_transactions WHERE id = ?1",
            params![tid],
            |row| row.get::<_, String>(0),
        ) {
            return method;
        }
    }

    "cash".to_string()
}

impl Database {
    /// Open (or create) the database at the given path
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let data_dir = db_path
            .parent()
            .ok_or_else(|| "Invalid database path".to_string())?;
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;

        let turso_config = Self::load_turso_config(data_dir);
        let active_db_path = if turso_config.is_some() {
            data_dir.join(SYNC_DB_FILE)
        } else {
            db_path.clone()
        };

        let sync_db = if let Some(config) = turso_config.clone() {
            let sync_path = active_db_path.clone();
            let db = tauri::async_runtime::block_on(async move {
                let db = Builder::new_synced_database(
                    &sync_path,
                    config.database_url,
                    config.auth_token,
                )
                .read_your_writes(true)
                .remote_writes(false)
                .sync_interval(Duration::from_secs(SYNC_INTERVAL_SECS))
                .build()
                .await
                .map_err(|e| e.to_string())?;

                db.sync().await.map_err(|e| e.to_string())?;
                Ok::<libsql::Database, String>(db)
            })?;
            Some(Arc::new(db))
        } else {
            None
        };

        let conn = Connection::open(&active_db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -20000;
            ",
        )
        .map_err(|e| e.to_string())?;

        let db = Database {
            conn: Mutex::new(conn),
            sync_db,
            db_path: active_db_path.clone(),
        };

        db.create_tables()?;
        db.run_migrations()?;

        if turso_config.is_some()
            && db.is_business_data_empty()?
            && db_path.exists()
            && db_path != active_db_path
        {
            db.import_legacy_sqlite(&db_path)?;
            let _ = db.sync_now_blocking();
        }

        if turso_config.is_some() {
            let _ = db.sync_now_blocking();
            println!(
                "Database initialized in Turso sync mode at: {:?}",
                active_db_path
            );
        } else {
            println!(
                "Database initialized in local mode at: {:?}",
                active_db_path
            );
        }
        Ok(db)
    }

    fn load_turso_config(data_dir: &Path) -> Option<TursoConfig> {
        let env_url = std::env::var("TURSO_DATABASE_URL").ok();
        let env_token = std::env::var("TURSO_AUTH_TOKEN").ok();
        if let (Some(database_url), Some(auth_token)) = (env_url, env_token) {
            if !database_url.trim().is_empty() && !auth_token.trim().is_empty() {
                return Some(TursoConfig {
                    database_url,
                    auth_token,
                });
            }
        }

        let config_path = data_dir.join(TURSO_CONFIG_FILE);
        let raw = std::fs::read_to_string(config_path).ok()?;
        let parsed: TursoConfig = serde_json::from_str(&raw).ok()?;
        if parsed.database_url.trim().is_empty() || parsed.auth_token.trim().is_empty() {
            return None;
        }
        Some(parsed)
    }

    fn sync_now_blocking(&self) -> Result<(), String> {
        if let Some(sync_db) = self.sync_db.clone() {
            tauri::async_runtime::block_on(async move {
                sync_db.sync().await.map(|_| ()).map_err(|e| e.to_string())
            })
        } else {
            Ok(())
        }
    }

    fn synced_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.sync_now_blocking().map_err(|e| {
            format!(
                "Could not sync with Turso before accessing the database. Check internet connection: {}",
                e
            )
        })?;
        self.conn.lock().map_err(|e| e.to_string())
    }

    fn finish_write<T>(&self, value: T) -> Result<T, String> {
        self.sync_now_blocking().map_err(|e| {
            format!(
                "Database change was saved locally, but could not sync to Turso. Check internet connection: {}",
                e
            )
        })?;
        Ok(value)
    }

    fn is_business_data_empty(&self) -> Result<bool, String> {
        let conn = self.synced_conn()?;
        let total: i64 = conn
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM products) +
                    (SELECT COUNT(*) FROM stock) +
                    (SELECT COUNT(*) FROM sales) +
                    (SELECT COUNT(*) FROM debts) +
                    (SELECT COUNT(*) FROM debt_payments) +
                    (SELECT COUNT(*) FROM services) +
                    (SELECT COUNT(*) FROM service_transactions) +
                    (SELECT COUNT(*) FROM printing_materials) +
                    (SELECT COUNT(*) FROM users)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(total == 0)
    }

    fn legacy_table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM legacy.sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(exists > 0)
    }

    fn import_legacy_sqlite(&self, legacy_path: &Path) -> Result<(), String> {
        if !legacy_path.exists() {
            return Ok(());
        }

        let conn = self.synced_conn()?;
        let legacy_path = legacy_path.to_string_lossy().to_string();

        conn.execute("ATTACH DATABASE ?1 AS legacy", params![legacy_path])
            .map_err(|e| e.to_string())?;

        let import_result = (|| -> Result<(), String> {
            conn.execute_batch("BEGIN IMMEDIATE;")
                .map_err(|e| e.to_string())?;

            if Self::legacy_table_exists(&conn, "products")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO products (id, name, product_type, color, size, selling_price, stock, created_at, updated_at)
                     SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM legacy.products;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "stock")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO stock (id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at)
                     SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM legacy.stock;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "services")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO services (id, name, description, price, unit, uses_stock, is_active, created_at, updated_at)
                     SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM legacy.services;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "printing_materials")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO printing_materials (id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at)
                     SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM legacy.printing_materials;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "sales")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO sales (id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp)
                     SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp FROM legacy.sales;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "service_transactions")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO service_transactions (id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp)
                     SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp FROM legacy.service_transactions;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "debts")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO debts (id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at)
                     SELECT id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at FROM legacy.debts;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "debt_payments")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO debt_payments (id, debt_id, amount, payment_method, notes, payment_date)
                     SELECT id, debt_id, amount, payment_method, notes, payment_date FROM legacy.debt_payments;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "users")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO users (id, username, password_hash, role, permissions, created_at, updated_at)
                     SELECT id, username, password_hash, role, permissions, created_at, updated_at FROM legacy.users;",
                ).map_err(|e| e.to_string())?;
            }

            conn.execute_batch(
                "DELETE FROM sqlite_sequence WHERE name IN ('products', 'stock', 'sales', 'debts', 'debt_payments', 'services', 'service_transactions', 'printing_materials', 'users');
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('products', COALESCE((SELECT MAX(id) FROM products), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('stock', COALESCE((SELECT MAX(id) FROM stock), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('sales', COALESCE((SELECT MAX(id) FROM sales), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('debts', COALESCE((SELECT MAX(id) FROM debts), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('debt_payments', COALESCE((SELECT MAX(id) FROM debt_payments), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('services', COALESCE((SELECT MAX(id) FROM services), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('service_transactions', COALESCE((SELECT MAX(id) FROM service_transactions), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('printing_materials', COALESCE((SELECT MAX(id) FROM printing_materials), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('users', COALESCE((SELECT MAX(id) FROM users), 0));
                 COMMIT;",
            ).map_err(|e| e.to_string())?;

            Ok(())
        })();

        let _ = conn.execute_batch("DETACH DATABASE legacy");
        import_result?;

        println!(
            "Imported legacy local database into synced database at {:?}",
            self.db_path
        );
        self.finish_write(())
    }

    // ==================== Table Creation ====================

    fn create_tables(&self) -> Result<(), String> {
        let conn = self.synced_conn()?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                product_type TEXT NOT NULL,
                color TEXT,
                size TEXT,
                selling_price REAL NOT NULL DEFAULT 0,
                stock INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS stock (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                color TEXT NOT NULL,
                size TEXT NOT NULL DEFAULT '1',
                sticker_type TEXT NOT NULL DEFAULT 'colored',
                rolls INTEGER NOT NULL DEFAULT 0,
                metres_per_roll REAL NOT NULL DEFAULT 50,
                total_metres REAL NOT NULL DEFAULT 0,
                metres_used REAL NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS sales (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                product_id INTEGER,
                stock_id INTEGER,
                product_name TEXT,
                product_type TEXT,
                sticker_type TEXT,
                quantity TEXT,
                amount REAL NOT NULL DEFAULT 0,
                payment_method TEXT NOT NULL DEFAULT 'cash',
                customer_name TEXT DEFAULT 'Walk-in',
                is_debt INTEGER DEFAULT 0,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE SET NULL,
                FOREIGN KEY (stock_id) REFERENCES stock(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS debts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                customer_name TEXT NOT NULL,
                phone TEXT,
                amount REAL NOT NULL DEFAULT 0,
                paid_amount REAL NOT NULL DEFAULT 0,
                remaining_amount REAL NOT NULL DEFAULT 0,
                due_date TEXT,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                sale_id INTEGER,
                service_transaction_id INTEGER,
                paid_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS debt_payments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                debt_id INTEGER NOT NULL,
                amount REAL NOT NULL DEFAULT 0,
                payment_method TEXT NOT NULL DEFAULT 'cash',
                notes TEXT,
                payment_date DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (debt_id) REFERENCES debts(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS services (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                price REAL NOT NULL DEFAULT 0,
                unit TEXT,
                uses_stock INTEGER NOT NULL DEFAULT 0,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS service_transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_id INTEGER,
                service_name TEXT NOT NULL,
                quantity REAL NOT NULL DEFAULT 1,
                price REAL NOT NULL DEFAULT 0,
                amount REAL NOT NULL DEFAULT 0,
                payment_method TEXT NOT NULL DEFAULT 'cash',
                customer_name TEXT DEFAULT 'Walk-in',
                notes TEXT,
                stock_id INTEGER,
                stock_metres_used REAL DEFAULT 0,
                material_size TEXT,
                material_type TEXT,
                printing_material_id INTEGER,
                is_debt INTEGER DEFAULT 0,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE SET NULL,
                FOREIGN KEY (stock_id) REFERENCES stock(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS printing_materials (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                material_type TEXT NOT NULL,
                width REAL NOT NULL DEFAULT 1,
                rolls INTEGER NOT NULL DEFAULT 0,
                metres_per_roll REAL NOT NULL DEFAULT 50,
                total_metres REAL NOT NULL DEFAULT 0,
                metres_used REAL NOT NULL DEFAULT 0,
                color TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'employee',
                permissions TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_products_created_at ON products(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_products_variant_lookup ON products(product_type, color, size);

            CREATE INDEX IF NOT EXISTS idx_stock_created_at ON stock(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_stock_lookup ON stock(color COLLATE NOCASE, size, sticker_type);

            CREATE INDEX IF NOT EXISTS idx_sales_timestamp ON sales(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_sales_product_id ON sales(product_id);
            CREATE INDEX IF NOT EXISTS idx_sales_stock_id ON sales(stock_id);

            CREATE INDEX IF NOT EXISTS idx_debts_created_at ON debts(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_debts_status_created_at ON debts(status, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_debts_due_date ON debts(due_date);
            CREATE INDEX IF NOT EXISTS idx_debts_sale_id ON debts(sale_id);
            CREATE INDEX IF NOT EXISTS idx_debts_service_transaction_id ON debts(service_transaction_id);

            CREATE INDEX IF NOT EXISTS idx_debt_payments_debt_id_payment_date ON debt_payments(debt_id, payment_date DESC);

            CREATE INDEX IF NOT EXISTS idx_services_active_name ON services(is_active, name);
            CREATE INDEX IF NOT EXISTS idx_services_created_at ON services(created_at DESC);

            CREATE INDEX IF NOT EXISTS idx_service_transactions_timestamp ON service_transactions(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_service_transactions_service_id ON service_transactions(service_id);
            CREATE INDEX IF NOT EXISTS idx_service_transactions_stock_id ON service_transactions(stock_id);

            CREATE INDEX IF NOT EXISTS idx_printing_materials_created_at ON printing_materials(created_at DESC);
            ",
        )
        .map_err(|e| e.to_string())?;

        println!("Database tables created");
        self.finish_write(())
    }

    fn run_migrations(&self) -> Result<(), String> {
        let conn = self.synced_conn()?;

        // Check and add missing columns (same migration logic as JS version)
        let migrations = [
            (
                "service_transactions",
                "stock_id",
                "ALTER TABLE service_transactions ADD COLUMN stock_id INTEGER",
            ),
            (
                "service_transactions",
                "stock_metres_used",
                "ALTER TABLE service_transactions ADD COLUMN stock_metres_used REAL DEFAULT 0",
            ),
            (
                "service_transactions",
                "material_size",
                "ALTER TABLE service_transactions ADD COLUMN material_size TEXT",
            ),
            (
                "service_transactions",
                "material_type",
                "ALTER TABLE service_transactions ADD COLUMN material_type TEXT",
            ),
            (
                "service_transactions",
                "printing_material_id",
                "ALTER TABLE service_transactions ADD COLUMN printing_material_id INTEGER",
            ),
            (
                "service_transactions",
                "is_debt",
                "ALTER TABLE service_transactions ADD COLUMN is_debt INTEGER DEFAULT 0",
            ),
            (
                "debts",
                "paid_amount",
                "ALTER TABLE debts ADD COLUMN paid_amount REAL NOT NULL DEFAULT 0",
            ),
            (
                "debts",
                "remaining_amount",
                "ALTER TABLE debts ADD COLUMN remaining_amount REAL NOT NULL DEFAULT 0",
            ),
            (
                "debts",
                "sale_id",
                "ALTER TABLE debts ADD COLUMN sale_id INTEGER",
            ),
            (
                "debts",
                "service_transaction_id",
                "ALTER TABLE debts ADD COLUMN service_transaction_id INTEGER",
            ),
            (
                "sales",
                "is_debt",
                "ALTER TABLE sales ADD COLUMN is_debt INTEGER DEFAULT 0",
            ),
        ];

        for (table, column, alter_sql) in &migrations {
            let has_column: bool = conn
                .prepare(&format!("PRAGMA table_info({})", table))
                .map_err(|e| e.to_string())?
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .any(|name| name == *column);

            if !has_column {
                println!("Adding {} column to {}", column, table);
                conn.execute_batch(alter_sql).map_err(|e| e.to_string())?;
            }
        }

        // Update existing debts: remaining_amount = amount - paid_amount
        conn.execute_batch("UPDATE debts SET remaining_amount = amount - COALESCE(paid_amount, 0)")
            .map_err(|e| e.to_string())?;

        // Normalize debts already marked as paid
        conn.execute_batch(
            "UPDATE debts SET paid_amount = amount, remaining_amount = 0, paid_at = COALESCE(paid_at, CURRENT_TIMESTAMP) WHERE status = 'paid'"
        ).map_err(|e| e.to_string())?;

        // Ensure paid debts have a settlement payment entry for any unpaid remainder
        let mut stmt = conn
            .prepare(
                "SELECT d.id, d.amount, d.paid_at, d.sale_id, d.service_transaction_id, COALESCE(SUM(dp.amount), 0)
                 FROM debts d
                 LEFT JOIN debt_payments dp ON dp.debt_id = d.id
                 WHERE d.status = 'paid'
                 GROUP BY d.id, d.amount, d.paid_at, d.sale_id, d.service_transaction_id
                 HAVING COALESCE(SUM(dp.amount), 0) < d.amount"
            )
            .map_err(|e| e.to_string())?;

        let paid_debts_missing_settlement = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, f64>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        for (id, amount, paid_at, sale_id, service_transaction_id, recorded_total) in
            paid_debts_missing_settlement
        {
            let missing_amount = (amount - recorded_total).max(0.0);
            if missing_amount > 0.0 {
                let payment_method =
                    infer_debt_payment_method(&conn, sale_id, service_transaction_id);
                let payment_date = paid_at.unwrap_or_else(|| {
                    chrono::Local::now()
                        .format("%Y-%m-%dT%H:%M:%S%.3f")
                        .to_string()
                });
                conn.execute(
                    "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, missing_amount, payment_method, Some("Marked as paid".to_string()), payment_date],
                ).map_err(|e| e.to_string())?;
            }
        }

        // Backfill missing payment history for debts that already have recorded payments
        let mut stmt = conn
            .prepare(
                "SELECT id, amount, paid_amount, remaining_amount, status, sale_id, service_transaction_id, paid_at, created_at
                 FROM debts
                 WHERE NOT EXISTS (SELECT 1 FROM debt_payments dp WHERE dp.debt_id = debts.id)
                   AND (paid_amount > 0 OR status = 'paid')"
            )
            .map_err(|e| e.to_string())?;

        let debts_to_backfill = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        for (
            id,
            amount,
            paid_amount,
            _remaining_amount,
            status,
            sale_id,
            service_transaction_id,
            paid_at,
            created_at,
        ) in debts_to_backfill
        {
            let payment_method = infer_debt_payment_method(&conn, sale_id, service_transaction_id);
            let fallback_ts = chrono::Local::now()
                .format("%Y-%m-%dT%H:%M:%S%.3f")
                .to_string();
            let initial_ts = created_at
                .clone()
                .or_else(|| paid_at.clone())
                .unwrap_or_else(|| fallback_ts.clone());
            let final_ts = paid_at
                .clone()
                .or_else(|| created_at.clone())
                .unwrap_or_else(|| fallback_ts.clone());

            let initial_paid = if status == "paid" {
                paid_amount.min(amount).max(0.0)
            } else {
                paid_amount.max(0.0)
            };

            if initial_paid > 0.0 {
                conn.execute(
                    "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, initial_paid, payment_method.clone(), Some("Initial payment".to_string()), initial_ts],
                ).map_err(|e| e.to_string())?;
            }

            if status == "paid" {
                let settlement_amount = (amount - initial_paid).max(0.0);
                if settlement_amount > 0.0 {
                    conn.execute(
                        "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![id, settlement_amount, payment_method, Some("Marked as paid".to_string()), final_ts.clone()],
                    ).map_err(|e| e.to_string())?;
                }

                conn.execute(
                    "UPDATE debts SET paid_amount = ?1, remaining_amount = 0, paid_at = COALESCE(paid_at, ?2) WHERE id = ?3",
                    params![amount, final_ts, id],
                ).map_err(|e| e.to_string())?;
            }
        }

        println!("Database migrations completed");
        self.finish_write(())
    }

    // ==================== Products CRUD ====================

    pub fn get_all_products(&self) -> Result<Vec<Product>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM products ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Product {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    product_type: row.get(2)?,
                    color: row.get(3)?,
                    size: row.get(4)?,
                    selling_price: row.get(5)?,
                    stock: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_product(&self, id: i64) -> Result<Option<Product>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM products WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(Product {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    product_type: row.get(2)?,
                    color: row.get(3)?,
                    size: row.get(4)?,
                    selling_price: row.get(5)?,
                    stock: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn get_products_page(&self, query: ProductsPageQuery) -> Result<ProductsPageData, String> {
        let conn = self.synced_conn()?;
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let total_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM products ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![per_page as i64, offset], |row| {
                Ok(Product {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    product_type: row.get(2)?,
                    color: row.get(3)?,
                    size: row.get(4)?,
                    selling_price: row.get(5)?,
                    stock: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let items = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let (total_stock_units, life_saver_stock, chevron_stock, stripes_stock, stock_value): (
            i64,
            i64,
            i64,
            i64,
            f64,
        ) = conn
            .query_row(
                "SELECT
                COALESCE(SUM(stock), 0),
                COALESCE(SUM(CASE WHEN product_type = 'life_saver' THEN stock ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN product_type = 'chevron' THEN stock ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN product_type = 'stripes' THEN stock ELSE 0 END), 0),
                COALESCE(SUM(stock * selling_price), 0)
             FROM products",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;

        Ok(ProductsPageData {
            items,
            total_count,
            total_stock_units,
            life_saver_stock,
            chevron_stock,
            stripes_stock,
            stock_value,
        })
    }

    pub fn add_product(&self, product: NewProduct) -> Result<Product, String> {
        let conn = self.synced_conn()?;

        // Preserve legacy behavior: find existing variant by type/color/size, update stock instead of insert
        let existing_id: Option<i64> = {
            let mut stmt = conn
                .prepare("SELECT id, color, size FROM products WHERE product_type = ?1")
                .map_err(|e| e.to_string())?;
            let rows: Vec<(i64, Option<String>, Option<String>)> = stmt
                .query_map(params![product.product_type], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            rows.into_iter()
                .find(|(_, c, s)| c == &product.color && s == &product.size)
                .map(|(id, _, _)| id)
        };

        let id = if let Some(id) = existing_id {
            conn.execute(
                "UPDATE products SET stock = stock + ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![product.stock, id],
            ).map_err(|e| e.to_string())?;
            id
        } else {
            conn.execute(
                "INSERT INTO products (name, product_type, color, size, selling_price, stock) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![product.name, product.product_type, product.color, product.size, product.selling_price, product.stock],
            ).map_err(|e| e.to_string())?;
            conn.last_insert_rowid()
        };

        // IMPORTANT: do not call self.get_product(id) here while holding the mutex,
        // because get_product() also locks the same mutex and deadlocks the UI.
        let product = conn.query_row(
            "SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM products WHERE id = ?1",
            params![id],
            |row| {
                Ok(Product {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    product_type: row.get(2)?,
                    color: row.get(3)?,
                    size: row.get(4)?,
                    selling_price: row.get(5)?,
                    stock: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        ).map_err(|e| e.to_string())?;
        self.finish_write(product)
    }

    pub fn update_product(&self, id: i64, updates: ProductUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let mut sql = String::from("UPDATE products SET ");
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut first = true;

        if let Some(v) = updates.name {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("name = ?");
            params.push(Box::new(v));
            first = false;
        }
        if let Some(v) = updates.product_type {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("product_type = ?");
            params.push(Box::new(v));
            first = false;
        }
        if let Some(v) = updates.color {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("color = ?");
            params.push(Box::new(v));
            first = false;
        }
        if let Some(v) = updates.size {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("size = ?");
            params.push(Box::new(v));
            first = false;
        }
        if let Some(v) = updates.selling_price {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("selling_price = ?");
            params.push(Box::new(v));
            first = false;
        }
        if let Some(v) = updates.stock {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("stock = ?");
            params.push(Box::new(v));
            first = false;
        }

        if first {
            return Ok(());
        }
        sql.push_str(", updated_at = CURRENT_TIMESTAMP WHERE id = ?");
        params.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_product(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM products WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Stock CRUD ====================

    pub fn get_all_stock(&self) -> Result<Vec<StockItem>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(StockItem {
                    id: row.get(0)?,
                    color: row.get(1)?,
                    size: row.get(2)?,
                    sticker_type: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_stock(&self, id: i64) -> Result<Option<StockItem>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(StockItem {
                    id: row.get(0)?,
                    color: row.get(1)?,
                    size: row.get(2)?,
                    sticker_type: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn get_stock_by_color_size_type(
        &self,
        color: &str,
        size: &str,
        sticker_type: &str,
    ) -> Result<Option<StockItem>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock WHERE LOWER(color) = LOWER(?1) AND size = ?2 AND sticker_type = ?3")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![color, size, sticker_type], |row| {
                Ok(StockItem {
                    id: row.get(0)?,
                    color: row.get(1)?,
                    size: row.get(2)?,
                    sticker_type: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn get_stock_page(&self, query: StockPageQuery) -> Result<StockPageData, String> {
        let conn = self.synced_conn()?;
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let total_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stock", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![per_page as i64, offset], |row| {
                Ok(StockItem {
                    id: row.get(0)?,
                    color: row.get(1)?,
                    size: row.get(2)?,
                    sticker_type: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let items = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let (total_rolls, total_metres, remaining_metres): (i64, f64, f64) = conn
            .query_row(
                "SELECT
                COALESCE(SUM(rolls), 0),
                COALESCE(SUM(total_metres), 0),
                COALESCE(SUM(total_metres - COALESCE(metres_used, 0)), 0)
             FROM stock",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| e.to_string())?;

        Ok(StockPageData {
            items,
            total_count,
            total_rolls,
            total_metres,
            remaining_metres,
        })
    }

    pub fn add_stock(&self, item: NewStockItem) -> Result<StockItem, String> {
        let conn = self.synced_conn()?;

        let base_metres_per_roll = 50.0_f64;
        let (metres_per_roll, total_metres) = if item.sticker_type == "reflective" {
            if let Some(custom) = item.custom_metres_per_roll {
                (custom, item.rolls as f64 * custom)
            } else {
                (
                    base_metres_per_roll,
                    item.rolls as f64 * base_metres_per_roll,
                )
            }
        } else {
            (
                base_metres_per_roll,
                item.rolls as f64 * base_metres_per_roll,
            )
        };

        conn.execute(
            "INSERT INTO stock (color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![item.color, item.size, item.sticker_type, item.rolls, metres_per_roll, total_metres],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        // IMPORTANT: do not call self.get_stock(id) here while holding the mutex,
        // because get_stock() also locks the same mutex and deadlocks the UI.
        let stock_item = conn.query_row(
            "SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock WHERE id = ?1",
            params![id],
            |row| {
                Ok(StockItem {
                    id: row.get(0)?, color: row.get(1)?, size: row.get(2)?,
                    sticker_type: row.get(3)?, rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?, total_metres: row.get(6)?,
                    metres_used: row.get(7)?, created_at: row.get(8)?, updated_at: row.get(9)?,
                })
            },
        ).map_err(|e| e.to_string())?;
        self.finish_write(stock_item)
    }

    pub fn update_stock(&self, id: i64, updates: StockUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.color {
            set_clauses.push("color = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.size {
            set_clauses.push("size = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.sticker_type {
            set_clauses.push("sticker_type = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.rolls {
            set_clauses.push("rolls = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.metres_per_roll {
            set_clauses.push("metres_per_roll = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.total_metres {
            set_clauses.push("total_metres = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.metres_used {
            set_clauses.push("metres_used = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!("UPDATE stock SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_stock(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM stock WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Sales CRUD ====================

    pub fn get_all_sales(&self) -> Result<Vec<Sale>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp FROM sales ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Sale {
                    id: row.get(0)?,
                    r#type: row.get(1)?,
                    product_id: row.get(2)?,
                    stock_id: row.get(3)?,
                    product_name: row.get(4)?,
                    product_type: row.get(5)?,
                    sticker_type: row.get(6)?,
                    quantity: row.get(7)?,
                    amount: row.get(8)?,
                    payment_method: row.get(9)?,
                    customer_name: row.get(10)?,
                    is_debt: row.get(11)?,
                    timestamp: row.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_today_sales(&self) -> Result<Vec<Sale>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp FROM sales WHERE DATE(timestamp) = DATE('now', 'localtime') ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Sale {
                    id: row.get(0)?,
                    r#type: row.get(1)?,
                    product_id: row.get(2)?,
                    stock_id: row.get(3)?,
                    product_name: row.get(4)?,
                    product_type: row.get(5)?,
                    sticker_type: row.get(6)?,
                    quantity: row.get(7)?,
                    amount: row.get(8)?,
                    payment_method: row.get(9)?,
                    customer_name: row.get(10)?,
                    is_debt: row.get(11)?,
                    timestamp: row.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_sale(&self, sale: NewSale) -> Result<Sale, String> {
        let conn = self.synced_conn()?;
        let timestamp = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();

        conn.execute_batch("BEGIN IMMEDIATE;")
            .map_err(|e| e.to_string())?;

        let insert_result = (|| -> Result<i64, String> {
            if let (Some(product_id), Some(quantity)) = (sale.product_id, sale.product_quantity) {
                if quantity > 0 {
                    let updated = conn
                        .execute(
                            "UPDATE products SET stock = stock - ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND stock >= ?1",
                            params![quantity, product_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if updated == 0 {
                        return Err("Insufficient product stock or product was not found.".into());
                    }
                }
            }

            if let (Some(stock_id), Some(metres_used)) = (sale.stock_id, sale.stock_metres_used) {
                if metres_used > 0.0 {
                    let updated = conn
                        .execute(
                            "UPDATE stock SET metres_used = metres_used + ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND (total_metres - metres_used) >= ?1",
                            params![metres_used, stock_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if updated == 0 {
                        return Err("Insufficient stock metres or stock item was not found.".into());
                    }
                }
            }

            conn.execute(
                "INSERT INTO sales (type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![sale.r#type, sale.product_id, sale.stock_id, sale.product_name, sale.product_type, sale.sticker_type, sale.quantity, sale.amount, sale.payment_method, sale.customer_name, sale.is_debt, timestamp],
            ).map_err(|e| e.to_string())?;

            Ok(conn.last_insert_rowid())
        })();

        let id = match insert_result {
            Ok(id) => {
                conn.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
                id
            }
            Err(err) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(err);
            }
        };

        drop(conn);
        self.finish_write(Sale {
            id,
            r#type: sale.r#type,
            product_id: sale.product_id,
            stock_id: sale.stock_id,
            product_name: sale.product_name,
            product_type: sale.product_type,
            sticker_type: sale.sticker_type,
            quantity: sale.quantity,
            amount: sale.amount,
            payment_method: sale.payment_method,
            customer_name: sale.customer_name,
            is_debt: sale.is_debt,
            timestamp: Some(timestamp),
        })
    }

    pub fn get_today_total_sales(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        let total: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0) FROM sales WHERE DATE(timestamp) = DATE('now', 'localtime')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(total)
    }

    pub fn get_sales_page(&self, query: SalesPageQuery) -> Result<SalesPageData, String> {
        let conn = self.synced_conn()?;

        let search = query.search.unwrap_or_default().trim().to_lowercase();
        let sort_by = query.sort_by.unwrap_or_else(|| "newest".to_string());
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let search_filter = if search.is_empty() {
            String::new()
        } else {
            "WHERE (
                LOWER(COALESCE(product_name, '')) LIKE ?1 OR
                LOWER(customer_name) LIKE ?1 OR
                LOWER(payment_method) LIKE ?1 OR
                LOWER(COALESCE(quantity, '')) LIKE ?1 OR
                LOWER(type) LIKE ?1 OR
                LOWER(COALESCE(timestamp, '')) LIKE ?1
            )"
            .to_string()
        };

        let order_by = match sort_by.as_str() {
            "oldest" => "timestamp ASC",
            "amount_desc" => "amount DESC, timestamp DESC",
            "amount_asc" => "amount ASC, timestamp DESC",
            _ => "timestamp DESC",
        };

        let total_count: i64 = if search.is_empty() {
            conn.query_row("SELECT COUNT(*) FROM sales", [], |row| row.get(0))
        } else {
            let term = format!("%{}%", search);
            conn.query_row(
                &format!("SELECT COUNT(*) FROM sales {}", search_filter),
                params![term],
                |row| row.get(0),
            )
        }
        .map_err(|e| e.to_string())?;

        let sql = format!(
            "SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp
             FROM sales
             {}
             ORDER BY {}
             LIMIT ?{} OFFSET ?{}",
            search_filter,
            order_by,
            if search.is_empty() { 1 } else { 2 },
            if search.is_empty() { 2 } else { 3 },
        );

        let items = if search.is_empty() {
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![per_page as i64, offset], |row| {
                    Ok(Sale {
                        id: row.get(0)?,
                        r#type: row.get(1)?,
                        product_id: row.get(2)?,
                        stock_id: row.get(3)?,
                        product_name: row.get(4)?,
                        product_type: row.get(5)?,
                        sticker_type: row.get(6)?,
                        quantity: row.get(7)?,
                        amount: row.get(8)?,
                        payment_method: row.get(9)?,
                        customer_name: row.get(10)?,
                        is_debt: row.get(11)?,
                        timestamp: row.get(12)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        } else {
            let term = format!("%{}%", search);
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![term, per_page as i64, offset], |row| {
                    Ok(Sale {
                        id: row.get(0)?,
                        r#type: row.get(1)?,
                        product_id: row.get(2)?,
                        stock_id: row.get(3)?,
                        product_name: row.get(4)?,
                        product_type: row.get(5)?,
                        sticker_type: row.get(6)?,
                        quantity: row.get(7)?,
                        amount: row.get(8)?,
                        payment_method: row.get(9)?,
                        customer_name: row.get(10)?,
                        is_debt: row.get(11)?,
                        timestamp: row.get(12)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let today_total: f64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM sales WHERE DATE(timestamp) = DATE('now', 'localtime')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        let all_revenue: f64 = conn
            .query_row("SELECT COALESCE(SUM(amount), 0) FROM sales", [], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;

        let product_sales_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sales WHERE type = 'product'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        Ok(SalesPageData {
            items,
            total_count,
            today_total,
            all_revenue,
            product_sales_count,
        })
    }

    pub fn update_sale(&self, id: i64, updates: SaleUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.r#type {
            set_clauses.push("type = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.amount {
            set_clauses.push("amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.payment_method {
            set_clauses.push("payment_method = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.customer_name {
            set_clauses.push("customer_name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.is_debt {
            set_clauses.push("is_debt = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE sales SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_sale(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM sales WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Debts CRUD ====================

    pub fn get_all_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at FROM debts d ORDER BY d.created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_pending_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at FROM debts d WHERE d.status = 'pending' ORDER BY d.created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_debt(&self, debt: NewDebt) -> Result<Debt, String> {
        let conn = self.synced_conn()?;
        let paid_amount = debt.paid_amount.unwrap_or(0.0);
        let remaining_amount = debt.remaining_amount.unwrap_or(debt.amount - paid_amount);
        let created_at = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();
        let status = if remaining_amount <= 0.0 {
            "paid"
        } else {
            "pending"
        };
        let paid_at = if paid_amount > 0.0 && remaining_amount <= 0.0 {
            Some(created_at.clone())
        } else {
            None
        };

        conn.execute(
            "INSERT INTO debts (customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![debt.customer_name, debt.phone, debt.amount, paid_amount, remaining_amount, debt.due_date, debt.description, status, debt.sale_id, debt.service_transaction_id, paid_at, created_at],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();

        if paid_amount > 0.0 {
            let payment_method =
                infer_debt_payment_method(&conn, debt.sale_id, debt.service_transaction_id);
            conn.execute(
                "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, paid_amount, payment_method, Some("Initial payment".to_string()), created_at.clone()],
            ).map_err(|e| e.to_string())?;
        }

        self.finish_write(Debt {
            id,
            customer_name: debt.customer_name,
            phone: debt.phone,
            amount: debt.amount,
            paid_amount,
            remaining_amount,
            due_date: debt.due_date,
            description: debt.description,
            status: status.to_string(),
            sale_id: debt.sale_id,
            service_transaction_id: debt.service_transaction_id,
            paid_at: paid_at.clone(),
            last_payment_at: if paid_amount > 0.0 {
                Some(created_at.clone())
            } else {
                None
            },
            created_at: Some(created_at),
        })
    }

    pub fn update_debt(&self, id: i64, updates: DebtUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.customer_name {
            set_clauses.push("customer_name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.phone {
            set_clauses.push("phone = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.amount {
            set_clauses.push("amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.paid_amount {
            set_clauses.push("paid_amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.remaining_amount {
            set_clauses.push("remaining_amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.due_date {
            set_clauses.push("due_date = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.description {
            set_clauses.push("description = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.status {
            set_clauses.push("status = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE debts SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn mark_debt_paid(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let (sale_id, transaction_id, amount, paid_amount, remaining_amount): (Option<i64>, Option<i64>, f64, f64, f64) = conn
            .query_row(
                "SELECT sale_id, service_transaction_id, amount, paid_amount, remaining_amount FROM debts WHERE id = ?1",
                params![id],
                |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
                }
            )
            .map_err(|e| e.to_string())?;

        let paid_at = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();
        let settlement_amount = remaining_amount.max(0.0);

        if settlement_amount > 0.0 {
            let payment_method = infer_debt_payment_method(&conn, sale_id, transaction_id);
            conn.execute(
                "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, settlement_amount, payment_method, Some("Marked as paid".to_string()), paid_at.clone()],
            ).map_err(|e| e.to_string())?;
        }

        conn.execute(
            "UPDATE debts SET status = 'paid', paid_amount = ?1, paid_at = ?2, remaining_amount = 0 WHERE id = ?3",
            params![amount.max(paid_amount), paid_at.clone(), id],
        ).map_err(|e| e.to_string())?;

        if let Some(sid) = sale_id {
            conn.execute("UPDATE sales SET is_debt = 2 WHERE id = ?1", params![sid])
                .map_err(|e| e.to_string())?;
        }
        if let Some(tid) = transaction_id {
            conn.execute(
                "UPDATE service_transactions SET is_debt = 2 WHERE id = ?1",
                params![tid],
            )
            .map_err(|e| e.to_string())?;
        }

        self.finish_write(())
    }

    pub fn get_debt_by_sale_id(&self, sale_id: i64) -> Result<Option<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at FROM debts d WHERE d.sale_id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![sale_id], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn get_debt_by_transaction_id(&self, transaction_id: i64) -> Result<Option<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at FROM debts d WHERE d.service_transaction_id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![transaction_id], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn delete_debt(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM debts WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn get_total_outstanding(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        conn.query_row(
            "SELECT COALESCE(SUM(remaining_amount), 0) FROM debts WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    }

    pub fn get_debts_page(&self, query: DebtsPageQuery) -> Result<DebtsPageData, String> {
        let conn = self.synced_conn()?;

        let search = query.search.unwrap_or_default().trim().to_lowercase();
        let sort_by = query.sort_by.unwrap_or_else(|| "newest".to_string());
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let search_filter = if search.is_empty() {
            String::new()
        } else {
            "WHERE (
                LOWER(customer_name) LIKE ?1 OR
                LOWER(COALESCE(phone, '')) LIKE ?1 OR
                LOWER(COALESCE(description, '')) LIKE ?1 OR
                LOWER(status) LIKE ?1 OR
                LOWER(COALESCE(due_date, '')) LIKE ?1
            )"
            .to_string()
        };

        let order_by = match sort_by.as_str() {
            "oldest" => "created_at ASC",
            "amount_desc" => "remaining_amount DESC, created_at DESC",
            "amount_asc" => "remaining_amount ASC, created_at DESC",
            _ => "created_at DESC",
        };

        let total_count: i64 = if search.is_empty() {
            conn.query_row("SELECT COUNT(*) FROM debts", [], |row| row.get(0))
        } else {
            let term = format!("%{}%", search);
            conn.query_row(
                &format!("SELECT COUNT(*) FROM debts {}", search_filter),
                params![term],
                |row| row.get(0),
            )
        }
        .map_err(|e| e.to_string())?;

        let sql = format!(
            "SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at
             FROM debts d
             {}
             ORDER BY {}
             LIMIT ?{} OFFSET ?{}",
            search_filter,
            order_by,
            if search.is_empty() { 1 } else { 2 },
            if search.is_empty() { 2 } else { 3 },
        );

        let items = if search.is_empty() {
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![per_page as i64, offset], |row| {
                    Ok(Debt {
                        id: row.get(0)?,
                        customer_name: row.get(1)?,
                        phone: row.get(2)?,
                        amount: row.get(3)?,
                        paid_amount: row.get(4)?,
                        remaining_amount: row.get(5)?,
                        due_date: row.get(6)?,
                        description: row.get(7)?,
                        status: row.get(8)?,
                        sale_id: row.get(9)?,
                        service_transaction_id: row.get(10)?,
                        paid_at: row.get(11)?,
                        last_payment_at: row.get(12)?,
                        created_at: row.get(13)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        } else {
            let term = format!("%{}%", search);
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![term, per_page as i64, offset], |row| {
                    Ok(Debt {
                        id: row.get(0)?,
                        customer_name: row.get(1)?,
                        phone: row.get(2)?,
                        amount: row.get(3)?,
                        paid_amount: row.get(4)?,
                        remaining_amount: row.get(5)?,
                        due_date: row.get(6)?,
                        description: row.get(7)?,
                        status: row.get(8)?,
                        sale_id: row.get(9)?,
                        service_transaction_id: row.get(10)?,
                        paid_at: row.get(11)?,
                        last_payment_at: row.get(12)?,
                        created_at: row.get(13)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let total_outstanding: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(remaining_amount), 0) FROM debts WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let paid_this_month: f64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM debt_payments WHERE strftime('%Y-%m', payment_date) = strftime('%Y-%m', 'now')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        let overdue_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
             FROM debts
             WHERE status = 'pending' AND due_date IS NOT NULL AND DATE(due_date) < DATE('now')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let mut all_stmt = conn.prepare(
            "SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at
             FROM debts d
             ORDER BY d.created_at DESC"
        ).map_err(|e| e.to_string())?;

        let all_rows = all_stmt
            .query_map([], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let all_debts = all_rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(DebtsPageData {
            items,
            total_count,
            total_outstanding,
            paid_this_month,
            overdue_count,
            all_debts,
        })
    }

    pub fn get_paid_this_month(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM debt_payments WHERE strftime('%Y-%m', payment_date) = strftime('%Y-%m', 'now')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn get_overdue_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at FROM debts d WHERE d.status = 'pending' AND d.due_date IS NOT NULL AND DATE(d.due_date) < DATE('now')")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    // ==================== Debt Payments CRUD ====================

    pub fn get_debt_payments(&self, debt_id: i64) -> Result<Vec<DebtPayment>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, debt_id, amount, payment_method, notes, payment_date FROM debt_payments WHERE debt_id = ?1 ORDER BY payment_date DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![debt_id], |row| {
                Ok(DebtPayment {
                    id: row.get(0)?,
                    debt_id: row.get(1)?,
                    amount: row.get(2)?,
                    payment_method: row.get(3)?,
                    notes: row.get(4)?,
                    payment_date: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_debt_payment(&self, payment: NewDebtPayment) -> Result<DebtPayment, String> {
        let conn = self.synced_conn()?;
        let payment_date = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();

        conn.execute(
            "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![payment.debt_id, payment.amount, payment.payment_method, payment.notes, payment_date],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();

        // Update debt paid_amount, remaining_amount, status
        let debt: (f64, f64) = conn
            .query_row(
                "SELECT paid_amount, amount FROM debts WHERE id = ?1",
                params![payment.debt_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let new_paid_amount = debt.0 + payment.amount;
        let new_remaining = (debt.1 - new_paid_amount).max(0.0);
        let new_status = if new_remaining <= 0.0 {
            "paid"
        } else {
            "pending"
        };
        let paid_at_value = if new_remaining <= 0.0 {
            Some(payment_date.clone())
        } else {
            None
        };

        conn.execute(
            "UPDATE debts SET paid_amount = ?1, remaining_amount = ?2, status = ?3, paid_at = COALESCE(?4, paid_at) WHERE id = ?5",
            params![new_paid_amount, new_remaining, new_status, paid_at_value, payment.debt_id],
        ).map_err(|e| e.to_string())?;

        self.finish_write(DebtPayment {
            id,
            debt_id: payment.debt_id,
            amount: payment.amount,
            payment_method: payment.payment_method,
            notes: payment.notes,
            payment_date: Some(payment_date),
        })
    }

    pub fn delete_debt_payment(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let payment: (i64, f64) = conn
            .query_row(
                "SELECT debt_id, amount FROM debt_payments WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let debt: (f64, f64) = conn
            .query_row(
                "SELECT paid_amount, amount FROM debts WHERE id = ?1",
                params![payment.0],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let new_paid_amount = (debt.0 - payment.1).max(0.0);
        let new_remaining = debt.1 - new_paid_amount;
        let new_status = if new_remaining > 0.0 {
            "pending"
        } else {
            "paid"
        };

        conn.execute(
            "UPDATE debts SET paid_amount = ?1, remaining_amount = ?2, status = ?3 WHERE id = ?4",
            params![new_paid_amount, new_remaining, new_status, payment.0],
        )
        .map_err(|e| e.to_string())?;

        conn.execute("DELETE FROM debt_payments WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;

        self.finish_write(())
    }

    // ==================== Services CRUD ====================

    pub fn get_all_services(&self) -> Result<Vec<Service>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Service {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    price: row.get(3)?,
                    unit: row.get(4)?,
                    uses_stock: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_active_services(&self) -> Result<Vec<Service>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE is_active = 1 ORDER BY name")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Service {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    price: row.get(3)?,
                    unit: row.get(4)?,
                    uses_stock: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_service(&self, id: i64) -> Result<Option<Service>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(Service {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    price: row.get(3)?,
                    unit: row.get(4)?,
                    uses_stock: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn add_service(&self, service: NewService) -> Result<Service, String> {
        let conn = self.synced_conn()?;
        let created_at = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();

        conn.execute(
            "INSERT INTO services (name, description, price, unit, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![service.name, service.description, service.price.unwrap_or(0.0), service.unit, service.is_active, created_at],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        // IMPORTANT: do not call self.get_service(id) here while holding the mutex,
        // because get_service() also locks the same mutex and deadlocks the UI.
        let service = conn.query_row(
            "SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE id = ?1",
            params![id],
            |row| {
                Ok(Service {
                    id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                    price: row.get(3)?, unit: row.get(4)?, uses_stock: row.get(5)?,
                    is_active: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
                })
            },
        ).map_err(|e| e.to_string())?;
        self.finish_write(service)
    }

    pub fn update_service(&self, id: i64, updates: ServiceUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.name {
            set_clauses.push("name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.description {
            set_clauses.push("description = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.price {
            set_clauses.push("price = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.unit {
            set_clauses.push("unit = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.is_active {
            set_clauses.push("is_active = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!(
            "UPDATE services SET {} WHERE id = ?",
            set_clauses.join(", ")
        );
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_service(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM services WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Service Transactions CRUD ====================

    pub fn get_all_service_transactions(&self) -> Result<Vec<ServiceTransaction>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp FROM service_transactions ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ServiceTransaction {
                    id: row.get(0)?,
                    service_id: row.get(1)?,
                    service_name: row.get(2)?,
                    quantity: row.get(3)?,
                    price: row.get(4)?,
                    amount: row.get(5)?,
                    payment_method: row.get(6)?,
                    customer_name: row.get(7)?,
                    notes: row.get(8)?,
                    stock_id: row.get(9)?,
                    stock_metres_used: row.get(10)?,
                    material_size: row.get(11)?,
                    material_type: row.get(12)?,
                    printing_material_id: row.get(13)?,
                    is_debt: row.get(14)?,
                    timestamp: row.get(15)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_today_service_transactions(&self) -> Result<Vec<ServiceTransaction>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp FROM service_transactions WHERE DATE(timestamp) = DATE('now', 'localtime') ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ServiceTransaction {
                    id: row.get(0)?,
                    service_id: row.get(1)?,
                    service_name: row.get(2)?,
                    quantity: row.get(3)?,
                    price: row.get(4)?,
                    amount: row.get(5)?,
                    payment_method: row.get(6)?,
                    customer_name: row.get(7)?,
                    notes: row.get(8)?,
                    stock_id: row.get(9)?,
                    stock_metres_used: row.get(10)?,
                    material_size: row.get(11)?,
                    material_type: row.get(12)?,
                    printing_material_id: row.get(13)?,
                    is_debt: row.get(14)?,
                    timestamp: row.get(15)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_service_transaction(
        &self,
        tx: NewServiceTransaction,
    ) -> Result<ServiceTransaction, String> {
        let conn = self.synced_conn()?;
        let timestamp = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();
        let amount = tx.amount.unwrap_or(tx.quantity * tx.price.unwrap_or(0.0));

        conn.execute(
            "INSERT INTO service_transactions (service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![tx.service_id, tx.service_name, tx.quantity, tx.price.unwrap_or(0.0), amount, tx.payment_method, tx.customer_name, tx.notes, tx.stock_id, tx.stock_metres_used, tx.material_size, tx.material_type, tx.printing_material_id, tx.is_debt, timestamp],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();

        // Auto-deduct stock metres_used if stock was used
        // IMPORTANT: use inline queries to avoid deadlocking the mutex
        if let (Some(stock_id), stock_metres) = (tx.stock_id, tx.stock_metres_used) {
            if stock_metres > 0.0 {
                let current_metres_used: Option<f64> = conn
                    .query_row(
                        "SELECT metres_used FROM stock WHERE id = ?1",
                        params![stock_id],
                        |row| row.get(0),
                    )
                    .ok();
                if let Some(current_used) = current_metres_used {
                    let new_metres_used = current_used + stock_metres;
                    conn.execute(
                        "UPDATE stock SET metres_used = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                        params![new_metres_used, stock_id],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }

        if let Some(material_id) = tx.printing_material_id {
            if tx.stock_metres_used > 0.0 {
                let current_metres_used: Option<f64> = conn
                    .query_row(
                        "SELECT metres_used FROM printing_materials WHERE id = ?1",
                        params![material_id],
                        |row| row.get(0),
                    )
                    .ok();
                if let Some(current_used) = current_metres_used {
                    let new_metres_used = current_used + tx.stock_metres_used;
                    conn.execute(
                        "UPDATE printing_materials SET metres_used = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                        params![new_metres_used, material_id],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }

        self.finish_write(ServiceTransaction {
            id,
            service_id: tx.service_id,
            service_name: tx.service_name,
            quantity: tx.quantity,
            price: tx.price.unwrap_or(0.0),
            amount,
            payment_method: tx.payment_method,
            customer_name: tx.customer_name,
            notes: tx.notes,
            stock_id: tx.stock_id,
            stock_metres_used: tx.stock_metres_used,
            material_size: tx.material_size,
            material_type: tx.material_type,
            printing_material_id: tx.printing_material_id,
            is_debt: tx.is_debt,
            timestamp: Some(timestamp),
        })
    }

    pub fn get_today_total_service_earnings(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM service_transactions WHERE DATE(timestamp) = DATE('now', 'localtime')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn get_total_service_earnings(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM service_transactions",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    }

    pub fn get_printing_page(&self, query: PrintingPageQuery) -> Result<PrintingPageData, String> {
        let conn = self.synced_conn()?;

        let search = query.search.unwrap_or_default().trim().to_lowercase();
        let sort_by = query.sort_by.unwrap_or_else(|| "newest".to_string());
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let base_filter = "stock_metres_used > 0";
        let search_filter = if search.is_empty() {
            base_filter.to_string()
        } else {
            format!(
                "{} AND (
                    LOWER(service_name) LIKE ?1 OR
                    LOWER(customer_name) LIKE ?1 OR
                    LOWER(payment_method) LIKE ?1 OR
                    LOWER(COALESCE(material_type, '')) LIKE ?1 OR
                    LOWER(COALESCE(material_size, '')) LIKE ?1 OR
                    LOWER(COALESCE(timestamp, '')) LIKE ?1
                )",
                base_filter
            )
        };

        let order_by = match sort_by.as_str() {
            "oldest" => "timestamp ASC",
            "amount_desc" => "amount DESC, timestamp DESC",
            "amount_asc" => "amount ASC, timestamp DESC",
            _ => "timestamp DESC",
        };

        let total_count: i64 = if search.is_empty() {
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM service_transactions WHERE {}",
                    base_filter
                ),
                [],
                |row| row.get(0),
            )
        } else {
            let term = format!("%{}%", search);
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM service_transactions WHERE {}",
                    search_filter
                ),
                params![term],
                |row| row.get(0),
            )
        }
        .map_err(|e| e.to_string())?;

        let sql = format!(
            "SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp
             FROM service_transactions
             WHERE {}
             ORDER BY {}
             LIMIT ?{} OFFSET ?{}",
            search_filter,
            order_by,
            if search.is_empty() { 1 } else { 2 },
            if search.is_empty() { 2 } else { 3 },
        );

        let items = if search.is_empty() {
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![per_page as i64, offset], |row| {
                    Ok(ServiceTransaction {
                        id: row.get(0)?,
                        service_id: row.get(1)?,
                        service_name: row.get(2)?,
                        quantity: row.get(3)?,
                        price: row.get(4)?,
                        amount: row.get(5)?,
                        payment_method: row.get(6)?,
                        customer_name: row.get(7)?,
                        notes: row.get(8)?,
                        stock_id: row.get(9)?,
                        stock_metres_used: row.get(10)?,
                        material_size: row.get(11)?,
                        material_type: row.get(12)?,
                        printing_material_id: row.get(13)?,
                        is_debt: row.get(14)?,
                        timestamp: row.get(15)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        } else {
            let term = format!("%{}%", search);
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![term, per_page as i64, offset], |row| {
                    Ok(ServiceTransaction {
                        id: row.get(0)?,
                        service_id: row.get(1)?,
                        service_name: row.get(2)?,
                        quantity: row.get(3)?,
                        price: row.get(4)?,
                        amount: row.get(5)?,
                        payment_method: row.get(6)?,
                        customer_name: row.get(7)?,
                        notes: row.get(8)?,
                        stock_id: row.get(9)?,
                        stock_metres_used: row.get(10)?,
                        material_size: row.get(11)?,
                        material_type: row.get(12)?,
                        printing_material_id: row.get(13)?,
                        is_debt: row.get(14)?,
                        timestamp: row.get(15)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let today_earnings: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0)
             FROM service_transactions
             WHERE stock_metres_used > 0 AND DATE(timestamp) = DATE('now', 'localtime')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let total_jobs_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM service_transactions WHERE stock_metres_used > 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let material_used: f64 = conn.query_row(
            "SELECT COALESCE(SUM(stock_metres_used), 0) FROM service_transactions WHERE stock_metres_used > 0",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        let total_revenue: f64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM service_transactions WHERE stock_metres_used > 0",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        Ok(PrintingPageData {
            items,
            total_count,
            today_earnings,
            total_jobs_count,
            material_used,
            total_revenue,
        })
    }

    pub fn update_service_transaction(
        &self,
        id: i64,
        updates: ServiceTransactionUpdate,
    ) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.service_name {
            set_clauses.push("service_name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.quantity {
            set_clauses.push("quantity = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.price {
            set_clauses.push("price = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.amount {
            set_clauses.push("amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.payment_method {
            set_clauses.push("payment_method = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.customer_name {
            set_clauses.push("customer_name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.notes {
            set_clauses.push("notes = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.stock_id {
            set_clauses.push("stock_id = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.stock_metres_used {
            set_clauses.push("stock_metres_used = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.material_size {
            set_clauses.push("material_size = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.material_type {
            set_clauses.push("material_type = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.is_debt {
            set_clauses.push("is_debt = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        let sql = format!(
            "UPDATE service_transactions SET {} WHERE id = ?",
            set_clauses.join(", ")
        );
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_service_transaction(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let txn: Option<(Option<i64>, f64)> = conn
            .query_row(
                "SELECT printing_material_id, stock_metres_used FROM service_transactions WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((Some(material_id), used_metres)) = txn {
            let current_metres_used: Option<f64> = conn
                .query_row(
                    "SELECT metres_used FROM printing_materials WHERE id = ?1",
                    params![material_id],
                    |row| row.get(0),
                )
                .ok();
            if let Some(current_used) = current_metres_used {
                let new_used = (current_used - used_metres).max(0.0);
                conn.execute(
                    "UPDATE printing_materials SET metres_used = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                    params![new_used, material_id],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        conn.execute(
            "DELETE FROM service_transactions WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Printing Materials CRUD ====================

    pub fn get_all_printing_materials(&self) -> Result<Vec<PrintingMaterial>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM printing_materials ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(PrintingMaterial {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    material_type: row.get(2)?,
                    width: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    color: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_printing_material(&self, id: i64) -> Result<Option<PrintingMaterial>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM printing_materials WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(PrintingMaterial {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    material_type: row.get(2)?,
                    width: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    color: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn add_printing_material(
        &self,
        material: NewPrintingMaterial,
    ) -> Result<PrintingMaterial, String> {
        let conn = self.synced_conn()?;
        let total_metres = material
            .total_metres
            .unwrap_or(material.rolls as f64 * material.metres_per_roll);

        conn.execute(
            "INSERT INTO printing_materials (name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
            params![material.name, material.material_type, material.width, material.rolls, material.metres_per_roll, total_metres, material.color],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        // IMPORTANT: do not call self.get_printing_material(id) here while holding the mutex,
        // because get_printing_material() also locks the same mutex and deadlocks the UI.
        let printing_material = conn.query_row(
            "SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM printing_materials WHERE id = ?1",
            params![id],
            |row| {
                Ok(PrintingMaterial {
                    id: row.get(0)?, name: row.get(1)?, material_type: row.get(2)?,
                    width: row.get(3)?, rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?, total_metres: row.get(6)?,
                    metres_used: row.get(7)?, color: row.get(8)?,
                    created_at: row.get(9)?, updated_at: row.get(10)?,
                })
            },
        ).map_err(|e| e.to_string())?;
        self.finish_write(printing_material)
    }

    pub fn update_printing_material(
        &self,
        id: i64,
        updates: PrintingMaterialUpdate,
    ) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.name {
            set_clauses.push("name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.material_type {
            set_clauses.push("material_type = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.width {
            set_clauses.push("width = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.rolls {
            set_clauses.push("rolls = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.metres_per_roll {
            set_clauses.push("metres_per_roll = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.total_metres {
            set_clauses.push("total_metres = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.metres_used {
            set_clauses.push("metres_used = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.color {
            set_clauses.push("color = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!(
            "UPDATE printing_materials SET {} WHERE id = ?",
            set_clauses.join(", ")
        );
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_printing_material(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM printing_materials WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Users (for auth) ====================

    pub fn get_user_by_username(&self, username: &str) -> Result<Option<UserRow>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, username, password_hash, role, permissions, created_at, updated_at FROM users WHERE username = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![username], |row| {
                Ok(UserRow {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    role: row.get(3)?,
                    permissions: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn add_user(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
        permissions: &str,
    ) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute(
            "INSERT INTO users (username, password_hash, role, permissions) VALUES (?1, ?2, ?3, ?4)",
            params![username, password_hash, role, permissions],
        ).map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn update_user_password(&self, username: &str, new_hash: &str) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute(
            "UPDATE users SET password_hash = ?1, updated_at = CURRENT_TIMESTAMP WHERE username = ?2",
            params![new_hash, username],
        ).map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn update_username(&self, old_username: &str, new_username: &str) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute(
            "UPDATE users SET username = ?1, updated_at = CURRENT_TIMESTAMP WHERE username = ?2",
            params![new_username, old_username],
        )
        .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn get_all_users(&self) -> Result<Vec<User>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, username, role, created_at FROM users ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    role: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn delete_user(&self, username: &str) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM users WHERE username = ?1", params![username])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Clear All Data ====================

    pub fn clear_all_data(&self) -> Result<(), String> {
        let conn = self.synced_conn()?;

        conn.execute_batch(
            "
            DELETE FROM debt_payments;
            DELETE FROM debts;
            DELETE FROM sales;
            DELETE FROM service_transactions;
            DELETE FROM services;
            DELETE FROM printing_materials;
            DELETE FROM stock;
            DELETE FROM products;
            DELETE FROM sqlite_sequence WHERE name IN ('debt_payments', 'debts', 'sales', 'service_transactions', 'services', 'printing_materials', 'stock', 'products');
            "
        ).map_err(|e| e.to_string())?;

        println!("All business data cleared");
        self.finish_write(())
    }

    // ==================== Migration from localStorage ====================

    pub fn get_dashboard_summary(&self) -> Result<DashboardSummary, String> {
        let conn = self.synced_conn()?;

        let total_revenue: f64 = conn
            .query_row(
                "SELECT
                COALESCE((SELECT SUM(amount) FROM sales), 0) +
                COALESCE((SELECT SUM(amount) FROM service_transactions), 0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let (today_sales_count, today_sales_revenue): (i64, f64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(amount), 0)
             FROM sales
             WHERE DATE(timestamp) = DATE('now', 'localtime')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let (today_service_count, today_service_revenue): (i64, f64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(amount), 0)
             FROM service_transactions
             WHERE DATE(timestamp) = DATE('now', 'localtime')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let (outstanding_debts, pending_debts_count): (f64, i64) = conn
            .query_row(
                "SELECT COALESCE(SUM(remaining_amount), 0), COUNT(*)
             FROM debts
             WHERE status = 'pending'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let recent_transactions = {
            let mut stmt = conn.prepare(
                "SELECT COALESCE(product_name, type), COALESCE(SUBSTR(timestamp, 1, 10), ''), amount, is_debt, 'Sale', COALESCE(timestamp, '')
                 FROM sales
                 UNION ALL
                 SELECT service_name, COALESCE(SUBSTR(timestamp, 1, 10), ''), amount, is_debt, 'Printing', COALESCE(timestamp, '')
                 FROM service_transactions
                 WHERE stock_metres_used > 0
                 ORDER BY 6 DESC
                 LIMIT 5"
            ).map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(DashboardRecentTransaction {
                        name: row.get(0)?,
                        date: row.get(1)?,
                        amount: row.get(2)?,
                        is_debt: row.get::<_, i64>(3)? > 0,
                        type_label: row.get(4)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let activity_items = {
            let mut stmt = conn
                .prepare(
                    "SELECT item_type, text, time FROM (
                    SELECT 'sale' AS item_type,
                           printf('%s — KSh %.0f', COALESCE(product_name, type), amount) AS text,
                           COALESCE(SUBSTR(timestamp, 12, 5), '') AS time,
                           COALESCE(timestamp, '') AS sort_ts
                    FROM sales
                    UNION ALL
                    SELECT 'debt' AS item_type,
                           printf('Debt: %s — KSh %.0f', customer_name, amount) AS text,
                           COALESCE(SUBSTR(created_at, 12, 5), '') AS time,
                           COALESCE(created_at, '') AS sort_ts
                    FROM debts
                )
                ORDER BY sort_ts DESC
                LIMIT 8",
                )
                .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(DashboardActivityItem {
                        item_type: row.get(0)?,
                        text: row.get(1)?,
                        time: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let top_products = {
            let mut stmt = conn.prepare(
                "SELECT s.product_id,
                        COALESCE(p.name, COALESCE(s.product_name, printf('Product #%d', s.product_id))),
                        CAST(SUM(COALESCE(CAST(s.quantity AS INTEGER), 1)) AS INTEGER) AS qty
                 FROM sales s
                 LEFT JOIN products p ON p.id = s.product_id
                 WHERE s.product_id IS NOT NULL
                 GROUP BY s.product_id, COALESCE(p.name, COALESCE(s.product_name, printf('Product #%d', s.product_id)))
                 ORDER BY qty DESC
                 LIMIT 4"
            ).map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(DashboardTopProduct {
                        product_id: row.get(0)?,
                        name: row.get(1)?,
                        quantity: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        Ok(DashboardSummary {
            total_revenue,
            today_sales_count: today_sales_count + today_service_count,
            today_revenue: today_sales_revenue + today_service_revenue,
            outstanding_debts,
            pending_debts_count,
            recent_transactions,
            activity_items,
            top_products,
        })
    }

    pub fn get_dashboard_chart(&self, period: &str) -> Result<Vec<DashboardChartPoint>, String> {
        let conn = self.synced_conn()?;
        let points = match period {
            "month" => {
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE weeks(idx, start_date, end_date) AS (
                        SELECT 0, DATE('now', 'localtime', '-6 days'), DATE('now', 'localtime')
                        UNION ALL
                        SELECT idx + 1,
                               DATE(start_date, '-7 days'),
                               DATE(end_date, '-7 days')
                        FROM weeks
                        WHERE idx < 3
                    ), revenues AS (
                        SELECT DATE(timestamp) AS tx_date, amount FROM sales
                        UNION ALL
                        SELECT DATE(timestamp) AS tx_date, amount FROM service_transactions
                    )
                    SELECT strftime('%d', start_date) || '–' || strftime('%d', end_date) || ' ' ||
                           CASE strftime('%m', end_date)
                               WHEN '01' THEN 'Jan'
                               WHEN '02' THEN 'Feb'
                               WHEN '03' THEN 'Mar'
                               WHEN '04' THEN 'Apr'
                               WHEN '05' THEN 'May'
                               WHEN '06' THEN 'Jun'
                               WHEN '07' THEN 'Jul'
                               WHEN '08' THEN 'Aug'
                               WHEN '09' THEN 'Sep'
                               WHEN '10' THEN 'Oct'
                               WHEN '11' THEN 'Nov'
                               WHEN '12' THEN 'Dec'
                           END AS label,
                           COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.tx_date BETWEEN w.start_date AND w.end_date), 0) AS amount,
                           idx
                    FROM weeks w
                    ORDER BY idx DESC"
                ).map_err(|e| e.to_string())?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                    })
                    .map_err(|e| e.to_string())?;

                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|(label, amount)| DashboardChartPoint { label, amount })
                    .collect()
            }
            "year" => {
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE months(idx, month_start) AS (
                        SELECT 0, DATE('now', 'localtime', 'start of month')
                        UNION ALL
                        SELECT idx + 1, DATE(month_start, '-1 month')
                        FROM months
                        WHERE idx < 11
                    ), revenues AS (
                        SELECT strftime('%Y-%m', timestamp) AS ym, amount FROM sales
                        UNION ALL
                        SELECT strftime('%Y-%m', timestamp) AS ym, amount FROM service_transactions
                    )
                    SELECT CASE strftime('%m', month_start)
                               WHEN '01' THEN 'Jan'
                               WHEN '02' THEN 'Feb'
                               WHEN '03' THEN 'Mar'
                               WHEN '04' THEN 'Apr'
                               WHEN '05' THEN 'May'
                               WHEN '06' THEN 'Jun'
                               WHEN '07' THEN 'Jul'
                               WHEN '08' THEN 'Aug'
                               WHEN '09' THEN 'Sep'
                               WHEN '10' THEN 'Oct'
                               WHEN '11' THEN 'Nov'
                               WHEN '12' THEN 'Dec'
                           END AS label,
                           COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.ym = strftime('%Y-%m', m.month_start)), 0) AS amount,
                           idx
                    FROM months m
                    ORDER BY idx DESC"
                ).map_err(|e| e.to_string())?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                    })
                    .map_err(|e| e.to_string())?;

                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|(label, amount)| DashboardChartPoint { label, amount })
                    .collect()
            }
            _ => {
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE days(idx, day_date) AS (
                        SELECT 0, DATE('now', 'localtime')
                        UNION ALL
                        SELECT idx + 1, DATE(day_date, '-1 day')
                        FROM days
                        WHERE idx < 6
                    ), revenues AS (
                        SELECT DATE(timestamp) AS tx_date, amount FROM sales
                        UNION ALL
                        SELECT DATE(timestamp) AS tx_date, amount FROM service_transactions
                    )
                    SELECT CASE strftime('%w', day_date)
                               WHEN '0' THEN 'Sun'
                               WHEN '1' THEN 'Mon'
                               WHEN '2' THEN 'Tue'
                               WHEN '3' THEN 'Wed'
                               WHEN '4' THEN 'Thu'
                               WHEN '5' THEN 'Fri'
                               WHEN '6' THEN 'Sat'
                           END AS label,
                           COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.tx_date = d.day_date), 0) AS amount,
                           idx
                    FROM days d
                    ORDER BY idx DESC"
                ).map_err(|e| e.to_string())?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                    })
                    .map_err(|e| e.to_string())?;

                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|(label, amount)| DashboardChartPoint { label, amount })
                    .collect()
            }
        };

        Ok(points)
    }

    pub fn migrate_from_localstorage(&self, data: LocalStorageData) -> Result<(), String> {
        let conn = self.synced_conn()?;

        println!("Migrating data from localStorage...");

        // Migrate products
        if let Some(products) = &data.products {
            for product in products {
                conn.execute(
                    "INSERT INTO products (name, product_type, color, size, selling_price, stock) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        product["name"].as_str().unwrap_or(""),
                        product["product_type"].as_str().unwrap_or("life_saver"),
                        product["color"].as_str(),
                        product["size"].as_str(),
                        product["selling_price"].as_f64().unwrap_or(0.0),
                        product["stock"].as_i64().unwrap_or(0),
                    ],
                ).map_err(|e| e.to_string())?;
            }
            println!("Migrated {} products", products.len());
        }

        // Migrate stock
        if let Some(stock) = &data.stock {
            for item in stock {
                conn.execute(
                    "INSERT INTO stock (color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        item["color"].as_str().unwrap_or(""),
                        item["size"].as_str().unwrap_or("1"),
                        item["sticker_type"].as_str().unwrap_or("colored"),
                        item["rolls"].as_i64().unwrap_or(0),
                        item["metres_per_roll"].as_f64().unwrap_or(50.0),
                        item["total_metres"].as_f64().unwrap_or(0.0),
                        item["metres_used"].as_f64().unwrap_or(0.0),
                    ],
                ).map_err(|e| e.to_string())?;
            }
            println!("Migrated {} stock items", stock.len());
        }

        // Migrate sales
        if let Some(sales) = &data.sales {
            for sale in sales {
                conn.execute(
                    "INSERT INTO sales (type, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        sale["type"].as_str().unwrap_or("product"),
                        sale["product_name"].as_str(),
                        sale["product_type"].as_str(),
                        sale["sticker_type"].as_str(),
                        sale["quantity"].as_str(),
                        sale["amount"].as_f64().unwrap_or(0.0),
                        sale["payment_method"].as_str().unwrap_or("cash"),
                        sale["customer_name"].as_str().unwrap_or("Walk-in"),
                        sale["timestamp"].as_str(),
                    ],
                ).map_err(|e| e.to_string())?;
            }
            println!("Migrated {} sales", sales.len());
        }

        // Migrate debts
        if let Some(debts) = &data.debts {
            for debt in debts {
                conn.execute(
                    "INSERT INTO debts (customer_name, phone, amount, due_date, description, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        debt["customer_name"].as_str().unwrap_or(""),
                        debt["phone"].as_str(),
                        debt["amount"].as_f64().unwrap_or(0.0),
                        debt["due_date"].as_str(),
                        debt["description"].as_str(),
                        debt["status"].as_str().unwrap_or("pending"),
                    ],
                ).map_err(|e| e.to_string())?;
            }
            println!("Migrated {} debts", debts.len());
        }

        println!("Migration completed!");
        self.finish_write(())
    }
}
