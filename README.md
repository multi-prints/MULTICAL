# MULTIPRINTS

A modern offline desktop inventory and sales management application built with **Tauri + Rust + Leptos**. MULTIPRINTS provides a comprehensive solution for printing businesses to manage products, sticker stock, printing services, sales, and customer debts.

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
- **Desktop Notifications**: Admin reminders for overdue debts.

### 🔒 Security & Architecture
- **Offline First**: All data stored locally in a secure SQLite database.
- **Role-Based Access**: Separate views and permissions for Admin and Employee roles.
- **Rust Backend Commands**: Frontend communicates with the Tauri backend through typed Rust commands.

## Technology Stack

- **Desktop Framework**: Tauri 2
- **Frontend**: Leptos (Rust/WASM)
- **Backend**: Rust
- **Database**: SQLite via `rusqlite`
- **Bundling**: Tauri Bundler

## Development

### Prerequisites
- Rust toolchain
- `trunk`
- Tauri system dependencies for your OS

### Run in development
```bash
cargo tauri dev
```

### Build production bundles
```bash
cargo tauri build
```

## Project Structure

```text
MULTICAL/
├── frontend/        # Leptos frontend (WASM)
├── src-tauri/       # Tauri desktop shell + Rust backend
├── src/             # Generated Trunk build output
└── docs/            # Notes and plans
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
