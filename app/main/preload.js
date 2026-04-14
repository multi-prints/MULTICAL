const { contextBridge, ipcRenderer } = require('electron');

/**
 * Preload script - Securely exposes APIs to renderer
 * Using contextBridge for security
 */

// API exposed to renderer
contextBridge.exposeInMainWorld('electronAPI', {
  // Application info
  getAppVersion: () => ipcRenderer.invoke('app:version'),
  getPlatform: () => process.platform,
  
  // Window controls
  minimizeWindow: () => ipcRenderer.send('window:minimize'),
  maximizeWindow: () => ipcRenderer.send('window:maximize'),
  closeWindow: () => ipcRenderer.send('window:close'),
  
  // File system
  selectFile: (options) => ipcRenderer.invoke('dialog:openFile', options),
  selectDirectory: (options) => ipcRenderer.invoke('dialog:openDirectory', options),
});

// Authentication API exposed to renderer
contextBridge.exposeInMainWorld('api', {
  // Authentication
  login: (username, password) => ipcRenderer.invoke('auth:login', username, password),
  logout: (token) => ipcRenderer.invoke('auth:logout', token),
  validateSession: () => ipcRenderer.invoke('auth:validateToken', localStorage.getItem('sessionToken')),
  getSession: (token) => ipcRenderer.invoke('auth:getSession', token),
  
  // User management (admin only)
  addUser: (username, password, role) => ipcRenderer.invoke('auth:addUser', username, password, role),
  updatePassword: (username, oldPassword, newPassword) => ipcRenderer.invoke('auth:updatePassword', username, oldPassword, newPassword),
  updateUsername: (oldUsername, newUsername) => ipcRenderer.invoke('auth:updateUsername', oldUsername, newUsername),
  getAllUsers: () => ipcRenderer.invoke('auth:getAllUsers'),
  deleteUser: (username) => ipcRenderer.invoke('auth:deleteUser', username)
});

// App API exposed to renderer (for settings page)
contextBridge.exposeInMainWorld('app', {
  getVersion: () => ipcRenderer.invoke('app:version'),
  getPlatform: async () => {
    const platform = process.platform;
    const version = process.versions.electron;
    return `Electron ${version || 'Unknown'} (${platform})`;
  }
});

// Auth API alias for easier access
contextBridge.exposeInMainWorld('auth', {
  updatePassword: (username, oldPassword, newPassword) => ipcRenderer.invoke('auth:updatePassword', username, oldPassword, newPassword),
  updateUsername: (oldUsername, newUsername) => ipcRenderer.invoke('auth:updateUsername', oldUsername, newUsername),
});

// Database API exposed to renderer
contextBridge.exposeInMainWorld('db', {
  // ==================== Products ====================
  products: {
    getAll: () => ipcRenderer.invoke('db:products:getAll'),
    get: (id) => ipcRenderer.invoke('db:products:get', id),
    add: (product) => ipcRenderer.invoke('db:products:add', product),
    update: (id, updates) => ipcRenderer.invoke('db:products:update', id, updates),
    delete: (id) => ipcRenderer.invoke('db:products:delete', id)
  },
  
  // ==================== Stock ====================
  stock: {
    getAll: () => ipcRenderer.invoke('db:stock:getAll'),
    get: (id) => ipcRenderer.invoke('db:stock:get', id),
    getByColorSizeType: (color, size, stickerType) => 
      ipcRenderer.invoke('db:stock:getByColorSizeType', color, size, stickerType),
    add: (stockItem) => ipcRenderer.invoke('db:stock:add', stockItem),
    update: (id, updates) => ipcRenderer.invoke('db:stock:update', id, updates),
    delete: (id) => ipcRenderer.invoke('db:stock:delete', id)
  },
  
  // ==================== Sales ====================
  sales: {
    getAll: () => ipcRenderer.invoke('db:sales:getAll'),
    getToday: () => ipcRenderer.invoke('db:sales:getToday'),
    add: (sale) => ipcRenderer.invoke('db:sales:add', sale),
    update: (id, updates) => ipcRenderer.invoke('db:sales:update', id, updates),
    getTodayTotal: () => ipcRenderer.invoke('db:sales:getTodayTotal'),
    delete: (id) => ipcRenderer.invoke('db:sales:delete', id)
  },
  
  // ==================== Debts ====================
  debts: {
    getAll: () => ipcRenderer.invoke('db:debts:getAll'),
    getPending: () => ipcRenderer.invoke('db:debts:getPending'),
    add: (debt) => ipcRenderer.invoke('db:debts:add', debt),
    update: (id, updates) => ipcRenderer.invoke('db:debts:update', id, updates),
    getBySaleId: (id) => ipcRenderer.invoke('db:debts:getBySaleId', id),
    getByTransactionId: (id) => ipcRenderer.invoke('db:debts:getByTransactionId', id),
    markPaid: (id) => ipcRenderer.invoke('db:debts:markPaid', id),
    delete: (id) => ipcRenderer.invoke('db:debts:delete', id),
    getTotalOutstanding: () => ipcRenderer.invoke('db:debts:getTotalOutstanding'),
    getPaidThisMonth: () => ipcRenderer.invoke('db:debts:getPaidThisMonth'),
    getOverdue: () => ipcRenderer.invoke('db:debts:getOverdue')
  },
  debtPayments: {
    add: (payment) => ipcRenderer.invoke('db:debtPayments:add', payment),
    getByDebt: (debtId) => ipcRenderer.invoke('db:debtPayments:getByDebt', debtId),
    delete: (id) => ipcRenderer.invoke('db:debtPayments:delete', id)
  },
  
  // ==================== Services ====================
  services: {
    getAll: () => ipcRenderer.invoke('db:services:getAll'),
    getActive: () => ipcRenderer.invoke('db:services:getActive'),
    get: (id) => ipcRenderer.invoke('db:services:get', id),
    add: (service) => ipcRenderer.invoke('db:services:add', service),
    update: (id, updates) => ipcRenderer.invoke('db:services:update', id, updates),
    delete: (id) => ipcRenderer.invoke('db:services:delete', id)
  },
  
  // ==================== Service Transactions ====================
  serviceTransactions: {
    getAll: () => ipcRenderer.invoke('db:serviceTransactions:getAll'),
    getToday: () => ipcRenderer.invoke('db:serviceTransactions:getToday'),
    add: (transaction) => ipcRenderer.invoke('db:serviceTransactions:add', transaction),
    update: (id, updates) => ipcRenderer.invoke('db:serviceTransactions:update', id, updates),
    getTodayTotal: () => ipcRenderer.invoke('db:serviceTransactions:getTodayTotal'),
    getTotal: () => ipcRenderer.invoke('db:serviceTransactions:getTotal'),
    delete: (id) => ipcRenderer.invoke('db:serviceTransactions:delete', id)
  },
  
  // ==================== Printing Materials ====================
  printingMaterials: {
    getAll: () => ipcRenderer.invoke('db:printingMaterials:getAll'),
    get: (id) => ipcRenderer.invoke('db:printingMaterials:get', id),
    add: (material) => ipcRenderer.invoke('db:printingMaterials:add', material),
    update: (id, updates) => ipcRenderer.invoke('db:printingMaterials:update', id, updates),
    delete: (id) => ipcRenderer.invoke('db:printingMaterials:delete', id)
  },
  
  // ==================== Migration ====================
  migrate: (localStorageData) => ipcRenderer.invoke('db:migrate', localStorageData)
});

// Notify main process that preload is ready
window.addEventListener('DOMContentLoaded', () => {
  console.log('Preload script loaded');
});
