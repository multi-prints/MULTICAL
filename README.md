# MULTIPRINTS

MULTIPRINTS is an offline desktop inventory and sales management application for printing businesses. It is built with Tauri, Rust, and Leptos, and helps manage products, sticker stock, printing services, sales, and customer debts from a local SQLite database.

## Overview

The application is designed for small to medium printing shops that need a fast local-first desktop system for day-to-day operations. It supports role-based access for admins and employees, tracks inventory movement, records sales and service jobs, and manages debt balances and overdue reminders.

## Features

### Dashboard
- Real-time overview of revenue, sales, material usage, and outstanding debts
- Charts for week, month, and year reporting
- Recent activity feed for sales and debt updates

### Product Management
- Manage products such as Life Saver, Chevron, and Stripes
- Support product variants by color and size
- Track available product stock

### Stock and Material Management
- Manage sticker rolls such as colored, reflective, and clear stock
- Track printing materials by width, rolls, and remaining metres
- Automatically deduct material usage from stock during sales and printing jobs

### Printing Services
- Record printing jobs with dimensions, pricing, and material usage
- Track service income separately from product sales
- Update material balances automatically after each job

### Sales and Finance
- Record product sales, sticker sales, and printing jobs
- Support cash, M-Pesa, and till payments
- Convert unpaid transactions into debts
- Track partial payments, outstanding balances, and due dates
- Show overdue debt notifications for admins

### Security and Access Control
- Offline-first local storage using SQLite
- Role-based access for admin and employee users
- Typed Rust commands between the frontend and Tauri backend

## Technology Stack

- Desktop framework: Tauri 2
- Frontend: Leptos with Rust/WASM
- Backend: Rust
- Database: SQLite via `rusqlite`
- Frontend build tool: Trunk
- Desktop bundling: Tauri Bundler

## Development

### Prerequisites
- Rust toolchain
- `trunk`
- Tauri CLI
- Tauri system dependencies for your operating system

### Install tooling
```bash
cargo install trunk
cargo install tauri-cli --version '^2'
```

### Clone the repository
```bash
git clone git@github.com:multi-prints/MULTICAL.git
cd MULTICAL
```

### Turso configuration (automatic)

The app enables shared multi-PC mode when it finds Turso credentials. **You should not need to do this by hand on every install.**

Priority order:

1. **Release builds (recommended)** — credentials are compiled into the binary from GitHub Secrets during CI:
   - `TURSO_DATABASE_URL`
   - `TURSO_AUTH_TOKEN`  
   Set once under **GitHub → Settings → Secrets and variables → Actions**.  
   On first launch the app writes them to app data as `turso.json` (mode `600` on Linux).

2. **Environment variables** (dev / override)
```bash
export TURSO_DATABASE_URL="libsql://your-database-name.region.turso.io"
export TURSO_AUTH_TOKEN="your-turso-auth-token"
```

3. **App data / config files** (optional manual override)
   - Linux: `~/.local/share/com.multiprints.desktop/turso.json` or `~/.config/com.multiprints.desktop/turso.json`
   - Linux system-wide: `/etc/multiprints/turso.json`
   - Windows: `%APPDATA%\com.multiprints.desktop\turso.json` or `%ProgramData%\multiprints\turso.json`
   - Copy from `src-tauri/turso.example.json` and fill in URL + token

When Turso is configured, the app uses a synced local replica per PC and syncs with the shared Turso database. Local-only `multiprints.db` data is imported into the replica if the shared DB is empty.

### Multi-PC live updates
With Turso enabled on every PC:
- Each open page auto-refreshes about every **12 seconds** (stock, products, sales, printing, debts, dashboard)
- Writes push to Turso immediately; reads use the local replica and pull from Turso at most every ~8s (plus background replica sync)
- Stock and product quantity changes use **relative atomic updates** so concurrent tills do not overwrite each other
- Sales and printing jobs refuse insufficient stock/material instead of going negative

### Multi-PC conflict prevention
- **Natural keys** on products (`type|color|size`), stock (`color|size|type`), and printing materials so the same item cannot be created twice
- **Upserts** on add: concurrent “add stock/product” on different PCs merges quantities instead of duplicating rows
- **Distributed IDs** for new sales, debts, payments, services, and other inserts (not plain AUTOINCREMENT), so two PCs creating rows at once do not clash on primary keys
- On upgrade, existing duplicate product/stock/material rows are merged automatically

### Run in development
```bash
cargo tauri dev
```

### Build production bundles
```bash
cargo tauri build
```

### Updates
Releases are installed from GitHub Releases through the in-app update button.

## Project Structure

```text
MULTICAL/
├── frontend/        # Leptos frontend source
├── src-tauri/       # Tauri app shell and Rust backend
├── src/             # Generated Trunk build output
├── docs/            # Project notes and plans
└── Cargo.toml       # Workspace manifest
```

## Database Schema Highlights

- `products`: Pre-made product records
- `stock`: Sticker stock and roll data
- `printing_materials`: Banner, vinyl, satin, and related materials
- `sales`: Product and stock sales records
- `service_transactions`: Printing service transactions
- `debts`: Customer debt balances and status

## Uninstall

Uninstalling the app removes both the binary and all app data (database, config files).

### Windows
Run the bundled uninstaller (`uninstall.exe`) from the installation directory, or use **Settings > Apps > MULTIPRINTS > Uninstall**. The uninstaller automatically removes app data from `%APPDATA%\com.multiprints.desktop` and `%LOCALAPPDATA%\com.multiprints.desktop`.

### Linux
```bash
sudo dpkg -r multiprints
# or
sudo apt remove multiprints
```
The post-removal script automatically cleans up `~/.local/share/com.multiprints.desktop`, `~/.config/com.multiprints.desktop`, and `~/.cache/com.multiprints.desktop`.

## License

MIT License. Copyright © 2026 Godwin Mayodi (codegoddy@gmail.com)
