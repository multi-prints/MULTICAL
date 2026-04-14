/**
 * Receipt Component
 * Generates and prints thermal receipts (80mm) for sales and printing jobs
 */

const Receipt = {
  // Store original document title to restore after printing
  originalTitle: null,

  /**
   * Get business settings from localStorage
   */
  getSettings() {
    try {
      const saved = localStorage.getItem('app_settings');
      if (saved) {
        return JSON.parse(saved);
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
    }
    return {
      businessName: 'MULTIPRINTS',
      businessPhone: '',
      businessAddress: '',
      businessPIN: '',
      currencySymbol: 'KSh'
    };
  },

  /**
   * Format date for receipt
   * Handles SQLite timestamp format (stored in UTC)
   */
  formatDate(date) {
    let d;
    
    // Check if it's a SQLite timestamp format (YYYY-MM-DD HH:MM:SS) without timezone
    if (typeof date === 'string' && /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(date)) {
      // SQLite stores in UTC, so append 'Z' to indicate UTC
      d = new Date(date + 'Z');
    } else if (typeof date === 'string' && date.endsWith('Z')) {
      // Already has UTC indicator
      d = new Date(date);
    } else {
      // ISO string or Date object
      d = new Date(date);
    }
    
    // Check for invalid date
    if (isNaN(d.getTime())) {
      d = new Date();
    }
    
    return d.toLocaleDateString('en-GB', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric'
    }) + ' ' + d.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      hour12: true
    });
  },

  /**
   * Generate receipt number from sale/transaction ID
   */
  generateReceiptNumber(id, prefix = 'RCP') {
    return `${prefix}-${String(id).padStart(4, '0')}`;
  },

  /**
   * Generate receipt HTML for a sale
   */
  generateSaleReceipt(sale) {
    const settings = this.getSettings();
    const receiptNumber = this.generateReceiptNumber(sale.id);
    const dateStr = this.formatDate(sale.timestamp);
    const currency = settings.currencySymbol || 'KSh';

    return `
      <div class="receipt-container">
        <div class="receipt-header">
          <div class="receipt-business-name">${settings.businessName || 'MULTIPRINTS'}</div>
          ${settings.businessAddress ? `<div class="receipt-info">${settings.businessAddress}</div>` : ''}
          ${settings.businessPhone ? `<div class="receipt-info">Tel: ${settings.businessPhone}</div>` : ''}
          ${settings.businessPIN ? `<div class="receipt-info">PIN: ${settings.businessPIN}</div>` : ''}
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-meta">
          <div class="receipt-row">
            <span>Receipt #:</span>
            <span>${receiptNumber}</span>
          </div>
          <div class="receipt-row">
            <span>Date:</span>
            <span>${dateStr}</span>
          </div>
          <div class="receipt-row">
            <span>Customer:</span>
            <span>${sale.customer_name || 'Walk-in'}</span>
          </div>
        </div>

        <div class="receipt-divider">--------------------------------</div>

        <div class="receipt-items">
          <div class="receipt-items-header">Items:</div>
          <div class="receipt-item">
            <div class="receipt-item-name">${sale.product_name}</div>
            <div class="receipt-item-details">
              <span>${sale.quantity}</span>
              <span class="receipt-item-price">${currency} ${sale.amount.toLocaleString()}</span>
            </div>
          </div>
        </div>

        <div class="receipt-divider">--------------------------------</div>

        <div class="receipt-totals">
          <div class="receipt-row">
            <span>Subtotal:</span>
            <span>${currency} ${sale.amount.toLocaleString()}</span>
          </div>
          <div class="receipt-row">
            <span>Payment:</span>
            <span class="receipt-payment">${this.formatPaymentMethod(sale.payment_method)}</span>
          </div>
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-total-row">
          <span>TOTAL:</span>
          <span>${currency} ${sale.amount.toLocaleString()}</span>
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-footer">
          <div>Thank you for your business!</div>
        </div>
      </div>
    `;
  },

  /**
   * Generate receipt HTML for a printing job
   */
  generatePrintingReceipt(transaction) {
    const settings = this.getSettings();
    const receiptNumber = this.generateReceiptNumber(transaction.id, 'PRT');
    const dateStr = this.formatDate(transaction.timestamp);
    const currency = settings.currencySymbol || 'KSh';

    return `
      <div class="receipt-container">
        <div class="receipt-header">
          <div class="receipt-business-name">${settings.businessName || 'MULTIPRINTS'}</div>
          ${settings.businessAddress ? `<div class="receipt-info">${settings.businessAddress}</div>` : ''}
          ${settings.businessPhone ? `<div class="receipt-info">Tel: ${settings.businessPhone}</div>` : ''}
          ${settings.businessPIN ? `<div class="receipt-info">PIN: ${settings.businessPIN}</div>` : ''}
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-meta">
          <div class="receipt-row">
            <span>Receipt #:</span>
            <span>${receiptNumber}</span>
          </div>
          <div class="receipt-row">
            <span>Date:</span>
            <span>${dateStr}</span>
          </div>
          <div class="receipt-row">
            <span>Customer:</span>
            <span>${transaction.customer_name || 'Walk-in'}</span>
          </div>
        </div>

        <div class="receipt-divider">--------------------------------</div>

        <div class="receipt-items">
          <div class="receipt-items-header">Printing Job:</div>
          <div class="receipt-item">
            <div class="receipt-item-name">${transaction.service_name}</div>
            <div class="receipt-item-details">
              <span>${transaction.stock_metres_used ? transaction.stock_metres_used.toFixed(1) + 'm' : ''}</span>
              <span class="receipt-item-price">${currency} ${transaction.amount.toLocaleString()}</span>
            </div>
          </div>
          ${transaction.notes ? `<div class="receipt-notes">${transaction.notes}</div>` : ''}
        </div>

        <div class="receipt-divider">--------------------------------</div>

        <div class="receipt-totals">
          <div class="receipt-row">
            <span>Subtotal:</span>
            <span>${currency} ${transaction.amount.toLocaleString()}</span>
          </div>
          <div class="receipt-row">
            <span>Payment:</span>
            <span class="receipt-payment">${this.formatPaymentMethod(transaction.payment_method)}</span>
          </div>
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-total-row">
          <span>TOTAL:</span>
          <span>${currency} ${transaction.amount.toLocaleString()}</span>
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-footer">
          <div>Thank you for your business!</div>
        </div>
      </div>
    `;
  },

  /**
   * Format payment method for display
   */
  formatPaymentMethod(method) {
    const methods = {
      'cash': 'Cash',
      'mpesa': 'M-Pesa',
      'till': 'Till Number',
      'credit': 'Credit'
    };
    return methods[method] || method;
  },

  /**
   * Show print preview modal
   */
  showPreview(receiptHtml, title = 'Print Receipt') {
    // Remove existing modal if any
    const existingModal = document.getElementById('receipt-preview-modal');
    if (existingModal) existingModal.remove();

    const modalHtml = `
      <div id="receipt-preview-modal" class="modal-overlay open">
        <div class="modal-container receipt-preview-modal">
          <div class="modal-header">
            <h3 class="modal-title">${title}</h3>
            <button class="modal-close-btn" onclick="Receipt.closePreview()">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          <div class="modal-body receipt-preview-body">
            <div class="receipt-preview-wrapper">
              ${receiptHtml}
            </div>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn-secondary px-4 py-2 rounded-lg" onclick="Receipt.closePreview()">Close</button>
            <button type="button" class="btn-primary px-4 py-2 rounded-lg flex items-center gap-2" onclick="Receipt.print()">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"></path>
              </svg>
              Print
            </button>
          </div>
        </div>
      </div>
    `;

    document.body.insertAdjacentHTML('beforeend', modalHtml);
  },

  /**
   * Close the preview modal
   */
  closePreview() {
    const modal = document.getElementById('receipt-preview-modal');
    if (modal) modal.remove();
  },

  /**
   * Print the receipt
   */
  print() {
    window.print();
  },

  /**
   * Handle after print - restore original title
   */
  afterPrint() {
    if (this.originalTitle) {
      document.title = this.originalTitle;
    }
    window.removeEventListener('afterprint', this.afterPrint);
  },

  /**
   * Print a sale receipt
   */
  printSale(saleId) {
    const sale = Store.sales.find(s => s.id === saleId);
    if (!sale) {
      Toast.error('Sale Not Found', 'Could not find the sale record');
      return;
    }

    const receiptHtml = this.generateSaleReceipt(sale);
    const receiptNumber = this.generateReceiptNumber(saleId);
    this.showPreview(receiptHtml, 'Print Sale Receipt');

    // Set document title for meaningful PDF filename
    const customerPart = sale.customer_name ? `_${sale.customer_name.replace(/\s+/g, '-')}` : '';
    const datePart = new Date(sale.timestamp).toISOString().split('T')[0];
    this.setPrintTitle(`Receipt_${receiptNumber}${customerPart}_${datePart}`);
  },

  /**
   * Print a printing job receipt
   */
  printPrintingJob(transactionId) {
    const transaction = Store.serviceTransactions.find(t => t.id === transactionId);
    if (!transaction) {
      Toast.error('Job Not Found', 'Could not find the printing job record');
      return;
    }

    const receiptHtml = this.generatePrintingReceipt(transaction);
    const receiptNumber = this.generateReceiptNumber(transactionId, 'PRT');
    this.showPreview(receiptHtml, 'Print Job Receipt');

    // Set document title for meaningful PDF filename
    const customerPart = transaction.customer_name ? `_${transaction.customer_name.replace(/\s+/g, '-')}` : '';
    const datePart = new Date(transaction.timestamp).toISOString().split('T')[0];
    this.setPrintTitle(`Printing_Receipt_${receiptNumber}${customerPart}_${datePart}`);
  },

  /**
   * Generate receipt HTML for multiple sales
   */
  generateMultipleSalesReceipt(sales) {
    const settings = this.getSettings();
    const currency = settings.currencySymbol || 'KSh';
    const totalAmount = sales.reduce((sum, sale) => sum + sale.amount, 0);
    const receiptNumbers = sales.map(s => this.generateReceiptNumber(s.id)).join(', ');
    const dateStr = this.formatDate(new Date());
    
    // Get unique customer names
    const customers = [...new Set(sales.map(s => s.customer_name || 'Walk-in'))];
    const customerDisplay = customers.length === 1 ? customers[0] : 'Multiple Customers';

    // Generate items HTML
    const itemsHtml = sales.map(sale => `
      <div class="receipt-item">
        <div class="receipt-item-name">${sale.product_name}</div>
        <div class="receipt-item-details">
          <span>${sale.quantity}</span>
          <span class="receipt-item-price">${currency} ${sale.amount.toLocaleString()}</span>
        </div>
      </div>
    `).join('');

    // Get payment methods used
    const paymentMethods = [...new Set(sales.map(s => this.formatPaymentMethod(s.payment_method)))];
    const paymentDisplay = paymentMethods.join(', ');

    return `
      <div class="receipt-container">
        <div class="receipt-header">
          <div class="receipt-business-name">${settings.businessName || 'MULTIPRINTS'}</div>
          ${settings.businessAddress ? `<div class="receipt-info">${settings.businessAddress}</div>` : ''}
          ${settings.businessPhone ? `<div class="receipt-info">Tel: ${settings.businessPhone}</div>` : ''}
          ${settings.businessPIN ? `<div class="receipt-info">PIN: ${settings.businessPIN}</div>` : ''}
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-meta">
          <div class="receipt-row">
            <span>Receipt #:</span>
            <span style="font-size: 10px;">${receiptNumbers}</span>
          </div>
          <div class="receipt-row">
            <span>Date:</span>
            <span>${dateStr}</span>
          </div>
          <div class="receipt-row">
            <span>Customer:</span>
            <span>${customerDisplay}</span>
          </div>
          <div class="receipt-row">
            <span>Items:</span>
            <span>${sales.length}</span>
          </div>
        </div>

        <div class="receipt-divider">--------------------------------</div>

        <div class="receipt-items">
          <div class="receipt-items-header">Items:</div>
          ${itemsHtml}
        </div>

        <div class="receipt-divider">--------------------------------</div>

        <div class="receipt-totals">
          <div class="receipt-row">
            <span>Subtotal:</span>
            <span>${currency} ${totalAmount.toLocaleString()}</span>
          </div>
          <div class="receipt-row">
            <span>Payment:</span>
            <span class="receipt-payment">${paymentDisplay}</span>
          </div>
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-total-row">
          <span>TOTAL:</span>
          <span>${currency} ${totalAmount.toLocaleString()}</span>
        </div>

        <div class="receipt-divider">================================</div>

        <div class="receipt-footer">
          <div>Thank you for your business!</div>
        </div>
      </div>
    `;
  },

  /**
   * Print multiple sales on one receipt
   */
  printMultipleSales(sales) {
    if (!sales || sales.length === 0) {
      Toast.error('No Sales', 'No sales selected for printing');
      return;
    }

    const receiptHtml = this.generateMultipleSalesReceipt(sales);
    const title = sales.length === 1 ? 'Print Sale Receipt' : `Print ${sales.length} Sales Receipt`;
    this.showPreview(receiptHtml, title);

    // Set document title for meaningful PDF filename
    const datePart = new Date().toISOString().split('T')[0];
    const countPart = sales.length === 1 ? this.generateReceiptNumber(sales[0].id) : `${sales.length}_items`;
    const customerPart = sales.length === 1 && sales[0].customer_name 
      ? `_${sales[0].customer_name.replace(/\s+/g, '-')}` 
      : '';
    this.setPrintTitle(`Receipt_${countPart}${customerPart}_${datePart}`);
  },

  /**
   * Set the document title for printing (used as PDF filename)
   */
  setPrintTitle(filename) {
    this.originalTitle = document.title;
    document.title = filename;

    // Restore title after printing
    window.addEventListener('afterprint', () => this.afterPrint());
  }
};

window.Receipt = Receipt;
