const DashboardPage = {
  chart: null,
  currentChartPeriod: 'week',

  init() {
    // Initialize welcome message first
    this.updateWelcomeMessage();

    // Bind chart filter events
    this.bindChartEvents();

    // Initialize chart
    this.initChart();

    // Subscribe to store updates
    Store.subscribe('products', () => {
        this.updateStats();
        this.renderTopProducts();
    });
    Store.subscribe('sales', () => {
        this.updateStats();
        this.updateChart();
        this.renderRecentSales();
        this.renderActivityFeed();
        this.renderTopProducts();
    });
    Store.subscribe('serviceTransactions', () => {
        this.updateStats();
        this.updateChart();
    });
    Store.subscribe('debts', () => {
        this.updateStats();
        this.renderActivityFeed();
    });
    
    // Wait for next tick to ensure DOM is ready
    requestAnimationFrame(() => {
      this.updateStats();
      this.renderRecentSales();
      this.renderActivityFeed();
      this.renderTopProducts();
    });
    
    // Force immediate update in case Store is already initialized
    if (Store.initialized) {
      this.updateStats();
      this.renderRecentSales();
      this.renderActivityFeed();
      this.renderTopProducts();
    } else {
      // Backup: Force update after Store is definitely ready
      const checkStore = setInterval(() => {
        if (Store.initialized) {
          clearInterval(checkStore);
          this.updateStats();
          this.renderRecentSales();
          this.renderActivityFeed();
          this.renderTopProducts();
        }
      }, 100);
      // Stop checking after 5 seconds
      setTimeout(() => clearInterval(checkStore), 5000);
    }
  },

  updateWelcomeMessage() {
      const welcomeEl = document.getElementById('welcome-message');
      if (welcomeEl) {
          const hour = new Date().getHours();
          let greeting = 'Good Morning';
          if (hour >= 12 && hour < 17) {
              greeting = 'Good Afternoon';
          } else if (hour >= 17) {
              greeting = 'Good Evening';
          }
          // Get username from localStorage or use default
          const currentUser = JSON.parse(localStorage.getItem('currentUser') || '{}');
          const username = currentUser.username || 'Admin';
          welcomeEl.textContent = `${greeting}, ${username}`;
      }
  },

  updateStats() {
    const revenueEl = document.getElementById('stat-revenue');
    const salesEl = document.getElementById('stat-sales');
    const productsEl = document.getElementById('stat-products');
    const debtsEl = document.getElementById('stat-debts');
    const todayRevenueEl = document.getElementById('stat-today-revenue');
    const debtsCountEl = document.getElementById('stat-debts-count');

    // Check if elements exist - if not, retry later
    if (!revenueEl || !salesEl || !productsEl || !debtsEl) {
      setTimeout(() => this.updateStats(), 100);
      return;
    }

    // Calculate total revenue from both sales and service transactions
    const salesRevenue = Store.sales?.reduce((sum, s) => sum + s.amount, 0) || 0;
    const serviceRevenue = Store.serviceTransactions?.reduce((sum, t) => sum + t.amount, 0) || 0;
    const totalRevenue = salesRevenue + serviceRevenue;

    // Today's sales and revenue
    const today = new Date().toDateString();
    const todaySales = Store.sales?.filter(s => new Date(s.timestamp).toDateString() === today) || [];
    const todayServiceTransactions = Store.serviceTransactions?.filter(t => new Date(t.timestamp).toDateString() === today) || [];
    const todaySalesCount = todaySales.length + todayServiceTransactions.filter(t => t.stock_metres_used > 0).length;
    const todayRevenue = todaySales.reduce((sum, s) => sum + s.amount, 0) +
                         todayServiceTransactions.reduce((sum, t) => sum + t.amount, 0);

    const salesCount = Store.sales?.length || 0;
    const productsCount = Store.products?.length || 0;
    const outstanding = Store.getTotalOutstanding?.() || 0;
    const pendingDebts = Store.debts?.filter(d => d.status === 'pending').length || 0;

    revenueEl.textContent = `KSh ${totalRevenue.toLocaleString('en-KE', { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;
    salesEl.textContent = todaySalesCount;
    productsEl.textContent = productsCount;
    debtsEl.textContent = `KSh ${outstanding.toLocaleString('en-KE', { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;

    if (todayRevenueEl) {
      todayRevenueEl.innerHTML = `<span>Today: KSh ${todayRevenue.toLocaleString()}</span>`;
    }

    if (debtsCountEl) {
      debtsCountEl.innerHTML = `<span>${pendingDebts} pending</span>`;
    }
  },

  bindChartEvents() {
    const btnWeek = document.getElementById('btn-chart-week');
    const btnMonth = document.getElementById('btn-chart-month');
    const btnYear = document.getElementById('btn-chart-year');

    const setActiveBtn = (activeBtn) => {
      [btnWeek, btnMonth, btnYear].forEach(btn => {
        if (btn) {
          btn.classList.remove('bg-black', 'text-white', 'shadow-sm');
          btn.classList.add('text-gray-600');
        }
      });
      if (activeBtn) {
        activeBtn.classList.add('bg-black', 'text-white', 'shadow-sm');
        activeBtn.classList.remove('text-gray-600');
      }
    };

    if (btnWeek) {
      btnWeek.addEventListener('click', () => {
        this.currentChartPeriod = 'week';
        setActiveBtn(btnWeek);
        this.initChart();
      });
    }

    if (btnMonth) {
      btnMonth.addEventListener('click', () => {
        this.currentChartPeriod = 'month';
        setActiveBtn(btnMonth);
        this.initChart();
      });
    }

    if (btnYear) {
      btnYear.addEventListener('click', () => {
        this.currentChartPeriod = 'year';
        setActiveBtn(btnYear);
        this.initChart();
      });
    }
  },

  /**
   * Get revenue data for the selected period
   */
  getRevenueData() {
    const period = this.currentChartPeriod;
    const now = new Date();
    let labels = [];
    let data = [];

    if (period === 'week') {
      // Last 7 days
      for (let i = 6; i >= 0; i--) {
        const date = new Date(now);
        date.setDate(date.getDate() - i);
        labels.push(date.toLocaleDateString('en-US', { weekday: 'short' }));

        // Calculate revenue for this day
        const dayStart = new Date(date);
        dayStart.setHours(0, 0, 0, 0);
        const dayEnd = new Date(date);
        dayEnd.setHours(23, 59, 59, 999);

        const salesRevenue = Store.sales
          .filter(s => {
            const saleDate = new Date(s.timestamp);
            return saleDate >= dayStart && saleDate <= dayEnd;
          })
          .reduce((sum, s) => sum + s.amount, 0);

        const serviceRevenue = Store.serviceTransactions
          .filter(t => {
            const tDate = new Date(t.timestamp);
            return tDate >= dayStart && tDate <= dayEnd;
          })
          .reduce((sum, t) => sum + t.amount, 0);

        data.push(salesRevenue + serviceRevenue);
      }
    } else if (period === 'month') {
      // Last 30 days, grouped by week
      const weekLabels = ['This Week', 'Last Week', '2 Weeks Ago', '3 Weeks Ago'];
      labels = weekLabels;

      for (let w = 0; w < 4; w++) {
        const weekStart = new Date(now);
        weekStart.setDate(weekStart.getDate() - (w * 7) - 6);
        weekStart.setHours(0, 0, 0, 0);
        const weekEnd = new Date(now);
        weekEnd.setDate(weekEnd.getDate() - (w * 7));
        weekEnd.setHours(23, 59, 59, 999);

        const salesRevenue = Store.sales
          .filter(s => {
            const saleDate = new Date(s.timestamp);
            return saleDate >= weekStart && saleDate <= weekEnd;
          })
          .reduce((sum, s) => sum + s.amount, 0);

        const serviceRevenue = Store.serviceTransactions
          .filter(t => {
            const tDate = new Date(t.timestamp);
            return tDate >= weekStart && tDate <= weekEnd;
          })
          .reduce((sum, t) => sum + t.amount, 0);

        data.push(salesRevenue + serviceRevenue);
      }
    } else if (period === 'year') {
      // Last 12 months
      for (let i = 11; i >= 0; i--) {
        const monthDate = new Date(now.getFullYear(), now.getMonth() - i, 1);
        labels.push(monthDate.toLocaleDateString('en-US', { month: 'short' }));

        const monthStart = new Date(monthDate);
        const monthEnd = new Date(monthDate.getFullYear(), monthDate.getMonth() + 1, 0, 23, 59, 59, 999);

        const salesRevenue = Store.sales
          .filter(s => {
            const saleDate = new Date(s.timestamp);
            return saleDate >= monthStart && saleDate <= monthEnd;
          })
          .reduce((sum, s) => sum + s.amount, 0);

        const serviceRevenue = Store.serviceTransactions
          .filter(t => {
            const tDate = new Date(t.timestamp);
            return tDate >= monthStart && tDate <= monthEnd;
          })
          .reduce((sum, t) => sum + t.amount, 0);

        data.push(salesRevenue + serviceRevenue);
      }
    }

    return { labels, data };
  },

  /**
   * Update chart summary statistics
   */
  updateChartStats() {
    const { labels, data } = this.getRevenueData();

    // Find highest day/period
    const maxIndex = data.indexOf(Math.max(...data));
    const highestValue = data[maxIndex];
    const highestLabel = labels[maxIndex];

    // Calculate average
    const total = data.reduce((sum, val) => sum + val, 0);
    const avg = data.length > 0 ? total / data.length : 0;

    // Update DOM
    const highestEl = document.getElementById('stat-highest-day');
    const avgEl = document.getElementById('stat-avg-revenue');
    const totalEl = document.getElementById('stat-period-total');
    const periodLabel = document.getElementById('chart-period-label');

    if (highestEl) {
      highestEl.textContent = highestValue > 0 ? `KSh ${highestValue.toLocaleString()}` : 'KSh 0';
    }
    if (avgEl) {
      avgEl.textContent = `KSh ${Math.round(avg).toLocaleString()}`;
    }
    if (totalEl) {
      totalEl.textContent = `KSh ${total.toLocaleString()}`;
    }
    if (periodLabel) {
      const periodTexts = {
        'week': 'Last 7 days revenue breakdown',
        'month': 'Last 4 weeks revenue breakdown',
        'year': 'Last 12 months revenue breakdown'
      };
      periodLabel.textContent = periodTexts[this.currentChartPeriod];
    }
  },

  initChart() {
    const ctx = document.getElementById('revenueChart');
    if (!ctx) return;

    // Destroy existing chart if it exists
    if (this.chart) {
        this.chart.destroy();
    }

    // Get actual revenue data
    const { labels, data } = this.getRevenueData();

    // Update summary stats
    this.updateChartStats();

    this.chart = new Chart(ctx, {
      type: 'bar',
      data: {
        labels: labels,
        datasets: [{
          label: 'Revenue',
          data: data,
          backgroundColor: '#000000',
          borderRadius: 6,
          borderSkipped: false,
          barThickness: this.currentChartPeriod === 'week' ? 24 : 32,
          maxBarThickness: 40
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: {
            display: false
          },
          tooltip: {
            backgroundColor: '#000000',
            titleColor: '#ffffff',
            bodyColor: '#ffffff',
            padding: 12,
            cornerRadius: 8,
            displayColors: false,
            titleFont: {
              size: 12,
              weight: '600'
            },
            bodyFont: {
              size: 14,
              weight: 'bold'
            },
            callbacks: {
                label: function(context) {
                    return `KSh ${context.parsed.y.toLocaleString()}`;
                }
            }
          }
        },
        scales: {
          y: {
            beginAtZero: true,
            grid: {
              color: '#f3f4f6',
              drawBorder: false
            },
            ticks: {
                color: '#9ca3af',
                font: {
                    family: 'Inter',
                    size: 11
                },
                callback: function(value) {
                    if (value >= 1000000) return 'KSh ' + (value / 1000000).toFixed(1) + 'M';
                    if (value >= 1000) return 'KSh ' + (value / 1000) + 'k';
                    return 'KSh ' + value;
                }
            }
          },
          x: {
            grid: {
              display: false
            },
            ticks: {
                color: '#9ca3af',
                font: {
                    family: 'Inter',
                    size: 11
                }
            }
          }
        }
      }
    });
  },
  
  updateChart() {
      // Reinitialize chart with updated data
      this.initChart();
  },

  renderRecentSales() {
    const tbody = document.getElementById('dashboard-recent-sales-table');
    if (!tbody) return;

    // Combine sales and service transactions
    const salesTransactions = Store.sales.map(s => ({
      ...s,
      transactionType: 'sale'
    }));

    const serviceTransactions = Store.serviceTransactions
      .filter(t => t.stock_metres_used > 0) // Only printing jobs
      .map(t => ({
        ...t,
        transactionType: 'printing',
        product_name: t.service_name
      }));

    const allTransactions = [...salesTransactions, ...serviceTransactions]
      .sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp))
      .slice(0, 5);

    if (allTransactions.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="4" class="px-6 py-8 text-center text-gray-400 italic">
            No transactions yet
          </td>
        </tr>
      `;
      return;
    }

    tbody.innerHTML = allTransactions.map(transaction => {
        const productName = transaction.product_name || 'Unknown';
        const date = new Date(transaction.timestamp).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute:'2-digit' });
        const typeLabel = transaction.transactionType === 'printing' ? 'Printing' : 'Sale';
        const statusClass = transaction.is_debt === 1 ? 'status-badge--warning' : 'status-badge--success';
        const statusLabel = transaction.is_debt === 1 ? 'Debt' : 'Completed';

        return `
        <tr class="hover:bg-gray-50 transition-colors">
            <td class="font-medium text-gray-900">
              ${productName}
              <span class="text-xs text-gray-400 ml-1">(${typeLabel})</span>
            </td>
            <td class="text-gray-500">${date}</td>
            <td class="font-medium text-gray-900">KSh ${transaction.amount.toLocaleString()}</td>
            <td><span class="status-badge ${statusClass}">${statusLabel}</span></td>
        </tr>
      `;
    }).join('');
  },

  renderActivityFeed() {
      const container = document.getElementById('dashboard-activity-feed');
      if (!container) return;

      // Combine sales and debts for activity
      const salesActivity = Store.sales.map(s => ({
          type: 'sale',
          date: new Date(s.timestamp),
          data: s
      }));
      
      const debtsActivity = Store.debts.map(d => ({
          type: 'debt',
          date: new Date(d.created_at),
          data: d
      }));

      const allActivity = [...salesActivity, ...debtsActivity]
          .sort((a, b) => b.date - a.date)
          .slice(0, 5);

      if (allActivity.length === 0) {
          container.innerHTML = '<p class="text-center text-gray-400 py-4 italic">No recent activity</p>';
          return;
      }

      container.innerHTML = allActivity.map((item, index) => {
          const isSale = item.type === 'sale';
          const time = item.date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
          let content = '';

          if (isSale) {
              const product = Store.products.find(p => p.id === item.data.product_id);
              const productName = item.data.product_name || (product ? product.name : 'Item');
              content = `
                <p class="text-sm text-gray-900"><span class="font-semibold">New Sale:</span> ${productName}</p>
                <p class="text-xs text-gray-500 mt-0.5">Sale of KSh ${item.data.amount.toLocaleString()} • ${time}</p>
              `;
          } else {
              content = `
                <p class="text-sm text-gray-900"><span class="font-semibold">Debt Added:</span> ${item.data.customer_name}</p>
                <p class="text-xs text-gray-500 mt-0.5">Amount: KSh ${item.data.amount.toLocaleString()} • ${time}</p>
              `;
          }

          return `
            <div class="activity-item">
                <div class="activity-line"></div>
                <div class="activity-dot ${!isSale ? 'warning' : 'active'}"></div>
                ${content}
            </div>
          `;
      }).join('');
  },

  renderTopProducts() {
      const container = document.getElementById('dashboard-top-products');
      if (!container) return;

      // Calculate top products
      const productSales = {};
      Store.sales.forEach(sale => {
          // Only count product sales, not stock sales
          if (sale.product_id && sale.type === 'product') {
              if (!productSales[sale.product_id]) productSales[sale.product_id] = 0;
              // Parse quantity - it might be a string like "2" or a number
              const qty = typeof sale.quantity === 'string' ? parseFloat(sale.quantity) || 1 : (sale.quantity || 1);
              productSales[sale.product_id] += qty;
          }
      });

      const sortedProducts = Object.entries(productSales)
          .sort(([, a], [, b]) => b - a)
          .slice(0, 3);
          
      if (sortedProducts.length === 0) {
           container.innerHTML = '<p class="text-center text-gray-400 py-4 italic">No sales data available</p>';
           return;
      }

      const maxSales = sortedProducts[0][1];

      container.innerHTML = sortedProducts.map(([id, quantity]) => {
          const product = Store.products.find(p => p.id === parseInt(id));
          if (!product) return '';
          
          const percentage = (quantity / maxSales) * 100;
          
          return `
            <div>
                <div class="flex justify-between text-sm mb-1">
                    <span class="font-medium text-gray-900">${product.name}</span>
                    <span class="text-gray-500">${quantity} sold</span>
                </div>
                <div class="w-full bg-gray-100 rounded-full h-1.5">
                    <div class="bg-black h-1.5 rounded-full" style="width: ${percentage}%"></div>
                </div>
            </div>
          `;
      }).join('');
  }
};

// Make DashboardPage available globally
window.DashboardPage = DashboardPage;
