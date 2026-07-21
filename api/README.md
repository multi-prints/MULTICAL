# MULTIPRINTS API (Cloudflare Workers + Turso)

Free edge backend. **Database stays on Turso.** Desktop apps will call this API instead of dual-opening the local Turso replica (fixes multi-PC / Windows sync pain).

```
[ Desktop / till ]  --HTTPS-->  [ Cloudflare Worker ]  --libsql HTTP-->  [ Turso ]
```

## Prerequisites

1. Cloudflare account — https://dash.cloudflare.com/sign-up  
2. Node 18+  
3. Your Turso URL + token (same as the desktop app)

## Setup

```bash
cd api
npm install
npx wrangler login
```

### Secrets (never commit)

```bash
# Same values as turso.json / GitHub Actions secrets
npx wrangler secret put TURSO_DATABASE_URL
# paste: libsql://your-db.turso.io

npx wrangler secret put TURSO_AUTH_TOKEN
# paste: eyJ...

# Shared key every shop PC will send as Bearer token
npx wrangler secret put API_SECRET
# paste something long & random, e.g. openssl rand -hex 32
```

### Local dev

```bash
# Create api/.dev.vars (gitignored pattern via .env style — use wrangler local secrets file)
cat > .dev.vars << 'EOF'
TURSO_DATABASE_URL=libsql://...
TURSO_AUTH_TOKEN=eyJ...
API_SECRET=dev-secret-change-me
EOF

npm run dev
# → http://127.0.0.1:8787
```

### Deploy (free Workers tier)

```bash
npm run deploy
```

Note the URL, e.g. `https://multiprints-api.<your-subdomain>.workers.dev`

## Auth

All `/v1/*` routes require:

```http
Authorization: Bearer <API_SECRET>
```

## Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/` | Service info |
| GET | `/health` | Worker + Turso connectivity (no auth) |
| GET | `/v1/products` | List products (`?page=&per_page=`) |
| POST | `/v1/products` | Add / upsert product |
| PATCH | `/v1/products/:id` | Update fields |
| POST | `/v1/products/:id/adjust-stock` | Relative stock delta |
| DELETE | `/v1/products/:id` | Delete |
| GET | `/v1/stock` | Sticker stock page |
| POST | `/v1/stock` | Add / merge rolls |
| POST | `/v1/stock/:id/add-rolls` | Add rolls |
| DELETE | `/v1/stock/:id` | Delete |
| GET | `/v1/materials` | Print materials |
| POST | `/v1/materials` | Add material (metres/roll required) |
| POST | `/v1/materials/:id/add-rolls` | Add rolls |
| DELETE | `/v1/materials/:id` | Delete |
| GET | `/v1/sales` | Sales list |
| POST | `/v1/sales` | Record sale (+ deduct stock) |
| DELETE | `/v1/sales/:id` | Delete |
| GET | `/v1/printing/jobs` | Printing jobs + metrics |
| POST | `/v1/printing/jobs` | Record job (+ deduct material) |
| DELETE | `/v1/printing/jobs/:id` | Delete |

## Quick test

```bash
export API=https://multiprints-api.YOUR_SUBDOMAIN.workers.dev
export KEY=your-api-secret

curl -s "$API/health" | jq

curl -s -H "Authorization: Bearer $KEY" "$API/v1/products" | jq

curl -s -X POST -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" \
  -d '{"product_type":"life_saver","stock":5,"selling_price":100}' \
  "$API/v1/products" | jq
```

## Desktop integration (shipped)

The Leptos frontend embeds `MULTIPRINTS_API_BASE_URL` + `MULTIPRINTS_API_SECRET` at build time
(see `frontend/build.rs`). When present:

- products / stock / materials / sales / printing jobs prefer **this API**
- on network failure, **local Tauri DB still works** (offline-safe)
- open pages poll ~every **4s** and also refresh immediately after local mutations

### Release CI secrets

Set under GitHub → Settings → Secrets:

| Secret | Example |
|--------|---------|
| `MULTIPRINTS_API_BASE_URL` | `https://multiprints-api.codegoddy.workers.dev` |
| `MULTIPRINTS_API_SECRET` | same value as Worker `API_SECRET` |

### Local trunk / tauri dev

```bash
export MULTIPRINTS_API_BASE_URL=https://multiprints-api.codegoddy.workers.dev
export MULTIPRINTS_API_SECRET=$(cat api/.api_secret)
cargo tauri dev
```

If `api/.api_secret` exists, frontend `build.rs` loads it automatically when env is unset.

## Free tier notes

Cloudflare Workers free tier is more than enough for a few shop tills. Keep secrets in Wrangler secrets only.
