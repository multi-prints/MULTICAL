use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::*;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the database at the given path
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(db_path.parent().unwrap()).map_err(|e| e.to_string())?;

        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys = ON")
            .map_err(|e| e.to_string())?;

        let db = Database {
            conn: Mutex::new(conn),
        };

        db.create_tables()?;
        db.run_migrations()?;

        println!("Database initialized at: {:?}", db_path);
        Ok(db)
    }

    // ==================== Table Creation ====================

    fn create_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

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
            ",
        )
        .map_err(|e| e.to_string())?;

        println!("Database tables created");
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Check and add missing columns (same migration logic as JS version)
        let migrations = [
            ("service_transactions", "stock_id", "ALTER TABLE service_transactions ADD COLUMN stock_id INTEGER"),
            ("service_transactions", "stock_metres_used", "ALTER TABLE service_transactions ADD COLUMN stock_metres_used REAL DEFAULT 0"),
            ("service_transactions", "material_size", "ALTER TABLE service_transactions ADD COLUMN material_size TEXT"),
            ("service_transactions", "material_type", "ALTER TABLE service_transactions ADD COLUMN material_type TEXT"),
            ("service_transactions", "printing_material_id", "ALTER TABLE service_transactions ADD COLUMN printing_material_id INTEGER"),
            ("service_transactions", "is_debt", "ALTER TABLE service_transactions ADD COLUMN is_debt INTEGER DEFAULT 0"),
            ("debts", "paid_amount", "ALTER TABLE debts ADD COLUMN paid_amount REAL NOT NULL DEFAULT 0"),
            ("debts", "remaining_amount", "ALTER TABLE debts ADD COLUMN remaining_amount REAL NOT NULL DEFAULT 0"),
            ("debts", "sale_id", "ALTER TABLE debts ADD COLUMN sale_id INTEGER"),
            ("debts", "service_transaction_id", "ALTER TABLE debts ADD COLUMN service_transaction_id INTEGER"),
            ("sales", "is_debt", "ALTER TABLE sales ADD COLUMN is_debt INTEGER DEFAULT 0"),
        ];

        for (table, column, alter_sql) in &migrations {
            let has_column: bool = conn
                .prepare(&format!("PRAGMA table_info({})", table))
                .map_err(|e| e.to_string())?
                .query_map([], |row| {
                    Ok(row.get::<_, String>(1)?)
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .any(|name| name == *column);

            if !has_column {
                println!("Adding {} column to {}", column, table);
                conn.execute_batch(alter_sql).map_err(|e| e.to_string())?;
            }
        }

        // Update existing debts: remaining_amount = amount - paid_amount
        conn.execute_batch(
            "UPDATE debts SET remaining_amount = amount - COALESCE(paid_amount, 0)"
        ).map_err(|e| e.to_string())?;

        println!("Database migrations completed");
        Ok(())
    }

    // ==================== Products CRUD ====================

    pub fn get_all_products(&self) -> Result<Vec<Product>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
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

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_product(&self, id: i64) -> Result<Option<Product>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
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

        Ok(rows.next().transpose().map_err(|e| e.to_string())?)
    }

    pub fn add_product(&self, product: NewProduct) -> Result<Product, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Match Electron: find existing variant by type/color/size, update stock instead of insert
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
        conn.query_row(
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
        ).map_err(|e| e.to_string())
    }

    pub fn update_product(&self, id: i64, updates: ProductUpdate) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut sql = String::from("UPDATE products SET ");
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut first = true;

        if let Some(v) = updates.name { if !first { sql.push_str(", "); } sql.push_str("name = ?"); params.push(Box::new(v)); first = false; }
        if let Some(v) = updates.product_type { if !first { sql.push_str(", "); } sql.push_str("product_type = ?"); params.push(Box::new(v)); first = false; }
        if let Some(v) = updates.color { if !first { sql.push_str(", "); } sql.push_str("color = ?"); params.push(Box::new(v)); first = false; }
        if let Some(v) = updates.size { if !first { sql.push_str(", "); } sql.push_str("size = ?"); params.push(Box::new(v)); first = false; }
        if let Some(v) = updates.selling_price { if !first { sql.push_str(", "); } sql.push_str("selling_price = ?"); params.push(Box::new(v)); first = false; }
        if let Some(v) = updates.stock { if !first { sql.push_str(", "); } sql.push_str("stock = ?"); params.push(Box::new(v)); first = false; }

        if first { return Ok(()); }
        sql.push_str(", updated_at = CURRENT_TIMESTAMP WHERE id = ?");
        params.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_product(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM products WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ==================== Stock CRUD ====================

    pub fn get_all_stock(&self) -> Result<Vec<StockItem>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(StockItem {
                id: row.get(0)?, color: row.get(1)?, size: row.get(2)?,
                sticker_type: row.get(3)?, rolls: row.get(4)?,
                metres_per_roll: row.get(5)?, total_metres: row.get(6)?,
                metres_used: row.get(7)?, created_at: row.get(8)?, updated_at: row.get(9)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_stock(&self, id: i64) -> Result<Option<StockItem>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query_map(params![id], |row| {
            Ok(StockItem {
                id: row.get(0)?, color: row.get(1)?, size: row.get(2)?,
                sticker_type: row.get(3)?, rolls: row.get(4)?,
                metres_per_roll: row.get(5)?, total_metres: row.get(6)?,
                metres_used: row.get(7)?, created_at: row.get(8)?, updated_at: row.get(9)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.next().transpose().map_err(|e| e.to_string())?)
    }

    pub fn get_stock_by_color_size_type(&self, color: &str, size: &str, sticker_type: &str) -> Result<Option<StockItem>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock WHERE LOWER(color) = LOWER(?1) AND size = ?2 AND sticker_type = ?3")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query_map(params![color, size, sticker_type], |row| {
            Ok(StockItem {
                id: row.get(0)?, color: row.get(1)?, size: row.get(2)?,
                sticker_type: row.get(3)?, rolls: row.get(4)?,
                metres_per_roll: row.get(5)?, total_metres: row.get(6)?,
                metres_used: row.get(7)?, created_at: row.get(8)?, updated_at: row.get(9)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.next().transpose().map_err(|e| e.to_string())?)
    }

    pub fn add_stock(&self, item: NewStockItem) -> Result<StockItem, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let base_metres_per_roll = 50.0_f64;
        let (metres_per_roll, total_metres) = if item.sticker_type == "reflective" {
            if let Some(custom) = item.custom_metres_per_roll {
                (custom, item.rolls as f64 * custom)
            } else {
                (base_metres_per_roll, item.rolls as f64 * base_metres_per_roll)
            }
        } else {
            (base_metres_per_roll, item.rolls as f64 * base_metres_per_roll)
        };

        conn.execute(
            "INSERT INTO stock (color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![item.color, item.size, item.sticker_type, item.rolls, metres_per_roll, total_metres],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        // IMPORTANT: do not call self.get_stock(id) here while holding the mutex,
        // because get_stock() also locks the same mutex and deadlocks the UI.
        conn.query_row(
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
        ).map_err(|e| e.to_string())
    }

    pub fn update_stock(&self, id: i64, updates: StockUpdate) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.color { set_clauses.push("color = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.size { set_clauses.push("size = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.sticker_type { set_clauses.push("sticker_type = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.rolls { set_clauses.push("rolls = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.metres_per_roll { set_clauses.push("metres_per_roll = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.total_metres { set_clauses.push("total_metres = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.metres_used { set_clauses.push("metres_used = ?".to_string()); params_vec.push(Box::new(v)); }

        if set_clauses.is_empty() { return Ok(()); }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!("UPDATE stock SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_stock(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM stock WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    // ==================== Sales CRUD ====================

    pub fn get_all_sales(&self) -> Result<Vec<Sale>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp FROM sales ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(Sale {
                id: row.get(0)?, r#type: row.get(1)?, product_id: row.get(2)?,
                stock_id: row.get(3)?, product_name: row.get(4)?, product_type: row.get(5)?,
                sticker_type: row.get(6)?, quantity: row.get(7)?, amount: row.get(8)?,
                payment_method: row.get(9)?, customer_name: row.get(10)?,
                is_debt: row.get(11)?, timestamp: row.get(12)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_today_sales(&self) -> Result<Vec<Sale>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp FROM sales WHERE DATE(timestamp) = DATE('now', 'localtime') ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(Sale {
                id: row.get(0)?, r#type: row.get(1)?, product_id: row.get(2)?,
                stock_id: row.get(3)?, product_name: row.get(4)?, product_type: row.get(5)?,
                sticker_type: row.get(6)?, quantity: row.get(7)?, amount: row.get(8)?,
                payment_method: row.get(9)?, customer_name: row.get(10)?,
                is_debt: row.get(11)?, timestamp: row.get(12)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn add_sale(&self, sale: NewSale) -> Result<Sale, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();

        conn.execute(
            "INSERT INTO sales (type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![sale.r#type, sale.product_id, sale.stock_id, sale.product_name, sale.product_type, sale.sticker_type, sale.quantity, sale.amount, sale.payment_method, sale.customer_name, sale.is_debt, timestamp],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        Ok(Sale {
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
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let total: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0) FROM sales WHERE DATE(timestamp) = DATE('now', 'localtime')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(total)
    }

    pub fn update_sale(&self, id: i64, updates: SaleUpdate) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.r#type { set_clauses.push("type = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.amount { set_clauses.push("amount = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.payment_method { set_clauses.push("payment_method = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.customer_name { set_clauses.push("customer_name = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.is_debt { set_clauses.push("is_debt = ?".to_string()); params_vec.push(Box::new(v)); }

        if set_clauses.is_empty() { return Ok(()); }

        let sql = format!("UPDATE sales SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_sale(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM sales WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    // ==================== Debts CRUD ====================

    pub fn get_all_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at FROM debts ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(Debt {
                id: row.get(0)?, customer_name: row.get(1)?, phone: row.get(2)?,
                amount: row.get(3)?, paid_amount: row.get(4)?, remaining_amount: row.get(5)?,
                due_date: row.get(6)?, description: row.get(7)?, status: row.get(8)?,
                sale_id: row.get(9)?, service_transaction_id: row.get(10)?,
                paid_at: row.get(11)?, created_at: row.get(12)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_pending_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at FROM debts WHERE status = 'pending' ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(Debt {
                id: row.get(0)?, customer_name: row.get(1)?, phone: row.get(2)?,
                amount: row.get(3)?, paid_amount: row.get(4)?, remaining_amount: row.get(5)?,
                due_date: row.get(6)?, description: row.get(7)?, status: row.get(8)?,
                sale_id: row.get(9)?, service_transaction_id: row.get(10)?,
                paid_at: row.get(11)?, created_at: row.get(12)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn add_debt(&self, debt: NewDebt) -> Result<Debt, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let paid_amount = debt.paid_amount.unwrap_or(0.0);
        let remaining_amount = debt.remaining_amount.unwrap_or(debt.amount - paid_amount);
        let created_at = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();

        conn.execute(
            "INSERT INTO debts (customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?9, ?10)",
            params![debt.customer_name, debt.phone, debt.amount, paid_amount, remaining_amount, debt.due_date, debt.description, debt.sale_id, debt.service_transaction_id, created_at],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        Ok(Debt {
            id, customer_name: debt.customer_name, phone: debt.phone,
            amount: debt.amount, paid_amount, remaining_amount,
            due_date: debt.due_date, description: debt.description,
            status: "pending".to_string(), sale_id: debt.sale_id,
            service_transaction_id: debt.service_transaction_id,
            paid_at: None, created_at: Some(created_at),
        })
    }

    pub fn update_debt(&self, id: i64, updates: DebtUpdate) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.customer_name { set_clauses.push("customer_name = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.phone { set_clauses.push("phone = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.amount { set_clauses.push("amount = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.paid_amount { set_clauses.push("paid_amount = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.remaining_amount { set_clauses.push("remaining_amount = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.due_date { set_clauses.push("due_date = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.description { set_clauses.push("description = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.status { set_clauses.push("status = ?".to_string()); params_vec.push(Box::new(v)); }

        if set_clauses.is_empty() { return Ok(()); }

        let sql = format!("UPDATE debts SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn mark_debt_paid(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (sale_id, transaction_id): (Option<i64>, Option<i64>) = conn
            .query_row("SELECT sale_id, service_transaction_id FROM debts WHERE id = ?1", params![id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE debts SET status = 'paid', paid_at = CURRENT_TIMESTAMP, remaining_amount = 0 WHERE id = ?1",
            params![id],
        ).map_err(|e| e.to_string())?;

        if let Some(sid) = sale_id {
            conn.execute("UPDATE sales SET is_debt = 2 WHERE id = ?1", params![sid])
                .map_err(|e| e.to_string())?;
        }
        if let Some(tid) = transaction_id {
            conn.execute("UPDATE service_transactions SET is_debt = 2 WHERE id = ?1", params![tid])
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn get_debt_by_sale_id(&self, sale_id: i64) -> Result<Option<Debt>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at FROM debts WHERE sale_id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query_map(params![sale_id], |row| {
            Ok(Debt {
                id: row.get(0)?, customer_name: row.get(1)?, phone: row.get(2)?,
                amount: row.get(3)?, paid_amount: row.get(4)?, remaining_amount: row.get(5)?,
                due_date: row.get(6)?, description: row.get(7)?, status: row.get(8)?,
                sale_id: row.get(9)?, service_transaction_id: row.get(10)?,
                paid_at: row.get(11)?, created_at: row.get(12)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.next().transpose().map_err(|e| e.to_string())?)
    }

    pub fn get_debt_by_transaction_id(&self, transaction_id: i64) -> Result<Option<Debt>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at FROM debts WHERE service_transaction_id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query_map(params![transaction_id], |row| {
            Ok(Debt {
                id: row.get(0)?, customer_name: row.get(1)?, phone: row.get(2)?,
                amount: row.get(3)?, paid_amount: row.get(4)?, remaining_amount: row.get(5)?,
                due_date: row.get(6)?, description: row.get(7)?, status: row.get(8)?,
                sale_id: row.get(9)?, service_transaction_id: row.get(10)?,
                paid_at: row.get(11)?, created_at: row.get(12)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.next().transpose().map_err(|e| e.to_string())?)
    }

    pub fn delete_debt(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM debts WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_total_outstanding(&self) -> Result<f64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT COALESCE(SUM(remaining_amount), 0) FROM debts WHERE status = 'pending'",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn get_paid_this_month(&self) -> Result<f64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM debt_payments WHERE strftime('%Y-%m', payment_date) = strftime('%Y-%m', 'now')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn get_overdue_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at FROM debts WHERE status = 'pending' AND due_date IS NOT NULL AND DATE(due_date) < DATE('now')")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(Debt {
                id: row.get(0)?, customer_name: row.get(1)?, phone: row.get(2)?,
                amount: row.get(3)?, paid_amount: row.get(4)?, remaining_amount: row.get(5)?,
                due_date: row.get(6)?, description: row.get(7)?, status: row.get(8)?,
                sale_id: row.get(9)?, service_transaction_id: row.get(10)?,
                paid_at: row.get(11)?, created_at: row.get(12)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    // ==================== Debt Payments CRUD ====================

    pub fn get_debt_payments(&self, debt_id: i64) -> Result<Vec<DebtPayment>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, debt_id, amount, payment_method, notes, payment_date FROM debt_payments WHERE debt_id = ?1 ORDER BY payment_date DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![debt_id], |row| {
            Ok(DebtPayment {
                id: row.get(0)?, debt_id: row.get(1)?, amount: row.get(2)?,
                payment_method: row.get(3)?, notes: row.get(4)?, payment_date: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn add_debt_payment(&self, payment: NewDebtPayment) -> Result<DebtPayment, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let payment_date = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();

        conn.execute(
            "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![payment.debt_id, payment.amount, payment.payment_method, payment.notes, payment_date],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();

        // Update debt paid_amount, remaining_amount, status
        let debt: (f64, f64) = conn.query_row(
            "SELECT paid_amount, amount FROM debts WHERE id = ?1",
            params![payment.debt_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| e.to_string())?;

        let new_paid_amount = debt.0 + payment.amount;
        let new_remaining = (debt.1 - new_paid_amount).max(0.0);
        let new_status = if new_remaining <= 0.0 { "paid" } else { "pending" };
        let paid_at_value = if new_remaining <= 0.0 { Some(payment_date.clone()) } else { None };

        conn.execute(
            "UPDATE debts SET paid_amount = ?1, remaining_amount = ?2, status = ?3, paid_at = COALESCE(?4, paid_at) WHERE id = ?5",
            params![new_paid_amount, new_remaining, new_status, paid_at_value, payment.debt_id],
        ).map_err(|e| e.to_string())?;

        Ok(DebtPayment {
            id, debt_id: payment.debt_id, amount: payment.amount,
            payment_method: payment.payment_method, notes: payment.notes,
            payment_date: Some(payment_date),
        })
    }

    pub fn delete_debt_payment(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let payment: (i64, f64) = conn.query_row(
            "SELECT debt_id, amount FROM debt_payments WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| e.to_string())?;

        let debt: (f64, f64) = conn.query_row(
            "SELECT paid_amount, amount FROM debts WHERE id = ?1",
            params![payment.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| e.to_string())?;

        let new_paid_amount = (debt.0 - payment.1).max(0.0);
        let new_remaining = debt.1 - new_paid_amount;
        let new_status = if new_remaining > 0.0 { "pending" } else { "paid" };

        conn.execute(
            "UPDATE debts SET paid_amount = ?1, remaining_amount = ?2, status = ?3 WHERE id = ?4",
            params![new_paid_amount, new_remaining, new_status, payment.0],
        ).map_err(|e| e.to_string())?;

        conn.execute("DELETE FROM debt_payments WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    // ==================== Services CRUD ====================

    pub fn get_all_services(&self) -> Result<Vec<Service>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(Service {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                price: row.get(3)?, unit: row.get(4)?, uses_stock: row.get(5)?,
                is_active: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_active_services(&self) -> Result<Vec<Service>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE is_active = 1 ORDER BY name")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(Service {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                price: row.get(3)?, unit: row.get(4)?, uses_stock: row.get(5)?,
                is_active: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_service(&self, id: i64) -> Result<Option<Service>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Service {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                price: row.get(3)?, unit: row.get(4)?, uses_stock: row.get(5)?,
                is_active: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.next().transpose().map_err(|e| e.to_string())?)
    }

    pub fn add_service(&self, service: NewService) -> Result<Service, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let created_at = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();

        conn.execute(
            "INSERT INTO services (name, description, price, unit, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![service.name, service.description, service.price.unwrap_or(0.0), service.unit, service.is_active, created_at],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        // IMPORTANT: do not call self.get_service(id) here while holding the mutex,
        // because get_service() also locks the same mutex and deadlocks the UI.
        conn.query_row(
            "SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE id = ?1",
            params![id],
            |row| {
                Ok(Service {
                    id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                    price: row.get(3)?, unit: row.get(4)?, uses_stock: row.get(5)?,
                    is_active: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
                })
            },
        ).map_err(|e| e.to_string())
    }

    pub fn update_service(&self, id: i64, updates: ServiceUpdate) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.name { set_clauses.push("name = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.description { set_clauses.push("description = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.price { set_clauses.push("price = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.unit { set_clauses.push("unit = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.is_active { set_clauses.push("is_active = ?".to_string()); params_vec.push(Box::new(v)); }

        if set_clauses.is_empty() { return Ok(()); }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!("UPDATE services SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_service(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM services WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    // ==================== Service Transactions CRUD ====================

    pub fn get_all_service_transactions(&self) -> Result<Vec<ServiceTransaction>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp FROM service_transactions ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(ServiceTransaction {
                id: row.get(0)?, service_id: row.get(1)?, service_name: row.get(2)?,
                quantity: row.get(3)?, price: row.get(4)?, amount: row.get(5)?,
                payment_method: row.get(6)?, customer_name: row.get(7)?,
                notes: row.get(8)?, stock_id: row.get(9)?,
                stock_metres_used: row.get(10)?, material_size: row.get(11)?,
                material_type: row.get(12)?, printing_material_id: row.get(13)?,
                is_debt: row.get(14)?, timestamp: row.get(15)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_today_service_transactions(&self) -> Result<Vec<ServiceTransaction>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp FROM service_transactions WHERE DATE(timestamp) = DATE('now', 'localtime') ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(ServiceTransaction {
                id: row.get(0)?, service_id: row.get(1)?, service_name: row.get(2)?,
                quantity: row.get(3)?, price: row.get(4)?, amount: row.get(5)?,
                payment_method: row.get(6)?, customer_name: row.get(7)?,
                notes: row.get(8)?, stock_id: row.get(9)?,
                stock_metres_used: row.get(10)?, material_size: row.get(11)?,
                material_type: row.get(12)?, printing_material_id: row.get(13)?,
                is_debt: row.get(14)?, timestamp: row.get(15)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn add_service_transaction(&self, tx: NewServiceTransaction) -> Result<ServiceTransaction, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
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
                    ).ok();
                if let Some(current_used) = current_metres_used {
                    let new_metres_used = current_used + stock_metres;
                    conn.execute(
                        "UPDATE stock SET metres_used = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                        params![new_metres_used, stock_id],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(ServiceTransaction {
            id, service_id: tx.service_id, service_name: tx.service_name,
            quantity: tx.quantity, price: tx.price.unwrap_or(0.0), amount,
            payment_method: tx.payment_method, customer_name: tx.customer_name,
            notes: tx.notes, stock_id: tx.stock_id,
            stock_metres_used: tx.stock_metres_used,
            material_size: tx.material_size, material_type: tx.material_type,
            printing_material_id: tx.printing_material_id,
            is_debt: tx.is_debt, timestamp: Some(timestamp),
        })
    }

    pub fn get_today_total_service_earnings(&self) -> Result<f64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM service_transactions WHERE DATE(timestamp) = DATE('now', 'localtime')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn get_total_service_earnings(&self) -> Result<f64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM service_transactions",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn update_service_transaction(&self, id: i64, updates: ServiceTransactionUpdate) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.service_name { set_clauses.push("service_name = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.quantity { set_clauses.push("quantity = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.price { set_clauses.push("price = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.amount { set_clauses.push("amount = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.payment_method { set_clauses.push("payment_method = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.customer_name { set_clauses.push("customer_name = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.notes { set_clauses.push("notes = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.stock_id { set_clauses.push("stock_id = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.stock_metres_used { set_clauses.push("stock_metres_used = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.material_size { set_clauses.push("material_size = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.material_type { set_clauses.push("material_type = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.is_debt { set_clauses.push("is_debt = ?".to_string()); params_vec.push(Box::new(v)); }

        if set_clauses.is_empty() { return Ok(()); }

        let sql = format!("UPDATE service_transactions SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_service_transaction(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM service_transactions WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ==================== Printing Materials CRUD ====================

    pub fn get_all_printing_materials(&self) -> Result<Vec<PrintingMaterial>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM printing_materials ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(PrintingMaterial {
                id: row.get(0)?, name: row.get(1)?, material_type: row.get(2)?,
                width: row.get(3)?, rolls: row.get(4)?, metres_per_roll: row.get(5)?,
                total_metres: row.get(6)?, metres_used: row.get(7)?,
                color: row.get(8)?, created_at: row.get(9)?, updated_at: row.get(10)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_printing_material(&self, id: i64) -> Result<Option<PrintingMaterial>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM printing_materials WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query_map(params![id], |row| {
            Ok(PrintingMaterial {
                id: row.get(0)?, name: row.get(1)?, material_type: row.get(2)?,
                width: row.get(3)?, rolls: row.get(4)?, metres_per_roll: row.get(5)?,
                total_metres: row.get(6)?, metres_used: row.get(7)?,
                color: row.get(8)?, created_at: row.get(9)?, updated_at: row.get(10)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.next().transpose().map_err(|e| e.to_string())?)
    }

    pub fn add_printing_material(&self, material: NewPrintingMaterial) -> Result<PrintingMaterial, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let total_metres = material.total_metres.unwrap_or(material.rolls as f64 * material.metres_per_roll);

        conn.execute(
            "INSERT INTO printing_materials (name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
            params![material.name, material.material_type, material.width, material.rolls, material.metres_per_roll, total_metres, material.color],
        ).map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        // IMPORTANT: do not call self.get_printing_material(id) here while holding the mutex,
        // because get_printing_material() also locks the same mutex and deadlocks the UI.
        conn.query_row(
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
        ).map_err(|e| e.to_string())
    }

    pub fn update_printing_material(&self, id: i64, updates: PrintingMaterialUpdate) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.name { set_clauses.push("name = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.material_type { set_clauses.push("material_type = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.width { set_clauses.push("width = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.rolls { set_clauses.push("rolls = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.metres_per_roll { set_clauses.push("metres_per_roll = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.total_metres { set_clauses.push("total_metres = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.metres_used { set_clauses.push("metres_used = ?".to_string()); params_vec.push(Box::new(v)); }
        if let Some(v) = updates.color { set_clauses.push("color = ?".to_string()); params_vec.push(Box::new(v)); }

        if set_clauses.is_empty() { return Ok(()); }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!("UPDATE printing_materials SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_printing_material(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM printing_materials WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ==================== Users (for auth) ====================

    pub fn get_user_by_username(&self, username: &str) -> Result<Option<UserRow>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, username, password_hash, role, permissions, created_at, updated_at FROM users WHERE username = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query_map(params![username], |row| {
            Ok(UserRow {
                id: row.get(0)?, username: row.get(1)?, password_hash: row.get(2)?,
                role: row.get(3)?, permissions: row.get(4)?, created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;

        Ok(rows.next().transpose().map_err(|e| e.to_string())?)
    }

    pub fn add_user(&self, username: &str, password_hash: &str, role: &str, permissions: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO users (username, password_hash, role, permissions) VALUES (?1, ?2, ?3, ?4)",
            params![username, password_hash, role, permissions],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_user_password(&self, username: &str, new_hash: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE users SET password_hash = ?1, updated_at = CURRENT_TIMESTAMP WHERE username = ?2",
            params![new_hash, username],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_username(&self, old_username: &str, new_username: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE users SET username = ?1, updated_at = CURRENT_TIMESTAMP WHERE username = ?2",
            params![new_username, old_username],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_all_users(&self) -> Result<Vec<User>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, username, role, created_at FROM users ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?, username: row.get(1)?, role: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn delete_user(&self, username: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM users WHERE username = ?1", params![username])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ==================== Clear All Data ====================

    pub fn clear_all_data(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

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
        Ok(())
    }

    // ==================== Migration from localStorage ====================

    pub fn migrate_from_localstorage(&self, data: LocalStorageData) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

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
        Ok(())
    }
}
