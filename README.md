# MULTIPRINTS

A modern offline desktop inventory and sales management application built with **Electron**. MULTIPRINTS provides a comprehensive solution for printing businesses to manage products, sticker stock, printing services, sales, and customer debts.

## Features

### 📊 Dashboard
- **Analytics**: Real-time overview of total revenue, sales count, material usage, and outstanding debts.
- **Charts**: Interactive charts (Week/Month/Year) filtering real-time revenue vs. sales data.
- **Activity Feed**: Live feed of recent sales and debt recordings.

### 📦 Product Management
- **Inventory**: Track "Life Saver", "Chevron", and "Stripes" products.
- **Variants**: Support for different colors (e.g., White & Red, Yellow & Red) and sizes (1x1, 1x2).
- **Stock Tracking**: Monitor individual product stock levels.

### 🧵 Stock & Material Management
- **Sticker Rolls**: Manage inventory of sticker rolls (Colored, Reflective, Clear) with length tracking.
- **Printing Materials**: Track printing media (Banner Vinyl, Satin, One-Way Vision, etc.) by width, rolls, and remaining metres.
- **Usage Tracking**: Automatically deduct metres used from stock when sales or printing jobs are recorded.

### 🖨️ Printing Services
- **Job Recording**: Record printing jobs with custom dimensions and material usage.
- **Material Deductions**: Automatically calculates and updates remaining roll length.
- **Cost Calculation**: Track service earnings separately from product sales.

### 💰 Sales & Finance
- **Point of Sale**: Unified interface for selling pre-made products, sticker metres, or recording printing jobs.
- **Payment Methods**: Support for Cash, M-Pesa, and Till Number payments.
- **Debt Management**: Convert unpaid sales/jobs to debts, track partial payments, and manage due dates.
- **Receipts**: (Planned/Implied feature set foundation)

### 🔒 Security & Architecture
- **Offline First**: All data stored locally in a secure SQLite database.
- **Role-Based Access**: separate views/permissions for Admin and Employee roles (e.g., hidden stats for employees).
- **Secure IPC**: Communication between renderer and main process handled via secure context-bridged IPC.

## Technology Stack

- **Framework**: Electron (v39.x)
- **Frontend**: API-free Vanilla JavaScript, HTML5, Custom CSS Components (Tailwind-inspired utility classes).
- **Database**: SQLite (via `better-sqlite3`) with robust schema handling.
- **Build Tool**: Electron Builder (for cross-platform installers).

## Installation

### Download
Check the [Releases](https://github.com/Spid3rmvn/MULTICAL/releases) page for the latest installer for valid platforms (Windows, Linux, macOS).

### Build from Source

**Prerequisites**: Node.js v18+ and pnpm.

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/Spid3rmvn/MULTICAL.git
    cd MULTICAL/app
    ```

2.  **Install dependencies**:
    ```bash
    pnpm install
    ```

3.  **Run in Development Mode**:
    ```bash
    pnpm dev
    ```

4.  **Build Production Installer**:
    ```bash
    pnpm build:linux  # Ubuntu/Debian .deb package
    pnpm build:win    # Windows .exe installer
    ```

## Project Structure

```
MULTIPRINTS/
└── app/
    ├── main/                 # Main Process (Electron)
    │   ├── handlers/         # IPC Handlers (DB access)
    │   └── preload.js        # Context Bridge
    ├── renderer/             # Renderer Process (UI)
    │   ├── assets/
    │   │   ├── js/
    │   │   │   ├── pages/    # Page-specific logic (dashboard.js, sales.js, etc.)
    │   │   │   ├── store.js  # Centralized Data Store
    │   │   │   └── app.js    # Router & App Controller
    │   └── pages/            # HTML Views
    └── database.js           # SQLite Schema & Migration Logic
```

## Database Schema Highlights

- **`products`**: pre-made items.
- **`stock`**: sticker rolls data.
- **`printing_materials`**: banner/vinyl rolls data.
- **`sales`**: record of product/stock sales.
- **`service_transactions`**: record of printing jobs.
- **`debts`**: customer outstanding balances.

## License

MIT License - Copyright © 2026 Godwin Mayodi (codegoddy@gmail.com)
