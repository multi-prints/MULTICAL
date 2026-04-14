/**
 * Database Module
 * SQLite database using better-sqlite3 for persistent storage
 */

const Database = require('better-sqlite3');
const path = require('path');
const { app } = require('electron');

class DatabaseManager {
  constructor() {
    this.db = null;
  }

  /**
   * Initialize the database
   * Creates tables if they don't exist
   */
  init() {
    // Get the user data path for storing the database
    const userDataPath = app.getPath('userData');
    const dbPath = path.join(userDataPath, 'multiprints.db');
    
    console.log('Database path:', dbPath);
    
    // Open/create the database
    this.db = new Database(dbPath);
    
    // Enable foreign keys
    this.db.pragma('foreign_keys = ON');
    
    // Create tables
    this.createTables();
    
    console.log('Database initialized successfully');
    return this;
  }

  /**
   * Create database tables
   */
  createTables() {
    // Products table (Life Savers, Chevrons)
    this.db.exec(`
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
      )
    `);

    // Stock table (Sticker rolls)
    this.db.exec(`
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
      )
    `);

    // Sales table
    this.db.exec(`
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
        timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE SET NULL,
        FOREIGN KEY (stock_id) REFERENCES stock(id) ON DELETE SET NULL
      )
    `);

    // Debts table
    this.db.exec(`
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
        paid_at DATETIME,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
      )
    `);

    // Debt payments table (for tracking installments)
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS debt_payments (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        debt_id INTEGER NOT NULL,
        amount REAL NOT NULL DEFAULT 0,
        payment_method TEXT NOT NULL DEFAULT 'cash',
        notes TEXT,
        payment_date DATETIME DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (debt_id) REFERENCES debts(id) ON DELETE CASCADE
      )
    `);

    // Services table
    this.db.exec(`
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
      )
    `);

    // Service transactions table
    this.db.exec(`
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
        timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE SET NULL,
        FOREIGN KEY (stock_id) REFERENCES stock(id) ON DELETE SET NULL
      )
    `);

    // Printing materials table (for non-sticker materials like banners, satin)
    this.db.exec(`
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
      )
    `);

    // Users table
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT UNIQUE NOT NULL,
        password_hash TEXT NOT NULL,
        role TEXT NOT NULL DEFAULT 'employee',
        permissions TEXT,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
      )
    `);

    console.log('Database tables created');
    
    // Run migrations to add any missing columns
    this.runMigrations();
  }

  /**
   * Run database migrations to add missing columns
   */
  runMigrations() {
    try {
      // Check if stock_id column exists in service_transactions
      const columns = this.db.prepare("PRAGMA table_info(service_transactions)").all();
      const hasStockId = columns.some(col => col.name === 'stock_id');
      const hasStockMetresUsed = columns.some(col => col.name === 'stock_metres_used');
      const hasMaterialSize = columns.some(col => col.name === 'material_size');
      const hasMaterialType = columns.some(col => col.name === 'material_type');
      const hasPrintingMaterialId = columns.some(col => col.name === 'printing_material_id');
      
      if (!hasStockId) {
        console.log('Adding stock_id column to service_transactions');
        this.db.exec('ALTER TABLE service_transactions ADD COLUMN stock_id INTEGER');
      }
      
      if (!hasStockMetresUsed) {
        console.log('Adding stock_metres_used column to service_transactions');
        this.db.exec('ALTER TABLE service_transactions ADD COLUMN stock_metres_used REAL DEFAULT 0');
      }
      
      if (!hasMaterialSize) {
        console.log('Adding material_size column to service_transactions');
        this.db.exec('ALTER TABLE service_transactions ADD COLUMN material_size TEXT');
      }
      
      if (!hasMaterialType) {
        console.log('Adding material_type column to service_transactions');
        this.db.exec('ALTER TABLE service_transactions ADD COLUMN material_type TEXT');
      }
      
      if (!hasPrintingMaterialId) {
        console.log('Adding printing_material_id column to service_transactions');
        this.db.exec('ALTER TABLE service_transactions ADD COLUMN printing_material_id INTEGER');
      }
      
      // Check debts table columns
      const debtColumns = this.db.prepare("PRAGMA table_info(debts)").all();
      const hasPaidAmount = debtColumns.some(col => col.name === 'paid_amount');
      const hasRemainingAmount = debtColumns.some(col => col.name === 'remaining_amount');
      
      if (!hasPaidAmount) {
        console.log('Adding paid_amount column to debts');
        this.db.exec('ALTER TABLE debts ADD COLUMN paid_amount REAL NOT NULL DEFAULT 0');
      }
      
      // Check if sale_id exists in debts
      const debtCols = this.db.prepare("PRAGMA table_info(debts)").all();
      const hasSaleId = debtCols.some(col => col.name === 'sale_id');
      const hasStId = debtCols.some(col => col.name === 'service_transaction_id');
      
      if (!hasSaleId) {
        console.log('Adding sale_id column to debts');
        this.db.exec('ALTER TABLE debts ADD COLUMN sale_id INTEGER');
      }
      if (!hasStId) {
        console.log('Adding service_transaction_id column to debts');
        this.db.exec('ALTER TABLE debts ADD COLUMN service_transaction_id INTEGER');
      }
      
      // Update existing debts: remaining_amount = amount - paid_amount
      this.db.exec('UPDATE debts SET remaining_amount = amount - COALESCE(paid_amount, 0)');
      
      // Check if is_debt column exists in sales
      const salesColumns = this.db.prepare("PRAGMA table_info(sales)").all();
      const hasSaleIsDebt = salesColumns.some(col => col.name === 'is_debt');
      if (!hasSaleIsDebt) {
        console.log('Adding is_debt column to sales');
        this.db.exec('ALTER TABLE sales ADD COLUMN is_debt INTEGER DEFAULT 0');
      }

      // Check if is_debt column exists in service_transactions
      const stColumns = this.db.prepare("PRAGMA table_info(service_transactions)").all();
      const hasStIsDebt = stColumns.some(col => col.name === 'is_debt');
      if (!hasStIsDebt) {
        console.log('Adding is_debt column to service_transactions');
        this.db.exec('ALTER TABLE service_transactions ADD COLUMN is_debt INTEGER DEFAULT 0');
      }
      
      console.log('Database migrations completed');
    } catch (error) {
      console.error('Migration error:', error);
    }
  }

  // ==================== Products CRUD ====================

  getAllProducts() {
    return this.db.prepare('SELECT * FROM products ORDER BY created_at DESC').all();
  }

  getProduct(id) {
    return this.db.prepare('SELECT * FROM products WHERE id = ?').get(id);
  }

  addProduct(product) {
    const stmt = this.db.prepare(`
      INSERT INTO products (name, product_type, color, size, selling_price, stock)
      VALUES (@name, @product_type, @color, @size, @selling_price, @stock)
    `);
    const result = stmt.run(product);
    return { ...product, id: result.lastInsertRowid };
  }

  updateProduct(id, updates) {
    const fields = Object.keys(updates).map(key => `${key} = @${key}`).join(', ');
    const stmt = this.db.prepare(`UPDATE products SET ${fields}, updated_at = CURRENT_TIMESTAMP WHERE id = @id`);
    stmt.run({ ...updates, id });
  }

  deleteProduct(id) {
    this.db.prepare('DELETE FROM products WHERE id = ?').run(id);
  }

  // ... (existing stock methods) ...

  // ==================== Sales CRUD ====================

  getAllSales() {
    return this.db.prepare('SELECT * FROM sales ORDER BY timestamp DESC').all();
  }

  getTodaySales() {
    return this.db.prepare(`
      SELECT * FROM sales 
      WHERE DATE(timestamp) = DATE('now', 'localtime')
      ORDER BY timestamp DESC
    `).all();
  }

  addSale(sale) {
    const timestamp = new Date().toISOString();
    const stmt = this.db.prepare(`
      INSERT INTO sales (type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp)
      VALUES (@type, @product_id, @stock_id, @product_name, @product_type, @sticker_type, @quantity, @amount, @payment_method, @customer_name, @is_debt, @timestamp)
    `);
    const result = stmt.run({
      type: sale.type,
      product_id: sale.product_id || null,
      stock_id: sale.stock_id || null,
      product_name: sale.product_name || null,
      product_type: sale.product_type || null,
      sticker_type: sale.sticker_type || null,
      quantity: sale.quantity ? String(sale.quantity) : null,
      amount: sale.amount || 0,
      payment_method: sale.payment_method || 'cash',
      customer_name: sale.customer_name || 'Walk-in',
      is_debt: sale.is_debt || 0,
      timestamp: timestamp
    });
    return { ...sale, id: result.lastInsertRowid, timestamp: timestamp };
  }

  updateSale(id, updates) {
    const fields = Object.keys(updates).map(key => `${key} = @${key}`).join(', ');
    const stmt = this.db.prepare(`UPDATE sales SET ${fields} WHERE id = @id`);
    stmt.run({ ...updates, id });
  }

  getTodayTotalSales() {
    const result = this.db.prepare(`
      SELECT COALESCE(SUM(amount), 0) as total 
      FROM sales 
      WHERE DATE(timestamp) = DATE('now', 'localtime')
    `).get();
    return result.total;
  }

  deleteSale(id) {
    this.db.prepare('DELETE FROM sales WHERE id = ?').run(id);
  }

  // ... (existing debts methods) ...

  // ==================== Service Transactions CRUD ====================

  getAllServiceTransactions() {
    return this.db.prepare('SELECT * FROM service_transactions ORDER BY timestamp DESC').all();
  }

  getTodayServiceTransactions() {
    return this.db.prepare(`
      SELECT * FROM service_transactions 
      WHERE DATE(timestamp) = DATE('now', 'localtime')
      ORDER BY timestamp DESC
    `).all();
  }

  addServiceTransaction(transaction) {
    const timestamp = new Date().toISOString();
    const stmt = this.db.prepare(`
      INSERT INTO service_transactions (
        service_id, service_name, quantity, price, amount, payment_method, customer_name, notes,
        stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp
      )
      VALUES (
        @service_id, @service_name, @quantity, @price, @amount, @payment_method, @customer_name, @notes,
        @stock_id, @stock_metres_used, @material_size, @material_type, @printing_material_id, @is_debt, @timestamp
      )
    `);
    const result = stmt.run({
      service_id: transaction.service_id || null,
      service_name: transaction.service_name,
      quantity: transaction.quantity || 1,
      price: transaction.price || 0,
      amount: transaction.amount || 0,
      payment_method: transaction.payment_method || 'cash',
      customer_name: transaction.customer_name || 'Walk-in',
      notes: transaction.notes || null,
      stock_id: transaction.stock_id || null,
      stock_metres_used: transaction.stock_metres_used || 0,
      material_size: transaction.material_size || null,
      material_type: transaction.material_type || null,
      printing_material_id: transaction.printing_material_id || null,
      is_debt: transaction.is_debt || 0,
      timestamp: timestamp
    });
    
    // If stock was used, update the stock metres_used
    if (transaction.stock_id && transaction.stock_metres_used > 0) {
      const stock = this.getStock(transaction.stock_id);
      if (stock) {
        const newMetresUsed = stock.metres_used + transaction.stock_metres_used;
        this.updateStock(transaction.stock_id, { metres_used: newMetresUsed });
      }
    }
    
    return { ...transaction, id: result.lastInsertRowid, timestamp: timestamp };
  }

  updateServiceTransaction(id, updates) {
    const fields = Object.keys(updates).map(key => `${key} = @${key}`).join(', ');
    const stmt = this.db.prepare(`UPDATE service_transactions SET ${fields} WHERE id = @id`);
    stmt.run({ ...updates, id });
  }

  // ==================== Stock CRUD ====================

  getAllStock() {
    return this.db.prepare('SELECT * FROM stock ORDER BY created_at DESC').all();
  }

  getStock(id) {
    return this.db.prepare('SELECT * FROM stock WHERE id = ?').get(id);
  }

  getStockByColorSizeAndType(color, size, stickerType) {
    return this.db.prepare(`
      SELECT * FROM stock 
      WHERE LOWER(color) = LOWER(?) AND size = ? AND sticker_type = ?
    `).get(color, size, stickerType);
  }

  addStock(stockItem) {
    const stmt = this.db.prepare(`
      INSERT INTO stock (color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used)
      VALUES (@color, @size, @sticker_type, @rolls, @metres_per_roll, @total_metres, @metres_used)
    `);
    const result = stmt.run(stockItem);
    return { ...stockItem, id: result.lastInsertRowid };
  }

  updateStock(id, updates) {
    const fields = Object.keys(updates).map(key => `${key} = @${key}`).join(', ');
    const stmt = this.db.prepare(`UPDATE stock SET ${fields}, updated_at = CURRENT_TIMESTAMP WHERE id = @id`);
    stmt.run({ ...updates, id });
  }

  deleteStock(id) {
    this.db.prepare('DELETE FROM stock WHERE id = ?').run(id);
  }

  // ==================== Sales CRUD ====================

  getAllSales() {
    return this.db.prepare('SELECT * FROM sales ORDER BY timestamp DESC').all();
  }

  getTodaySales() {
    return this.db.prepare(`
      SELECT * FROM sales 
      WHERE DATE(timestamp) = DATE('now', 'localtime')
      ORDER BY timestamp DESC
    `).all();
  }

  addSale(sale) {
    const timestamp = new Date().toISOString();
    const stmt = this.db.prepare(`
      INSERT INTO sales (type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, timestamp)
      VALUES (@type, @product_id, @stock_id, @product_name, @product_type, @sticker_type, @quantity, @amount, @payment_method, @customer_name, @timestamp)
    `);
    const result = stmt.run({
      type: sale.type,
      product_id: sale.product_id || null,
      stock_id: sale.stock_id || null,
      product_name: sale.product_name || null,
      product_type: sale.product_type || null,
      sticker_type: sale.sticker_type || null,
      quantity: sale.quantity ? String(sale.quantity) : null,
      amount: sale.amount || 0,
      payment_method: sale.payment_method || 'cash',
      customer_name: sale.customer_name || 'Walk-in',
      timestamp: timestamp
    });
    return { ...sale, id: result.lastInsertRowid, timestamp: timestamp };
  }

  getTodayTotalSales() {
    const result = this.db.prepare(`
      SELECT COALESCE(SUM(amount), 0) as total 
      FROM sales 
      WHERE DATE(timestamp) = DATE('now', 'localtime')
    `).get();
    return result.total;
  }

  deleteSale(id) {
    this.db.prepare('DELETE FROM sales WHERE id = ?').run(id);
  }

  // ==================== Debts CRUD ====================

  getAllDebts() {
    return this.db.prepare('SELECT * FROM debts ORDER BY created_at DESC').all();
  }

  getPendingDebts() {
    return this.db.prepare("SELECT * FROM debts WHERE status = 'pending' ORDER BY created_at DESC").all();
  }

  addDebt(debt) {
    // Calculate paid_amount and remaining_amount if not provided
    const paidAmount = debt.paid_amount !== undefined ? debt.paid_amount : 0;
    const remainingAmount = debt.remaining_amount !== undefined ? debt.remaining_amount : (debt.amount - paidAmount);
    const createdAt = new Date().toISOString();
    
    const stmt = this.db.prepare(`
      INSERT INTO debts (customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, created_at)
      VALUES (@customer_name, @phone, @amount, @paid_amount, @remaining_amount, @due_date, @description, 'pending', @sale_id, @service_transaction_id, @created_at)
    `);
    const result = stmt.run({
      customer_name: debt.customer_name,
      phone: debt.phone || null,
      amount: debt.amount,
      paid_amount: paidAmount,
      remaining_amount: remainingAmount,
      due_date: debt.due_date || null,
      description: debt.description || null,
      sale_id: debt.sale_id || null,
      service_transaction_id: debt.service_transaction_id || null,
      created_at: createdAt
    });
    return { ...debt, id: result.lastInsertRowid, status: 'pending', paid_amount: paidAmount, remaining_amount: remainingAmount, created_at: createdAt };
  }

  updateDebt(id, updates) {
    const fields = Object.keys(updates).map(key => `${key} = @${key}`).join(', ');
    const stmt = this.db.prepare(`UPDATE debts SET ${fields} WHERE id = @id`);
    stmt.run({ ...updates, id });
  }

  getDebtBySaleId(saleId) {
    return this.db.prepare('SELECT * FROM debts WHERE sale_id = ?').get(saleId);
  }

  getDebtByTransactionId(transactionId) {
    return this.db.prepare('SELECT * FROM debts WHERE service_transaction_id = ?').get(transactionId);
  }

  markDebtPaid(id) {
    const debt = this.db.prepare('SELECT sale_id, service_transaction_id FROM debts WHERE id = ?').get(id);
    
    this.db.prepare("UPDATE debts SET status = 'paid', paid_at = CURRENT_TIMESTAMP, remaining_amount = 0 WHERE id = ?").run(id);
    
    if (debt) {
      if (debt.sale_id) {
        this.db.prepare('UPDATE sales SET is_debt = 2 WHERE id = ?').run(debt.sale_id);
      } else if (debt.service_transaction_id) {
        this.db.prepare('UPDATE service_transactions SET is_debt = 2 WHERE id = ?').run(debt.service_transaction_id);
      }
    }
  }

  deleteDebt(id) {
    this.db.prepare('DELETE FROM debts WHERE id = ?').run(id);
  }

  getTotalOutstanding() {
    const result = this.db.prepare("SELECT COALESCE(SUM(remaining_amount), 0) as total FROM debts WHERE status = 'pending'").get();
    return result.total;
  }

  getPaidThisMonth() {
    const result = this.db.prepare(`
      SELECT COALESCE(SUM(amount), 0) as total 
      FROM debt_payments 
      WHERE strftime('%Y-%m', payment_date) = strftime('%Y-%m', 'now')
    `).get();
    return result.total;
  }

  getOverdueDebts() {
    return this.db.prepare(`
      SELECT * FROM debts 
      WHERE status = 'pending' 
      AND due_date IS NOT NULL 
      AND DATE(due_date) < DATE('now')
    `).all();
  }

  // ==================== Debt Payments CRUD ====================

  getDebtPayments(debtId) {
    return this.db.prepare('SELECT * FROM debt_payments WHERE debt_id = ? ORDER BY payment_date DESC').all(debtId);
  }

  addDebtPayment(payment) {
    const paymentDate = new Date().toISOString();
    const stmt = this.db.prepare(`
      INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date)
      VALUES (@debt_id, @amount, @payment_method, @notes, @payment_date)
    `);
    const result = stmt.run({
      debt_id: payment.debt_id,
      amount: payment.amount,
      payment_method: payment.payment_method || 'cash',
      notes: payment.notes || null,
      payment_date: paymentDate
    });
    
    // Update debt paid_amount and remaining_amount
    const debt = this.db.prepare('SELECT * FROM debts WHERE id = ?').get(payment.debt_id);
    if (debt) {
      const newPaidAmount = debt.paid_amount + payment.amount;
      const newRemainingAmount = Math.max(0, debt.amount - newPaidAmount);
      const newStatus = newRemainingAmount <= 0 ? 'paid' : 'pending';
      
      this.db.prepare(`
        UPDATE debts 
        SET paid_amount = ?, remaining_amount = ?, status = ?, paid_at = CASE WHEN ? = 'paid' THEN ? ELSE paid_at END
        WHERE id = ?
      `).run(newPaidAmount, newRemainingAmount, newStatus, newStatus, paymentDate, payment.debt_id);
    }
    
    return { ...payment, id: result.lastInsertRowid, payment_date: paymentDate };
  }

  deleteDebtPayment(id) {
    // Get payment details before deleting
    const payment = this.db.prepare('SELECT * FROM debt_payments WHERE id = ?').get(id);
    if (payment) {
      // Reverse the payment from debt
      const debt = this.db.prepare('SELECT * FROM debts WHERE id = ?').get(payment.debt_id);
      if (debt) {
        const newPaidAmount = Math.max(0, debt.paid_amount - payment.amount);
        const newRemainingAmount = debt.amount - newPaidAmount;
        const newStatus = newRemainingAmount > 0 ? 'pending' : 'paid';
        
        this.db.prepare(`
          UPDATE debts 
          SET paid_amount = ?, remaining_amount = ?, status = ?
          WHERE id = ?
        `).run(newPaidAmount, newRemainingAmount, newStatus, payment.debt_id);
      }
      
      // Delete the payment
      this.db.prepare('DELETE FROM debt_payments WHERE id = ?').run(id);
    }
  }

  // ==================== Services CRUD ====================

  getAllServices() {
    return this.db.prepare('SELECT * FROM services ORDER BY created_at DESC').all();
  }

  getActiveServices() {
    return this.db.prepare('SELECT * FROM services WHERE is_active = 1 ORDER BY name').all();
  }

  getService(id) {
    return this.db.prepare('SELECT * FROM services WHERE id = ?').get(id);
  }

  addService(service) {
    const createdAt = new Date().toISOString();
    const stmt = this.db.prepare(`
      INSERT INTO services (name, description, price, unit, is_active, created_at)
      VALUES (@name, @description, @price, @unit, @is_active, @created_at)
    `);
    const result = stmt.run({
      name: service.name,
      description: service.description || null,
      price: service.price || 0,
      unit: service.unit || null,
      is_active: service.is_active !== undefined ? service.is_active : 1,
      created_at: createdAt
    });
    return { ...service, id: result.lastInsertRowid, created_at: createdAt };
  }

  updateService(id, updates) {
    const fields = Object.keys(updates).map(key => `${key} = @${key}`).join(', ');
    const stmt = this.db.prepare(`UPDATE services SET ${fields}, updated_at = CURRENT_TIMESTAMP WHERE id = @id`);
    stmt.run({ ...updates, id });
  }

  deleteService(id) {
    this.db.prepare('DELETE FROM services WHERE id = ?').run(id);
  }

  // ==================== Service Transactions CRUD ====================

  getAllServiceTransactions() {
    return this.db.prepare('SELECT * FROM service_transactions ORDER BY timestamp DESC').all();
  }

  getTodayServiceTransactions() {
    return this.db.prepare(`
      SELECT * FROM service_transactions 
      WHERE DATE(timestamp) = DATE('now', 'localtime')
      ORDER BY timestamp DESC
    `).all();
  }

  addServiceTransaction(transaction) {
    const timestamp = new Date().toISOString();
    const stmt = this.db.prepare(`
      INSERT INTO service_transactions (
        service_id, service_name, quantity, price, amount, payment_method, customer_name, notes,
        stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp
      )
      VALUES (
        @service_id, @service_name, @quantity, @price, @amount, @payment_method, @customer_name, @notes,
        @stock_id, @stock_metres_used, @material_size, @material_type, @printing_material_id, @is_debt, @timestamp
      )
    `);
    const result = stmt.run({
      service_id: transaction.service_id || null,
      service_name: transaction.service_name,
      quantity: transaction.quantity || 1,
      price: transaction.price || 0,
      amount: transaction.amount || 0,
      payment_method: transaction.payment_method || 'cash',
      customer_name: transaction.customer_name || 'Walk-in',
      notes: transaction.notes || null,
      stock_id: transaction.stock_id || null,
      stock_metres_used: transaction.stock_metres_used || 0,
      material_size: transaction.material_size || null,
      material_type: transaction.material_type || null,
      printing_material_id: transaction.printing_material_id || null,
      is_debt: transaction.is_debt || 0,
      timestamp: timestamp
    });
    
    // If stock was used, update the stock metres_used
    if (transaction.stock_id && transaction.stock_metres_used > 0) {
      const stock = this.getStock(transaction.stock_id);
      if (stock) {
        const newMetresUsed = stock.metres_used + transaction.stock_metres_used;
        this.updateStock(transaction.stock_id, { metres_used: newMetresUsed });
      }
    }
    
    return { ...transaction, id: result.lastInsertRowid, timestamp: timestamp };
  }

  getTodayTotalServiceEarnings() {
    const result = this.db.prepare(`
      SELECT COALESCE(SUM(amount), 0) as total 
      FROM service_transactions 
      WHERE DATE(timestamp) = DATE('now', 'localtime')
    `).get();
    return result.total;
  }

  getTotalServiceEarnings() {
    const result = this.db.prepare(`
      SELECT COALESCE(SUM(amount), 0) as total 
      FROM service_transactions
    `).get();
    return result.total;
  }

  deleteServiceTransaction(id) {
    this.db.prepare('DELETE FROM service_transactions WHERE id = ?').run(id);
  }

  // ==================== Printing Materials CRUD ====================

  getAllPrintingMaterials() {
    return this.db.prepare('SELECT * FROM printing_materials ORDER BY created_at DESC').all();
  }

  getPrintingMaterial(id) {
    return this.db.prepare('SELECT * FROM printing_materials WHERE id = ?').get(id);
  }

  addPrintingMaterial(material) {
    const stmt = this.db.prepare(`
      INSERT INTO printing_materials (name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color)
      VALUES (@name, @material_type, @width, @rolls, @metres_per_roll, @total_metres, @metres_used, @color)
    `);
    const result = stmt.run(material);
    return { ...material, id: result.lastInsertRowid };
  }

  updatePrintingMaterial(id, updates) {
    const fields = Object.keys(updates).map(key => `${key} = @${key}`).join(', ');
    const stmt = this.db.prepare(`UPDATE printing_materials SET ${fields}, updated_at = CURRENT_TIMESTAMP WHERE id = @id`);
    stmt.run({ ...updates, id });
  }

  deletePrintingMaterial(id) {
    this.db.prepare('DELETE FROM printing_materials WHERE id = ?').run(id);
  }

  // ==================== Users CRUD ====================

  getUserByUsername(username) {
    return this.db.prepare('SELECT * FROM users WHERE username = ?').get(username);
  }

  updateUserPassword(username, newHash) {
    const stmt = this.db.prepare('UPDATE users SET password_hash = ?, updated_at = CURRENT_TIMESTAMP WHERE username = ?');
    stmt.run(newHash, username);
  }

  updateUsername(oldUsername, newUsername) {
    const stmt = this.db.prepare('UPDATE users SET username = ?, updated_at = CURRENT_TIMESTAMP WHERE username = ?');
    stmt.run(newUsername, oldUsername);
  }

  getAllUsers() {
    return this.db.prepare('SELECT * FROM users ORDER BY created_at DESC').all();
  }

  addUser(user) {
    const stmt = this.db.prepare(`
      INSERT INTO users (username, password_hash, role, permissions)
      VALUES (@username, @password_hash, @role, @permissions)
    `);
    const result = stmt.run({
      username: user.username,
      password_hash: user.password_hash,
      role: user.role || 'employee',
      permissions: user.permissions ? (Array.isArray(user.permissions) ? JSON.stringify(user.permissions) : user.permissions) : null
    });
    return { ...user, id: result.lastInsertRowid };
  }

  deleteUser(username) {
    this.db.prepare('DELETE FROM users WHERE username = ?').run(username);
  }

  // ==================== Migration from localStorage ====================

  migrateFromLocalStorage(localStorageData) {
    console.log('Migrating data from localStorage...');
    
    const transaction = this.db.transaction(() => {
      // Migrate products
      if (localStorageData.products && localStorageData.products.length > 0) {
        const insertProduct = this.db.prepare(`
          INSERT INTO products (name, product_type, color, size, selling_price, stock, min_sale_qty, sale_unit)
          VALUES (@name, @product_type, @color, @size, @selling_price, @stock, @min_sale_qty, @sale_unit)
        `);
        
        for (const product of localStorageData.products) {
          insertProduct.run({
            name: product.name,
            product_type: product.product_type || 'life_saver',
            color: product.color || null,
            size: product.size || null,
            selling_price: product.selling_price || 0,
            stock: product.stock || 0,
            min_sale_qty: product.min_sale_qty || 1,
            sale_unit: product.sale_unit || null
          });
        }
        console.log(`Migrated ${localStorageData.products.length} products`);
      }

      // Migrate stock
      if (localStorageData.stock && localStorageData.stock.length > 0) {
        const insertStock = this.db.prepare(`
          INSERT INTO stock (color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used)
          VALUES (@color, @size, @sticker_type, @rolls, @metres_per_roll, @total_metres, @metres_used)
        `);
        
        for (const item of localStorageData.stock) {
          insertStock.run({
            color: item.color,
            size: item.size || '1',
            sticker_type: item.sticker_type || 'colored',
            rolls: item.rolls || 0,
            metres_per_roll: item.metres_per_roll || 50,
            total_metres: item.total_metres || 0,
            metres_used: item.metres_used || 0
          });
        }
        console.log(`Migrated ${localStorageData.stock.length} stock items`);
      }

      // Migrate sales
      if (localStorageData.sales && localStorageData.sales.length > 0) {
        const insertSale = this.db.prepare(`
          INSERT INTO sales (type, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, timestamp)
          VALUES (@type, @product_name, @product_type, @sticker_type, @quantity, @amount, @payment_method, @customer_name, @timestamp)
        `);
        
        for (const sale of localStorageData.sales) {
          insertSale.run({
            type: sale.type || 'product',
            product_name: sale.product_name || null,
            product_type: sale.product_type || null,
            sticker_type: sale.sticker_type || null,
            quantity: sale.quantity ? String(sale.quantity) : null,
            amount: sale.amount || 0,
            payment_method: sale.payment_method || 'cash',
            customer_name: sale.customer_name || 'Walk-in',
            timestamp: sale.timestamp || new Date().toISOString()
          });
        }
        console.log(`Migrated ${localStorageData.sales.length} sales`);
      }

      // Migrate debts
      if (localStorageData.debts && localStorageData.debts.length > 0) {
        const insertDebt = this.db.prepare(`
          INSERT INTO debts (customer_name, phone, amount, due_date, description, status)
          VALUES (@customer_name, @phone, @amount, @due_date, @description, @status)
        `);
        
        for (const debt of localStorageData.debts) {
          insertDebt.run({
            customer_name: debt.customer_name,
            phone: debt.phone || null,
            amount: debt.amount || 0,
            due_date: debt.due_date || null,
            description: debt.description || null,
            status: debt.status || 'pending'
          });
        }
        console.log(`Migrated ${localStorageData.debts.length} debts`);
      }
    });

    transaction();
    console.log('Migration completed!');
  }

  /**
   * Close the database connection
   */
  close() {
    if (this.db) {
      this.db.close();
      console.log('Database connection closed');
    }
  }
}

module.exports = new DatabaseManager();
