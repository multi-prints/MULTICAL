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
git clone git@github.com:codegoddy/MULTICAL.git
cd MULTICAL
```

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

## License

MIT License. Copyright © 2026 Godwin Mayodi (codegoddy@gmail.com)
