/**
 * Settings Page Controller
 */

const SettingsPage = {
  currentTab: 'account',
  settings: {
    currency: 'KES',
    currencySymbol: 'KSh'
  },

  init() {
    this.loadSettings();
    this.loadCurrentUser();
    this.bindEvents();
    this.loadAppVersion();
    this.loadPlatformInfo();
    this.hideAdminTabsForEmployee();
  },

  loadCurrentUser() {
    try {
      const currentUser = JSON.parse(localStorage.getItem('currentUser') || '{}');
      const currentUsernameEl = document.getElementById('current-username');
      if (currentUsernameEl && currentUser.username) {
        currentUsernameEl.value = currentUser.username;
      }
    } catch (error) {
      console.error('Error loading current user:', error);
    }
  },

  hideAdminTabsForEmployee() {
    const role = window.Permissions ? Permissions.getCurrentRole() : 'user';
    if (role === 'employee') {
      // Hide Backup & Data tab for employees
      const backupTab = document.getElementById('tab-backup');
      if (backupTab) backupTab.style.display = 'none';
    }
  },

  bindEvents() {
    // Tab switching
    const tabs = document.querySelectorAll('.settings-tab');
    tabs.forEach(tab => {
      tab.addEventListener('click', (e) => {
        const tabId = e.currentTarget.id.replace('tab-', '');
        this.switchTab(tabId);
      });
    });

    // Change username form
    const usernameForm = document.getElementById('change-username-form');
    if (usernameForm) {
      usernameForm.addEventListener('submit', (e) => {
        e.preventDefault();
        this.changeUsername();
      });
    }

    // Change password form
    const passwordForm = document.getElementById('change-password-form');
    if (passwordForm) {
      passwordForm.addEventListener('submit', (e) => {
        e.preventDefault();
        this.changePassword();
      });
    }

    // Password visibility toggles
    document.querySelectorAll('.password-toggle').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.preventDefault();
        this.togglePasswordVisibility(btn);
      });
    });

    // Backup & Data actions
    document.getElementById('btn-export-data')?.addEventListener('click', () => {
      this.exportData();
    });

    document.getElementById('btn-import-data')?.addEventListener('click', () => {
      this.importData();
    });

    document.getElementById('btn-clear-data')?.addEventListener('click', () => {
      this.clearAllData();
    });
  },

  togglePasswordVisibility(btn) {
    const targetId = btn.getAttribute('data-target');
    const input = document.getElementById(targetId);
    const eyeOpen = btn.querySelector('.eye-open');
    const eyeClosed = btn.querySelector('.eye-closed');

    if (input.type === 'password') {
      input.type = 'text';
      eyeOpen.style.display = 'none';
      eyeClosed.style.display = 'block';
      btn.classList.add('active');
    } else {
      input.type = 'password';
      eyeOpen.style.display = 'block';
      eyeClosed.style.display = 'none';
      btn.classList.remove('active');
    }
  },

  switchTab(tabId) {
    // Update active tab
    document.querySelectorAll('.settings-tab').forEach(tab => {
      tab.classList.remove('active', 'border-black', 'text-gray-900');
      tab.classList.add('border-transparent', 'text-gray-500');
    });

    const activeTab = document.getElementById(`tab-${tabId}`);
    if (activeTab) {
      activeTab.classList.add('active', 'border-black', 'text-gray-900');
      activeTab.classList.remove('border-transparent', 'text-gray-500');
    }

    // Show corresponding panel
    document.querySelectorAll('.settings-panel').forEach(panel => {
      panel.classList.add('hidden');
    });

    const activePanel = document.getElementById(`panel-${tabId}`);
    if (activePanel) {
      activePanel.classList.remove('hidden');
    }

    this.currentTab = tabId;
  },

  loadSettings() {
    try {
      const saved = localStorage.getItem('app_settings');
      if (saved) {
        this.settings = { ...this.settings, ...JSON.parse(saved) };
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
    }
  },

  saveSettings() {
    try {
      localStorage.setItem('app_settings', JSON.stringify(this.settings));
      Toast.success('Settings Saved', 'Your preferences have been updated');
    } catch (error) {
      console.error('Failed to save settings:', error);
      Toast.error('Save Failed', 'Could not save settings');
    }
  },

  async changeUsername() {
    const currentUser = JSON.parse(localStorage.getItem('currentUser') || '{}');
    const oldUsername = currentUser.username;
    const newUsername = document.getElementById('new-username')?.value?.trim();

    if (!newUsername) {
      Toast.error('Invalid Username', 'Please enter a new username');
      return;
    }

    if (newUsername === oldUsername) {
      Toast.error('Same Username', 'New username must be different from current username');
      return;
    }

    if (newUsername.length < 3) {
      Toast.error('Too Short', 'Username must be at least 3 characters');
      return;
    }

    try {
      const result = await window.auth.updateUsername(oldUsername, newUsername);
      
      if (result.success) {
        // Update localStorage
        currentUser.username = newUsername;
        localStorage.setItem('currentUser', JSON.stringify(currentUser));
        
        // Update display
        document.getElementById('current-username').value = newUsername;
        document.getElementById('new-username').value = '';
        
        Toast.success('Username Changed', `Your username is now ${newUsername}`);
      } else {
        Toast.error('Failed', result.error || 'Could not change username');
      }
    } catch (error) {
      console.error('Error changing username:', error);
      Toast.error('Error', 'Failed to change username');
    }
  },

  async changePassword() {
    const currentUser = JSON.parse(localStorage.getItem('currentUser') || '{}');
    const currentPassword = document.getElementById('current-password')?.value;
    const newPassword = document.getElementById('new-password')?.value;
    const confirmPassword = document.getElementById('confirm-password')?.value;

    if (!currentPassword || !newPassword || !confirmPassword) {
      Toast.error('Missing Fields', 'Please fill in all password fields');
      return;
    }

    if (newPassword !== confirmPassword) {
      Toast.error('Password Mismatch', 'New passwords do not match');
      return;
    }

    if (newPassword.length < 4) {
      Toast.error('Too Short', 'Password must be at least 4 characters');
      return;
    }

    try {
      const result = await window.auth.updatePassword(currentUser.username, currentPassword, newPassword);
      
      if (result.success) {
        // Clear form
        document.getElementById('current-password').value = '';
        document.getElementById('new-password').value = '';
        document.getElementById('confirm-password').value = '';
        
        Toast.success('Password Changed', 'Your password has been updated');
      } else {
        Toast.error('Failed', result.error || 'Current password is incorrect');
      }
    } catch (error) {
      console.error('Error changing password:', error);
      Toast.error('Error', 'Failed to change password');
    }
  },

  async loadAppVersion() {
    try {
      if (window.app && window.app.getVersion) {
        const version = await window.app.getVersion();
        const versionEl = document.getElementById('app-version');
        if (versionEl) versionEl.textContent = version;
      }
    } catch (error) {
      console.error('Failed to load app version:', error);
    }
  },

  async loadPlatformInfo() {
    try {
      const platformEl = document.getElementById('platform-info');
      if (platformEl && window.app && window.app.getPlatform) {
        const platform = await window.app.getPlatform();
        platformEl.textContent = platform;
      } else if (platformEl) {
        platformEl.textContent = 'Desktop Application';
      }
    } catch (error) {
      console.error('Failed to load platform info:', error);
      const platformEl = document.getElementById('platform-info');
      if (platformEl) platformEl.textContent = 'Desktop Application';
    }
  },

  async exportData() {
    try {
      const data = {
        products: Store.products,
        stock: Store.stock,
        sales: Store.sales,
        debts: Store.debts,
        services: Store.services,
        serviceTransactions: Store.serviceTransactions,
        printingMaterials: Store.printingMaterials,
        settings: this.settings,
        exportDate: new Date().toISOString(),
        version: '1.0.0'
      };

      const dataStr = JSON.stringify(data, null, 2);
      const dataBlob = new Blob([dataStr], { type: 'application/json' });
      
      const url = URL.createObjectURL(dataBlob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `multiprints-backup-${new Date().toISOString().split('T')[0]}.json`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);

      // Update last backup date
      const lastBackupEl = document.getElementById('last-backup');
      if (lastBackupEl) {
        lastBackupEl.textContent = new Date().toLocaleString();
      }

      Toast.success('Export Complete', 'Database backup downloaded successfully');
    } catch (error) {
      console.error('Export failed:', error);
      Toast.error('Export Failed', 'Could not export data');
    }
  },

  async importData() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    
    input.onchange = async (e) => {
      const file = e.target.files[0];
      if (!file) return;

      try {
        const text = await file.text();
        const data = JSON.parse(text);

        // Validate data structure
        if (!data.version || !data.exportDate) {
          throw new Error('Invalid backup file format');
        }

        ConfirmModal.show({
          title: 'Import Database?',
          message: 'This will replace all current data with the backup. This action cannot be undone.',
          itemName: 'Database Import',
          itemDetails: `Backup from ${new Date(data.exportDate).toLocaleDateString()}`,
          onConfirm: async () => {
            await this.performImport(data);
          }
        });
      } catch (error) {
        console.error('Import failed:', error);
        Toast.error('Import Failed', 'Invalid backup file or corrupt data');
      }
    };

    input.click();
  },

  async performImport(data) {
    try {
      // Import products
      if (data.products && data.products.length > 0) {
        for (const product of data.products) {
          await Store.addProduct(product);
        }
      }

      // Import stock
      if (data.stock && data.stock.length > 0) {
        for (const stockItem of data.stock) {
          await Store.addStock(stockItem);
        }
      }

      // Import other data...
      if (data.settings) {
        this.settings = { ...this.settings, ...data.settings };
        localStorage.setItem('app_settings', JSON.stringify(this.settings));
      }

      Toast.success('Import Complete', 'Database restored successfully');
      
      // Reload the page to refresh all data
      setTimeout(() => {
        window.location.reload();
      }, 1500);
    } catch (error) {
      console.error('Import failed:', error);
      Toast.error('Import Failed', 'Could not restore data');
    }
  },

  clearAllData() {
    ConfirmModal.show({
      title: 'Clear All Data?',
      message: 'This will permanently delete ALL products, sales, debts, and stock data. This action CANNOT be undone!',
      itemName: 'All Business Data',
      itemDetails: 'Make sure you have a backup before proceeding',
      onConfirm: async () => {
        try {
          // Clear all store data
          Store.products = [];
          Store.stock = [];
          Store.sales = [];
          Store.debts = [];
          Store.services = [];
          Store.serviceTransactions = [];
          Store.printingMaterials = [];

          // Notify all subscriptions
          Store.notify('products');
          Store.notify('stock');
          Store.notify('sales');
          Store.notify('debts');
          Store.notify('services');
          Store.notify('serviceTransactions');
          Store.notify('printingMaterials');

          Toast.success('Data Cleared', 'All business data has been deleted');
          
          // Reload after a moment
          setTimeout(() => {
            window.location.reload();
          }, 1500);
        } catch (error) {
          console.error('Clear data failed:', error);
          Toast.error('Clear Failed', 'Could not clear all data');
        }
      }
    });
  }
};

window.SettingsPage = SettingsPage;
