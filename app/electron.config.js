/**
 * Electron Configuration
 */
module.exports = {
  // Window configuration
  window: {
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    title: 'MULTIPRINTS',
    // Window behavior
    show: false, // Show when ready
    center: true,
    // Frame options
    frame: true,
    titleBarStyle: 'default',
    // Background color (shown while loading)
    backgroundColor: '#1a1a2e'
  },

  // Web preferences for renderer
  webPreferences: {
    nodeIntegration: false,
    contextIsolation: true,
    sandbox: false, // Required for better-sqlite3
    webSecurity: true
  },

  // Development options
  development: {
    openDevTools: false,
    devToolsPosition: 'right'
  }
};

