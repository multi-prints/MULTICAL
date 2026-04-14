/**
 * Sales Page Controller
 * Handles both stock-based sales (stickers) and product sales
 */

const SalesPage = {
  metresPerRoll: 50, // 1 roll = 50 metres
  stockDropdown: null,
  saleTypeDropdown: null,
  paymentDropdown: null,
  productDropdown: null,
  productPaymentDropdown: null,
  servicePaymentDropdown: null,
  pickerMonth: new Date().getMonth(),
  pickerYear: new Date().getFullYear(),
  pickerSelectedDate: null,
  currentPage: 1,
  itemsPerPage: 10,
  selectedSales: new Set(), // Track selected sale IDs

  init() {
    this.initCustomDropdowns();
    this.bindEvents();
    this.bindSelectionEvents();
    this.render();
    this.updateStats();

    // Hide stats if employee
    if (window.Permissions && window.Permissions.getCurrentRole() === 'employee') {
      const statsContainer = document.getElementById('sales-stats-container');
      if (statsContainer) statsContainer.style.display = 'none';
    }
    
    // Subscribe to store changes
    Store.subscribe('sales', () => {
      this.render();
      this.updateStats();
    });
    Store.subscribe('products', () => this.updateProductDropdownItems());
    Store.subscribe('stock', () => this.updateStockDropdownItems());
  },

  initCustomDropdowns() {
    // Stock Color Dropdown
    const stockContainer = document.getElementById('stock-color-dropdown');
    if (stockContainer) {
      const availableStock = Store.getAvailableStockColors();
      
      this.stockDropdown = new CustomDropdown(stockContainer, {
        placeholder: availableStock.length > 0 ? 'Choose sticker' : 'No stock - add stock first',
        showColorSwatch: false,
        items: availableStock.map(s => {
          const typeConfig = STICKER_TYPES[s.sticker_type] || STICKER_TYPES.colored;
          return {
            value: s.id.toString(),
            label: `${s.color} - ${s.size}m (${typeConfig.name}) - ${s.remaining.toLocaleString()}m`,
            stickerType: s.sticker_type,
            remaining: s.remaining,
            badge: `${Math.floor(s.remaining / this.metresPerRoll)} rolls`
          };
        }),
        onSelect: (selected) => {
          const hiddenInput = document.getElementById('sale-stock-id-input');
          if (hiddenInput) hiddenInput.value = selected.value;
          this.updateStockInfo(selected);
          this.calculateStockSaleTotal();
        }
      });
    }

    // Sale Type Dropdown
    const saleTypeContainer = document.getElementById('sale-type-dropdown');
    if (saleTypeContainer) {
      this.saleTypeDropdown = new CustomDropdown(saleTypeContainer, {
        placeholder: 'Metres',
        items: [
          { value: 'metres', label: 'Metres', badge: 'per metre' },
          { value: 'rolls', label: 'Whole Rolls', badge: '50m each' }
        ],
        onSelect: (selected) => {
          const hiddenInput = document.getElementById('sale-unit-input');
          if (hiddenInput) hiddenInput.value = selected.value;
          this.updateUnitLabel(selected.value);
          this.calculateStockSaleTotal();
        }
      });
      // Auto-select first item
      this.saleTypeDropdown.selectItem(saleTypeContainer.querySelector('.dropdown-item'));
    }

    // Payment Method Dropdown (Stock Sale)
    const paymentContainer = document.getElementById('payment-method-dropdown');
    if (paymentContainer) {
      this.paymentDropdown = new CustomDropdown(paymentContainer, {
        placeholder: 'Cash',
        items: [
          { value: 'cash', label: 'Cash' },
          { value: 'mpesa', label: 'M-Pesa' },
          { value: 'till', label: 'Till Number' }
        ],
        onSelect: (selected) => {
          const hiddenInput = document.getElementById('payment-method-input');
          if (hiddenInput) hiddenInput.value = selected.value;
        }
      });
      // Auto-select first item
      this.paymentDropdown.selectItem(paymentContainer.querySelector('.dropdown-item'));
    }

    // ==================== Product Sale Dropdowns ====================
    
    // Product Dropdown
    const productContainer = document.getElementById('product-dropdown');
    if (productContainer) {
      const availableProducts = Store.products.filter(p => p.stock > 0);
      
      this.productDropdown = new CustomDropdown(productContainer, {
        placeholder: availableProducts.length > 0 ? 'Choose product' : 'No products - add products first',
        items: availableProducts.map(p => {
          return {
            value: p.id.toString(),
            label: p.name,
            price: p.selling_price,
            stock: p.stock,
            badge: `${p.stock} in stock`
          };
        }),
        onSelect: (selected) => {
          const hiddenInput = document.getElementById('sale-product-id-input');
          if (hiddenInput) hiddenInput.value = selected.value;
          this.updateProductInfo(selected);
          this.calculateTotal();
        }
      });
    }

    // Payment Method Dropdown (Product Sale)
    const productPaymentContainer = document.getElementById('product-payment-dropdown');
    if (productPaymentContainer) {
      this.productPaymentDropdown = new CustomDropdown(productPaymentContainer, {
        placeholder: 'Cash',
        items: [
          { value: 'cash', label: 'Cash' },
          { value: 'mpesa', label: 'M-Pesa' },
          { value: 'till', label: 'Till Number' }
        ],
        onSelect: (selected) => {
          const hiddenInput = document.getElementById('product-payment-input');
          if (hiddenInput) hiddenInput.value = selected.value;
        }
      });
      // Auto-select first item
      this.productPaymentDropdown.selectItem(productPaymentContainer.querySelector('.dropdown-item'));
    }

    // ==================== Service Sale Dropdowns ====================
    
    // Payment Method Dropdown (Service Sale)
    const servicePaymentContainer = document.getElementById('service-payment-dropdown');
    if (servicePaymentContainer) {
      this.servicePaymentDropdown = new CustomDropdown(servicePaymentContainer, {
        placeholder: 'Cash',
        items: [
          { value: 'cash', label: 'Cash' },
          { value: 'mpesa', label: 'M-Pesa' },
          { value: 'till', label: 'Till Number' }
        ],
        onSelect: (selected) => {
          const hiddenInput = document.getElementById('service-payment-input');
          if (hiddenInput) hiddenInput.value = selected.value;
        }
      });
      // Auto-select first item
      this.servicePaymentDropdown.selectItem(servicePaymentContainer.querySelector('.dropdown-item'));
    }
  },

  bindEvents() {
    // Modal Handling
    const modal = document.getElementById('modal-record-sale');
    const btnRecord = document.getElementById('btn-record-sale');
    const btnClose = document.getElementById('btn-close-sale-modal');
    const btnCancel1 = document.getElementById('btn-cancel-sale-1');
    const btnCancel2 = document.getElementById('btn-cancel-sale-2');
    const btnCancel3 = document.getElementById('btn-cancel-sale-3');

    const openModal = () => {
        if (modal) modal.classList.add('open');
    };

    const closeModal = () => {
        if (modal) {
            modal.classList.remove('open');
            // Reset forms
            document.getElementById('stock-sale-form')?.reset();
            document.getElementById('sale-form')?.reset();
            document.getElementById('service-sale-form')?.reset();
            this.resetStockSaleDisplay();
            this.resetProductSaleDisplay();
            this.resetServiceSaleDisplay();
            // Reset stock sale dropdowns
            this.stockDropdown?.reset();
            this.saleTypeDropdown?.reset();
            this.paymentDropdown?.reset();
            // Reset product sale dropdowns
            this.productDropdown?.reset();
            this.productPaymentDropdown?.reset();
            // Reset service sale dropdown
            this.servicePaymentDropdown?.reset();
            // Re-select defaults for stock sale
            const saleTypeContainer = document.getElementById('sale-type-dropdown');
            const paymentContainer = document.getElementById('payment-method-dropdown');
            if (saleTypeContainer) this.saleTypeDropdown?.selectItem(saleTypeContainer.querySelector('.dropdown-item'));
            if (paymentContainer) this.paymentDropdown?.selectItem(paymentContainer.querySelector('.dropdown-item'));
            // Re-select defaults for product sale
            const productPaymentContainer = document.getElementById('product-payment-dropdown');
            if (productPaymentContainer) this.productPaymentDropdown?.selectItem(productPaymentContainer.querySelector('.dropdown-item'));
            // Re-select defaults for service sale
            const servicePaymentContainer = document.getElementById('service-payment-dropdown');
            if (servicePaymentContainer) this.servicePaymentDropdown?.selectItem(servicePaymentContainer.querySelector('.dropdown-item'));
        }
    };

    if (btnRecord) btnRecord.addEventListener('click', openModal);
    if (btnClose) btnClose.addEventListener('click', closeModal);
    if (btnCancel1) btnCancel1.addEventListener('click', closeModal);
    if (btnCancel2) btnCancel2.addEventListener('click', closeModal);
    if (btnCancel3) btnCancel3.addEventListener('click', closeModal);
    
    if (modal) {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) closeModal();
        });
    }

    // Tab switching
    const tabStock = document.getElementById('tab-stock-sale');
    const tabProduct = document.getElementById('tab-product-sale');
    const tabService = document.getElementById('tab-service-sale');
    const stockSection = document.getElementById('stock-sale-section');
    const productSection = document.getElementById('product-sale-section');
    const serviceSection = document.getElementById('service-sale-section');

    if (tabStock) {
      tabStock.addEventListener('click', (e) => {
        e.preventDefault();
        tabStock.className = 'px-4 py-2 bg-white text-gray-900 font-medium text-sm shadow-sm transition-all';
        tabProduct.className = 'px-4 py-2 text-gray-500 font-medium text-sm hover:bg-gray-200 transition-all';
        tabService.className = 'px-4 py-2 text-gray-500 font-medium text-sm hover:bg-gray-200 transition-all';
        stockSection?.classList.remove('hidden');
        productSection?.classList.add('hidden');
        serviceSection?.classList.add('hidden');
      });
    }

    if (tabProduct) {
      tabProduct.addEventListener('click', (e) => {
        e.preventDefault();
        tabProduct.className = 'px-4 py-2 bg-white text-gray-900 font-medium text-sm shadow-sm transition-all';
        tabStock.className = 'px-4 py-2 text-gray-500 font-medium text-sm hover:bg-gray-200 transition-all';
        tabService.className = 'px-4 py-2 text-gray-500 font-medium text-sm hover:bg-gray-200 transition-all';
        productSection?.classList.remove('hidden');
        stockSection?.classList.add('hidden');
        serviceSection?.classList.add('hidden');
      });
    }

    if (tabService) {
      tabService.addEventListener('click', (e) => {
        e.preventDefault();
        tabService.className = 'px-4 py-2 bg-white text-gray-900 font-medium text-sm shadow-sm transition-all';
        tabStock.className = 'px-4 py-2 text-gray-500 font-medium text-sm hover:bg-gray-200 transition-all';
        tabProduct.className = 'px-4 py-2 text-gray-500 font-medium text-sm hover:bg-gray-200 transition-all';
        serviceSection?.classList.remove('hidden');
        stockSection?.classList.add('hidden');
        productSection?.classList.add('hidden');
      });
    }

    // Stock sale form events
    const stockForm = document.getElementById('stock-sale-form');
    const stockQuantity = document.getElementById('sale-stock-quantity');
    const stockTotalPrice = document.getElementById('stock-total-price');

    if (stockQuantity) stockQuantity.addEventListener('input', () => this.calculateStockSaleTotal());
    if (stockTotalPrice) stockTotalPrice.addEventListener('input', () => this.calculateStockSaleTotal());

    if (stockForm && !stockForm.dataset.bound) {
      stockForm.dataset.bound = 'true';
      stockForm.addEventListener('submit', (e) => {
        e.preventDefault();
        const formData = new FormData(stockForm);
        this.handleStockSaleSubmit(formData);
        closeModal();
      });
    }

    // Product sale form events
    const form = document.getElementById('sale-form');
    const totalPriceInput = document.getElementById('sale-total-price');

    if (totalPriceInput) totalPriceInput.addEventListener('input', () => this.calculateTotal());

    if (form && !form.dataset.bound) {
      form.dataset.bound = 'true';
      form.addEventListener('submit', (e) => {
        e.preventDefault();
        this.handleSubmit(new FormData(form));
        closeModal();
      });
    }

    // Service sale form events
    const serviceForm = document.getElementById('service-sale-form');
    const servicePriceInput = document.getElementById('service-price');

    if (servicePriceInput) servicePriceInput.addEventListener('input', () => this.calculateServiceSaleTotal());

    if (serviceForm && !serviceForm.dataset.bound) {
      serviceForm.dataset.bound = 'true';
      serviceForm.addEventListener('submit', (e) => {
        e.preventDefault();
        this.handleServiceSaleSubmit(new FormData(serviceForm));
        closeModal();
      });
    }
  },

  // ==================== Selection Events ====================

  bindSelectionEvents() {
    // Select all checkbox
    const selectAllCheckbox = document.getElementById('select-all-sales');
    if (selectAllCheckbox) {
      selectAllCheckbox.addEventListener('change', (e) => {
        this.toggleSelectAll(e.target.checked);
      });
    }

    // Print selected button
    const printSelectedBtn = document.getElementById('btn-print-selected-sales');
    if (printSelectedBtn) {
      printSelectedBtn.addEventListener('click', () => {
        this.printSelected();
      });
    }
  },

  toggleSelectAll(checked) {
    const allSales = Store.sales;
    if (checked) {
      allSales.forEach(sale => this.selectedSales.add(sale.id));
    } else {
      this.selectedSales.clear();
    }
    this.render();
    this.updateSelectionUI();
  },

  toggleSaleSelection(saleId) {
    if (this.selectedSales.has(saleId)) {
      this.selectedSales.delete(saleId);
    } else {
      this.selectedSales.add(saleId);
    }
    this.updateSelectionUI();
    this.updateSelectAllCheckbox();
  },

  updateSelectionUI() {
    const printSelectedBtn = document.getElementById('btn-print-selected-sales');
    const selectedCountEl = document.getElementById('selected-count');
    const count = this.selectedSales.size;

    if (printSelectedBtn) {
      if (count > 0) {
        printSelectedBtn.classList.remove('hidden');
        printSelectedBtn.classList.add('flex');
      } else {
        printSelectedBtn.classList.add('hidden');
        printSelectedBtn.classList.remove('flex');
      }
    }

    if (selectedCountEl) {
      selectedCountEl.textContent = count;
    }
  },

  updateSelectAllCheckbox() {
    const selectAllCheckbox = document.getElementById('select-all-sales');
    if (selectAllCheckbox) {
      const allSales = Store.sales;
      selectAllCheckbox.checked = allSales.length > 0 && this.selectedSales.size === allSales.length;
      selectAllCheckbox.indeterminate = this.selectedSales.size > 0 && this.selectedSales.size < allSales.length;
    }
  },

  printSelected() {
    if (this.selectedSales.size === 0) {
      Toast.error('No Selection', 'Please select at least one sale to print');
      return;
    }

    const selectedSalesArray = Store.sales.filter(sale => this.selectedSales.has(sale.id));
    Receipt.printMultipleSales(selectedSalesArray);
  },

  // ==================== Stock Sale Functions ====================

  updateStockDropdownItems() {
    if (!this.stockDropdown) return;

    const availableStock = Store.getAvailableStockColors();
    
    this.stockDropdown.setItems(availableStock.map(s => {
      const typeConfig = STICKER_TYPES[s.sticker_type] || STICKER_TYPES.colored;
      return {
        value: s.id.toString(),
        label: `${s.color} - ${s.size}m (${typeConfig.name}) - ${s.remaining.toLocaleString()}m`,
        color: s.sticker_type === 'colored' ? s.color : null,
        stickerType: s.sticker_type,
        remaining: s.remaining,
        badge: `${Math.floor(s.remaining / this.metresPerRoll)} rolls`
      };
    }));
  },

  updateStockInfo(selected) {
    const infoEl = document.getElementById('stock-remaining-info');
    if (!infoEl || !selected) return;

    const remaining = parseFloat(selected.remaining) || 0;
    
    if (remaining > 0) {
      const rollsLeft = Math.floor(remaining / this.metresPerRoll);
      infoEl.textContent = `${remaining.toLocaleString()}m available (${rollsLeft} full rolls)`;
    } else {
      infoEl.textContent = '';
    }
  },

  updateUnitLabel(selectedValue) {
    const label = document.getElementById('quantity-label');
    const deductLabel = document.getElementById('metres-deducted-label');
    
    if (!label) return;

    // Use passed value or get from hidden input
    const value = selectedValue || document.getElementById('sale-unit-input')?.value || 'metres';

    if (value === 'rolls') {
      label.textContent = 'Rolls Sold';
      if (deductLabel) deductLabel.textContent = 'Metres to Deduct';
    } else {
      label.textContent = 'Metres Sold';
      if (deductLabel) deductLabel.textContent = 'Metres to Deduct';
    }
  },

  calculateStockSaleTotal() {
    const unitInput = document.getElementById('sale-unit-input');
    const quantityInput = document.getElementById('sale-stock-quantity');
    const totalPriceInput = document.getElementById('stock-total-price');
    const metresToDeductEl = document.getElementById('metres-to-deduct');
    const totalEl = document.getElementById('stock-sale-total');

    if (!quantityInput || !totalPriceInput) return;

    const quantity = parseFloat(quantityInput.value) || 0;
    const totalPrice = parseFloat(totalPriceInput.value) || 0;
    const isRolls = unitInput?.value === 'rolls';

    // Calculate metres to deduct
    const metresToDeduct = isRolls ? quantity * this.metresPerRoll : quantity;

    if (metresToDeductEl) {
      metresToDeductEl.textContent = `${metresToDeduct.toLocaleString()}m`;
    }

    if (totalEl) {
      totalEl.textContent = `KSh ${totalPrice.toFixed(2)}`;
    }
  },

  resetStockSaleDisplay() {
    const metresToDeductEl = document.getElementById('metres-to-deduct');
    const totalEl = document.getElementById('stock-sale-total');
    const infoEl = document.getElementById('stock-remaining-info');

    if (metresToDeductEl) metresToDeductEl.textContent = '0m';
    if (totalEl) totalEl.textContent = 'KSh 0.00';
    if (infoEl) infoEl.textContent = '';
  },

  resetProductSaleDisplay() {
    const totalEl = document.getElementById('product-sale-total');
    const infoEl = document.getElementById('product-sale-info');
    const hintEl = document.getElementById('quantity-hint');

    if (totalEl) totalEl.textContent = 'KSh 0.00';
    if (infoEl) infoEl.textContent = '';
    if (hintEl) hintEl.textContent = '';
  },

  async handleStockSaleSubmit(formData) {
    const stockId = parseInt(formData.get('stock_id'));
    const saleUnit = formData.get('sale_unit');
    const quantity = parseFloat(formData.get('stock_quantity'));
    const totalPrice = parseFloat(formData.get('total_price'));
    const paymentMethod = formData.get('payment_method');
    const customerName = formData.get('customer_name') || 'Walk-in';

    if (!stockId) {
      Toast.error('No Sticker Selected', 'Please select a sticker color');
      return;
    }

    if (!quantity || quantity <= 0) {
      Toast.error('Invalid Quantity', 'Please enter a valid quantity');
      return;
    }

    if (!totalPrice || totalPrice <= 0) {
      Toast.error('Invalid Price', 'Please enter a valid selling price');
      return;
    }

    // Calculate metres to deduct
    const metresToDeduct = saleUnit === 'rolls' ? quantity * this.metresPerRoll : quantity;

    // Await the async stock deduction
    const result = await Store.deductStockMetres(stockId, metresToDeduct);
    
    if (!result.success) {
      Toast.error('Stock Error', result.error || 'Unable to deduct stock');
      return;
    }

    // Get stock info for sale record
    const stockItem = Store.getStock(stockId);
    const typeConfig = STICKER_TYPES[stockItem.sticker_type] || STICKER_TYPES.colored;

    // Create sale record (await so Store.sales is updated before we proceed)
    const sale = await Store.addSale({
      type: 'stock',
      stock_id: stockId,
      product_name: `${stockItem.color} ${typeConfig.name} Sticker`,
      sticker_type: stockItem.sticker_type,
      quantity: `${metresToDeduct}m`,
      amount: totalPrice,
      payment_method: paymentMethod,
      customer_name: customerName
    });

    // Add to debts if credit
    if (paymentMethod === 'credit') {
      await Store.addDebt({
        customer_name: customerName,
        phone: '',
        amount: totalPrice,
        due_date: null,
        description: `Sale: ${stockItem.color} Sticker - ${metresToDeduct}m`
      });
    }

    // Refresh dropdown items with updated remaining metres
    this.updateStockDropdownItems();

    // Show success toast
    Toast.success('Sale Completed', `${metresToDeduct}m of ${stockItem.color} sticker sold for KSh ${totalPrice.toLocaleString()}`);
  },

  // ==================== Product Sale Functions ====================

  async handleSubmit(formData) {
    const productId = parseInt(formData.get('product_id'));
    const quantity = parseInt(formData.get('quantity'));
    const totalPrice = parseFloat(formData.get('total_price')) || 0;
    const product = Store.getProduct(productId);

    if (!product) {
      Toast.error('No Product Selected', 'Please select a product');
      return;
    }

    if (!totalPrice || totalPrice <= 0) {
      Toast.error('Invalid Price', 'Please enter a valid selling price');
      return;
    }

    if (quantity > product.stock) {
      Toast.error('Insufficient Stock', `Only ${product.stock} units available`);
      return;
    }

    const paymentMethod = formData.get('payment_method');
    const customerName = formData.get('customer_name') || 'Walk-in';

    // Create sale (await to ensure data is updated)
    await Store.addSale({
      type: 'product',
      product_id: productId,
      product_name: product.name,
      product_type: product.product_type,
      quantity: quantity,
      amount: totalPrice,
      payment_method: paymentMethod,
      customer_name: customerName
    });

    // Update stock
    await Store.updateProduct(productId, { stock: product.stock - quantity });

    // Update product dropdown
    this.updateProductDropdownItems();

    // Show success toast
    Toast.success('Sale Completed', `${quantity}x ${product.name} sold for KSh ${totalPrice.toLocaleString()}`);
  },

  calculateTotal() {
    const totalPriceInput = document.getElementById('sale-total-price');
    const totalEl = document.getElementById('product-sale-total');

    if (!totalPriceInput) return;
    
    const totalPrice = parseFloat(totalPriceInput.value) || 0;

    if (totalEl) {
      totalEl.textContent = `KSh ${totalPrice.toFixed(2)}`;
    }
  },

  updateProductInfo(selected) {
    const infoEl = document.getElementById('product-sale-info');
    const hintEl = document.getElementById('quantity-hint');
    
    if (infoEl && selected) {
      infoEl.textContent = `${selected.stock} in stock`;
    }
    
    if (hintEl) {
      hintEl.textContent = '';
    }
  },

  updateProductDropdownItems() {
    if (!this.productDropdown) return;

    const availableProducts = Store.products.filter(p => p.stock > 0);
    
    this.productDropdown.setItems(availableProducts.map(p => {
      return {
        value: p.id.toString(),
        label: p.name,
        price: p.selling_price,
        stock: p.stock,
        badge: `${p.stock} in stock`
      };
    }));
  },

  // ==================== Service Sale Functions ====================

  calculateServiceSaleTotal() {
    const priceInput = document.getElementById('service-price');
    const totalEl = document.getElementById('service-sale-total');

    if (!priceInput) return;

    const price = parseFloat(priceInput.value) || 0;

    if (totalEl) {
      totalEl.textContent = `KSh ${price.toFixed(2)}`;
    }
  },

  resetServiceSaleDisplay() {
    const totalEl = document.getElementById('service-sale-total');
    const serviceNameInput = document.getElementById('service-name-input');
    const priceInput = document.getElementById('service-price');
    const descriptionInput = document.getElementById('service-description');

    if (totalEl) totalEl.textContent = 'KSh 0.00';
    if (serviceNameInput) serviceNameInput.value = '';
    if (priceInput) priceInput.value = '0';
    if (descriptionInput) descriptionInput.value = '';
  },

  async handleServiceSaleSubmit(formData) {
    const serviceName = formData.get('service_name')?.trim();
    const price = parseFloat(formData.get('price'));
    const paymentMethod = formData.get('payment_method');
    const customerName = formData.get('customer_name')?.trim() || 'Walk-in';
    const description = formData.get('description')?.trim() || '';

    if (!serviceName) {
      Toast.error('Service Name Required', 'Please enter a service name');
      return;
    }

    if (!price || price <= 0) {
      Toast.error('Invalid Price', 'Please enter a valid price');
      return;
    }

    // Create sale record as service type
    const sale = await Store.addSale({
      type: 'service',
      product_name: serviceName,
      quantity: description || '-',
      amount: price,
      payment_method: paymentMethod,
      customer_name: customerName
    });

    // Show success toast
    Toast.success('Service Sale Completed', `${serviceName} - KSh ${price.toLocaleString()}`);
  },

  // ==================== Render Sales Table ====================

  render() {
    const tbody = document.getElementById('sales-table-body');
    if (!tbody) return;

    const allSales = Store.sales;

    if (allSales.length === 0) {
      tbody.innerHTML = `
        <tr class="text-center">
            <td colspan="8" class="px-5 py-8 text-gray-500">
                <div class="flex flex-col items-center justify-center">
                    <svg class="w-12 h-12 text-gray-300 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M16 11V7a4 4 0 00-8 0v4M5 9h14l1 12H4L5 9z"></path>
                    </svg>
                    <p>No sales recorded.</p>
                </div>
            </td>
        </tr>
      `;
      this.updatePaginationControls(0);
      return;
    }

    // Calculate pagination
    const totalPages = Math.ceil(allSales.length / this.itemsPerPage);
    const startIndex = (this.currentPage - 1) * this.itemsPerPage;
    const endIndex = startIndex + this.itemsPerPage;
    const paginatedSales = allSales.slice(startIndex, endIndex);

    tbody.innerHTML = paginatedSales.map(sale => {
      const saleDate = new Date(sale.timestamp);
      const today = new Date();
      const isToday = saleDate.toDateString() === today.toDateString();
      
      // Show time if today, show date + time if older
      const timeDisplay = isToday 
        ? saleDate.toLocaleTimeString('en-US', {hour: '2-digit', minute:'2-digit'})
        : saleDate.toLocaleDateString('en-US', {month: 'short', day: 'numeric'}) + ' ' + 
          saleDate.toLocaleTimeString('en-US', {hour: '2-digit', minute:'2-digit'});
      
      return `
      <tr class="hover:bg-gray-50 transition-colors ${this.selectedSales.has(sale.id) ? 'bg-green-50' : ''}">
        <td class="px-5 py-4">
          <input type="checkbox" 
            class="sale-checkbox w-4 h-4 rounded border-gray-300 cursor-pointer" 
            data-sale-id="${sale.id}"
            ${this.selectedSales.has(sale.id) ? 'checked' : ''}
            onchange="SalesPage.toggleSaleSelection(${sale.id})">
        </td>
        <td class="px-5 py-4 text-sm text-gray-600">${timeDisplay}</td>
        <td class="px-5 py-4">
          <div class="flex items-center gap-2">
            <span class="status-badge ${sale.type === 'stock' ? 'bg-gray-800 text-white' : sale.type === 'service' ? 'bg-blue-500 text-white' : 'bg-gray-100 text-gray-700'}">${sale.type === 'stock' ? 'Stock' : sale.type === 'service' ? 'Service' : 'Product'}</span>
            <span class="text-sm font-medium text-gray-900">${sale.product_name}</span>
          </div>
        </td>
        <td class="px-5 py-4 text-sm text-gray-600">${sale.quantity}</td>
        <td class="px-5 py-4 text-sm font-medium text-gray-900">KSh ${sale.amount.toLocaleString(undefined, {minimumFractionDigits: 2})}</td>
        <td class="px-5 py-4">
          <div class="flex flex-col gap-1">
            <span class="status-badge status-badge--success capitalize">${sale.payment_method}</span>
            ${sale.is_debt === 1 ? '<span class="text-[10px] font-bold text-red-600 uppercase tracking-wider">Converted to Debt</span>' : ''}
            ${sale.is_debt === 2 ? '<span class="text-[10px] font-bold text-green-600 uppercase tracking-wider">Debt Paid</span>' : ''}
          </div>
        </td>
        <td class="px-5 py-4 text-sm text-gray-600">${sale.customer_name}</td>
        <td class="px-5 py-4">
          <div class="flex items-center gap-2">
            <button onclick="Receipt.printSale(${sale.id})" 
              class="text-gray-400 hover:text-green-600 transition-colors" 
              title="Print receipt">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"></path>
              </svg>
            </button>
            <button onclick="SalesPage.convertToDebt(${sale.id})" 
              class="text-gray-400 hover:text-blue-600 transition-colors" 
              title="Convert to debt">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"></path>
              </svg>
            </button>
            ${Permissions.canDelete() ? `
            <button onclick="SalesPage.deleteSale(${sale.id})" 
              class="text-gray-400 hover:text-red-600 transition-colors" 
              title="Delete sale">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
              </svg>
            </button>` : ''}
          </div>
        </td>
      </tr>
    `}).join('');

    // Update pagination controls
    this.updatePaginationControls(allSales.length);
  },

  updatePaginationControls(totalItems) {
    const paginationEl = document.getElementById('sales-pagination');
    if (!paginationEl) return;

    if (totalItems === 0) {
      paginationEl.innerHTML = '';
      return;
    }

    const totalPages = Math.ceil(totalItems / this.itemsPerPage);
    const startItem = (this.currentPage - 1) * this.itemsPerPage + 1;
    const endItem = Math.min(this.currentPage * this.itemsPerPage, totalItems);

    paginationEl.innerHTML = `
      <div class="flex items-center justify-between px-5 py-3 bg-gray-50 border-t border-gray-200">
        <div class="text-sm text-gray-600">
          Showing <span class="font-medium">${startItem}</span> to <span class="font-medium">${endItem}</span> of <span class="font-medium">${totalItems}</span> sales
        </div>
        <div class="flex gap-2">
          <button onclick="SalesPage.previousPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === 1 ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === 1 ? 'disabled' : ''}>
            Previous
          </button>
          <span class="px-3 py-1 text-sm font-medium text-gray-700">
            Page ${this.currentPage} of ${totalPages}
          </span>
          <button onclick="SalesPage.nextPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === totalPages ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === totalPages ? 'disabled' : ''}>
            Next
          </button>
        </div>
      </div>
    `;
  },

  nextPage() {
    const totalPages = Math.ceil(Store.sales.length / this.itemsPerPage);
    if (this.currentPage < totalPages) {
      this.currentPage++;
      this.render();
    }
  },

  previousPage() {
    if (this.currentPage > 1) {
      this.currentPage--;
      this.render();
    }
  },

  updateStats() {
    const todayEl = document.getElementById('stat-today-sales');
    const totalTransactionsEl = document.getElementById('stat-total-transactions');
    const productSalesEl = document.getElementById('stat-product-sales');
    const totalSalesEl = document.getElementById('stat-total-sales');

    const todaySales = Store.getTodaySales();
    const allSales = Store.sales;

    const todayTotal = todaySales.reduce((sum, s) => sum + s.amount, 0);
    const totalRevenue = allSales.reduce((sum, s) => sum + s.amount, 0);
    const productSalesCount = allSales.filter(s => s.type === 'product').length;

    if (todayEl) todayEl.textContent = `KSh ${todayTotal.toLocaleString()}`;
    if (totalTransactionsEl) totalTransactionsEl.textContent = allSales.length;
    if (productSalesEl) productSalesEl.textContent = productSalesCount;
    if (totalSalesEl) totalSalesEl.textContent = `KSh ${totalRevenue.toLocaleString()}`;
  },

  async deleteSale(id) {
    const sale = Store.sales.find(s => s.id === id);
    if (!sale) return;
    
    ConfirmModal.show({
      title: 'Delete Sale?',
      message: 'Are you sure you want to delete this sale? Stock/products will be returned to inventory.',
      itemName: sale.product_name,
      itemDetails: `${sale.quantity} - KSh ${sale.amount.toLocaleString()}`,
      onConfirm: async () => {
        // Restore stock or product based on sale type
        if (sale.type === 'product' && sale.product_id) {
          // Return product stock
          const product = Store.getProduct(sale.product_id);
          if (product) {
            const quantityToReturn = parseInt(sale.quantity) || 1;
            await Store.updateProduct(sale.product_id, {
              stock: product.stock + quantityToReturn
            });
          }
        } else if (sale.type === 'stock' && sale.stock_id) {
          // Return sticker metres
          const stockItem = Store.getStock(sale.stock_id);
          if (stockItem) {
            const metresMatch = sale.quantity?.match(/([\d.]+)m/);
            const metresToReturn = metresMatch ? parseFloat(metresMatch[1]) : 0;
            if (metresToReturn > 0) {
              const newMetresUsed = Math.max(0, stockItem.metres_used - metresToReturn);
              await Store.updateStock(sale.stock_id, {
                metres_used: newMetresUsed
              });
            }
          }
        }
        
        await Store.deleteSale(id);
        Toast.success('Sale Deleted', `${sale.product_name} removed and stock returned`);
        
        // Update stats and refresh views
        this.updateStats();
        this.render();
      }
    });
  },

  async convertToDebt(saleId) {
    const sale = Store.sales.find(s => s.id === saleId);
    if (!sale) return;

    let existingDebt = null;
    if (sale.is_debt) {
      existingDebt = await Store.getDebtBySaleId(saleId);
    }

    // Create and show modal for converting to debt
    const modalHTML = `
      <div id="modal-convert-debt" class="modal-overlay open">
        <div class="modal-container" style="max-width: 500px;">
          <div class="modal-header">
            <h3 class="modal-title">${existingDebt ? 'Edit Debt Information' : 'Convert Sale to Debt'}</h3>
            <button class="modal-close-btn" onclick="SalesPage.closeConvertDebtModal()">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="bg-gray-50 p-4 mb-4">
              <p class="text-xs text-gray-500 uppercase tracking-wide">Sale Details</p>
              <p class="font-semibold text-gray-900">${sale.product_name}</p>
              <p class="text-sm text-gray-600">${sale.quantity} - KSh ${sale.amount.toLocaleString()}</p>
            </div>

            <form id="convert-debt-form" class="space-y-4">
              <input type="hidden" id="convert-sale-id" value="${saleId}">
              <input type="hidden" id="convert-sale-amount" value="${sale.amount}">
              <input type="hidden" id="convert-debt-id" value="${existingDebt ? existingDebt.id : ''}">

              <div>
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Customer Name *</label>
                <input type="text" id="convert-customer-name" value="${existingDebt ? existingDebt.customer_name : sale.customer_name}"
                  class="w-full" placeholder="Enter customer name" required>
              </div>

              <div>
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Customer Phone</label>
                <input type="tel" id="convert-customer-phone" value="${existingDebt ? (existingDebt.phone || '') : ''}"
                  class="w-full" placeholder="Optional">
              </div>

              <div>
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Total Sale Amount</label>
                <div class="px-3 py-2 bg-gray-100 text-lg font-bold text-gray-900">
                  KSh ${sale.amount.toLocaleString()}
                </div>
              </div>

              <div>
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Amount Paid *</label>
                <input type="number" id="convert-amount-paid" min="0" max="${sale.amount}"
                  step="0.01" value="${existingDebt ? existingDebt.paid_amount : 0}" class="w-full" placeholder="0.00" required>
                <p class="text-xs text-gray-500 mt-1">How much the customer has already paid</p>
              </div>

              <div>
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Remaining Debt</label>
                <div class="px-3 py-2 bg-red-50 border border-red-200 text-lg font-bold text-red-600"
                  id="convert-remaining-debt">
                  KSh ${(existingDebt ? existingDebt.remaining_amount : sale.amount).toLocaleString()}
                </div>
              </div>

              <div>
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Due Date</label>
                <div class="relative">
                  <input type="text" id="convert-due-date-display" readonly class="w-full cursor-pointer"
                    value="${existingDebt ? (existingDebt.due_date || '') : ''}"
                    placeholder="Select due date">
                  <input type="hidden" id="convert-due-date" value="${existingDebt ? (existingDebt.due_date || '') : ''}">
                  <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
                    <svg class="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z">
                      </path>
                    </svg>
                  </div>
                </div>
              </div>
            </form>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn-secondary px-4 py-2"
              onclick="SalesPage.closeConvertDebtModal()">Cancel</button>
            <button type="button" class="btn-primary px-4 py-2"
              onclick="SalesPage.submitConvertDebt()">${existingDebt ? 'Update Debt' : 'Create Debt'}</button>
          </div>
        </div>
      </div>
    `;

    // Add modal to page
    const existingModal = document.getElementById('modal-convert-debt');
    if (existingModal) existingModal.remove();
    document.body.insertAdjacentHTML('beforeend', modalHTML);

    // Add event listener for amount paid input
    const amountPaidInput = document.getElementById('convert-amount-paid');
    if (amountPaidInput) {
      amountPaidInput.addEventListener('input', () => {
        const totalAmount = parseFloat(document.getElementById('convert-sale-amount').value) || 0;
        const amountPaid = parseFloat(amountPaidInput.value) || 0;
        const remaining = Math.max(0, totalAmount - amountPaid);
        const remainingEl = document.getElementById('convert-remaining-debt');
        if (remainingEl) {
          remainingEl.textContent = `KSh ${remaining.toLocaleString()}`;
        }
      });
    }

    // Add date picker click handler
    const dueDateDisplay = document.getElementById('convert-due-date-display');
    if (dueDateDisplay) {
      dueDateDisplay.addEventListener('click', () => {
        this.openConvertDatePicker();
      });
    }
  },

  openConvertDatePicker() {
    // Open the sales page date picker modal
    const datePickerModal = document.getElementById('modal-sales-date-picker');
    if (!datePickerModal) return;

    // Initialize picker state
    this.pickerMonth = new Date().getMonth();
    this.pickerYear = new Date().getFullYear();
    this.pickerSelectedDate = null;
    
    datePickerModal.classList.add('open');
    this.renderDatePicker();

    // Bind calendar events if not already bound
    this.bindDatePickerEvents();
  },

  closeConvertDebtModal() {
    const modal = document.getElementById('modal-convert-debt');
    if (modal) modal.remove();
  },

  async submitConvertDebt() {
    const saleId = parseInt(document.getElementById('convert-sale-id').value);
    const debtId = document.getElementById('convert-debt-id').value;
    const totalAmount = parseFloat(document.getElementById('convert-sale-amount').value);
    const customerName = document.getElementById('convert-customer-name').value.trim();
    const customerPhone = document.getElementById('convert-customer-phone').value.trim();
    const amountPaid = parseFloat(document.getElementById('convert-amount-paid').value) || 0;
    const dueDate = document.getElementById('convert-due-date').value || null;

    if (!customerName) {
      Toast.error('Missing Information', 'Please enter customer name');
      return;
    }

    const remainingDebt = totalAmount - amountPaid;

    if (remainingDebt <= 0) {
      Toast.error('No Debt', 'The amount paid equals or exceeds the sale amount. No debt to create.');
      return;
    }

    const sale = Store.sales.find(s => s.id === saleId);
    if (!sale) return;

    // Create or update debt
    const debtData = {
      customer_name: customerName,
      phone: customerPhone || null,
      amount: totalAmount,
      paid_amount: amountPaid,
      remaining_amount: remainingDebt,
      due_date: dueDate,
      description: `Sale: ${sale.product_name} (${sale.quantity})`,
      sale_id: saleId
    };

    if (debtId) {
      await Store.updateDebt(parseInt(debtId), debtData);
      Toast.success('Debt Updated', `Debt for ${customerName} updated successfully`);
    } else {
      await Store.addDebt(debtData);
      Toast.success('Debt Created', `Debt of KSh ${remainingDebt.toLocaleString()} created for ${customerName}`);
    }
    
    // Mark the sale as a debt (ensures the badge shows)
    await Store.updateSale(saleId, { is_debt: 1 });
    
    this.closeConvertDebtModal();
  },

  bindDatePickerEvents() {
    const datePickerModal = document.getElementById('modal-sales-date-picker');
    const btnClose = document.getElementById('btn-close-sales-date-picker');
    const btnPrevMonth = document.getElementById('btn-sales-picker-prev-month');
    const btnNextMonth = document.getElementById('btn-sales-picker-next-month');
    const btnClear = document.getElementById('btn-sales-clear-date');
    const btnToday = document.getElementById('btn-sales-today-date');

    if (btnClose && !btnClose.dataset.bound) {
      btnClose.dataset.bound = 'true';
      btnClose.addEventListener('click', () => {
        datePickerModal.classList.remove('open');
      });
    }

    if (btnPrevMonth && !btnPrevMonth.dataset.bound) {
      btnPrevMonth.dataset.bound = 'true';
      btnPrevMonth.addEventListener('click', () => {
        this.pickerMonth--;
        if (this.pickerMonth < 0) {
          this.pickerMonth = 11;
          this.pickerYear--;
        }
        this.renderDatePicker();
      });
    }

    if (btnNextMonth && !btnNextMonth.dataset.bound) {
      btnNextMonth.dataset.bound = 'true';
      btnNextMonth.addEventListener('click', () => {
        this.pickerMonth++;
        if (this.pickerMonth > 11) {
          this.pickerMonth = 0;
          this.pickerYear++;
        }
        this.renderDatePicker();
      });
    }

    if (btnClear && !btnClear.dataset.bound) {
      btnClear.dataset.bound = 'true';
      btnClear.addEventListener('click', () => {
        const hiddenInput = document.getElementById('convert-due-date');
        const displayInput = document.getElementById('convert-due-date-display');
        if (hiddenInput) hiddenInput.value = '';
        if (displayInput) displayInput.value = '';
        datePickerModal.classList.remove('open');
      });
    }

    if (btnToday && !btnToday.dataset.bound) {
      btnToday.dataset.bound = 'true';
      btnToday.addEventListener('click', () => {
        const today = new Date();
        this.setPickerDate(today);
        datePickerModal.classList.remove('open');
      });
    }
  },

  renderDatePicker() {
    const monthYearEl = document.getElementById('sales-picker-month-year');
    const gridEl = document.getElementById('sales-picker-calendar-grid');
    
    if (!gridEl) return;

    // Update month/year display
    const monthNames = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
    if (monthYearEl) {
      monthYearEl.textContent = `${monthNames[this.pickerMonth]} ${this.pickerYear}`;
    }

    // Get first and last day of month
    const firstDay = new Date(this.pickerYear, this.pickerMonth, 1);
    const lastDay = new Date(this.pickerYear, this.pickerMonth + 1, 0);
    const daysInMonth = lastDay.getDate();
    const startingDayOfWeek = firstDay.getDay();

    // Clear existing days (keep headers)
    const headers = gridEl.querySelectorAll('.calendar-day-header');
    gridEl.innerHTML = '';
    headers.forEach(h => gridEl.appendChild(h));

    // Add empty cells for days before month starts
    for (let i = 0; i < startingDayOfWeek; i++) {
      const emptyDay = document.createElement('div');
      emptyDay.className = 'calendar-day other-month';
      gridEl.appendChild(emptyDay);
    }

    // Add days of the month
    const today = new Date();
    for (let day = 1; day <= daysInMonth; day++) {
      const dayEl = document.createElement('div');
      dayEl.className = 'calendar-day';
      
      // Check if today
      if (day === today.getDate() && this.pickerMonth === today.getMonth() && this.pickerYear === today.getFullYear()) {
        dayEl.classList.add('today');
      }

      // Check if selected
      if (this.pickerSelectedDate && 
          this.pickerSelectedDate.getDate() === day && 
          this.pickerSelectedDate.getMonth() === this.pickerMonth && 
          this.pickerSelectedDate.getFullYear() === this.pickerYear) {
        dayEl.classList.add('selected');
      }

      // Day number
      const dayNumber = document.createElement('div');
      dayNumber.className = 'calendar-day-number';
      dayNumber.textContent = day;
      dayEl.appendChild(dayNumber);

      // Click handler
      dayEl.addEventListener('click', () => {
        const selectedDate = new Date(this.pickerYear, this.pickerMonth, day);
        this.setPickerDate(selectedDate);
        // Close picker after selection
        const datePickerModal = document.getElementById('modal-sales-date-picker');
        if (datePickerModal) datePickerModal.classList.remove('open');
      });

      gridEl.appendChild(dayEl);
    }
  },

  setPickerDate(date) {
    this.pickerSelectedDate = date;
    
    // Format date as YYYY-MM-DD for the hidden input
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const formattedValue = `${year}-${month}-${day}`;
    
    // Format date for display (e.g., "January 15, 2024")
    const monthNames = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
    const formattedDisplay = `${monthNames[date.getMonth()]} ${date.getDate()}, ${date.getFullYear()}`;
    
    // Check if this is for the convert debt modal or printing debt modal
    if (this.pickerCallback === 'convertDebt' && window.convertDebtDateCallback) {
      window.convertDebtDateCallback(formattedValue, formattedDisplay);
      this.pickerCallback = null;
    } else if (this.pickerCallback === 'convertPrintingDebt' && window.convertPrintingDebtDateCallback) {
      window.convertPrintingDebtDateCallback(formattedValue, formattedDisplay);
      this.pickerCallback = null;
    } else {
      // Normal sales conversion
      const hiddenInput = document.getElementById('convert-due-date');
      const displayInput = document.getElementById('convert-due-date-display');
      
      if (hiddenInput) hiddenInput.value = formattedValue;
      if (displayInput) displayInput.value = formattedDisplay;
    }
    
    this.renderDatePicker();
  }
};

window.SalesPage = SalesPage;
