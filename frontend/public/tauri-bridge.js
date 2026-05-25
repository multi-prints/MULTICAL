/**
 * Tauri IPC Bridge
 * This file is copied to the output by Trunk when it builds.
 * It makes the Tauri invoke function available as a global JS function
 * that can be called from Rust WASM via wasm-bindgen.
 */

(function() {
  window.__TAURI_INVOKE__ = null;

  function initTauriInvoke() {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      window.__TAURI_INVOKE__ = function(cmd, args) {
        return window.__TAURI__.core.invoke(cmd, args);
      };
      console.log('Tauri IPC bridge initialized');
      return true;
    }
    return false;
  }

  // Try immediately
  if (!initTauriInvoke()) {
    // Retry with backoff
    let attempts = 0;
    const maxAttempts = 100;
    const interval = setInterval(function() {
      attempts++;
      if (initTauriInvoke() || attempts >= maxAttempts) {
        clearInterval(interval);
        if (attempts >= maxAttempts) {
          console.warn('Tauri IPC bridge: could not find __TAURI__ after ' + maxAttempts + ' attempts');
          // Provide a mock for development outside Tauri
          window.__TAURI_INVOKE__ = function(cmd, args) {
            console.log('Mock invoke:', cmd, args);
            return Promise.resolve(null);
          };
        }
      }
    }, 100);
  }
})();
