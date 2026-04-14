/**
 * Login Page Controller
 * Handles user authentication and form submission
 */

class LoginPage {
  constructor() {
    this.form = document.getElementById('loginForm');
    this.usernameInput = document.getElementById('username');
    this.passwordInput = document.getElementById('password');
    this.loginButton = document.getElementById('loginButton');
    this.buttonText = document.getElementById('buttonText');
    this.spinner = document.getElementById('spinner');
    this.loginError = document.getElementById('loginError');
    this.isLoading = false;
  }

  /**
   * Initialize login page
   */
  init() {
    this.setupEventListeners();
    this.checkExistingSession();
  }

  /**
   * Setup event listeners
   */
  setupEventListeners() {
    this.form.addEventListener('submit', (e) => this.handleLogin(e));
    
    // Clear error messages on input
    this.usernameInput.addEventListener('input', () => this.clearError('username'));
    this.passwordInput.addEventListener('input', () => this.clearError('password'));

    // Setup password toggle
    const togglePasswordBtn = document.getElementById('togglePassword');
    if (togglePasswordBtn) {
      togglePasswordBtn.addEventListener('click', (e) => this.togglePasswordVisibility(e));
    }
  }

  /**
   * Toggle password visibility
   */
  togglePasswordVisibility(e) {
    e.preventDefault();

    const eyeOpen = document.querySelector('.eye-open');
    const eyeClosed = document.querySelector('.eye-closed');
    const toggleBtn = document.getElementById('togglePassword');

    if (this.passwordInput.type === 'password') {
      this.passwordInput.type = 'text';
      eyeOpen.style.display = 'none';
      eyeClosed.style.display = 'block';
      toggleBtn.classList.add('active');
    } else {
      this.passwordInput.type = 'password';
      eyeOpen.style.display = 'block';
      eyeClosed.style.display = 'none';
      toggleBtn.classList.remove('active');
    }
  }

  /**
   * Check if user already has a valid session
   */
  async checkExistingSession() {
    try {
      const isValid = await window.api.validateSession();
      if (isValid) {
        // User is already logged in, redirect to dashboard
        this.redirectToDashboard();
      }
    } catch (error) {
      console.log('No existing session');
    }
  }

  /**
   * Handle login form submission
   */
  async handleLogin(e) {
    e.preventDefault();

    if (this.isLoading) return;

    const username = this.usernameInput.value.trim();
    const password = this.passwordInput.value;

    // Validate inputs
    if (!username) {
      this.showFieldError('username', 'Username is required');
      return;
    }

    if (!password) {
      this.showFieldError('password', 'Password is required');
      return;
    }

    this.setLoading(true);

    try {
      const result = await window.api.login(username, password);

      if (result.success) {
        // Store session token
        localStorage.setItem('sessionToken', result.token);
        localStorage.setItem('currentUser', JSON.stringify(result.user));

        // Clear form
        this.form.reset();

        // Redirect to dashboard
        this.redirectToDashboard();
      } else {
        this.showLoginError(result.error || 'Login failed');
      }
    } catch (error) {
      console.error('Login error:', error);
      this.showLoginError('An error occurred. Please try again.');
    } finally {
      this.setLoading(false);
    }
  }

  /**
   * Show field error
   */
  showFieldError(field, message) {
    const errorElement = document.getElementById(`${field}Error`);
    if (errorElement) {
      errorElement.textContent = message;
      errorElement.classList.add('show');
    }
  }

  /**
   * Clear field error
   */
  clearError(field) {
    const errorElement = document.getElementById(`${field}Error`);
    if (errorElement) {
      errorElement.textContent = '';
      errorElement.classList.remove('show');
    }
  }

  /**
   * Show login error
   */
  showLoginError(message) {
    this.loginError.textContent = message;
    this.loginError.classList.add('show');
    
    // Shake animation
    this.form.style.animation = 'none';
    setTimeout(() => {
      this.form.style.animation = '';
    }, 10);
  }

  /**
   * Set loading state
   */
  setLoading(isLoading) {
    this.isLoading = isLoading;
    this.loginButton.disabled = isLoading;
    
    if (isLoading) {
      this.buttonText.textContent = 'Logging in...';
      this.spinner.style.display = 'block';
    } else {
      this.buttonText.textContent = 'Login';
      this.spinner.style.display = 'none';
    }
  }

  /**
   * Redirect to dashboard
   */
  redirectToDashboard() {
    // Redirect to main app (go up one level from pages/ to root)
    window.location.href = '../index.html';
  }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  const loginPage = new LoginPage();
  loginPage.init();
});
