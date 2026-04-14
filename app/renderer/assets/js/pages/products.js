/**
 * Products Page Controller
 * Manages Life Savers and Chevrons inventory
 */

const ProductsPage = {
  selectedProductType: 'life_saver',
  selectedColor: 'white_red',
  selectedSize: '1x1',
  currentPage: 1,
  itemsPerPage: 10,

  init() {
    this.bindEvents();
    this.render();
    this.updateSummary();
    
    // Subscribe to store changes
    Store.subscribe('products', () => {
      this.render();
      this.updateSummary();
    });
  },

  bindEvents() {
    // Modal Elements
    const modal = document.getElementById('modal-add-product');
    const btnAdd = document.getElementById('btn-add-product');
    const btnClose = document.getElementById('btn-close-product-modal');
    const btnCancel = document.getElementById('btn-cancel-product');
    const addForm = document.getElementById('add-product-form');

    // Open Modal
    if (btnAdd && modal) {
      btnAdd.addEventListener('click', () => {
        modal.classList.add('open');
        this.resetModal();
      });
    }

    // Close Modal Helper
    const closeModal = () => {
        if (modal) {
            modal.classList.remove('open');
            addForm?.reset();
        }
    };

    // Close Button Actions
    if (btnClose) btnClose.addEventListener('click', closeModal);
    if (btnCancel) btnCancel.addEventListener('click', closeModal);

    // Close on Click Outside
    if (modal) {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                closeModal();
            }
        });
    }

    // Product Type Selector
    const typeButtons = document.querySelectorAll('.product-type-btn');
    typeButtons.forEach(btn => {
      btn.addEventListener('click', () => {
        this.selectProductType(btn.dataset.type);
      });
    });

    // Color Selector
    const colorButtons = document.querySelectorAll('.product-color-btn');
    colorButtons.forEach(btn => {
      btn.addEventListener('click', () => {
        this.selectColor(btn.dataset.color);
      });
    });

    // Size Selector
    const sizeButtons = document.querySelectorAll('.product-size-btn');
    sizeButtons.forEach(btn => {
      btn.addEventListener('click', () => {
        this.selectSize(btn.dataset.size);
      });
    });

    // Handle Form Submit
    if (addForm && !addForm.dataset.bound) {
      addForm.dataset.bound = 'true';
      addForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        await this.handleSubmit(new FormData(addForm));
        closeModal();
      });
    }
  },

  resetModal() {
    this.selectedProductType = 'life_saver';
    this.selectedColor = 'white_red';
    this.selectedSize = '1x1';

    // Explicitly reset hidden inputs
    const typeInput = document.getElementById('product-type-input');
    const colorInput = document.getElementById('product-color-input');
    const sizeInput = document.getElementById('product-size-input');

    if (typeInput) typeInput.value = 'life_saver';
    if (colorInput) colorInput.value = 'white_red';
    if (sizeInput) sizeInput.value = '1x1';

    this.updateTypeUI();
    this.updateColorUI();
    this.updateSizeUI();
    this.toggleColorSizeSections();
  },

  selectProductType(type) {
    this.selectedProductType = type;
    const hiddenInput = document.getElementById('product-type-input');
    if (hiddenInput) hiddenInput.value = type;
    this.updateTypeUI();
    this.toggleColorSizeSections();
  },

  toggleColorSizeSections() {
    const colorSection = document.getElementById('color-section');
    const sizeSection = document.getElementById('size-section');
    const chevronColors = document.querySelectorAll('.chevron-color');
    const stripeColors = document.querySelectorAll('.stripe-color');
    
    if (this.selectedProductType === 'chevron') {
      // Show color and size for Chevrons
      if (colorSection) colorSection.classList.remove('hidden');
      if (sizeSection) sizeSection.classList.remove('hidden');
      // Show chevron colors, hide stripe colors
      chevronColors.forEach(btn => btn.classList.remove('hidden'));
      stripeColors.forEach(btn => btn.classList.add('hidden'));
      // Set default to chevron color
      this.selectedColor = 'white_red';
      document.getElementById('product-color-input').value = 'white_red';
      this.updateColorUI();
    } else if (this.selectedProductType === 'stripes') {
      // Show color for Stripes, hide size
      if (colorSection) colorSection.classList.remove('hidden');
      if (sizeSection) sizeSection.classList.add('hidden');
      // Show stripe colors, hide chevron colors
      chevronColors.forEach(btn => btn.classList.add('hidden'));
      stripeColors.forEach(btn => btn.classList.remove('hidden'));
      // Set default to stripe color
      this.selectedColor = 'white';
      document.getElementById('product-color-input').value = 'white';
      this.updateColorUI();
    } else {
      // Hide color and size for Life Savers
      if (colorSection) colorSection.classList.add('hidden');
      if (sizeSection) sizeSection.classList.add('hidden');
    }
  },

  updateTypeUI() {
    const typeButtons = document.querySelectorAll('.product-type-btn');
    const typeLabels = {
      life_saver: { name: 'Life Saver', desc: 'Warning triangle', icon: '<svg viewBox="0 0 24 24" class="w-6 h-6"><polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2.5"/><text x="12" y="16" text-anchor="middle" font-size="10" font-weight="bold" fill="#1a1a1a">!</text></svg>' },
      chevron: { name: 'Chevron', desc: 'Arrow pattern', icon: null },
      stripes: { name: 'Stripes', desc: 'Line pattern', icon: null }
    };
    
    typeButtons.forEach(btn => {
      const btnType = btn.dataset.type;
      const info = typeLabels[btnType] || { name: btnType, desc: '', icon: null };
      
      if (btnType === this.selectedProductType) {
        btn.className = 'product-type-btn flex-1 px-4 py-3 border border-gray-900 bg-gray-50 text-sm transition-all flex items-center gap-3';
        if (btn.dataset.for === 'chevron') btn.classList.add('chevron-type');
        btn.innerHTML = info.icon ? `
          ${info.icon}
          <div>
            <div class="font-medium text-gray-900">${info.name}</div>
            <div class="text-xs text-gray-500">${info.desc}</div>
          </div>
        ` : `<div class="font-medium text-gray-900">${info.name}</div><div class="text-xs text-gray-500">${info.desc}</div>`;
      } else {
        btn.className = 'product-type-btn flex-1 px-4 py-3 border border-gray-200 bg-white text-sm transition-all hover:border-gray-300 flex items-center gap-3';
        if (btn.dataset.for === 'chevron') btn.classList.add('chevron-type');
        btn.innerHTML = info.icon ? `
          ${info.icon}
          <div>
            <div class="font-medium text-gray-500">${info.name}</div>
            <div class="text-xs text-gray-400">${info.desc}</div>
          </div>
        ` : `<div class="font-medium text-gray-500">${info.name}</div><div class="text-xs text-gray-400">${info.desc}</div>`;
      }
    });
  },

  selectColor(color) {
    this.selectedColor = color;
    const hiddenInput = document.getElementById('product-color-input');
    if (hiddenInput) hiddenInput.value = color;
    this.updateColorUI();
  },

  updateColorUI() {
    const colorButtons = document.querySelectorAll('.product-color-btn');
    const colorInfo = {
      white_red: { name: 'White / Red', desc: 'Chevron', gradient: 'linear-gradient(135deg, #ffffff 50%, #ef4444 50%)' },
      yellow_red: { name: 'Yellow / Red', desc: 'Chevron', gradient: 'linear-gradient(135deg, #eab308 50%, #ef4444 50%)' },
      white: { name: 'White', desc: 'Stripe', color: '#ffffff' },
      yellow: { name: 'Yellow', desc: 'Stripe', color: '#eab308' }
    };
    
    colorButtons.forEach(btn => {
      const btnColor = btn.dataset.color;
      const isHidden = btn.classList.contains('hidden');
      const info = colorInfo[btnColor] || { name: btnColor, desc: '' };
      
      // Skip hidden buttons
      if (isHidden) return;
      
      const isChevron = btn.dataset.for === 'chevron';
      const swatchStyle = isChevron 
        ? `background: ${info.gradient};` 
        : `background-color: ${info.color || '#fff'};${info.color === '#ffffff' ? ' border: 1px solid #d1d5db;' : ''}`;
      
      if (btnColor === this.selectedColor) {
        btn.className = 'product-color-btn flex-1 px-4 py-3 border border-gray-900 bg-gray-50 text-sm transition-all flex items-center gap-3';
        if (btn.dataset.for === 'chevron') btn.classList.add('chevron-color');
        else if (btn.dataset.for === 'stripes') btn.classList.add('stripe-color');
        btn.innerHTML = `
          <div class="w-8 h-8 rounded-sm shadow-sm flex-shrink-0" style="${swatchStyle}"></div>
          <div>
            <div class="font-medium text-gray-900">${info.name}</div>
            <div class="text-xs text-gray-500">${info.desc}</div>
          </div>`;
      } else {
        btn.className = 'product-color-btn flex-1 px-4 py-3 border border-gray-200 bg-white text-sm transition-all flex items-center gap-3 hover:border-gray-300';
        if (btn.dataset.for === 'chevron') btn.classList.add('chevron-color');
        else if (btn.dataset.for === 'stripes') btn.classList.add('stripe-color');
        btn.innerHTML = `
          <div class="w-8 h-8 rounded-sm shadow-sm flex-shrink-0" style="${swatchStyle}"></div>
          <div>
            <div class="font-medium text-gray-500">${info.name}</div>
            <div class="text-xs text-gray-400">${info.desc}</div>
          </div>`;
      }
    });
  },

  selectSize(size) {
    this.selectedSize = size;
    const hiddenInput = document.getElementById('product-size-input');
    if (hiddenInput) hiddenInput.value = size;
    this.updateSizeUI();
  },

  updateSizeUI() {
    const sizeButtons = document.querySelectorAll('.product-size-btn');
    
    sizeButtons.forEach(btn => {
      const btnSize = btn.dataset.size;
      const isSelected = btnSize === this.selectedSize;
      
      if (isSelected) {
        btn.className = 'product-size-btn flex-1 px-4 py-3 border border-gray-900 bg-gray-50 text-sm transition-all';
        // Update inner content
        const label = btnSize === '1x1' ? 'Standard' : 'Large';
        btn.innerHTML = `<div class="font-medium text-gray-900">${btnSize}</div><div class="text-xs text-gray-500">${label}</div>`;
      } else {
        btn.className = 'product-size-btn flex-1 px-4 py-3 border border-gray-200 bg-white text-sm transition-all hover:border-gray-300';
        const label = btnSize === '1x1' ? 'Standard' : 'Large';
        btn.innerHTML = `<div class="font-medium text-gray-500">${btnSize}</div><div class="text-xs text-gray-400">${label}</div>`;
      }
    });
  },

  async handleSubmit(formData) {
    const productType = formData.get('product_type');
    const productColor = (productType === 'chevron' || productType === 'stripes') ? formData.get('product_color') : null;
    const productSize = productType === 'chevron' ? formData.get('product_size') : null;
    
    const typeConfig = PRODUCT_TYPES[productType];
    const colorConfig = productColor ? PRODUCT_COLORS[productColor] : null;
    
    // Generate product name
    let productName;
    if (productType === 'life_saver') {
      productName = 'Life Saver';
    } else if (productType === 'stripes') {
      productName = `${colorConfig.name} ${typeConfig.name}`;
    } else {
      productName = `${colorConfig.name} ${typeConfig.name} (${productSize})`;
    }

    // Check if this product variant already exists
    const existing = Store.products.find(p => {
      if (productType === 'life_saver') {
        return p.product_type === 'life_saver';
      }
      if (productType === 'stripes') {
        return p.product_type === 'stripes' && p.color === productColor;
      }
      return p.product_type === productType && 
             p.color === productColor && 
             p.size === productSize;
    });

    if (existing) {
      // Update stock of existing product
      const additionalStock = parseInt(formData.get('stock'));
      await Store.updateProduct(existing.id, { 
        stock: existing.stock + additionalStock
      });
      Toast.success('Stock Updated', `Added ${additionalStock} units to ${productName}`);
      return;
    }

    const product = {
      name: productName,
      product_type: productType,
      color: productColor,
      size: productSize,
      selling_price: 0,
      stock: parseInt(formData.get('stock'))
    };
    
    await Store.addProduct(product);
    Toast.success('Product Added', `${productName} with ${product.stock} units`);
  },

  updateSummary() {
    const products = Store.products;
    
    const totalProducts = products.length;
    const lifeSavers = products.filter(p => p.product_type === 'life_saver').reduce((sum, p) => sum + p.stock, 0);
    const chevrons = products.filter(p => p.product_type === 'chevron').reduce((sum, p) => sum + p.stock, 0);
    const stripes = products.filter(p => p.product_type === 'stripes').reduce((sum, p) => sum + p.stock, 0);
    const stockValue = products.reduce((sum, p) => sum + (p.stock * (p.selling_price || 0)), 0);

    const totalEl = document.getElementById('summary-total-products');
    const lifeSaversEl = document.getElementById('summary-life-savers');
    const chevronsEl = document.getElementById('summary-chevrons');
    const stripesEl = document.getElementById('summary-stripes');
    const valueEl = document.getElementById('summary-stock-value');

    if (totalEl) totalEl.textContent = totalProducts;
    if (lifeSaversEl) lifeSaversEl.textContent = lifeSavers;
    if (chevronsEl) chevronsEl.textContent = chevrons;
    if (stripesEl) stripesEl.textContent = stripes;
    if (valueEl) valueEl.textContent = `KSh ${stockValue.toLocaleString()}`;
  },

  render() {
    const tbody = document.getElementById('products-table-body');
    if (!tbody) return;

    const products = Store.products;

    if (products.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="5" class="px-6 py-12 text-center text-gray-500">
             <div class="flex flex-col items-center justify-center">
                <svg class="w-12 h-12 text-gray-300 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path>
                </svg>
                <p>No products added yet</p>
                <button onclick="document.getElementById('btn-add-product').click()" class="text-black font-semibold hover:underline text-sm mt-2">Add your first product</button>
            </div>
          </td>
        </tr>
      `;
      this.updatePaginationControls(0);
      return;
    }

    // Calculate pagination
    const totalPages = Math.ceil(products.length / this.itemsPerPage);
    const startIndex = (this.currentPage - 1) * this.itemsPerPage;
    const endIndex = startIndex + this.itemsPerPage;
    const paginatedProducts = products.slice(startIndex, endIndex);

    tbody.innerHTML = paginatedProducts.map(product => {
      const typeConfig = PRODUCT_TYPES[product.product_type] || PRODUCT_TYPES.life_saver;
      const colorConfig = product.color ? PRODUCT_COLORS[product.color] : null;
      
      return `
      <tr class="hover:bg-gray-50 transition-colors">
        <td class="px-6 py-4">
          ${product.product_type === 'life_saver' ? `
            <div class="flex items-center gap-2">
              <svg viewBox="0 0 24 24" class="w-5 h-5">
                <polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/>
                <text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">!</text>
              </svg>
              <span class="text-sm font-medium text-gray-900">Life Saver</span>
            </div>
          ` : `
            <span class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium ${typeConfig.badgeClass}">
              ${typeConfig.name}
            </span>
          `}
        </td>
        <td class="px-6 py-4">
          ${colorConfig ? `
          <div class="flex items-center gap-2">
            ${product.product_type === 'life_saver' ? `
              <div class="w-6 h-6 relative flex items-center justify-center">
                <svg viewBox="0 0 24 24" class="w-6 h-6">
                  <polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/>
                  <text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">!</text>
                </svg>
              </div>
            ` : product.product_type === 'chevron' ? `
              <div class="w-6 h-6 rounded-sm shadow-sm border border-gray-200" 
                   style="background: linear-gradient(135deg, ${colorConfig.colors[0]} 50%, ${colorConfig.colors[1]} 50%);"></div>
            ` : `
              <div class="w-6 h-6 rounded-sm shadow-sm border border-gray-200" 
                   style="background-color: ${colorConfig.colors[0]};"></div>
            `}
            <span class="text-sm text-gray-700">${colorConfig.name}</span>
          </div>
          ` : product.product_type === 'life_saver' ? `
          <div class="flex items-center gap-2">
            <div class="w-6 h-6 relative flex items-center justify-center">
              <svg viewBox="0 0 24 24" class="w-6 h-6">
                <polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/>
                <text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">!</text>
              </svg>
            </div>
            <span class="text-sm text-gray-400">Standard</span>
          </div>
          ` : '<span class="text-sm text-gray-400">-</span>'}
        </td>
        <td class="px-6 py-4 text-sm text-gray-600 font-medium">${product.size || '-'}</td>
        <td class="px-6 py-4">
          <div class="flex items-center gap-2">
            <span class="status-badge ${product.stock > 10 ? 'status-badge--success' : product.stock > 0 ? 'status-badge--warning' : 'status-badge--error'}">
               ${product.stock} Units
            </span>
            <button onclick="ProductsPage.addStock(${product.id})" class="px-2 py-1 text-xs font-medium bg-black text-white rounded-md hover:bg-gray-800 transition-colors">+ Add</button>
          </div>
        </td>
        <td class="px-6 py-4">
          <button onclick="ProductsPage.delete(${product.id})" class="text-gray-400 hover:text-red-600 transition-colors">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
            </svg>
          </button>
        </td>
      </tr>
    `}).join('');

    // Update pagination controls
    this.updatePaginationControls(products.length);
  },

  updatePaginationControls(totalItems) {
    const paginationEl = document.getElementById('products-pagination');
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
          Showing <span class="font-medium">${startItem}</span> to <span class="font-medium">${endItem}</span> of <span class="font-medium">${totalItems}</span> products
        </div>
        <div class="flex gap-2">
          <button onclick="ProductsPage.previousPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === 1 ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === 1 ? 'disabled' : ''}>
            Previous
          </button>
          <span class="px-3 py-1 text-sm font-medium text-gray-700">
            Page ${this.currentPage} of ${totalPages}
          </span>
          <button onclick="ProductsPage.nextPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === totalPages ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === totalPages ? 'disabled' : ''}>
            Next
          </button>
        </div>
      </div>
    `;
  },

  nextPage() {
    const totalPages = Math.ceil(Store.products.length / this.itemsPerPage);
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

  addStock(id) {
    const product = Store.getProduct(id);
    if (!product) return;

    const typeConfig = PRODUCT_TYPES[product.product_type] || PRODUCT_TYPES.life_saver;

    // Create modal HTML
    const modalHTML = `
      <div id="modal-add-product-stock" class="modal-overlay open">
        <div class="modal-container" style="max-width: 500px;">
          <div class="modal-header">
            <h3 class="modal-title">Add Product Stock</h3>
            <button class="modal-close-btn" onclick="ProductsPage.closeAddStockModal()">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="bg-gray-50 p-4 mb-4">
              <p class="text-xs text-gray-500 uppercase tracking-wide">Product</p>
              <p class="font-semibold text-gray-900">${product.name}</p>
              <p class="text-sm text-gray-600 mt-1">
                <span class="inline-flex items-center px-2 py-0.5 text-xs font-medium ${typeConfig.badgeClass}">
                  ${typeConfig.name}
                </span>
                <span class="ml-2">Current stock: ${product.stock} units</span>
              </p>
            </div>

            <form id="add-product-stock-form" class="space-y-4" action="javascript:void(0);">
              <input type="hidden" id="add-stock-product-id" value="${id}">

              <div>
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Quantity to Add *</label>
                <input type="number" id="add-stock-quantity-input" min="1" step="1" class="w-full" placeholder="Enter units to add" required autofocus>
                <p class="text-xs text-gray-500 mt-1">Add units to existing stock</p>
              </div>

              <div class="bg-blue-50 border border-blue-200 p-3">
                <p class="text-sm text-gray-700">
                  <span class="font-medium">New Total Stock:</span>
                  <span id="new-total-stock">${product.stock}</span> units
                </p>
              </div>
            </form>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn-secondary px-4 py-2" onclick="ProductsPage.closeAddStockModal()">Cancel</button>
            <button type="submit" form="add-product-stock-form" class="btn-primary px-4 py-2">Add Stock</button>
          </div>
        </div>
      </div>
    `;

    // Add modal to page
    const existingModal = document.getElementById('modal-add-product-stock');
    if (existingModal) existingModal.remove();
    document.body.insertAdjacentHTML('beforeend', modalHTML);

    // Add form submit handler
    const form = document.getElementById('add-product-stock-form');
    if (form) {
      form.addEventListener('submit', (e) => {
        e.preventDefault();
        this.submitAddStock();
      });
    }

    // Add event listener for real-time calculation
    const qtyInput = document.getElementById('add-stock-quantity-input');
    if (qtyInput) {
      qtyInput.addEventListener('input', () => {
        const additionalQty = parseInt(qtyInput.value) || 0;
        const newTotalStock = product.stock + additionalQty;
        
        document.getElementById('new-total-stock').textContent = newTotalStock;
      });
    }
  },

  closeAddStockModal() {
    const modal = document.getElementById('modal-add-product-stock');
    if (modal) modal.remove();
  },

  submitAddStock() {
    const productId = parseInt(document.getElementById('add-stock-product-id').value);
    const qty = parseInt(document.getElementById('add-stock-quantity-input').value);

    if (!qty || qty <= 0) {
      Toast.error('Invalid Input', 'Please enter a valid quantity');
      return;
    }

    const product = Store.getProduct(productId);
    if (product) {
      Store.updateProduct(productId, { stock: product.stock + qty });
      Toast.success('Stock Added', `Added ${qty} unit${qty > 1 ? 's' : ''} to ${product.name}`);
    }

    this.closeAddStockModal();
  },

  delete(id) {
    const product = Store.getProduct(id);
    if (!product) return;

    const typeConfig = PRODUCT_TYPES[product.product_type] || {};
    
    ConfirmModal.show({
      title: 'Delete Product?',
      message: 'Are you sure you want to delete this product? This action cannot be undone.',
      itemName: product.name,
      itemDetails: `${typeConfig.name || 'Product'} • ${product.stock} units in stock`,
      onConfirm: () => {
        Store.deleteProduct(id);
        Toast.success('Product Deleted', `${product.name} has been removed`);
      }
    });
  }
};

window.ProductsPage = ProductsPage;
