/**
 * Debts Page Controller
 */

const DebtsPage = {
  currentMonth: new Date().getMonth(),
  currentYear: new Date().getFullYear(),
  selectedDate: null,
  pickerMonth: new Date().getMonth(),
  pickerYear: new Date().getFullYear(),
  pickerSelectedDate: null,
  paymentMethodDropdown: null,
  currentDebtId: null,
  currentPage: 1,
  itemsPerPage: 10,

  init() {
    this.initPaymentDropdown();
    this.bindEvents();
    this.render();
    this.updateSummary();
    
    // Subscribe to store changes
    Store.subscribe('debts', () => {
      this.render();
      this.updateSummary();
      // Refresh calendar if modal is open
      const calendarModal = document.getElementById('modal-calendar');
      if (calendarModal && calendarModal.classList.contains('open')) {
        this.renderCalendar();
      }
    });
  },

  initPaymentDropdown() {
    const paymentContainer = document.getElementById('payment-method-dropdown');
    if (paymentContainer) {
      this.paymentMethodDropdown = new CustomDropdown(paymentContainer, {
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
      this.paymentMethodDropdown.selectItem(paymentContainer.querySelector('.dropdown-item'));
    }
  },

  bindEvents() {
    const modal = document.getElementById('modal-add-debt');
    const calendarModal = document.getElementById('modal-calendar');
    const datePickerModal = document.getElementById('modal-date-picker');
    const paymentModal = document.getElementById('modal-record-payment');
    const historyModal = document.getElementById('modal-payment-history');

    const btnAdd = document.getElementById('btn-add-debt');
    const btnClose = document.getElementById('btn-close-debt-modal');
    const btnCancel = document.getElementById('btn-cancel-debt');
    const addForm = document.getElementById('add-debt-form');

    // Payment modal elements
    const btnClosePayment = document.getElementById('btn-close-payment-modal');
    const btnCancelPayment = document.getElementById('btn-cancel-payment');
    const paymentForm = document.getElementById('record-payment-form');

    // History modal elements
    const btnCloseHistory = document.getElementById('btn-close-history-modal');

    // Calendar button
    const btnCalendar = document.getElementById('btn-calendar-view');
    const btnCloseCalendar = document.getElementById('btn-close-calendar-modal');
    const btnPrevMonth = document.getElementById('btn-prev-month');
    const btnNextMonth = document.getElementById('btn-next-month');

    // Date picker elements
    const dueDateDisplay = document.getElementById('due-date-display');
    const btnCloseDatePicker = document.getElementById('btn-close-date-picker');
    const btnPickerPrevMonth = document.getElementById('btn-picker-prev-month');
    const btnPickerNextMonth = document.getElementById('btn-picker-next-month');
    const btnClearDate = document.getElementById('btn-clear-date');
    const btnTodayDate = document.getElementById('btn-today-date');

    // Open Add Debt Modal
    if (btnAdd && modal) {
      btnAdd.addEventListener('click', () => {
        modal.classList.add('open');
      });
    }

    // Open Calendar Modal
    if (btnCalendar && calendarModal) {
      btnCalendar.addEventListener('click', () => {
        this.currentMonth = new Date().getMonth();
        this.currentYear = new Date().getFullYear();
        this.selectedDate = null;
        calendarModal.classList.add('open');
        this.renderCalendar();
      });
    }

    // Close Modal Helper
    const closeModal = () => {
        if (modal) {
            modal.classList.remove('open');
            addForm?.reset();
        }
    };

    const closeCalendarModal = () => {
        if (calendarModal) {
            calendarModal.classList.remove('open');
            this.selectedDate = null;
            const detailsSection = document.getElementById('selected-day-details');
            if (detailsSection) detailsSection.classList.add('hidden');
        }
    };

    // Close Button Actions
    if (btnClose) btnClose.addEventListener('click', closeModal);
    if (btnCancel) btnCancel.addEventListener('click', closeModal);
    if (btnCloseCalendar) btnCloseCalendar.addEventListener('click', closeCalendarModal);

    // Payment modal close handlers
    const closePaymentModal = () => {
      if (paymentModal) {
        paymentModal.classList.remove('open');
        paymentForm?.reset();
        this.paymentMethodDropdown?.reset();
        const container = document.getElementById('payment-method-dropdown');
        if (container) this.paymentMethodDropdown?.selectItem(container.querySelector('.dropdown-item'));
      }
    };

    const closeHistoryModal = () => {
      if (historyModal) {
        historyModal.classList.remove('open');
      }
    };

    if (btnClosePayment) btnClosePayment.addEventListener('click', closePaymentModal);
    if (btnCancelPayment) btnCancelPayment.addEventListener('click', closePaymentModal);
    if (btnCloseHistory) btnCloseHistory.addEventListener('click', closeHistoryModal);

    if (paymentModal) {
      paymentModal.addEventListener('click', (e) => {
        if (e.target === paymentModal) closePaymentModal();
      });
    }

    if (historyModal) {
      historyModal.addEventListener('click', (e) => {
        if (e.target === historyModal) closeHistoryModal();
      });
    }

    // Payment form submit
    if (paymentForm) {
      paymentForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        await this.handlePaymentSubmit(new FormData(paymentForm));
        closePaymentModal();
      });
    }

    // Month Navigation
    if (btnPrevMonth) {
      btnPrevMonth.addEventListener('click', () => {
        this.currentMonth--;
        if (this.currentMonth < 0) {
          this.currentMonth = 11;
          this.currentYear--;
        }
        this.renderCalendar();
      });
    }

    if (btnNextMonth) {
      btnNextMonth.addEventListener('click', () => {
        this.currentMonth++;
        if (this.currentMonth > 11) {
          this.currentMonth = 0;
          this.currentYear++;
        }
        this.renderCalendar();
      });
    }

    // Date Picker
    if (dueDateDisplay && datePickerModal) {
      dueDateDisplay.addEventListener('click', () => {
        this.pickerMonth = new Date().getMonth();
        this.pickerYear = new Date().getFullYear();
        this.pickerSelectedDate = null;
        datePickerModal.classList.add('open');
        this.renderDatePicker();
      });
    }

    const closeDatePicker = () => {
      if (datePickerModal) {
        datePickerModal.classList.remove('open');
      }
    };

    if (btnCloseDatePicker) {
      btnCloseDatePicker.addEventListener('click', closeDatePicker);
    }

    if (datePickerModal) {
      datePickerModal.addEventListener('click', (e) => {
        if (e.target === datePickerModal) closeDatePicker();
      });
    }

    // Date picker month navigation
    if (btnPickerPrevMonth) {
      btnPickerPrevMonth.addEventListener('click', () => {
        this.pickerMonth--;
        if (this.pickerMonth < 0) {
          this.pickerMonth = 11;
          this.pickerYear--;
        }
        this.renderDatePicker();
      });
    }

    if (btnPickerNextMonth) {
      btnPickerNextMonth.addEventListener('click', () => {
        this.pickerMonth++;
        if (this.pickerMonth > 11) {
          this.pickerMonth = 0;
          this.pickerYear++;
        }
        this.renderDatePicker();
      });
    }

    // Date picker actions
    if (btnClearDate) {
      btnClearDate.addEventListener('click', () => {
        this.pickerSelectedDate = null;
        const dueDateValue = document.getElementById('due-date-value');
        const dueDateDisplay = document.getElementById('due-date-display');
        if (dueDateValue) dueDateValue.value = '';
        if (dueDateDisplay) dueDateDisplay.value = '';
        closeDatePicker();
      });
    }

    if (btnTodayDate) {
      btnTodayDate.addEventListener('click', () => {
        const today = new Date();
        this.setPickerDate(today);
        closeDatePicker();
      });
    }

    // Close on Click Outside
    if (modal) {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                closeModal();
            }
        });
    }

    // Handle Form Submit
    if (addForm) {
      addForm.addEventListener('submit', (e) => {
        e.preventDefault();
        this.handleSubmit(new FormData(addForm));
        closeModal();
      });
    }
  },

  handleSubmit(formData) {
    const debt = {
      customer_name: formData.get('customer_name'),
      phone: formData.get('phone'),
      amount: parseFloat(formData.get('amount')),
      due_date: formData.get('due_date'),
      description: formData.get('description')
    };
    Store.addDebt(debt);
    Toast.success('Debt Added', `KSh ${debt.amount.toLocaleString()} for ${debt.customer_name}`);
  },

  render() {
    const tbody = document.getElementById('debts-table-body');
    if (!tbody) return;

    const allDebts = Store.debts;

    if (allDebts.length === 0) {
      tbody.innerHTML = `
        <tr class="text-center">
            <td colspan="8" class="px-5 py-8 text-gray-500">
                <div class="flex flex-col items-center justify-center">
                    <svg class="w-12 h-12 text-gray-300 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                    </svg>
                    <p>No debts recorded.</p>
                </div>
            </td>
        </tr>
      `;
      this.updatePaginationControls(0);
      return;
    }

    // Calculate pagination
    const totalPages = Math.ceil(allDebts.length / this.itemsPerPage);
    const startIndex = (this.currentPage - 1) * this.itemsPerPage;
    const endIndex = startIndex + this.itemsPerPage;
    const paginatedDebts = allDebts.slice(startIndex, endIndex);

    tbody.innerHTML = paginatedDebts.map(debt => {
      const isPaid = debt.status === 'paid';
      const isOverdue = !isPaid && debt.due_date && new Date(debt.due_date) < new Date();
      const paidAmount = debt.paid_amount || 0;
      const remainingAmount = debt.remaining_amount || debt.amount;
      const hasPayments = paidAmount > 0;
      
      // Determine status badge
      let statusBadge = '';
      if (isPaid) {
        statusBadge = '<span class="status-badge status-badge--success">Paid</span>';
      } else if (isOverdue) {
        statusBadge = '<span class="status-badge status-badge--error">Overdue</span>';
      } else {
        statusBadge = '<span class="status-badge status-badge--pending">Pending</span>';
      }
      
      return `
        <tr class="hover:bg-gray-50 transition-colors ${isPaid ? 'opacity-60' : ''}">
          <td class="px-5 py-4 text-sm font-medium text-gray-900">${debt.customer_name}</td>
          <td class="px-5 py-4 text-sm text-gray-600">${debt.phone || '-'}</td>
          <td class="px-5 py-4 text-sm font-medium text-gray-900">KSh ${debt.amount.toLocaleString(undefined, {minimumFractionDigits: 2})}</td>
          <td class="px-5 py-4 text-sm font-medium text-green-600">KSh ${paidAmount.toLocaleString(undefined, {minimumFractionDigits: 2})}</td>
          <td class="px-5 py-4 text-sm font-medium text-red-600">KSh ${remainingAmount.toLocaleString(undefined, {minimumFractionDigits: 2})}</td>
          <td class="px-5 py-4 text-sm text-gray-600">${debt.due_date || '-'}</td>
          <td class="px-5 py-4">
            ${statusBadge}
          </td>
          <td class="px-5 py-4">
            <div class="flex items-center gap-2">
              ${!isPaid ? `
                <button onclick="DebtsPage.recordPayment(${debt.id})" 
                  class="text-sm font-medium px-3 py-1 rounded-md ${hasPayments ? 'bg-blue-50 text-blue-600 hover:bg-blue-100' : 'bg-green-50 text-green-600 hover:bg-green-100'} transition-colors"
                  title="Record payment">
                  ${hasPayments ? 'Add Payment' : 'Pay'}
                </button>
              ` : ''}
              ${hasPayments ? `
                <button onclick="DebtsPage.viewPaymentHistory(${debt.id})" 
                  class="text-blue-600 hover:text-blue-800 text-sm font-medium"
                  title="View payment history">
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                  </svg>
                </button>
              ` : ''}
              <button onclick="DebtsPage.delete(${debt.id})" class="text-gray-400 hover:text-red-600 transition-colors"
                title="Delete debt">
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
    this.updatePaginationControls(allDebts.length);
  },

  updatePaginationControls(totalItems) {
    const paginationEl = document.getElementById('debts-pagination');
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
          Showing <span class="font-medium">${startItem}</span> to <span class="font-medium">${endItem}</span> of <span class="font-medium">${totalItems}</span> debts
        </div>
        <div class="flex gap-2">
          <button onclick="DebtsPage.previousPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === 1 ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === 1 ? 'disabled' : ''}>
            Previous
          </button>
          <span class="px-3 py-1 text-sm font-medium text-gray-700">
            Page ${this.currentPage} of ${totalPages}
          </span>
          <button onclick="DebtsPage.nextPage()" 
            class="px-3 py-1 text-sm font-medium rounded-md ${this.currentPage === totalPages ? 'bg-gray-200 text-gray-400 cursor-not-allowed' : 'bg-black text-white hover:bg-gray-800'}"
            ${this.currentPage === totalPages ? 'disabled' : ''}>
            Next
          </button>
        </div>
      </div>
    `;
  },

  nextPage() {
    const totalPages = Math.ceil(Store.debts.length / this.itemsPerPage);
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

  updateSummary() {
    const totalEl = document.getElementById('total-debt');
    const paidEl = document.getElementById('paid-month');
    const overdueEl = document.getElementById('overdue-count');

    if (totalEl) totalEl.textContent = `KSh ${Store.getTotalOutstanding().toLocaleString(undefined, {minimumFractionDigits: 2})}`;
    if (paidEl) paidEl.textContent = `KSh ${Store.getPaidThisMonth().toLocaleString(undefined, {minimumFractionDigits: 2})}`;
    if (overdueEl) overdueEl.textContent = Store.getOverdueDebts().length;
  },

  markPaid(id) {
    const debt = Store.debts.find(d => d.id === id);
    Store.markDebtPaid(id);
    if (debt) {
      Toast.success('Debt Paid', `KSh ${debt.amount.toLocaleString()} from ${debt.customer_name} marked as paid`);
    }
  },

  delete(id) {
    const debt = Store.debts.find(d => d.id === id);
    if (!debt) return;
    
    ConfirmModal.show({
      title: 'Delete Debt Record?',
      message: 'Are you sure you want to delete this debt record? This action cannot be undone.',
      itemName: debt.customer_name,
      itemDetails: `KSh ${debt.amount.toLocaleString()} • ${debt.description || 'No description'}`,
      onConfirm: () => {
        Store.deleteDebt(id);
        Toast.success('Debt Deleted', `Debt record for ${debt.customer_name} has been removed`);
      }
    });
  },

  recordPayment(debtId) {
    const debt = Store.debts.find(d => d.id === debtId);
    if (!debt) return;

    this.currentDebtId = debtId;
    const paidAmount = debt.paid_amount || 0;
    const remainingAmount = debt.remaining_amount || debt.amount;

    // Update modal with debt info
    document.getElementById('payment-customer-name').textContent = debt.customer_name;
    document.getElementById('payment-total-amount').textContent = `KSh ${debt.amount.toLocaleString()}`;
    document.getElementById('payment-paid-amount').textContent = `KSh ${paidAmount.toLocaleString()}`;
    document.getElementById('payment-remaining-amount').textContent = `KSh ${remainingAmount.toLocaleString()}`;
    document.getElementById('payment-debt-id').value = debtId;
    
    // Set max payment amount to remaining amount
    const amountInput = document.getElementById('payment-amount');
    if (amountInput) {
      amountInput.max = remainingAmount;
      amountInput.value = remainingAmount; // Default to full remaining amount
    }

    // Open modal
    const modal = document.getElementById('modal-record-payment');
    if (modal) modal.classList.add('open');
  },

  async handlePaymentSubmit(formData) {
    const debtId = parseInt(formData.get('debt_id'));
    const amount = parseFloat(formData.get('amount'));
    const paymentMethod = formData.get('payment_method');
    const notes = formData.get('notes');

    const debt = Store.debts.find(d => d.id === debtId);
    if (!debt) return;

    const remainingAmount = debt.remaining_amount || debt.amount;
    
    // Validate payment amount
    if (amount <= 0) {
      Toast.error('Invalid Amount', 'Payment amount must be greater than 0');
      return;
    }

    if (amount > remainingAmount) {
      Toast.error('Amount Exceeds Debt', `Payment cannot exceed remaining amount of KSh ${remainingAmount.toLocaleString()}`);
      return;
    }

    const payment = {
      debt_id: debtId,
      amount: amount,
      payment_method: paymentMethod,
      notes: notes
    };

    await Store.addDebtPayment(payment);
    
    const newRemaining = remainingAmount - amount;
    if (newRemaining <= 0) {
      Toast.success('Debt Fully Paid!', `${debt.customer_name} has paid the full debt of KSh ${debt.amount.toLocaleString()}`);
    } else {
      Toast.success('Payment Recorded', `KSh ${amount.toLocaleString()} paid • KSh ${newRemaining.toLocaleString()} remaining`);
    }
  },

  viewPaymentHistory(debtId) {
    const debt = Store.debts.find(d => d.id === debtId);
    if (!debt) return;

    this.currentDebtId = debtId;
    const paidAmount = debt.paid_amount || 0;
    const remainingAmount = debt.remaining_amount || debt.amount;

    // Update modal with debt info
    document.getElementById('history-customer-name').textContent = debt.customer_name;
    document.getElementById('history-total-amount').textContent = `KSh ${debt.amount.toLocaleString()}`;
    document.getElementById('history-paid-amount').textContent = `KSh ${paidAmount.toLocaleString()}`;
    document.getElementById('history-remaining-amount').textContent = `KSh ${remainingAmount.toLocaleString()}`;

    // Load payment history
    this.renderPaymentHistory(debtId);

    // Open modal
    const modal = document.getElementById('modal-payment-history');
    if (modal) modal.classList.add('open');
  },

  async renderPaymentHistory(debtId) {
    const tbody = document.getElementById('payment-history-body');
    if (!tbody) return;

    const payments = await Store.getDebtPayments(debtId);

    if (payments.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="4" class="px-5 py-8 text-center text-gray-500">No payments recorded yet.</td>
        </tr>
      `;
      return;
    }

    tbody.innerHTML = payments.map(payment => {
      const paymentDate = new Date(payment.payment_date);
      const isToday = paymentDate.toDateString() === new Date().toDateString();
      
      // Show time if today, show date + time if older
      const timeDisplay = isToday 
        ? paymentDate.toLocaleTimeString('en-US', {hour: '2-digit', minute:'2-digit'})
        : paymentDate.toLocaleDateString('en-US', {month: 'short', day: 'numeric', year: 'numeric'}) + ' ' + 
          paymentDate.toLocaleTimeString('en-US', {hour: '2-digit', minute:'2-digit'});
      
      const paymentLabel = payment.payment_method === 'mpesa' ? 'M-Pesa' : 
                           payment.payment_method === 'till' ? 'Till Number' : 'Cash';
      
      return `
        <tr class="hover:bg-gray-50 transition-colors">
          <td class="px-5 py-4 text-sm text-gray-600">${timeDisplay}</td>
          <td class="px-5 py-4 text-sm font-medium text-green-600">KSh ${payment.amount.toLocaleString(undefined, {minimumFractionDigits: 2})}</td>
          <td class="px-5 py-4">
            <span class="status-badge status-badge--success">${paymentLabel}</span>
          </td>
          <td class="px-5 py-4">
            <button onclick="DebtsPage.deletePayment(${payment.id}, ${debtId})" 
              class="text-gray-400 hover:text-red-600 transition-colors" 
              title="Delete payment">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
              </svg>
            </button>
          </td>
        </tr>
      `;
    }).join('');
  },

  async deletePayment(paymentId, debtId) {
    ConfirmModal.show({
      title: 'Delete Payment?',
      message: 'Are you sure you want to delete this payment record? The debt balance will be adjusted.',
      itemName: 'Payment Record',
      itemDetails: 'This will increase the remaining debt amount',
      onConfirm: async () => {
        await Store.deleteDebtPayment(paymentId);
        Toast.success('Payment Deleted', 'Payment record removed and debt balance updated');
        
        // Refresh payment history
        this.renderPaymentHistory(debtId);
        
        // Update summary in history modal
        const debt = Store.debts.find(d => d.id === debtId);
        if (debt) {
          const paidAmount = debt.paid_amount || 0;
          const remainingAmount = debt.remaining_amount || debt.amount;
          document.getElementById('history-paid-amount').textContent = `KSh ${paidAmount.toLocaleString()}`;
          document.getElementById('history-remaining-amount').textContent = `KSh ${remainingAmount.toLocaleString()}`;
        }
      }
    });
  },

  renderCalendar() {
    const monthYearEl = document.getElementById('calendar-month-year');
    const gridEl = document.getElementById('calendar-grid');
    
    if (!gridEl) return;

    // Update month/year display
    const monthNames = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
    if (monthYearEl) {
      monthYearEl.textContent = `${monthNames[this.currentMonth]} ${this.currentYear}`;
    }

    // Get first and last day of month
    const firstDay = new Date(this.currentYear, this.currentMonth, 1);
    const lastDay = new Date(this.currentYear, this.currentMonth + 1, 0);
    const daysInMonth = lastDay.getDate();
    const startingDayOfWeek = firstDay.getDay();

    // Get debts for this month (both pending and paid)
    const debtsThisMonth = Store.debts.filter(d => {
      if (!d.due_date) return false;
      const dueDate = new Date(d.due_date);
      return dueDate.getMonth() === this.currentMonth && dueDate.getFullYear() === this.currentYear;
    });

    // Group debts by day
    const debtsByDay = {};
    debtsThisMonth.forEach(debt => {
      const day = new Date(debt.due_date).getDate();
      if (!debtsByDay[day]) debtsByDay[day] = [];
      debtsByDay[day].push(debt);
    });

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
      if (day === today.getDate() && this.currentMonth === today.getMonth() && this.currentYear === today.getFullYear()) {
        dayEl.classList.add('today');
      }

      // Check if selected
      if (this.selectedDate && this.selectedDate.getDate() === day && this.selectedDate.getMonth() === this.currentMonth && this.selectedDate.getFullYear() === this.currentYear) {
        dayEl.classList.add('selected');
      }

      // Day number
      const dayNumber = document.createElement('div');
      dayNumber.className = 'calendar-day-number';
      dayNumber.textContent = day;
      dayEl.appendChild(dayNumber);

      // Add debt indicators
      const debtsForDay = debtsByDay[day] || [];
      if (debtsForDay.length > 0) {
        const currentDate = new Date();
        currentDate.setHours(0, 0, 0, 0);
        const dayDate = new Date(this.currentYear, this.currentMonth, day);
        dayDate.setHours(0, 0, 0, 0);
        const daysUntilDue = Math.ceil((dayDate - currentDate) / (1000 * 60 * 60 * 24));

        // Check if all debts are paid
        const allPaid = debtsForDay.every(d => d.status === 'paid');
        const hasPending = debtsForDay.some(d => d.status !== 'paid');

        let statusClass = 'upcoming';
        let statusLabel = 'Upcoming';
        
        if (allPaid) {
          statusClass = 'paid';
          statusLabel = 'Paid';
        } else if (daysUntilDue < 0) {
          statusClass = 'overdue';
          statusLabel = 'Overdue';
        } else if (daysUntilDue <= 3) {
          statusClass = 'due-soon';
          statusLabel = daysUntilDue === 0 ? 'Due Today' : daysUntilDue === 1 ? 'Due Tomorrow' : 'Due Soon';
        }

        // Add status class to day
        dayEl.classList.add('has-debts', `${statusClass}-day`);

        const indicator = document.createElement('div');
        indicator.className = `calendar-debt-indicator ${statusClass}`;
        dayEl.appendChild(indicator);

        const count = document.createElement('div');
        count.className = 'calendar-debt-count';
        if (allPaid) {
          count.textContent = `${debtsForDay.length} paid`;
        } else if (hasPending) {
          const pendingCount = debtsForDay.filter(d => d.status !== 'paid').length;
          const paidCount = debtsForDay.length - pendingCount;
          count.textContent = paidCount > 0 
            ? `${pendingCount} pending, ${paidCount} paid`
            : `${pendingCount} debt${pendingCount > 1 ? 's' : ''}`;
        }
        dayEl.appendChild(count);
      }

      // Click handler
      dayEl.addEventListener('click', () => {
        this.selectedDate = new Date(this.currentYear, this.currentMonth, day);
        this.renderCalendar();
        this.showDayDetails(day, debtsByDay[day] || []);
      });

      gridEl.appendChild(dayEl);
    }
  },

  showDayDetails(day, debts) {
    const detailsSection = document.getElementById('selected-day-details');
    const titleEl = document.getElementById('selected-day-title');
    const debtsContainer = document.getElementById('selected-day-debts');

    if (!detailsSection || !titleEl || !debtsContainer) return;

    const monthNames = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
    titleEl.textContent = `Debts for ${monthNames[this.currentMonth]} ${day}, ${this.currentYear}`;

    if (debts.length === 0) {
      debtsContainer.innerHTML = '<p class="text-sm text-gray-500 italic">No debts due on this day</p>';
    } else {
      debtsContainer.innerHTML = debts.map(debt => {
        const isPaid = debt.status === 'paid';
        
        // For paid debts, show simple green card
        if (isPaid) {
          return `
            <div class="p-4 rounded-lg border border-green-200 bg-green-50 opacity-75">
              <div class="flex items-start justify-between">
                <div class="flex-1">
                  <div class="flex items-center gap-2">
                    <h6 class="font-medium text-gray-900">${debt.customer_name}</h6>
                    <span class="px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700">Paid</span>
                  </div>
                  <p class="text-lg font-semibold text-gray-900 mt-1 line-through">KSh ${debt.amount.toLocaleString(undefined, {minimumFractionDigits: 2})}</p>
                  ${debt.description ? `<p class="text-xs text-gray-500 mt-1">${debt.description}</p>` : ''}
                </div>
              </div>
            </div>
          `;
        }
        
        const today = new Date();
        today.setHours(0, 0, 0, 0);
        const dueDate = new Date(debt.due_date);
        dueDate.setHours(0, 0, 0, 0);
        const daysDiff = Math.ceil((dueDate - today) / (1000 * 60 * 60 * 24));
        
        let statusClass = '';
        let statusText = '';
        let borderClass = 'border-gray-200';
        
        if (daysDiff < 0) {
          statusClass = 'bg-red-100 text-red-700';
          borderClass = 'border-red-200 bg-red-50';
          statusText = `Overdue by ${Math.abs(daysDiff)} day${Math.abs(daysDiff) > 1 ? 's' : ''}`;
        } else if (daysDiff <= 3) {
          statusClass = 'bg-amber-100 text-amber-700';
          borderClass = 'border-amber-200 bg-amber-50';
          statusText = daysDiff === 0 ? 'Due Today' : daysDiff === 1 ? 'Due Tomorrow' : `Due in ${daysDiff} days`;
        } else {
          statusClass = 'bg-blue-100 text-blue-700';
          borderClass = 'border-blue-200 bg-blue-50';
          statusText = `Due in ${daysDiff} days`;
        }
        
        return `
          <div class="p-4 rounded-lg border ${borderClass}">
            <div class="flex items-start justify-between">
              <div class="flex-1">
                <div class="flex items-center gap-2">
                  <h6 class="font-medium text-gray-900">${debt.customer_name}</h6>
                  <span class="px-2 py-0.5 rounded-full text-xs font-medium ${statusClass}">${statusText}</span>
                </div>
                <p class="text-lg font-semibold text-gray-900 mt-1">KSh ${debt.remaining_amount.toLocaleString(undefined, {minimumFractionDigits: 2})}</p>
                ${debt.description ? `<p class="text-xs text-gray-500 mt-1">${debt.description}</p>` : ''}
              </div>
            </div>
          </div>
        `;
      }).join('');
    }

    detailsSection.classList.remove('hidden');
  },

  renderDatePicker() {
    const monthYearEl = document.getElementById('picker-month-year');
    const gridEl = document.getElementById('picker-calendar-grid');
    
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
        const datePickerModal = document.getElementById('modal-date-picker');
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
    
    // Check if this is for the convert debt modal
    if (this.pickerCallback === 'convertDebt' && window.convertDebtDateCallback) {
      window.convertDebtDateCallback(formattedValue, formattedDisplay);
      this.pickerCallback = null;
    } else {
      // Normal debt form date picker
      const dueDateValue = document.getElementById('due-date-value');
      const dueDateDisplay = document.getElementById('due-date-display');
      
      if (dueDateValue) dueDateValue.value = formattedValue;
      if (dueDateDisplay) dueDateDisplay.value = formattedDisplay;
    }
    
    this.renderDatePicker();
  }
};

window.DebtsPage = DebtsPage;
