/**
 * Printing Services Page Controller
 * Handles printing jobs for one-way vision, banners, satin, and reflective materials
 */

const PrintingPage = {
  materialDropdown: null,
  paymentDropdown: null,
  pickerMonth: new Date().getMonth(),
  pickerYear: new Date().getFullYear(),
  pickerSelectedDate: null,
  currentPage: 1,
  itemsPerPage: 10,

  init() {
    this.initCustomDropdowns();
    this.bindEvents();
    this.render();
    this.updateStats();

    // Hide stats if employee
    if (window.Permissions && window.Permissions.getCurrentRole() === 'employee') {
      const statsContainer = document.getElementById('printing-stats-container');
      if (statsContainer) statsContainer.style.display = 'none';

      // Hide Add Material button for employees
      const addMaterialBtn = document.getElementById('btn-add-material');
      if (addMaterialBtn) addMaterialBtn.style.display = 'none';
    }
    
    // Subscribe to store changes
    Store.subscribe('serviceTransactions', () => {
      this.render();
      this.updateStats();
    });
    Store.subscribe('printingMaterials', () => {
      this.renderPrintingMaterials();
      this.initMaterialDropdown();
    });
  },

  initCustomDropdowns() {
    // Material Dropdown
    this.initMaterialDropdown();

    // Payment Method Dropdown
    const paymentContainer = document.getElementById('print-payment-dropdown');
    if (paymentContainer) {
      this.paymentDropdown = new CustomDropdown(paymentContainer, {
        placeholder: 'Cash',
        items: [
          { value: 'cash', label: 'Cash' },
          { value: 'mpesa', label: 'M-Pesa' },
          { value: 'till', label: 'Till Number' }
        ],
        onSelect: (selected) => {
          const hiddenInput = document.getElementById('print-payment-input');
          if (hiddenInput) hiddenInput.value = selected.value;
        }
      });
      // Auto-select first item
      this.paymentDropdown.selectItem(paymentContainer.querySelector('.dropdown-item'));
    }
  },

  initMaterialDropdown() {
    const materialContainer = document.getElementById('print-material-dropdown');
    if (materialContainer) {
      // Only show printing materials (Banner, Satin, Canvas, etc.) - NOT stock
      const printingMaterials = Store.getAvailablePrintingMaterials();
      
      const allMaterials = printingMaterials.map(m => ({
        value: `pm_${m.id}`,
        label: `${m.name} - ${m.width}m width`,
        type: 'printing_material',
        id: m.id,
        width: m.width,
        material_type: m.material_type,
        color: m.color,
        remaining: m.remaining,
        badge: `${m.remaining.toFixed(1)}m`
      }));
      
      this.materialDropdown = new CustomDropdown(materialContainer, {
        placeholder: allMaterials.length > 0 ? 'Select material' : 'No materials available',
        items: allMaterials,
        onSelect: (selected) => {
          const hiddenInput = document.getElementById('print-stock-id-input');
          if (hiddenInput) hiddenInput.value = selected.value;
          this.updateMaterialInfo(selected);
          this.updatePrintingCalculations();
        }
      });
    }
  },

  refreshMaterialDropdown() {
    // Reset the current material selection
    if (this.materialDropdown) {
      this.materialDropdown.reset();
    }
    
    // Clear material info
    const infoEl = document.getElementById('print-material-info');
    if (infoEl) infoEl.textContent = '';
    
    // Reinitialize
    this.initMaterialDropdown();
  },

  bindEvents() {
    // Open/Close Modal
    const modal = document.getElementById('modal-record-printing');
    const btnOpen = document.getElementById('btn-record-printing');
    const btnClose = document.getElementById('btn-close-printing-modal');
    const btnCancel = document.getElementById('btn-cancel-printing');
    const form = document.getElementById('record-printing-form');

    const openModal = () => {
      if (modal) {
        this.initMaterialDropdown();
        modal.classList.add('open');
      }
    };

    const closeModal = () => {
      if (modal) {
        modal.classList.remove('open');
        form?.reset();
        this.materialDropdown?.reset();
        this.paymentDropdown?.reset();
        // Re-select payment default
        const paymentContainer = document.getElementById('print-payment-dropdown');
        if (paymentContainer && this.paymentDropdown) {
          this.paymentDropdown.selectItem(paymentContainer.querySelector('.dropdown-item'));
        }
        this.updatePrintingCalculations();
      }
    };

    if (btnOpen) btnOpen.addEventListener('click', openModal);
    if (btnClose) btnClose.addEventListener('click', closeModal);
    if (btnCancel) btnCancel.addEventListener('click', closeModal);
    
    if (modal && !modal.dataset.bound) {
      modal.dataset.bound = 'true';
      modal.addEventListener('click', (e) => {
        if (e.target === modal) closeModal();
      });
    }

    if (form && !form.dataset.bound) {
      form.dataset.bound = 'true';
      form.addEventListener('submit', (e) => {
        e.preventDefault();
        this.handleRecordPrintingJob(new FormData(form));
        closeModal();
      });

      // Real-time calculation updates
      const metresPrintedInput = document.getElementById('metres-printed');
      const totalPriceInput = document.getElementById('print-total-price');

      [metresPrintedInput, totalPriceInput].forEach(input => {
        if (input) {
          input.addEventListener('input', () => this.updatePrintingCalculations());
        }
      });
    }
    
    // Add Material Modal
    const materialModal = document.getElementById('modal-add-material');
    const btnAddMaterial = document.getElementById('btn-add-material');
    const btnCloseMaterial = document.getElementById('btn-close-material-modal');
    const btnCancelMaterial = document.getElementById('btn-cancel-material');
    const materialForm = document.getElementById('add-material-form');

    const openMaterialModal = () => {
      if (materialModal) materialModal.classList.add('open');
    };

    const closeMaterialModal = () => {
      if (materialModal) {
        materialModal.classList.remove('open');
        materialForm?.reset();
      }
    };

    if (btnAddMaterial) btnAddMaterial.addEventListener('click', openMaterialModal);
    if (btnCloseMaterial) btnCloseMaterial.addEventListener('click', closeMaterialModal);
    if (btnCancelMaterial) btnCancelMaterial.addEventListener('click', closeMaterialModal);
    
    if (materialModal && !materialModal.dataset.bound) {
      materialModal.dataset.bound = 'true';
      materialModal.addEventListener('click', (e) => {
        if (e.target === materialModal) closeMaterialModal();
      });
    }

    if (materialForm && !materialForm.dataset.bound) {
      materialForm.dataset.bound = 'true';
      materialForm.addEventListener('submit', (e) => {
        e.preventDefault();
        this.handleAddMaterial(new FormData(materialForm));
        closeMaterialModal();
      });
    }
  },

  updateMaterialInfo(selected) {
    const infoEl = document.getElementById('print-material-info');
    if (!infoEl || !selected) return;
    
    const remaining = parseFloat(selected.remaining);
    let info = '';
    
    if (selected.type === 'printing_material') {
      const color = selected.color ? ` ${selected.color}` : '';
      info = `${selected.material_type}${color} - ${selected.width}m width (${remaining.toFixed(1)}m available)`;
    } else {
      // Stock material
      const size = selected.size || '1';
      const stickerType = selected.stickerType || 'colored';
      info = `${selected.color} - ${size}m ${stickerType} (${remaining.toFixed(1)}m available)`;
    }
    
    infoEl.textContent = info;
  },

  updatePrintingCalculations() {
    const totalPriceInput = document.getElementById('print-total-price');
    const totalDisplay = document.getElementById('printing-total');

    if (!totalPriceInput) return;

    const totalPrice = parseFloat(totalPriceInput.value) || 0;

    if (totalDisplay) totalDisplay.textContent = `KSh ${totalPrice.toFixed(2)}`;
  },

  async handleRecordPrintingJob(formData) {
    const metresPrinted = parseFloat(formData.get('metres_printed'));
    const totalPrice = parseFloat(formData.get('total_price'));
    const materialValue = formData.get('stock_id');

    // Validations
    if (!materialValue) {
      Toast.error('Missing Material', 'Please select material from available stock');
      return;
    }

    if (!metresPrinted || metresPrinted <= 0) {
      Toast.error('Invalid Metres', 'Please enter valid metres printed');
      return;
    }

    if (!totalPrice || totalPrice <= 0) {
      Toast.error('Invalid Price', 'Please enter valid total price');
      return;
    }

    // Parse material ID (format: pm_123)
    const [materialType, materialId] = materialValue.split('_');
    const id = parseInt(materialId);
    
    // Get printing material
    const stockItem = Store.getPrintingMaterial(id);
    if (!stockItem) {
      Toast.error('Material Not Found', 'Selected printing material not found');
      return;
    }
    
    const remaining = stockItem.total_metres - stockItem.metres_used;
    if (metresPrinted > remaining) {
      Toast.error('Insufficient Material', `Only ${remaining.toFixed(1)}m available. You need ${metresPrinted.toFixed(1)}m`);
      return;
    }

    // Create transaction
    const transaction = {
      service_id: null,
      service_name: `${stockItem.name} - ${metresPrinted}m`,
      quantity: 1,
      price: totalPrice,
      amount: totalPrice,
      payment_method: formData.get('payment_method') || 'cash',
      customer_name: formData.get('customer_name') || 'Walk-in',
      notes: formData.get('notes') || `Printing - ${metresPrinted}m`,
      stock_id: null,
      stock_metres_used: metresPrinted,
      material_size: stockItem.width,
      material_type: stockItem.material_type || 'Custom',
      printing_material_id: id // Save the material ID for reversal when deleting
    };

    const result = await Store.addServiceTransaction(transaction);
    
    if (result && result.success === false) {
      Toast.error('Job Failed', result.error);
      return;
    }
    
    // Deduct from printing material
    await Store.deductPrintingMaterial(id, metresPrinted);
    
    Toast.success('Job Recorded', `${stockItem.name} - KSh ${totalPrice.toLocaleString()} (${metresPrinted.toFixed(1)}m used)`);
  },

  render() {
    this.renderPrintingJobs();
    this.renderPrintingMaterials();
  },

  async handleAddMaterial(formData) {
    const name = formData.get('name');
    const width = parseFloat(formData.get('width'));
    const rolls = parseInt(formData.get('rolls'));
    const metresPerRoll = parseFloat(formData.get('metres_per_roll')) || 50;

    if (!name || !width || !rolls) {
      Toast.error('Missing Information', 'Please fill in all required fields');
      return;
    }

    const material = {
      name,
      material_type: 'Custom', // Default type since user enters custom name
      width,
      rolls,
      metres_per_roll: metresPerRoll,
      color: null // Color is now part of the name
    };

    await Store.addPrintingMaterial(material);
    Toast.success('Material Added', `${material.name} has been added successfully`);
  },

  renderPrintingMaterials() {
    const container = document.getElementById('printing-materials-list');
    if (!container) return;

    if (Store.printingMaterials.length === 0) {
      container.innerHTML = `
        <div class="text-center py-8 text-gray-400">
          <svg class="w-12 h-12 mx-auto mb-3 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path>
          </svg>
          <p>No materials added yet</p>
        </div>
      `;
      return;
    }

    container.innerHTML = Store.printingMaterials.map(material => {
      const remaining = material.total_metres - material.metres_used;
      const metresPerRoll = material.metres_per_roll; // length per roll in metres
      const rollsRemaining = (remaining / metresPerRoll).toFixed(1);
      const percentage = (remaining / material.total_metres) * 100;
      const statusColor = percentage > 20 ? 'text-green-600' : percentage > 10 ? 'text-yellow-600' : 'text-red-600';

      return `
        <div class="flex items-center justify-between p-4 bg-white border border-gray-200 rounded-lg hover:border-gray-300 transition-colors">
          <div class="flex-1">
            <div class="flex items-center gap-2">
              <h4 class="font-medium text-gray-900">${material.name}</h4>
            </div>
            <div class="flex items-center gap-4 mt-2 text-xs text-gray-600">
              <span>Width: ${material.width}m</span>
              <span>Total Rolls: ${material.rolls}</span>
              <span class="${statusColor} font-medium">${rollsRemaining} rolls left (${remaining.toFixed(1)}m)</span>
            </div>
          </div>
          <div class="flex gap-2 ml-4">
            ${Permissions.canDelete() ? `<button onclick="PrintingPage.addMoreRolls(${material.id})" class="px-3 py-1 text-xs font-medium bg-black text-white rounded-md hover:bg-gray-800 transition-colors">Add Rolls</button>` : ''}
            ${Permissions.canDelete() ? `
            <button onclick="PrintingPage.deleteMaterial(${material.id})"
              class="text-gray-400 hover:text-red-600 transition-colors">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
              </svg>
            </button>` : ''}
          </div>
        </div>
      `;
    }).join('');
  },

  addMoreRolls(id) {
    const material = Store.getPrintingMaterial(id);
    if (!material) return;

    const remaining = material.total_metres - material.metres_used;
    const rollsRemaining = (remaining / material.metres_per_roll).toFixed(1);

    // Create modal HTML
    const modalHTML = `
      <div id="modal-add-material-rolls" class="modal-overlay open">
        <div class="modal-container" style="max-width: 500px;">
          <div class="modal-header">
            <h3 class="modal-title">Add Rolls to Printing Material</h3>
            <button class="modal-close-btn" onclick="PrintingPage.closeAddRollsModal()">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="bg-gray-50 p-4 rounded-lg mb-4">
              <p class="text-sm text-gray-500">Printing Material</p>
              <p class="font-semibold text-gray-900">${material.name}</p>
              <p class="text-sm text-gray-600 mt-1">
                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800">
                  ${material.material_type}
                </span>
                <span class="ml-2">Width: ${material.width}m</span>
              </p>
              <p class="text-sm text-gray-600 mt-1">Current: ${remaining.toFixed(1)}m remaining (${rollsRemaining} rolls)</p>
            </div>
            
            <form id="add-material-rolls-form" class="space-y-4" action="javascript:void(0);">
              <input type="hidden" id="add-material-rolls-id" value="${id}">
              
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Number of Rolls to Add *</label>
                <input type="number" id="add-material-rolls-input" min="1" step="1" class="w-full" placeholder="Enter rolls to add" required autofocus>
                <p class="text-xs text-gray-500 mt-1">Each roll = ${material.metres_per_roll}m</p>
              </div>
              
              <div class="bg-blue-50 border border-blue-200 rounded-lg p-3">
                <p class="text-sm text-gray-700">
                  <span class="font-medium">New Total:</span> 
                  <span id="new-material-total-rolls">${material.rolls}</span> rolls 
                  (<span id="new-material-total-metres">${material.total_metres.toLocaleString()}</span>m)
                </p>
              </div>
            </form>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn-secondary px-4 py-2 rounded-lg" onclick="PrintingPage.closeAddRollsModal()">Cancel</button>
            <button type="submit" form="add-material-rolls-form" class="btn-primary px-4 py-2 rounded-lg">Add Rolls</button>
          </div>
        </div>
      </div>
    `;

    // Add modal to page
    const existingModal = document.getElementById('modal-add-material-rolls');
    if (existingModal) existingModal.remove();
    document.body.insertAdjacentHTML('beforeend', modalHTML);

    // Add form submit handler
    const form = document.getElementById('add-material-rolls-form');
    if (form) {
      form.addEventListener('submit', (e) => {
        e.preventDefault();
        this.submitAddRolls();
      });
    }

    // Add event listener for real-time calculation
    const rollsInput = document.getElementById('add-material-rolls-input');
    if (rollsInput) {
      rollsInput.addEventListener('input', () => {
        const additionalRolls = parseInt(rollsInput.value) || 0;
        const metresPerRoll = material.metres_per_roll;
        const newTotalRolls = material.rolls + additionalRolls;
        const newTotalMetres = material.total_metres + (additionalRolls * metresPerRoll);
        
        document.getElementById('new-material-total-rolls').textContent = newTotalRolls;
        document.getElementById('new-material-total-metres').textContent = newTotalMetres.toLocaleString();
      });
    }
  },

  closeAddRollsModal() {
    const modal = document.getElementById('modal-add-material-rolls');
    if (modal) modal.remove();
  },

  async submitAddRolls() {
    const materialId = parseInt(document.getElementById('add-material-rolls-id').value);
    const rolls = parseInt(document.getElementById('add-material-rolls-input').value);

    if (!rolls || rolls <= 0) {
      Toast.error('Invalid Input', 'Please enter a valid number of rolls');
      return;
    }

    const material = Store.getPrintingMaterial(materialId);
    if (material) {
      const newRolls = material.rolls + rolls;
      const additionalMetres = rolls * material.metres_per_roll;
      const newTotalMetres = material.total_metres + additionalMetres;
      
      await Store.updatePrintingMaterial(materialId, {
        rolls: newRolls,
        total_metres: newTotalMetres
      });
      
      Toast.success('Rolls Added', `Added ${rolls} roll${rolls > 1 ? 's' : ''} to ${material.name}`);
    }

    this.closeAddRollsModal();
  },

  async deleteMaterial(id) {
    const material = Store.getPrintingMaterial(id);
    if (!material) return;
    
    ConfirmModal.show({
      title: 'Delete Material?',
      message: 'Are you sure you want to delete this material? This action cannot be undone.',
      itemName: material.name,
      itemDetails: `${material.width}m ${material.material_type}`,
      onConfirm: async () => {
        await Store.deletePrintingMaterial(id);
        Toast.success('Material Deleted', `${material.name} has been removed`);
      }
    });
  },

  renderPrintingJobs() {
    const tbody = document.getElementById('printing-jobs-table-body');
    if (!tbody) return;

    // Get all transactions that are printing related (have stock_metres_used)
    const allTransactions = Store.serviceTransactions.filter(t => t.stock_metres_used > 0);

    if (allTransactions.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="8" class="px-5 py-8 text-center text-gray-500">No printing jobs recorded.</td>
        </tr>
      `;
      this.updatePaginationControls(0);
      return;
    }

    // Calculate pagination
    const totalPages = Math.ceil(allTransactions.length / this.itemsPerPage);
    const startIndex = (this.currentPage - 1) * this.itemsPerPage;
    const endIndex = startIndex + this.itemsPerPage;
    const paginatedTransactions = allTransactions.slice(startIndex, endIndex);

    tbody.innerHTML = paginatedTransactions.map(t => {
      const transactionDate = new Date(t.timestamp);
      const today = new Date();
      const isToday = transactionDate.toDateString() === today.toDateString();
      
      // Show time if today, show date + time if older
      const timeDisplay = isToday 
        ? transactionDate.toLocaleTimeString('en-US', {hour: '2-digit', minute:'2-digit'})
        : transactionDate.toLocaleDateString('en-US', {month: 'short', day: 'numeric'}) + ' ' + 
          transactionDate.toLocaleTimeString('en-US', {hour: '2-digit', minute:'2-digit'});
      
      const material = t.material_size ? `${t.material_size}m ${t.material_type || ''}` : 'N/A';
      const paymentLabel = t.payment_method === 'mpesa' ? 'M-Pesa' : 
                           t.payment_method === 'till' ? 'Till Number' : 'Cash';
      
      return `
        <tr class="hover:bg-gray-50 transition-colors">
          <td class="px-5 py-4 text-sm text-gray-500">${timeDisplay}</td>
          <td class="px-5 py-4">
            <p class="text-sm font-medium text-gray-900">${t.service_name}</p>
          </td>
          <td class="px-5 py-4 text-sm text-gray-600">${t.stock_metres_used.toFixed(1)}m</td>
          <td class="px-5 py-4 text-sm text-gray-600">${material}</td>
          <td class="px-5 py-4 text-sm font-medium text-gray-900">KSh ${t.amount.toLocaleString(undefined, {minimumFractionDigits: 2})}</td>
          <td class="px-5 py-4">
            <div class="flex flex-col gap-1">
              <span class="status-badge status-badge--success capitalize">${t.payment_method === 'mpesa' ? 'M-Pesa' : t.payment_method === 'till' ? 'Till Number' : 'Cash'}</span>
              ${t.is_debt === 1 ? '<span class="text-[10px] font-bold text-red-600 uppercase tracking-wider">Converted to Debt</span>' : ''}
              ${t.is_debt === 2 ? '<span class="text-[10px] font-bold text-green-600 uppercase tracking-wider">Debt Paid</span>' : ''}
            </div>
          </td>
          <td class="px-5 py-4 text-sm text-gray-600">${t.customer_name}</td>
          <td class="px-5 py-4">
            <div class="flex items-center gap-2">
              <button onclick="Receipt.printPrintingJob(${t.id})" 
                class="text-gray-400 hover:text-green-600 transition-colors" 
                title="Print receipt">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"></path>
                </svg>
              </button>
              <button onclick="PrintingPage.convertToDebt(${t.id})" 
                class="text-gray-400 hover:text-blue-600 transition-colors" 
                title="Convert to debt">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"></path>
                </svg>
              </button>
              ${Permissions.canDelete() ? `
              <button onclick="PrintingPage.deletePrintingJob(${t.id})" 
                class="text-gray-400 hover:text-red-600 transition-colors" 
                title="Delete job">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                </svg>
              </button>` : ''}
            </div>
          </td>
        </tr>
      `;
    }).join('');

    // Update pagination controls
    this.updatePaginationControls(allTransactions.length);
  },

  updatePaginationControls(totalItems) {
    const paginationEl = document.getElementById('printing-pagination');
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
          Showing <span class="font-medium">${startItem}</span> to <span class="font-medium">${endItem}</span> of <span class="font-medium">${totalItems}</span> jobs
        </div>
        <div class="flex gap-2">
          <button onclick="PrintingPage.previousPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === 1 ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === 1 ? 'disabled' : ''}>
            Previous
          </button>
          <span class="px-3 py-1 text-sm font-medium text-gray-700">
            Page ${this.currentPage} of ${totalPages}
          </span>
          <button onclick="PrintingPage.nextPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === totalPages ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === totalPages ? 'disabled' : ''}>
            Next
          </button>
        </div>
      </div>
    `;
  },

  nextPage() {
    const allTransactions = Store.serviceTransactions.filter(t => t.stock_metres_used > 0);
    const totalPages = Math.ceil(allTransactions.length / this.itemsPerPage);
    if (this.currentPage < totalPages) {
      this.currentPage++;
      this.renderPrintingJobs();
    }
  },

  previousPage() {
    if (this.currentPage > 1) {
      this.currentPage--;
      this.renderPrintingJobs();
    }
  },

  async deletePrintingJob(id) {
    const transaction = Store.serviceTransactions.find(t => t.id === id);
    if (!transaction) return;
    
    ConfirmModal.show({
      title: 'Delete Printing Job?',
      message: 'Are you sure you want to delete this printing job? The material will be returned to inventory.',
      itemName: transaction.service_name,
      itemDetails: `${transaction.stock_metres_used.toFixed(1)}m - KSh ${transaction.amount.toLocaleString()}`,
      onConfirm: async () => {
        // Return metres to printing material before deleting
        if (transaction.printing_material_id && transaction.stock_metres_used > 0) {
          const material = Store.getPrintingMaterial(transaction.printing_material_id);
          if (material) {
            const newMetresUsed = Math.max(0, material.metres_used - transaction.stock_metres_used);
            await Store.updatePrintingMaterial(transaction.printing_material_id, {
              metres_used: newMetresUsed
            });
          }
        }
        
        await Store.deleteServiceTransaction(id);
        Toast.success('Job Deleted', `${transaction.service_name} removed and material returned`);
        
        // Update stats and refresh views
        this.updateStats();
        this.renderPrintingJobs();
        this.renderPrintingMaterials();
      }
    });
  },

  updateStats() {
    const todayEl = document.getElementById('stat-today-printing');
    const totalJobsEl = document.getElementById('stat-total-jobs');
    const materialUsedEl = document.getElementById('stat-material-used');
    const totalEl = document.getElementById('stat-total-printing');

    // Get all printing transactions (those with stock usage)
    const allPrintingJobs = Store.serviceTransactions.filter(t => t.stock_metres_used > 0);
    const todayPrintingJobs = Store.getTodayServiceTransactions().filter(t => t.stock_metres_used > 0);

    const todayEarnings = todayPrintingJobs.reduce((sum, t) => sum + t.amount, 0);
    const totalEarnings = allPrintingJobs.reduce((sum, t) => sum + t.amount, 0);
    const totalMaterialUsed = allPrintingJobs.reduce((sum, t) => sum + t.stock_metres_used, 0);

    if (todayEl) todayEl.textContent = `KSh ${todayEarnings.toLocaleString()}`;
    if (totalJobsEl) totalJobsEl.textContent = allPrintingJobs.length;
    if (materialUsedEl) materialUsedEl.textContent = `${totalMaterialUsed.toFixed(1)}m`;
    if (totalEl) totalEl.textContent = `KSh ${totalEarnings.toLocaleString()}`;
  },

  async convertToDebt(transactionId) {
    const transaction = Store.serviceTransactions.find(t => t.id === transactionId);
    if (!transaction) return;

    let existingDebt = null;
    if (transaction.is_debt) {
      existingDebt = await Store.getDebtByTransactionId(transactionId);
    }

    // Create and show modal for converting to debt
    const modalHTML = `
      <div id="modal-convert-printing-debt" class="modal-overlay open">
        <div class="modal-container" style="max-width: 500px;">
          <div class="modal-header">
            <h3 class="modal-title">${existingDebt ? 'Edit Debt Information' : 'Convert Printing Job to Debt'}</h3>
            <button class="modal-close-btn" onclick="PrintingPage.closeConvertDebtModal()">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="bg-gray-50 p-4 rounded-lg mb-4">
              <p class="text-sm text-gray-500">Printing Job Details</p>
              <p class="font-semibold text-gray-900">${transaction.service_name}</p>
              <p class="text-sm text-gray-600">${transaction.stock_metres_used.toFixed(1)}m - KSh ${transaction.amount.toLocaleString()}</p>
            </div>
            
            <form id="convert-printing-debt-form" class="space-y-4">
              <input type="hidden" id="convert-printing-id" value="${transactionId}">
              <input type="hidden" id="convert-printing-amount" value="${transaction.amount}">
              <input type="hidden" id="convert-printing-debt-id" value="${existingDebt ? existingDebt.id : ''}">
              
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Customer Name *</label>
                <input type="text" id="convert-printing-customer-name" value="${existingDebt ? existingDebt.customer_name : transaction.customer_name}" 
                  class="w-full" placeholder="Enter customer name" required>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Customer Phone</label>
                <input type="tel" id="convert-printing-customer-phone" value="${existingDebt ? (existingDebt.phone || '') : ''}"
                  class="w-full" placeholder="Optional">
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Total Job Amount</label>
                <div class="px-3 py-2 bg-gray-100 rounded-lg text-lg font-bold text-gray-900">
                  KSh ${transaction.amount.toLocaleString()}
                </div>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Amount Paid *</label>
                <input type="number" id="convert-printing-amount-paid" min="0" max="${transaction.amount}" 
                  step="0.01" value="${existingDebt ? existingDebt.paid_amount : 0}" class="w-full" placeholder="0.00" required>
                <p class="text-xs text-gray-500 mt-1">How much the customer has already paid</p>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Remaining Debt</label>
                <div class="px-3 py-2 bg-red-50 border border-red-200 rounded-lg text-lg font-bold text-red-600" 
                  id="convert-printing-remaining-debt">
                  KSh ${(existingDebt ? existingDebt.remaining_amount : transaction.amount).toLocaleString()}
                </div>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">Due Date</label>
                <div class="relative">
                  <input type="text" id="convert-printing-due-date-display" readonly class="w-full cursor-pointer"
                    value="${existingDebt ? (existingDebt.due_date || '') : ''}"
                    placeholder="Select due date">
                  <input type="hidden" id="convert-printing-due-date" value="${existingDebt ? (existingDebt.due_date || '') : ''}">
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
            <button type="button" class="btn-secondary px-4 py-2 rounded-lg" 
              onclick="PrintingPage.closeConvertDebtModal()">Cancel</button>
            <button type="button" class="btn-primary px-4 py-2 rounded-lg" 
              onclick="PrintingPage.submitConvertDebt()">${existingDebt ? 'Update Debt' : 'Create Debt'}</button>
          </div>
        </div>
      </div>
    `;

    // Add modal to page
    const existingModal = document.getElementById('modal-convert-printing-debt');
    if (existingModal) existingModal.remove();
    document.body.insertAdjacentHTML('beforeend', modalHTML);

    // Add event listener for amount paid input
    const amountPaidInput = document.getElementById('convert-printing-amount-paid');
    if (amountPaidInput) {
      amountPaidInput.addEventListener('input', () => {
        const totalAmount = parseFloat(document.getElementById('convert-printing-amount').value) || 0;
        const amountPaid = parseFloat(amountPaidInput.value) || 0;
        const remaining = Math.max(0, totalAmount - amountPaid);
        const remainingEl = document.getElementById('convert-printing-remaining-debt');
        if (remainingEl) {
          remainingEl.textContent = `KSh ${remaining.toLocaleString()}`;
        }
      });
    }

    // Add date picker click handler
    const dueDateDisplay = document.getElementById('convert-printing-due-date-display');
    if (dueDateDisplay) {
      dueDateDisplay.addEventListener('click', () => {
        this.openConvertDatePicker();
      });
    }
  },

  openConvertDatePicker() {
    const datePickerModal = document.getElementById('modal-printing-date-picker');
    if (!datePickerModal) return;

    // Initialize picker state
    this.pickerMonth = new Date().getMonth();
    this.pickerYear = new Date().getFullYear();
    this.pickerSelectedDate = null;
    
    datePickerModal.classList.add('open');
    this.renderDatePicker();
    this.bindDatePickerEvents();
  },

  bindDatePickerEvents() {
    const datePickerModal = document.getElementById('modal-printing-date-picker');
    const btnClose = document.getElementById('btn-close-printing-date-picker');
    const btnPrevMonth = document.getElementById('btn-printing-picker-prev-month');
    const btnNextMonth = document.getElementById('btn-printing-picker-next-month');
    const btnClear = document.getElementById('btn-printing-clear-date');
    const btnToday = document.getElementById('btn-printing-today-date');

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
        const hiddenInput = document.getElementById('convert-printing-due-date');
        const displayInput = document.getElementById('convert-printing-due-date-display');
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
    const monthYearEl = document.getElementById('printing-picker-month-year');
    const gridEl = document.getElementById('printing-picker-calendar-grid');
    
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
        const datePickerModal = document.getElementById('modal-printing-date-picker');
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
    
    const hiddenInput = document.getElementById('convert-printing-due-date');
    const displayInput = document.getElementById('convert-printing-due-date-display');
    
    if (hiddenInput) hiddenInput.value = formattedValue;
    if (displayInput) displayInput.value = formattedDisplay;
    
    this.renderDatePicker();
  },

  closeConvertDebtModal() {
    const modal = document.getElementById('modal-convert-printing-debt');
    if (modal) modal.remove();
  },

  async submitConvertDebt() {
    const transactionId = parseInt(document.getElementById('convert-printing-id').value);
    const debtId = document.getElementById('convert-printing-debt-id').value;
    const totalAmount = parseFloat(document.getElementById('convert-printing-amount').value);
    const customerName = document.getElementById('convert-printing-customer-name').value.trim();
    const customerPhone = document.getElementById('convert-printing-customer-phone').value.trim();
    const amountPaid = parseFloat(document.getElementById('convert-printing-amount-paid').value) || 0;
    const dueDate = document.getElementById('convert-printing-due-date').value || null;

    if (!customerName) {
      Toast.error('Missing Information', 'Please enter customer name');
      return;
    }

    const remainingDebt = totalAmount - amountPaid;

    if (remainingDebt <= 0) {
      Toast.error('No Debt', 'The amount paid equals or exceeds the job amount. No debt to create.');
      return;
    }

    const transaction = Store.serviceTransactions.find(t => t.id === transactionId);
    if (!transaction) return;

    // Create or update debt
    const debtData = {
      customer_name: customerName,
      phone: customerPhone || null,
      amount: totalAmount,
      paid_amount: amountPaid,
      remaining_amount: remainingDebt,
      due_date: dueDate,
      description: `Printing Job: ${transaction.service_name}`,
      service_transaction_id: transactionId
    };

    if (debtId) {
      await Store.updateDebt(parseInt(debtId), debtData);
      Toast.success('Debt Updated', `Debt for ${customerName} updated successfully`);
    } else {
      await Store.addDebt(debtData);
      Toast.success('Debt Created', `Debt of KSh ${remainingDebt.toLocaleString()} created for ${customerName}`);
    }
    
    // Mark the transaction as a debt (ensures badge shows)
    await Store.updateServiceTransaction(transactionId, { is_debt: 1 });
    
    this.closeConvertDebtModal();
  },
};

window.PrintingPage = PrintingPage;
