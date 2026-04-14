/**
 * Stock Page Controller
 * Manages sticker inventory with auto-calculated metres
 */

const StockPage = {
  metresPerRoll: 50, // 1 roll = 50 metres
  selectedStickerType: 'colored', // Default sticker type
  currentPage: 1,
  itemsPerPage: 10,
  colorSuggestionsDebounce: null,
  boundDocumentClickHandler: null, // Store reference to document click handler
  storeSubscribed: false, // Track if store subscription added

  init() {
    this.sizeRowIdCounter = 0;
    this.selectedStickerType = 'colored';
    this.bindEvents();
    this.render();
    this.updateSummary();
    
    // Subscribe to store changes only once
    if (!this.storeSubscribed) {
      this.storeSubscribed = true;
      Store.subscribe('stock', () => {
        this.render();
        this.updateSummary();
      });
    }
  },

  bindEvents() {
    // Modal Elements
    const modal = document.getElementById('modal-add-stock');
    const btnAdd = document.getElementById('btn-add-stock');
    const btnClose = document.getElementById('btn-close-stock-modal');
    const btnCancel = document.getElementById('btn-cancel-stock');
    const addForm = document.getElementById('add-stock-form');
    const btnAddSizeRow = document.getElementById('btn-add-size-row');

    // Open Modal
    if (btnAdd && !btnAdd.dataset.bound) {
      btnAdd.dataset.bound = 'true';
      btnAdd.addEventListener('click', () => {
        const modalElement = document.getElementById('modal-add-stock');
        if (modalElement) {
          modalElement.classList.add('open');
          // Reset and add initial row
          this.resetModal();
        }
      });
    }

    // Close Modal Helper
    const closeModal = () => {
        const modalElement = document.getElementById('modal-add-stock');
        if (modalElement) {
            modalElement.classList.remove('open');
            this.resetModal();
        }
    };

    // Close Button Actions
    if (btnClose && !btnClose.dataset.bound) {
      btnClose.dataset.bound = 'true';
      btnClose.addEventListener('click', closeModal);
    }
    if (btnCancel && !btnCancel.dataset.bound) {
      btnCancel.dataset.bound = 'true';
      btnCancel.addEventListener('click', closeModal);
    }

    // Close on Click Outside
    const modalElement = document.getElementById('modal-add-stock');
    if (modalElement && !modalElement.dataset.bound) {
      modalElement.dataset.bound = 'true';
      modalElement.addEventListener('click', (e) => {
        if (e.target === modalElement) {
            closeModal();
        }
      });
    }

    // Add Size Row Button
    if (btnAddSizeRow && !btnAddSizeRow.dataset.bound) {
      btnAddSizeRow.dataset.bound = 'true';
      btnAddSizeRow.addEventListener('click', () => {
        this.addSizeRow();
      });
    }

    // Sticker Type Selector Buttons
    const typeButtons = document.querySelectorAll('.sticker-type-btn');
    typeButtons.forEach(btn => {
      if (!btn.dataset.bound) {
        btn.dataset.bound = 'true';
        btn.addEventListener('click', () => {
          this.selectStickerType(btn.dataset.type);
        });
      }
    });

    // Handle Form Submit
    if (addForm && !addForm.dataset.bound) {
      addForm.dataset.bound = 'true';
      addForm.addEventListener('submit', (e) => {
        e.preventDefault();
        this.handleSubmit();
        closeModal();
      });
    }

    // Color Input Autocomplete
    const colorInput = document.getElementById('stock-color-input');
    if (colorInput && !colorInput.dataset.bound) {
      colorInput.dataset.bound = 'true';
      // Handle input for suggestions
      colorInput.addEventListener('input', (e) => {
        this.handleColorInput(e.target.value);
      });

      // Handle focus to show suggestions
      colorInput.addEventListener('focus', () => {
        this.showColorSuggestions();
      });

      // Handle keyboard navigation
      colorInput.addEventListener('keydown', (e) => {
        this.handleColorKeydown(e);
      });
    }

    // Close suggestions when clicking outside
    // Remove old handler first to prevent duplicates
    if (this.boundDocumentClickHandler) {
      document.removeEventListener('click', this.boundDocumentClickHandler);
    }
    this.boundDocumentClickHandler = (e) => {
      const suggestions = document.getElementById('color-suggestions');
      const colorInputEl = document.getElementById('stock-color-input');
      if (suggestions && !suggestions.contains(e.target) && e.target !== colorInputEl) {
        this.hideColorSuggestions();
      }
    };
    document.addEventListener('click', this.boundDocumentClickHandler);
  },

  // ============================================================
  // COLOR AUTOCOMPLETE METHODS
  // ============================================================

  handleColorInput(value) {
    // Update color preview
    this.updateColorPreview(value);

    // Debounce suggestions
    clearTimeout(this.colorSuggestionsDebounce);
    this.colorSuggestionsDebounce = setTimeout(() => {
      this.showColorSuggestions(value);
    }, 150);
  },

  showColorSuggestions(filter = '') {
    const suggestionsEl = document.getElementById('color-suggestions');
    if (!suggestionsEl) return;

    const suggestions = window.VinylColorUtils 
      ? VinylColorUtils.getColorSuggestions(filter, this.selectedStickerType)
      : this.getBasicColorSuggestions(filter);

    if (suggestions.length === 0) {
      this.hideColorSuggestions();
      return;
    }

    suggestionsEl.innerHTML = suggestions.map(s => `
      <div class="color-suggestion-item px-3 py-2 hover:bg-gray-100 cursor-pointer flex items-center gap-3 transition-colors" data-color="${s.name}">
        <div class="w-6 h-6 rounded border border-gray-200 shadow-sm flex-shrink-0" 
             style="background: ${s.hex};"></div>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-medium text-gray-900">${s.name}</div>
          ${s.category ? `<div class="text-xs text-gray-500">${s.category}</div>` : ''}
        </div>
        ${s.oracalCode ? `<div class="text-xs text-gray-400">ORACAL ${s.oracalCode}</div>` : ''}
      </div>
    `).join('');

    suggestionsEl.classList.remove('hidden');

    // Add click handlers to suggestions
    suggestionsEl.querySelectorAll('.color-suggestion-item').forEach(item => {
      item.addEventListener('click', () => {
        this.selectColorSuggestion(item.dataset.color);
      });
    });
  },

  hideColorSuggestions() {
    const suggestionsEl = document.getElementById('color-suggestions');
    if (suggestionsEl) {
      suggestionsEl.classList.add('hidden');
    }
  },

  selectColorSuggestion(colorName) {
    const colorInput = document.getElementById('stock-color-input');
    if (colorInput) {
      colorInput.value = colorName;
      this.updateColorPreview(colorName);
    }
    this.hideColorSuggestions();
  },

  handleColorKeydown(e) {
    const suggestionsEl = document.getElementById('color-suggestions');
    if (!suggestionsEl || suggestionsEl.classList.contains('hidden')) return;

    const items = suggestionsEl.querySelectorAll('.color-suggestion-item');
    const activeItem = suggestionsEl.querySelector('.color-suggestion-item.active');
    let activeIndex = activeItem ? Array.from(items).indexOf(activeItem) : -1;

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        activeIndex = Math.min(activeIndex + 1, items.length - 1);
        this.highlightSuggestion(items, activeIndex);
        break;
      case 'ArrowUp':
        e.preventDefault();
        activeIndex = Math.max(activeIndex - 1, 0);
        this.highlightSuggestion(items, activeIndex);
        break;
      case 'Enter':
        e.preventDefault();
        if (activeItem) {
          this.selectColorSuggestion(activeItem.dataset.color);
        }
        break;
      case 'Escape':
        this.hideColorSuggestions();
        break;
    }
  },

  highlightSuggestion(items, index) {
    items.forEach((item, i) => {
      item.classList.toggle('active', i === index);
      item.classList.toggle('bg-blue-50', i === index);
    });

    // Scroll into view
    if (items[index]) {
      items[index].scrollIntoView({ block: 'nearest' });
    }
  },

  updateColorPreview(colorName) {
    const previewEl = document.getElementById('color-preview');
    if (!previewEl) return;

    const hex = window.VinylColorUtils 
      ? VinylColorUtils.parseColor(colorName).hex
      : this.getColorHex(colorName);
    
    previewEl.style.backgroundColor = hex;
  },

  getBasicColorSuggestions(filter) {
    // Fallback if VinylColorUtils is not available
    const colors = [
      { name: 'Red', hex: '#ef4444', category: 'basic' },
      { name: 'Blue', hex: '#3b82f6', category: 'basic' },
      { name: 'Green', hex: '#22c55e', category: 'basic' },
      { name: 'Yellow', hex: '#eab308', category: 'basic' },
      { name: 'Orange', hex: '#f97316', category: 'basic' },
      { name: 'Purple', hex: '#a855f7', category: 'basic' },
      { name: 'Pink', hex: '#ec4899', category: 'basic' },
      { name: 'Black', hex: '#1f2937', category: 'basic' },
      { name: 'White', hex: '#ffffff', category: 'basic' },
      { name: 'Dark Red', hex: '#991b1b', category: 'variant' },
      { name: 'Dark Blue', hex: '#1e3a8a', category: 'variant' },
      { name: 'Dark Green', hex: '#166534', category: 'variant' },
      { name: 'Light Blue', hex: '#93c5fd', category: 'variant' },
      { name: 'Light Green', hex: '#86efac', category: 'variant' }
    ];

    const filterLower = filter.toLowerCase();
    return colors.filter(c => !filter || c.name.toLowerCase().includes(filterLower));
  },

  resetModal() {
    // Clear size rows container
    const container = document.getElementById('size-rows-container');
    if (container) {
      container.innerHTML = '';
    }
    this.sizeRowIdCounter = 0;

    // Clear color input
    const colorInput = document.getElementById('stock-color-input');
    if (colorInput) {
      colorInput.value = '';
    }
    
    // Reset color preview
    const colorPreview = document.getElementById('color-preview');
    if (colorPreview) {
      colorPreview.style.backgroundColor = '#9ca3af';
    }
    
    // Hide color suggestions
    this.hideColorSuggestions();
    
    // Reset sticker type to default and update UI only (don't rebuild rows)
    this.selectedStickerType = 'colored';
    this.updateStickerTypeUIOnly();
    
    // Now add exactly one initial row
    this.addSizeRow();
    this.updateTotalSummary();
  },

  selectStickerType(type) {
    const previousType = this.selectedStickerType;
    this.selectedStickerType = type;
    const hiddenInput = document.getElementById('sticker-type-input');
    if (hiddenInput) hiddenInput.value = type;
    
    // Update UI elements (buttons, labels) without rebuilding rows
    this.updateStickerTypeUIOnly();
    
    // Check if there's user-entered data (excluding default values)
    const container = document.getElementById('size-rows-container');
    if (container && container.children.length > 0) {
      // Only consider data as "entered" if size or rolls inputs have values
      // Don't count metres-per-roll default value of 50
      const hasUserData = Array.from(container.querySelectorAll('.size-input, .rolls-input'))
        .some(input => input.value && input.value.trim() !== '');
      
      if (hasUserData) {
        // User has entered data, rebuild to preserve it
        this.rebuildSizeRows();
      } else {
        // Empty initial row or only default values, clear and add fresh one with new type
        container.innerHTML = '';
        this.sizeRowIdCounter = 0;
        this.addSizeRow();
        this.updateTotalSummary();
      }
    }
  },

  updateStickerTypeUI() {
    this.updateStickerTypeUIOnly();
    
    // Only rebuild size rows if there are existing rows (user switched types mid-entry)
    const container = document.getElementById('size-rows-container');
    if (container && container.children.length > 0) {
      this.rebuildSizeRows();
    }
  },

  updateStickerTypeUIOnly() {
    const typeButtons = document.querySelectorAll('.sticker-type-btn');
    const colorLabel = document.getElementById('color-label');
    const colorInput = document.getElementById('stock-color-input');
    const colorHint = document.getElementById('color-hint');
    
    const typeConfig = STICKER_TYPES[this.selectedStickerType];
    
    // Update button states - modern minimal style
    typeButtons.forEach(btn => {
      const btnType = btn.dataset.type;
      
      if (btnType === this.selectedStickerType) {
        // Active state - blue accent
        btn.className = 'sticker-type-btn flex-1 px-4 py-2 border border-brand-500 bg-brand-50 text-brand-600 font-medium text-sm transition-all';
      } else {
        // Inactive state
        btn.className = 'sticker-type-btn flex-1 px-4 py-2 border border-gray-200 bg-white text-gray-500 font-medium text-sm transition-all hover:border-gray-300';
      }
    });

    // Update color label and hint based on type
    if (this.selectedStickerType === 'colored') {
      if (colorLabel) colorLabel.textContent = 'Color';
      if (colorInput) colorInput.placeholder = 'e.g. Red Dark, Black Matte';
      if (colorHint) colorHint.textContent = 'Enter color with variant (dark, light, matte, gloss)';
    } else if (this.selectedStickerType === 'reflective') {
      if (colorLabel) colorLabel.textContent = 'Color';
      if (colorInput) colorInput.placeholder = 'e.g. Red, White, Yellow';
      if (colorHint) colorHint.textContent = 'Enter reflective color';
    }
  },

  rebuildSizeRows() {
    const container = document.getElementById('size-rows-container');
    if (!container) return;

    // Save current values
    const rows = Array.from(container.querySelectorAll('[data-row-id]'));
    const rowData = rows.map(row => {
      return {
        size: row.querySelector('.size-input')?.value || '',
        rolls: row.querySelector('.rolls-input')?.value || '',
        metresPerRoll: row.querySelector('.metres-per-roll-input')?.value || ''
      };
    });

    // Clear and rebuild rows
    container.innerHTML = '';
    this.sizeRowIdCounter = 0;
    
    if (rowData.length === 0) {
      // Add initial empty row if none existed
      this.addSizeRow();
    } else {
      // Rebuild rows with saved data
      rowData.forEach((data, index) => {
        this.addSizeRow();
        const row = container.children[index];
        if (row) {
          const sizeInput = row.querySelector('.size-input');
          const rollsInput = row.querySelector('.rolls-input');
          const metresPerRollInput = row.querySelector('.metres-per-roll-input');
          
          if (sizeInput && data.size) sizeInput.value = data.size;
          if (rollsInput && data.rolls) rollsInput.value = data.rolls;
          if (metresPerRollInput && data.metresPerRoll) metresPerRollInput.value = data.metresPerRoll;
          
          // Trigger calculation
          if (rollsInput && rollsInput.value) {
            this.updateRowMetres(parseInt(row.dataset.rowId));
          }
        }
      });
    }
    
    this.updateTotalSummary();
  },

  addSizeRow() {
    const container = document.getElementById('size-rows-container');
    if (!container) return;

    const rowId = this.sizeRowIdCounter++;
    const isFirstRow = container.children.length === 0;
    const isReflective = this.selectedStickerType === 'reflective';

    const row = document.createElement('div');
    row.className = 'grid grid-cols-2 gap-3';
    row.dataset.rowId = rowId;
    row.innerHTML = `
      <div>
        <label>Width (inches)${isFirstRow ? ' *' : ''}</label>
        <input type="number" 
               class="w-full size-input" 
               data-row-id="${rowId}"
               step="1" 
               min="1" 
               placeholder="e.g. 24" 
               ${isFirstRow ? 'required' : ''}>
      </div>
      <div>
        <label>Rolls${isFirstRow ? ' *' : ''}</label>
        <input type="number" 
               class="w-full rolls-input" 
               data-row-id="${rowId}"
               min="1" 
               placeholder="e.g. 5" 
               ${isFirstRow ? 'required' : ''}>
      </div>
      ${isReflective ? `
        <div>
          <label>Metres per Roll${isFirstRow ? ' *' : ''}</label>
          <input type="number" 
                 class="w-full metres-per-roll-input" 
                 data-row-id="${rowId}"
                 step="0.1" 
                 min="1" 
                 value="50"
                 ${isFirstRow ? 'required' : ''}>
        </div>
        <div>
          <label>Total Metres</label>
          <div class="px-3 py-2 bg-gray-50 border border-gray-200 text-gray-600 text-sm">
            <span class="metres-display font-medium" data-row-id="${rowId}">0</span>m
          </div>
        </div>
      ` : `
        <div>
          <label>Total Metres</label>
          <div class="px-3 py-2 bg-gray-50 border border-gray-200 text-gray-600 text-sm">
            <span class="metres-display font-medium" data-row-id="${rowId}">0</span>m
          </div>
        </div>
      `}
      ${!isFirstRow ? `
        <div class="col-span-2 flex justify-end">
          <button type="button" 
                  class="remove-row-btn text-xs text-gray-400 hover:text-red-500 font-medium" 
                  data-row-id="${rowId}">
            Remove
          </button>
        </div>
      ` : ''}
    `;

    container.appendChild(row);

    // Bind events for this row
    const sizeInput = row.querySelector('.size-input');
    const rollsInput = row.querySelector('.rolls-input');
    const metresPerRollInput = row.querySelector('.metres-per-roll-input');
    const removeBtn = row.querySelector('.remove-row-btn');

    if (rollsInput) {
      rollsInput.addEventListener('input', () => this.updateRowMetres(rowId));
    }
    if (metresPerRollInput) {
      metresPerRollInput.addEventListener('input', () => this.updateRowMetres(rowId));
    }
    if (removeBtn) {
      removeBtn.addEventListener('click', () => this.removeSizeRow(rowId));
    }
  },

  removeSizeRow(rowId) {
    const row = document.querySelector(`[data-row-id="${rowId}"]`);
    if (row) {
      row.remove();
      this.updateTotalSummary();
    }
  },

  updateRowMetres(rowId) {
    const rollsInput = document.querySelector(`.rolls-input[data-row-id="${rowId}"]`);
    const metresPerRollInput = document.querySelector(`.metres-per-roll-input[data-row-id="${rowId}"]`);
    const metresDisplay = document.querySelector(`.metres-display[data-row-id="${rowId}"]`);

    if (rollsInput && metresDisplay) {
      const rolls = parseInt(rollsInput.value) || 0;
      let totalMetres = 0;
      
      if (this.selectedStickerType === 'reflective' && metresPerRollInput) {
        // For reflective: use custom metres per roll
        const metresPerRoll = parseFloat(metresPerRollInput.value) || 0;
        totalMetres = rolls * metresPerRoll;
      } else {
        // For colored/clear: use fixed 50m per roll
        totalMetres = rolls * this.metresPerRoll;
      }
      
      metresDisplay.textContent = totalMetres.toLocaleString();
    }

    this.updateTotalSummary();
  },

  updateTotalSummary() {
    const rollsInputs = document.querySelectorAll('.rolls-input');
    const metresPerRollInputs = document.querySelectorAll('.metres-per-roll-input');
    
    let totalRolls = 0;
    let totalMetres = 0;
    const isReflective = this.selectedStickerType === 'reflective';

    rollsInputs.forEach((input, index) => {
      const rolls = parseInt(input.value) || 0;
      totalRolls += rolls;
      
      if (isReflective) {
        // For reflective stickers, use custom metres per roll input
        const metresPerRollInput = metresPerRollInputs[index];
        const metresPerRoll = parseFloat(metresPerRollInput?.value) || 0;
        totalMetres += rolls * metresPerRoll;
      } else {
        // For other types, calculate: rolls × 50
        totalMetres += rolls * this.metresPerRoll;
      }
    });

    const totalRollsEl = document.getElementById('total-rolls');
    const totalMetresEl = document.getElementById('total-metres');

    if (totalRollsEl) totalRollsEl.textContent = totalRolls.toLocaleString();
    if (totalMetresEl) totalMetresEl.textContent = totalMetres.toLocaleString();
  },

  handleSubmit() {
    const colorInput = document.getElementById('stock-color-input');
    const color = colorInput?.value.trim();

    if (!color) {
      Toast.error('Missing Color', 'Please enter a color');
      return;
    }

    // Get all size rows
    const sizeInputs = document.querySelectorAll('.size-input');
    const rollsInputs = document.querySelectorAll('.rolls-input');
    const metresPerRollInputs = document.querySelectorAll('.metres-per-roll-input');

    if (sizeInputs.length === 0) {
      Toast.error('No Size Variant', 'Please add at least one size variant');
      return;
    }

    // Get selected sticker type
    const stickerType = this.selectedStickerType || 'colored';
    const isReflective = stickerType === 'reflective';

    let addedCount = 0;
    let updatedCount = 0;

    // Process each size row
    sizeInputs.forEach((sizeInput, index) => {
      const size = sizeInput.value.trim();
      const rolls = parseInt(rollsInputs[index]?.value);
      const customMetresPerRoll = isReflective ? parseFloat(metresPerRollInputs[index]?.value) : null;

      if (!size || !rolls || rolls < 1) {
        return; // Skip invalid rows
      }

      if (isReflective && (!customMetresPerRoll || customMetresPerRoll <= 0)) {
        Toast.error('Missing Metres per Roll', 'Please enter metres per roll for reflective stickers');
        return;
      }

      // Check if this color, size, and sticker type combination already exists
      const existing = Store.getStockByColorSizeAndType(color, size, stickerType);
      if (existing) {
        // Add to existing stock
        if (isReflective) {
          Store.addRollsToStockWithCustomMetres(existing.id, rolls, customMetresPerRoll);
        } else {
          Store.addRollsToStock(existing.id, rolls);
        }
        updatedCount++;
      } else {
        // Create new stock entry
        const stockData = {
          color: color,
          size: size,
          sticker_type: stickerType,
          rolls: rolls
        };
        
        // For reflective, add custom metres per roll
        if (isReflective) {
          stockData.custom_metres_per_roll = customMetresPerRoll;
        }
        
        Store.addStock(stockData);
        addedCount++;
      }
    });

    // Show success message
    const messages = [];
    if (addedCount > 0) messages.push(`Added ${addedCount} new size variant${addedCount > 1 ? 's' : ''}`);
    if (updatedCount > 0) messages.push(`Updated ${updatedCount} existing variant${updatedCount > 1 ? 's' : ''}`);
    
    if (messages.length > 0) {
      Toast.success('Stock Added', `${messages.join(' and ')} for ${color}`);
    }
  },

  updateSummary() {
    const summary = Store.getStockSummary();
    
    const colorsEl = document.getElementById('summary-colors');
    const rollsEl = document.getElementById('summary-rolls');
    const metresEl = document.getElementById('summary-metres');
    const remainingEl = document.getElementById('summary-remaining');

    if (colorsEl) colorsEl.textContent = summary.totalItems;
    if (rollsEl) rollsEl.textContent = summary.totalRolls;
    if (metresEl) metresEl.textContent = `${summary.totalMetres.toLocaleString()}m`;
    if (remainingEl) remainingEl.textContent = `${summary.metresRemaining.toLocaleString()}m`;
  },

  getStockStatus(remaining, total) {
    const percentage = (remaining / total) * 100;
    if (percentage === 0) {
      return { label: 'Out of Stock', class: 'status-badge--error' };
    } else if (percentage <= 20) {
      return { label: 'Low Stock', class: 'status-badge--warning' };
    } else {
      return { label: 'In Stock', class: 'status-badge--success' };
    }
  },

  render() {
    const tbody = document.getElementById('stock-table-body');
    if (!tbody) return;

    const stock = Store.stock;

    if (stock.length === 0) {
      tbody.innerHTML = `
        <tr class="text-center">
            <td colspan="10" class="px-5 py-8 text-gray-500">
                <div class="flex flex-col items-center justify-center">
                    <svg class="w-12 h-12 text-gray-300 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path>
                    </svg>
                    <p>No stock added yet</p>
                    <button onclick="document.getElementById('btn-add-stock').click()" class="text-black font-semibold hover:underline text-sm mt-2">Add your first stock</button>
                </div>
            </td>
        </tr>
      `;
      this.updatePaginationControls(0);
      return;
    }

    // Calculate pagination
    const totalPages = Math.ceil(stock.length / this.itemsPerPage);
    const startIndex = (this.currentPage - 1) * this.itemsPerPage;
    const endIndex = startIndex + this.itemsPerPage;
    const paginatedStock = stock.slice(startIndex, endIndex);

    tbody.innerHTML = paginatedStock.map(item => {
      const remaining = Store.getRemainingMetres(item.id);
      const rollsLeft = Store.getRemainingRolls(item.id);
      const status = this.getStockStatus(remaining, item.total_metres);
      const stickerTypeConfig = STICKER_TYPES[item.sticker_type] || STICKER_TYPES.colored;

      return `
        <tr class="hover:bg-gray-50 transition-colors">
          <td class="px-6 py-4">
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${stickerTypeConfig.badgeClass}">
              ${stickerTypeConfig.name}
            </span>
          </td>
          <td class="px-6 py-4">
            <div class="flex items-center gap-3">
              ${item.sticker_type === 'colored' ? `<div class="w-8 h-8 rounded-lg border border-gray-200 shadow-sm" style="background-color: ${this.getColorHex(item.color)};"></div>` : 
                item.sticker_type === 'clear' ? `<div class="w-8 h-8 rounded-lg border-2 border-dashed border-gray-300 bg-gradient-to-br from-white to-gray-100"></div>` :
                `<div class="w-8 h-8 rounded-lg border border-gray-200 shadow-sm bg-gradient-to-br from-gray-200 via-white to-gray-300"></div>`
              }
              <span class="text-sm font-medium text-gray-900">${item.color}</span>
            </div>
          </td>
          <td class="px-6 py-4 text-sm text-gray-600">${item.size || '1'}in</td>
          <td class="px-6 py-4 text-sm text-gray-600">${item.rolls}</td>
          <td class="px-6 py-4 text-sm text-gray-600">${item.total_metres.toLocaleString()}m</td>
          <td class="px-6 py-4 text-sm text-gray-600">${item.metres_used.toLocaleString()}m</td>
          <td class="px-6 py-4 text-sm font-medium text-gray-900">${remaining.toLocaleString()}m</td>
          <td class="px-6 py-4 text-sm text-gray-900">${rollsLeft}</td>
          <td class="px-6 py-4">
            <span class="status-badge ${status.class}">${status.label}</span>
          </td>
          <td class="px-6 py-4">
            <div class="flex gap-2">
              <button onclick="StockPage.addMoreRolls(${item.id})" class="px-3 py-1 text-xs font-medium bg-black text-white rounded-md hover:bg-gray-800 transition-colors">Add Rolls</button>
              <button onclick="StockPage.delete(${item.id})" class="text-gray-400 hover:text-red-600 transition-colors">
                 <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                </svg>
              </button>
            </div>
          </td>
        </tr>
      `;
    }).join('');

    // Update pagination controls
    this.updatePaginationControls(stock.length);
  },

  updatePaginationControls(totalItems) {
    const paginationEl = document.getElementById('stock-pagination');
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
          Showing <span class="font-medium">${startItem}</span> to <span class="font-medium">${endItem}</span> of <span class="font-medium">${totalItems}</span> stock items
        </div>
        <div class="flex gap-2">
          <button onclick="StockPage.previousPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === 1 ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === 1 ? 'disabled' : ''}>
            Previous
          </button>
          <span class="px-3 py-1 text-sm font-medium text-gray-700">
            Page ${this.currentPage} of ${totalPages}
          </span>
          <button onclick="StockPage.nextPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === totalPages ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === totalPages ? 'disabled' : ''}>
            Next
          </button>
        </div>
      </div>
    `;
  },

  nextPage() {
    const totalPages = Math.ceil(Store.stock.length / this.itemsPerPage);
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

  // Get CSS color from vinyl color knowledge system
  // Uses comprehensive industry-standard color mapping for sticker/vinyl printing
  getColorHex(colorName) {
    // Use the vinyl color utilities if available
    if (window.VinylColorUtils) {
      const parsed = VinylColorUtils.parseColor(colorName);
      return parsed.hex || '#9ca3af';
    }
    
    // Fallback to basic colors if vinyl-colors.js not loaded
    const colors = {
      // Basic colors
      red: '#ef4444',
      blue: '#3b82f6',
      green: '#22c55e',
      yellow: '#eab308',
      orange: '#f97316',
      purple: '#a855f7',
      pink: '#ec4899',
      black: '#1f2937',
      white: '#ffffff',
      gold: '#fbbf24',
      silver: '#9ca3af',
      brown: '#92400e',
      grey: '#6b7280',
      gray: '#6b7280'
    };
    
    const lowerColor = colorName.toLowerCase().trim();
    return colors[lowerColor] || '#9ca3af'; // Default to gray if not found
  },

  addMoreRolls(id) {
    const stockItem = Store.getStock(id);
    if (!stockItem) return;

    const typeConfig = STICKER_TYPES[stockItem.sticker_type] || STICKER_TYPES.colored;
    const remaining = Store.getRemainingMetres(id);
    const rollsLeft = Store.getRemainingRolls(id);

    // Create modal HTML
    const modalHTML = `
      <div id="modal-add-rolls" class="modal-overlay open">
        <div class="modal-container" style="max-width: 500px;">
          <div class="modal-header">
            <h3 class="modal-title">Add Rolls to Stock</h3>
            <button class="modal-close-btn" onclick="StockPage.closeAddRollsModal()">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="bg-gray-50 p-4 mb-4">
              <p class="text-xs text-gray-500 uppercase tracking-wide">Stock Item</p>
              <p class="font-semibold text-gray-900">${stockItem.color} - ${stockItem.size || '1'}" ${typeConfig.name}</p>
              <p class="text-sm text-gray-600 mt-1">Current: ${remaining.toLocaleString()}m remaining (${rollsLeft} rolls)</p>
            </div>

            <form id="add-rolls-form" class="space-y-4" action="javascript:void(0);">
              <input type="hidden" id="add-rolls-stock-id" value="${id}">

              <div>
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">Number of Rolls to Add *</label>
                <input type="number" id="add-rolls-input" min="1" step="1" class="w-full" placeholder="Enter rolls to add" required autofocus>
                <p class="text-xs text-gray-500 mt-1">Each roll = ${stockItem.metres_per_roll || 50}m</p>
              </div>

              <div class="bg-blue-50 border border-blue-200 p-3">
                <p class="text-sm text-gray-700">
                  <span class="font-medium">New Total:</span>
                  <span id="new-total-rolls">${stockItem.rolls}</span> rolls
                  (<span id="new-total-metres">${stockItem.total_metres.toLocaleString()}</span>m)
                </p>
              </div>
            </form>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn-secondary px-4 py-2" onclick="StockPage.closeAddRollsModal()">Cancel</button>
            <button type="submit" form="add-rolls-form" class="btn-primary px-4 py-2">Add Rolls</button>
          </div>
        </div>
      </div>
    `;

    // Add modal to page
    const existingModal = document.getElementById('modal-add-rolls');
    if (existingModal) existingModal.remove();
    document.body.insertAdjacentHTML('beforeend', modalHTML);

    // Add form submit handler
    const form = document.getElementById('add-rolls-form');
    if (form) {
      form.addEventListener('submit', (e) => {
        e.preventDefault();
        this.submitAddRolls();
      });
    }

    // Add event listener for real-time calculation
    const rollsInput = document.getElementById('add-rolls-input');
    if (rollsInput) {
      rollsInput.addEventListener('input', () => {
        const additionalRolls = parseInt(rollsInput.value) || 0;
        const metresPerRoll = stockItem.metres_per_roll || 50;
        const newTotalRolls = stockItem.rolls + additionalRolls;
        const newTotalMetres = stockItem.total_metres + (additionalRolls * metresPerRoll);
        
        document.getElementById('new-total-rolls').textContent = newTotalRolls;
        document.getElementById('new-total-metres').textContent = newTotalMetres.toLocaleString();
      });
    }
  },

  closeAddRollsModal() {
    const modal = document.getElementById('modal-add-rolls');
    if (modal) modal.remove();
  },

  submitAddRolls() {
    const stockId = parseInt(document.getElementById('add-rolls-stock-id').value);
    const rolls = parseInt(document.getElementById('add-rolls-input').value);

    if (!rolls || rolls <= 0) {
      Toast.error('Invalid Input', 'Please enter a valid number of rolls');
      return;
    }

    const stockItem = Store.getStock(stockId);
    if (stockItem) {
      Store.addRollsToStock(stockId, rolls);
      Toast.success('Rolls Added', `Added ${rolls} roll${rolls > 1 ? 's' : ''} to ${stockItem.color}`);
    }

    this.closeAddRollsModal();
  },

  delete(id) {
    const stockItem = Store.stock.find(s => s.id === id);
    if (!stockItem) return;

    const typeConfig = STICKER_TYPES[stockItem.sticker_type] || STICKER_TYPES.colored;
    const remaining = Store.getRemainingMetres(id);
    
    ConfirmModal.show({
      title: 'Delete Stock Entry?',
      message: 'Are you sure you want to delete this stock entry? This action cannot be undone.',
      itemName: `${stockItem.color} - Size ${stockItem.size || '1'}`,
      itemDetails: `${typeConfig.name} Sticker • ${remaining.toLocaleString()}m remaining`,
      onConfirm: () => {
        Store.deleteStock(id);
        Toast.success('Stock Deleted', `${stockItem.color} sticker stock has been removed`);
      }
    });
  }
};

window.StockPage = StockPage;
