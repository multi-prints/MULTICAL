use libsql::Builder;
use rand::Rng;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::models::*;

/// Compile-time Turso credentials from build.rs (CI secrets → every installed PC).
mod embedded_turso {
    include!(concat!(env!("OUT_DIR"), "/embedded_turso.rs"));
}

const TURSO_CONFIG_FILE: &str = "turso.json";
const SYNC_DB_FILE: &str = "multiprints-sync.db";
/// Background libSQL replica interval (pull/push without blocking UI reads).
const SYNC_INTERVAL_SECS: u64 = 10;
/// Minimum gap between forced network syncs on ordinary reads.
/// Prevents every Tauri invoke from blocking on Turso (was the main lag source).
const MIN_READ_SYNC_GAP: Duration = Duration::from_secs(8);

/// Collision-resistant positive i64 for multi-PC inserts (avoids AUTOINCREMENT races).
///
/// Must stay within JavaScript `Number.MAX_SAFE_INTEGER` (2^53 - 1) because the
/// Tauri webview IPC still materializes JSON numbers as IEEE-754 doubles.
/// Layout: 42 bits UTC ms + 10 bits random (52 bits total, always safe).
fn new_distributed_id() -> i64 {
    let ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let ms_bits = ms & ((1u64 << 42) - 1);
    let rand_bits = (rand::thread_rng().gen::<u32>() as u64) & ((1u64 << 10) - 1);
    let id = (ms_bits << 10) | rand_bits;
    (id as i64).max(1)
}

fn product_natural_key(
    product_type: &str,
    color: &Option<String>,
    size: &Option<String>,
) -> String {
    format!(
        "{}|{}|{}",
        product_type.trim().to_lowercase(),
        color.as_deref().unwrap_or("").trim().to_lowercase(),
        size.as_deref().unwrap_or("").trim().to_lowercase()
    )
}

fn stock_natural_key(color: &str, size: &str, sticker_type: &str) -> String {
    format!(
        "{}|{}|{}",
        color.trim().to_lowercase(),
        size.trim(),
        sticker_type.trim().to_lowercase()
    )
}

fn material_natural_key(
    name: &str,
    material_type: &str,
    width: f64,
    color: &Option<String>,
) -> String {
    format!(
        "{}|{}|{:.4}|{}",
        name.trim().to_lowercase(),
        material_type.trim().to_lowercase(),
        width,
        color.as_deref().unwrap_or("").trim().to_lowercase()
    )
}

#[derive(Debug, Clone, Deserialize)]
struct TursoConfig {
    database_url: String,
    auth_token: String,
}

pub struct Database {
    pub conn: Mutex<Connection>,
    sync_db: Option<Arc<libsql::Database>>,
    db_path: PathBuf,
    /// Last successful Turso sync — used to throttle read-path network syncs.
    last_sync_at: Mutex<Option<Instant>>,
    /// Where credentials came from (environment / compile-time / path / none).
    turso_source: String,
    /// If credentials existed but the libsql engine failed to open.
    turso_engine_error: Option<String>,
    /// Set after a background libsql sync so the next read reopens rusqlite
    /// (libsql rewrites the replica file; Windows especially keeps a stale handle).
    needs_reopen: Arc<AtomicBool>,
}

fn infer_debt_payment_method(
    conn: &Connection,
    sale_id: Option<i64>,
    service_transaction_id: Option<i64>,
) -> String {
    if let Some(sid) = sale_id {
        if let Ok(method) = conn.query_row(
            "SELECT payment_method FROM sales WHERE id = ?1",
            params![sid],
            |row| row.get::<_, String>(0),
        ) {
            return method;
        }
    }

    if let Some(tid) = service_transaction_id {
        if let Ok(method) = conn.query_row(
            "SELECT payment_method FROM service_transactions WHERE id = ?1",
            params![tid],
            |row| row.get::<_, String>(0),
        ) {
            return method;
        }
    }

    "cash".to_string()
}

impl Database {
    /// Open (or create) the database at the given path
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let data_dir = db_path
            .parent()
            .ok_or_else(|| "Invalid database path".to_string())?;
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;

        // Load .env from app data if present (installed builds don't use the repo cwd).
        let _ = dotenvy::from_path(data_dir.join(".env"));

        let (turso_config, turso_source) = Self::load_turso_config(data_dir);
        if let Some(ref config) = turso_config {
            // Always refresh app-data turso.json from known-good credentials so every
            // PC keeps a working file even after reinstall / profile wipe.
            Self::persist_turso_config(data_dir, config, &turso_source);
        }

        // Prefer Turso replica path when credentials exist, but never hard-fail
        // startup if the embedded replica cannot be opened (common on Windows with
        // file locks, corrupt replicas, or TLS/runtime issues). Fall back carefully
        // so we do not abandon an existing replica that still has business data.
        let sync_path = data_dir.join(SYNC_DB_FILE);
        let mut active_db_path = if turso_config.is_some() {
            sync_path.clone()
        } else {
            db_path.clone()
        };

        // Build the embedded replica connection, but do NOT block startup on a full
        // network sync here. try_initial_sync / spawn_background_sync handle that.
        let mut sync_db = None;
        let mut turso_active = false;
        let mut turso_engine_error = None;
        if let Some(config) = turso_config.clone() {
            let path_for_build = active_db_path.clone();
            // libsql is happier with a stable path string on Windows.
            let path_str = path_for_build.to_string_lossy().replace('\\', "/");
            match tauri::async_runtime::block_on(async move {
                Builder::new_synced_database(
                    path_str.as_str(),
                    config.database_url,
                    config.auth_token,
                )
                .read_your_writes(true)
                .remote_writes(false)
                .sync_interval(Duration::from_secs(SYNC_INTERVAL_SECS))
                .build()
                .await
                .map_err(|e| e.to_string())
            }) {
                Ok(db) => {
                    sync_db = Some(Arc::new(db));
                    turso_active = true;
                }
                Err(e) => {
                    let msg = format!("Turso replica init failed (source={}): {}", turso_source, e);
                    eprintln!("{msg}");
                    turso_engine_error = Some(msg);
                    // Keep multiprints-sync.db if it already exists (has last-known multi-PC data).
                    // Only fall all the way back to multiprints.db when there is no replica yet.
                    if !sync_path.exists() {
                        active_db_path = db_path.clone();
                    }
                }
            }
        }

        // Pull remote rows BEFORE opening the rusqlite handle so the first open
        // sees Turso data (avoids stale empty snapshot, especially on Windows).
        if let Some(ref db) = sync_db {
            let db = db.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            tauri::async_runtime::spawn(async move {
                let r = db.sync().await.map(|_| ()).map_err(|e| e.to_string());
                let _ = tx.send(r);
            });
            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(Ok(())) => println!("Pre-open Turso sync OK ({})", turso_source),
                Ok(Err(e)) => eprintln!("Pre-open Turso sync failed ({}): {}", turso_source, e),
                Err(_) => eprintln!(
                    "Pre-open Turso sync still running after 30s ({}) — continuing open",
                    turso_source
                ),
            }
        }

        let conn = Connection::open(&active_db_path).map_err(|e| {
            format!(
                "Failed to open database at {}: {}",
                active_db_path.display(),
                e
            )
        })?;
        // PRAGMAs are best-effort — WAL can fail on some Windows/network volumes.
        if let Err(e) = conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -20000;
            PRAGMA journal_mode = WAL;
            ",
        ) {
            eprintln!("Database PRAGMA setup warning: {}", e);
            // Ensure foreign keys still on even if WAL failed.
            let _ = conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;");
        }

        let db = Database {
            conn: Mutex::new(conn),
            sync_db,
            db_path: active_db_path.clone(),
            // None until a real sync succeeds — first reads must be allowed to pull.
            last_sync_at: Mutex::new(None),
            turso_source: turso_source.clone(),
            turso_engine_error,
            needs_reopen: Arc::new(AtomicBool::new(false)),
        };

        // Schema work is purely local — never force a Turso round-trip during boot.
        db.create_tables_local()?;
        db.run_migrations_local()?;

        if turso_active
            && db.is_business_data_empty_local()?
            && db_path.exists()
            && db_path != active_db_path
        {
            db.import_legacy_sqlite(&db_path)?;
        }

        if turso_active {
            println!(
                "Database initialized in Turso sync mode at: {:?} (config: {}; awaiting initial sync)",
                active_db_path, turso_source
            );
        } else {
            println!(
                "Database initialized in local mode at: {:?} (turso source: {})",
                active_db_path, turso_source
            );
        }
        Ok(db)
    }

    /// Whether Turso sync is configured for this process.
    pub fn has_turso(&self) -> bool {
        self.sync_db.is_some()
    }

    /// Credential source label (e.g. compile-time, path, environment, none).
    pub fn turso_source(&self) -> &str {
        &self.turso_source
    }

    /// True when build.rs baked non-empty Turso credentials into this binary.
    pub fn has_embedded_turso() -> bool {
        embedded_turso::EMBEDDED_TURSO_PRESENT
            && !embedded_turso::EMBEDDED_TURSO_URL.is_empty()
            && !embedded_turso::EMBEDDED_TURSO_TOKEN.is_empty()
    }

    /// Engine open error if credentials existed but libsql failed.
    pub fn turso_engine_error(&self) -> Option<&str> {
        self.turso_engine_error.as_deref()
    }

    /// Path of the active local DB file (replica or plain sqlite).
    pub fn active_db_path(&self) -> &Path {
        &self.db_path
    }

    /// Local row counts for startup diagnostics (no network).
    pub fn local_row_summary(&self) -> String {
        let Ok(conn) = self.local_conn() else {
            return "rows: <lock error>".into();
        };
        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap_or(-1)
        };
        format!(
            "products={} stock={} sales={} debts={} users={} services={} jobs={}",
            count("products"),
            count("stock"),
            count("sales"),
            count("debts"),
            count("users"),
            count("services"),
            count("service_transactions"),
        )
    }

    /// Best-effort first pull so multi-PC data is present before default users / UI load.
    /// Returns Ok(true) if a sync completed, Ok(false) if Turso is not configured / still running.
    pub fn try_initial_sync(&self, timeout: Duration) -> Result<bool, String> {
        let Some(sync_db) = self.sync_db.clone() else {
            return Ok(false);
        };

        let (tx, rx) = std::sync::mpsc::channel();
        let sync_for_task = sync_db.clone();
        tauri::async_runtime::spawn(async move {
            let result = sync_for_task
                .sync()
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
            let _ = tx.send(result);
        });

        let result = match rx.recv_timeout(timeout) {
            Ok(Ok(())) => true,
            Ok(Err(e)) => {
                eprintln!("Initial Turso sync failed: {}", e);
                return Err(e);
            }
            Err(_) => {
                // Sync still running in background — do not treat as hard failure.
                eprintln!(
                    "Initial Turso sync still running after {:?} — will keep retrying in background",
                    timeout
                );
                false
            }
        };

        if result {
            if let Ok(mut last) = self.last_sync_at.lock() {
                *last = Some(Instant::now());
            }
            // libsql and rusqlite share the file — reopen so we don't keep a stale snapshot.
            self.reopen_local_connection()?;
            println!("Initial Turso sync completed within {:?}", timeout);
        }
        Ok(result)
    }

    /// Force WAL checkpoint so readers on this connection see the latest pages.
    fn checkpoint_wal(&self) {
        if let Ok(conn) = self.local_conn() {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
        }
    }

    /// Re-open the rusqlite handle after libsql rewrites the replica file.
    fn reopen_local_connection(&self) -> Result<(), String> {
        let mut guard = self.local_conn()?;
        // Drop the old connection first (end of replace), then open fresh.
        let new_conn = Connection::open(&self.db_path).map_err(|e| {
            format!(
                "Failed to reopen database at {}: {}",
                self.db_path.display(),
                e
            )
        })?;
        let _ = new_conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -20000;
            PRAGMA journal_mode = WAL;
            ",
        );
        *guard = new_conn;
        let _ = guard.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
        Ok(())
    }

    /// Continuous Turso pull/push so multi-PC changes appear without restarting.
    ///
    /// libsql may also run its own `sync_interval`, but that does not reopen our
    /// rusqlite handle — we flag `needs_reopen` after each successful sync so the
    /// next page load sees new rows (critical on Windows).
    pub fn spawn_background_sync(&self) {
        let Some(sync_db) = self.sync_db.clone() else {
            return;
        };
        let needs_reopen = Arc::clone(&self.needs_reopen);
        // Dedicated thread: avoids needing a direct tokio dependency and keeps
        // the UI runtime free of long-lived sleep tasks.
        std::thread::Builder::new()
            .name("multiprints-turso-sync".into())
            .spawn(move || {
                let run_sync = |label: &str| match tauri::async_runtime::block_on(async {
                    sync_db.sync().await
                }) {
                    Ok(_) => {
                        needs_reopen.store(true, Ordering::SeqCst);
                        println!("{label} Turso sync completed");
                    }
                    Err(e) => {
                        eprintln!("{label} Turso sync failed (will retry): {e}");
                    }
                };

                // One immediate pass after startup (initial sync may have timed out).
                run_sync("Background");

                loop {
                    std::thread::sleep(Duration::from_secs(SYNC_INTERVAL_SECS));
                    run_sync("Periodic");
                }
            })
            .ok();
    }

    /// If a background sync rewrote the replica, reopen rusqlite before reading.
    fn apply_pending_reopen(&self) {
        if !self.needs_reopen.swap(false, Ordering::SeqCst) {
            return;
        }
        if let Err(e) = self.reopen_local_connection() {
            eprintln!("Reopen after background sync failed: {}", e);
            // Retry on the next read.
            self.needs_reopen.store(true, Ordering::SeqCst);
            self.checkpoint_wal();
        }
    }

    /// Resolve Turso credentials automatically, in priority order:
    /// 1. Process env (dev override)
    /// 2. **Compile-time embed** from CI secrets (every installed PC — primary path)
    /// 3. App data `turso.json` / `.env`
    /// 4. User config / system-wide paths
    fn load_turso_config(data_dir: &Path) -> (Option<TursoConfig>, String) {
        if let Some(cfg) = Self::turso_from_env() {
            return (Some(cfg), "environment".into());
        }

        // Baked into release binaries so multi-PC works with zero setup.
        if let Some(cfg) = Self::turso_from_compile_time() {
            return (Some(cfg), "compile-time (embedded in app)".into());
        }

        for path in [data_dir.join(TURSO_CONFIG_FILE), data_dir.join(".env")] {
            if let Some(cfg) = Self::turso_from_path(&path) {
                return (Some(cfg), path.display().to_string());
            }
        }

        for path in Self::turso_search_paths() {
            if path.starts_with(data_dir) {
                continue; // already checked
            }
            if let Some(cfg) = Self::turso_from_path(&path) {
                return (Some(cfg), path.display().to_string());
            }
        }

        (None, "none".into())
    }

    fn turso_from_env() -> Option<TursoConfig> {
        let database_url = std::env::var("TURSO_DATABASE_URL")
            .or_else(|_| std::env::var("MULTIPRINTS_TURSO_DATABASE_URL"))
            .ok()?;
        let auth_token = std::env::var("TURSO_AUTH_TOKEN")
            .or_else(|_| std::env::var("MULTIPRINTS_TURSO_AUTH_TOKEN"))
            .ok()?;
        Self::turso_if_valid(database_url, auth_token)
    }

    fn turso_from_compile_time() -> Option<TursoConfig> {
        // Generated by build.rs as real Rust string constants (not fragile include_str txt).
        if !embedded_turso::EMBEDDED_TURSO_PRESENT {
            return None;
        }
        Self::turso_if_valid(
            embedded_turso::EMBEDDED_TURSO_URL.to_string(),
            embedded_turso::EMBEDDED_TURSO_TOKEN.to_string(),
        )
    }

    fn turso_if_valid(database_url: String, auth_token: String) -> Option<TursoConfig> {
        if database_url.trim().is_empty() || auth_token.trim().is_empty() {
            return None;
        }
        // Ignore placeholder example values
        if database_url.contains("your-database-name") || auth_token.contains("your-turso-auth") {
            return None;
        }
        Some(TursoConfig {
            database_url: database_url.trim().to_string(),
            auth_token: auth_token.trim().to_string(),
        })
    }

    fn turso_from_path(path: &Path) -> Option<TursoConfig> {
        let raw = std::fs::read_to_string(path).ok()?;
        let name = path.file_name()?.to_string_lossy();
        if name == ".env" {
            return Self::turso_from_dotenv_contents(&raw);
        }
        let parsed: TursoConfig = serde_json::from_str(&raw).ok()?;
        Self::turso_if_valid(parsed.database_url, parsed.auth_token)
    }

    fn turso_from_dotenv_contents(raw: &str) -> Option<TursoConfig> {
        let mut url = None;
        let mut token = None;
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
            match k.trim() {
                "TURSO_DATABASE_URL" | "MULTIPRINTS_TURSO_DATABASE_URL" => url = Some(v),
                "TURSO_AUTH_TOKEN" | "MULTIPRINTS_TURSO_AUTH_TOKEN" => token = Some(v),
                _ => {}
            }
        }
        Self::turso_if_valid(url?, token?)
    }

    /// Extra locations checked automatically so installs work without a manual step.
    fn turso_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // User config (Linux XDG)
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let base = PathBuf::from(xdg).join("com.multiprints.desktop");
            paths.push(base.join(TURSO_CONFIG_FILE));
            paths.push(base.join(".env"));
        } else if let Ok(home) = std::env::var("HOME") {
            let base = PathBuf::from(home).join(".config/com.multiprints.desktop");
            paths.push(base.join(TURSO_CONFIG_FILE));
            paths.push(base.join(".env"));
        }

        // Windows user config
        if let Ok(appdata) = std::env::var("APPDATA") {
            let base = PathBuf::from(appdata).join("com.multiprints.desktop");
            paths.push(base.join(TURSO_CONFIG_FILE));
            paths.push(base.join(".env"));
        }
        // Windows machine-wide (admin drop once for all users)
        if let Ok(program_data) = std::env::var("ProgramData") {
            paths.push(
                PathBuf::from(program_data)
                    .join("multiprints")
                    .join(TURSO_CONFIG_FILE),
            );
        }

        // Linux machine-wide (admin drop once for all users)
        paths.push(PathBuf::from("/etc/multiprints").join(TURSO_CONFIG_FILE));
        paths.push(PathBuf::from("/etc/multiprints").join(".env"));

        paths
    }

    /// Write credentials into app data so subsequent launches don't depend on env/cwd.
    /// Overwrites missing/invalid files. Keeps a valid existing file if it already works
    /// (so a hand-edited token is not clobbered unless it is broken).
    fn persist_turso_config(data_dir: &Path, config: &TursoConfig, source: &str) {
        let path = data_dir.join(TURSO_CONFIG_FILE);
        if path.exists() {
            if Self::turso_from_path(&path).is_some() {
                return;
            }
            eprintln!("Existing turso.json is invalid — replacing from {}", source);
        }

        let payload = serde_json::json!({
            "database_url": config.database_url,
            "auth_token": config.auth_token,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(body) => {
                if let Err(e) = std::fs::write(&path, format!("{body}\n")) {
                    eprintln!("Could not persist Turso config to {:?}: {}", path, e);
                    return;
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = std::fs::metadata(&path) {
                        let mut perms = meta.permissions();
                        perms.set_mode(0o600);
                        let _ = std::fs::set_permissions(&path, perms);
                    }
                }
                println!("Persisted Turso config to {:?} (from {})", path, source);
            }
            Err(e) => eprintln!("Could not serialize Turso config: {}", e),
        }
    }

    fn sync_now_blocking(&self) -> Result<(), String> {
        if let Some(sync_db) = self.sync_db.clone() {
            tauri::async_runtime::block_on(async move {
                sync_db.sync().await.map(|_| ()).map_err(|e| e.to_string())
            })?;
            if let Ok(mut last) = self.last_sync_at.lock() {
                *last = Some(Instant::now());
            }
            // libsql rewrites the replica — reopen so SELECT sees new rows (esp. Windows).
            if let Err(e) = self.reopen_local_connection() {
                eprintln!("Reopen after sync failed (checkpoint only): {}", e);
                self.checkpoint_wal();
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Pull from Turso when needed. `force` always syncs (writes / explicit).
    ///
    /// Ordinary reads **never** block the UI thread on network I/O. On Windows,
    /// `block_on(sync)` from a Tauri command after the app sat idle was a major
    /// source of “Not Responding”. Background `spawn_background_sync` keeps the
    /// replica fresh; reads only reopen the local handle when needed.
    fn maybe_sync(&self, force: bool) -> Result<(), String> {
        if self.sync_db.is_none() {
            return Ok(());
        }

        if !force {
            return Ok(());
        }

        if let Ok(last) = self.last_sync_at.lock() {
            if let Some(at) = *last {
                if at.elapsed() < MIN_READ_SYNC_GAP {
                    return Ok(());
                }
            }
        }

        self.sync_now_blocking()
    }

    /// Local SQLite only — never touches the network. Used for bootstrapping schema.
    fn local_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|e| e.to_string())
    }

    /// Read path: local SQLite only. Network sync runs on the background thread.
    fn synced_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        // Reopen if background libsql rewrote the replica file (critical on Windows).
        self.apply_pending_reopen();
        self.local_conn()
    }

    /// Write path: data is already on disk. Push to Turso in the background so the
    /// UI (and first-launch setup) never blocks on the network. Periodic
    /// background sync covers retries.
    fn finish_write<T>(&self, value: T) -> Result<T, String> {
        if let Some(sync_db) = self.sync_db.clone() {
            // Writes go through rusqlite while sync goes through libsql on the same
            // file. Checkpoint WAL so libsql's push actually sees the new rows.
            // Without this, other PCs only catch up after restart (initial sync).
            self.checkpoint_wal();
            let needs_reopen = Arc::clone(&self.needs_reopen);
            tauri::async_runtime::spawn(async move {
                match sync_db.sync().await {
                    Ok(_) => {
                        needs_reopen.store(true, Ordering::SeqCst);
                    }
                    Err(e) => {
                        eprintln!(
                            "Database change was saved locally, but could not sync to Turso: {}",
                            e
                        );
                    }
                }
            });
        }
        Ok(value)
    }

    fn is_business_data_empty_local(&self) -> Result<bool, String> {
        let conn = self.local_conn()?;
        let total: i64 = conn
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM products) +
                    (SELECT COUNT(*) FROM stock) +
                    (SELECT COUNT(*) FROM sales) +
                    (SELECT COUNT(*) FROM debts) +
                    (SELECT COUNT(*) FROM debt_payments) +
                    (SELECT COUNT(*) FROM services) +
                    (SELECT COUNT(*) FROM service_transactions) +
                    (SELECT COUNT(*) FROM printing_materials) +
                    (SELECT COUNT(*) FROM users)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(total == 0)
    }

    fn is_business_data_empty(&self) -> Result<bool, String> {
        let conn = self.synced_conn()?;
        let total: i64 = conn
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM products) +
                    (SELECT COUNT(*) FROM stock) +
                    (SELECT COUNT(*) FROM sales) +
                    (SELECT COUNT(*) FROM debts) +
                    (SELECT COUNT(*) FROM debt_payments) +
                    (SELECT COUNT(*) FROM services) +
                    (SELECT COUNT(*) FROM service_transactions) +
                    (SELECT COUNT(*) FROM printing_materials) +
                    (SELECT COUNT(*) FROM users)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(total == 0)
    }

    fn legacy_table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM legacy.sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(exists > 0)
    }

    fn import_legacy_sqlite(&self, legacy_path: &Path) -> Result<(), String> {
        if !legacy_path.exists() {
            return Ok(());
        }

        let conn = self.local_conn()?;
        let legacy_path = legacy_path.to_string_lossy().to_string();

        conn.execute("ATTACH DATABASE ?1 AS legacy", params![legacy_path])
            .map_err(|e| e.to_string())?;

        let import_result = (|| -> Result<(), String> {
            conn.execute_batch("BEGIN IMMEDIATE;")
                .map_err(|e| e.to_string())?;

            if Self::legacy_table_exists(&conn, "products")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO products (id, name, product_type, color, size, selling_price, stock, created_at, updated_at)
                     SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM legacy.products;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "stock")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO stock (id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at)
                     SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM legacy.stock;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "services")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO services (id, name, description, price, unit, uses_stock, is_active, created_at, updated_at)
                     SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM legacy.services;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "printing_materials")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO printing_materials (id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at)
                     SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM legacy.printing_materials;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "sales")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO sales (id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp)
                     SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp FROM legacy.sales;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "service_transactions")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO service_transactions (id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp)
                     SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp FROM legacy.service_transactions;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "debts")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO debts (id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at)
                     SELECT id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at FROM legacy.debts;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "debt_payments")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO debt_payments (id, debt_id, amount, payment_method, notes, payment_date)
                     SELECT id, debt_id, amount, payment_method, notes, payment_date FROM legacy.debt_payments;",
                ).map_err(|e| e.to_string())?;
            }
            if Self::legacy_table_exists(&conn, "users")? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO users (id, username, password_hash, role, permissions, created_at, updated_at)
                     SELECT id, username, password_hash, role, permissions, created_at, updated_at FROM legacy.users;",
                ).map_err(|e| e.to_string())?;
            }

            conn.execute_batch(
                "DELETE FROM sqlite_sequence WHERE name IN ('products', 'stock', 'sales', 'debts', 'debt_payments', 'services', 'service_transactions', 'printing_materials', 'users');
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('products', COALESCE((SELECT MAX(id) FROM products), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('stock', COALESCE((SELECT MAX(id) FROM stock), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('sales', COALESCE((SELECT MAX(id) FROM sales), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('debts', COALESCE((SELECT MAX(id) FROM debts), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('debt_payments', COALESCE((SELECT MAX(id) FROM debt_payments), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('services', COALESCE((SELECT MAX(id) FROM services), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('service_transactions', COALESCE((SELECT MAX(id) FROM service_transactions), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('printing_materials', COALESCE((SELECT MAX(id) FROM printing_materials), 0));
                 INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES ('users', COALESCE((SELECT MAX(id) FROM users), 0));
                 COMMIT;",
            ).map_err(|e| e.to_string())?;

            Ok(())
        })();

        let _ = conn.execute_batch("DETACH DATABASE legacy");
        import_result?;

        println!(
            "Imported legacy local database into synced database at {:?}",
            self.db_path
        );
        // Network push is deferred to background sync after startup.
        Ok(())
    }

    // ==================== Table Creation ====================

    fn create_tables_local(&self) -> Result<(), String> {
        let conn = self.local_conn()?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                product_type TEXT NOT NULL,
                color TEXT,
                size TEXT,
                selling_price REAL NOT NULL DEFAULT 0,
                stock INTEGER NOT NULL DEFAULT 0,
                natural_key TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS stock (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                color TEXT NOT NULL,
                size TEXT NOT NULL DEFAULT '1',
                sticker_type TEXT NOT NULL DEFAULT 'colored',
                rolls INTEGER NOT NULL DEFAULT 0,
                metres_per_roll REAL NOT NULL DEFAULT 50,
                total_metres REAL NOT NULL DEFAULT 0,
                metres_used REAL NOT NULL DEFAULT 0,
                natural_key TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS sales (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                product_id INTEGER,
                stock_id INTEGER,
                product_name TEXT,
                product_type TEXT,
                sticker_type TEXT,
                quantity TEXT,
                amount REAL NOT NULL DEFAULT 0,
                payment_method TEXT NOT NULL DEFAULT 'cash',
                customer_name TEXT DEFAULT 'Walk-in',
                is_debt INTEGER DEFAULT 0,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE SET NULL,
                FOREIGN KEY (stock_id) REFERENCES stock(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS debts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                customer_name TEXT NOT NULL,
                phone TEXT,
                amount REAL NOT NULL DEFAULT 0,
                paid_amount REAL NOT NULL DEFAULT 0,
                remaining_amount REAL NOT NULL DEFAULT 0,
                due_date TEXT,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                sale_id INTEGER,
                service_transaction_id INTEGER,
                paid_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS debt_payments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                debt_id INTEGER NOT NULL,
                amount REAL NOT NULL DEFAULT 0,
                payment_method TEXT NOT NULL DEFAULT 'cash',
                notes TEXT,
                payment_date DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (debt_id) REFERENCES debts(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS services (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                price REAL NOT NULL DEFAULT 0,
                unit TEXT,
                uses_stock INTEGER NOT NULL DEFAULT 0,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS service_transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_id INTEGER,
                service_name TEXT NOT NULL,
                quantity REAL NOT NULL DEFAULT 1,
                price REAL NOT NULL DEFAULT 0,
                amount REAL NOT NULL DEFAULT 0,
                payment_method TEXT NOT NULL DEFAULT 'cash',
                customer_name TEXT DEFAULT 'Walk-in',
                notes TEXT,
                stock_id INTEGER,
                stock_metres_used REAL DEFAULT 0,
                material_size TEXT,
                material_type TEXT,
                printing_material_id INTEGER,
                is_debt INTEGER DEFAULT 0,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE SET NULL,
                FOREIGN KEY (stock_id) REFERENCES stock(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS printing_materials (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                material_type TEXT NOT NULL,
                width REAL NOT NULL DEFAULT 1,
                rolls INTEGER NOT NULL DEFAULT 0,
                metres_per_roll REAL NOT NULL DEFAULT 50,
                total_metres REAL NOT NULL DEFAULT 0,
                metres_used REAL NOT NULL DEFAULT 0,
                color TEXT,
                natural_key TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'employee',
                permissions TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_products_created_at ON products(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_products_variant_lookup ON products(product_type, color, size);

            CREATE INDEX IF NOT EXISTS idx_stock_created_at ON stock(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_stock_lookup ON stock(color COLLATE NOCASE, size, sticker_type);

            CREATE INDEX IF NOT EXISTS idx_sales_timestamp ON sales(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_sales_product_id ON sales(product_id);
            CREATE INDEX IF NOT EXISTS idx_sales_stock_id ON sales(stock_id);

            CREATE INDEX IF NOT EXISTS idx_debts_created_at ON debts(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_debts_status_created_at ON debts(status, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_debts_due_date ON debts(due_date);
            CREATE INDEX IF NOT EXISTS idx_debts_sale_id ON debts(sale_id);
            CREATE INDEX IF NOT EXISTS idx_debts_service_transaction_id ON debts(service_transaction_id);

            CREATE INDEX IF NOT EXISTS idx_debt_payments_debt_id_payment_date ON debt_payments(debt_id, payment_date DESC);

            CREATE INDEX IF NOT EXISTS idx_services_active_name ON services(is_active, name);
            CREATE INDEX IF NOT EXISTS idx_services_created_at ON services(created_at DESC);

            CREATE INDEX IF NOT EXISTS idx_service_transactions_timestamp ON service_transactions(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_service_transactions_service_id ON service_transactions(service_id);
            CREATE INDEX IF NOT EXISTS idx_service_transactions_stock_id ON service_transactions(stock_id);

            CREATE INDEX IF NOT EXISTS idx_printing_materials_created_at ON printing_materials(created_at DESC);
            ",
        )
        .map_err(|e| e.to_string())?;

        println!("Database tables created");
        Ok(())
    }

    fn run_migrations_local(&self) -> Result<(), String> {
        let conn = self.local_conn()?;

        // Check and add missing columns (same migration logic as JS version)
        let migrations = [
            (
                "service_transactions",
                "stock_id",
                "ALTER TABLE service_transactions ADD COLUMN stock_id INTEGER",
            ),
            (
                "service_transactions",
                "stock_metres_used",
                "ALTER TABLE service_transactions ADD COLUMN stock_metres_used REAL DEFAULT 0",
            ),
            (
                "service_transactions",
                "material_size",
                "ALTER TABLE service_transactions ADD COLUMN material_size TEXT",
            ),
            (
                "service_transactions",
                "material_type",
                "ALTER TABLE service_transactions ADD COLUMN material_type TEXT",
            ),
            (
                "service_transactions",
                "printing_material_id",
                "ALTER TABLE service_transactions ADD COLUMN printing_material_id INTEGER",
            ),
            (
                "service_transactions",
                "is_debt",
                "ALTER TABLE service_transactions ADD COLUMN is_debt INTEGER DEFAULT 0",
            ),
            (
                "debts",
                "paid_amount",
                "ALTER TABLE debts ADD COLUMN paid_amount REAL NOT NULL DEFAULT 0",
            ),
            (
                "debts",
                "remaining_amount",
                "ALTER TABLE debts ADD COLUMN remaining_amount REAL NOT NULL DEFAULT 0",
            ),
            (
                "debts",
                "sale_id",
                "ALTER TABLE debts ADD COLUMN sale_id INTEGER",
            ),
            (
                "debts",
                "service_transaction_id",
                "ALTER TABLE debts ADD COLUMN service_transaction_id INTEGER",
            ),
            (
                "sales",
                "is_debt",
                "ALTER TABLE sales ADD COLUMN is_debt INTEGER DEFAULT 0",
            ),
        ];

        for (table, column, alter_sql) in &migrations {
            let has_column: bool = conn
                .prepare(&format!("PRAGMA table_info({})", table))
                .map_err(|e| e.to_string())?
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .any(|name| name == *column);

            if !has_column {
                println!("Adding {} column to {}", column, table);
                conn.execute_batch(alter_sql).map_err(|e| e.to_string())?;
            }
        }

        // Update existing debts: remaining_amount = amount - paid_amount
        conn.execute_batch("UPDATE debts SET remaining_amount = amount - COALESCE(paid_amount, 0)")
            .map_err(|e| e.to_string())?;

        // Normalize debts already marked as paid
        conn.execute_batch(
            "UPDATE debts SET paid_amount = amount, remaining_amount = 0, paid_at = COALESCE(paid_at, CURRENT_TIMESTAMP) WHERE status = 'paid'"
        ).map_err(|e| e.to_string())?;

        // Ensure paid debts have a settlement payment entry for any unpaid remainder
        let mut stmt = conn
            .prepare(
                "SELECT d.id, d.amount, d.paid_at, d.sale_id, d.service_transaction_id, COALESCE(SUM(dp.amount), 0)
                 FROM debts d
                 LEFT JOIN debt_payments dp ON dp.debt_id = d.id
                 WHERE d.status = 'paid'
                 GROUP BY d.id, d.amount, d.paid_at, d.sale_id, d.service_transaction_id
                 HAVING COALESCE(SUM(dp.amount), 0) < d.amount"
            )
            .map_err(|e| e.to_string())?;

        let paid_debts_missing_settlement = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, f64>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        for (id, amount, paid_at, sale_id, service_transaction_id, recorded_total) in
            paid_debts_missing_settlement
        {
            let missing_amount = (amount - recorded_total).max(0.0);
            if missing_amount > 0.0 {
                let payment_method =
                    infer_debt_payment_method(&conn, sale_id, service_transaction_id);
                let payment_date = paid_at.unwrap_or_else(|| {
                    chrono::Local::now()
                        .format("%Y-%m-%dT%H:%M:%S%.3f")
                        .to_string()
                });
                conn.execute(
                    "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, missing_amount, payment_method, Some("Marked as paid".to_string()), payment_date],
                ).map_err(|e| e.to_string())?;
            }
        }

        // Backfill missing payment history for debts that already have recorded payments
        let mut stmt = conn
            .prepare(
                "SELECT id, amount, paid_amount, remaining_amount, status, sale_id, service_transaction_id, paid_at, created_at
                 FROM debts
                 WHERE NOT EXISTS (SELECT 1 FROM debt_payments dp WHERE dp.debt_id = debts.id)
                   AND (paid_amount > 0 OR status = 'paid')"
            )
            .map_err(|e| e.to_string())?;

        let debts_to_backfill = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        for (
            id,
            amount,
            paid_amount,
            _remaining_amount,
            status,
            sale_id,
            service_transaction_id,
            paid_at,
            created_at,
        ) in debts_to_backfill
        {
            let payment_method = infer_debt_payment_method(&conn, sale_id, service_transaction_id);
            let fallback_ts = chrono::Local::now()
                .format("%Y-%m-%dT%H:%M:%S%.3f")
                .to_string();
            let initial_ts = created_at
                .clone()
                .or_else(|| paid_at.clone())
                .unwrap_or_else(|| fallback_ts.clone());
            let final_ts = paid_at
                .clone()
                .or_else(|| created_at.clone())
                .unwrap_or_else(|| fallback_ts.clone());

            let initial_paid = if status == "paid" {
                paid_amount.min(amount).max(0.0)
            } else {
                paid_amount.max(0.0)
            };

            if initial_paid > 0.0 {
                conn.execute(
                    "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, initial_paid, payment_method.clone(), Some("Initial payment".to_string()), initial_ts],
                ).map_err(|e| e.to_string())?;
            }

            if status == "paid" {
                let settlement_amount = (amount - initial_paid).max(0.0);
                if settlement_amount > 0.0 {
                    conn.execute(
                        "INSERT INTO debt_payments (debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![id, settlement_amount, payment_method, Some("Marked as paid".to_string()), final_ts.clone()],
                    ).map_err(|e| e.to_string())?;
                }

                conn.execute(
                    "UPDATE debts SET paid_amount = ?1, remaining_amount = 0, paid_at = COALESCE(paid_at, ?2) WHERE id = ?3",
                    params![amount, final_ts, id],
                ).map_err(|e| e.to_string())?;
            }
        }

        // Multi-PC hardening: natural keys, unique indexes, merge pre-existing duplicates
        Self::ensure_multi_pc_hardening(&conn)?;

        println!("Database migrations completed");
        Ok(())
    }

    fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
        let exists = conn
            .prepare(&format!("PRAGMA table_info({})", table))
            .map_err(|e| e.to_string())?
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .any(|name| name == column);
        Ok(exists)
    }

    fn ensure_column(
        conn: &Connection,
        table: &str,
        column: &str,
        alter_sql: &str,
    ) -> Result<(), String> {
        if !Self::table_has_column(conn, table, column)? {
            println!("Adding {} column to {}", column, table);
            conn.execute_batch(alter_sql).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Natural keys + unique constraints so concurrent multi-PC creates merge instead of duplicating.
    fn ensure_multi_pc_hardening(conn: &Connection) -> Result<(), String> {
        Self::ensure_column(
            conn,
            "products",
            "natural_key",
            "ALTER TABLE products ADD COLUMN natural_key TEXT",
        )?;
        Self::ensure_column(
            conn,
            "stock",
            "natural_key",
            "ALTER TABLE stock ADD COLUMN natural_key TEXT",
        )?;
        Self::ensure_column(
            conn,
            "printing_materials",
            "natural_key",
            "ALTER TABLE printing_materials ADD COLUMN natural_key TEXT",
        )?;

        // Backfill natural keys from business fields
        conn.execute_batch(
            "
            UPDATE products SET natural_key =
                lower(trim(product_type)) || '|' ||
                lower(trim(ifnull(color, ''))) || '|' ||
                lower(trim(ifnull(size, '')))
            WHERE natural_key IS NULL OR natural_key = '';

            UPDATE stock SET natural_key =
                lower(trim(color)) || '|' ||
                trim(size) || '|' ||
                lower(trim(sticker_type))
            WHERE natural_key IS NULL OR natural_key = '';

            UPDATE printing_materials SET natural_key =
                lower(trim(name)) || '|' ||
                lower(trim(material_type)) || '|' ||
                printf('%.4f', width) || '|' ||
                lower(trim(ifnull(color, '')))
            WHERE natural_key IS NULL OR natural_key = '';
            ",
        )
        .map_err(|e| e.to_string())?;

        Self::dedupe_by_natural_key(
            conn,
            "products",
            "stock",
            "SELECT natural_key, MIN(id) AS keeper, COUNT(*) AS c FROM products WHERE natural_key IS NOT NULL AND natural_key != '' GROUP BY natural_key HAVING c > 1",
            |conn, keeper, dupe| {
                conn.execute(
                    "UPDATE products SET
                        stock = stock + COALESCE((SELECT stock FROM products WHERE id = ?2), 0),
                        updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?1",
                    params![keeper, dupe],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE sales SET product_id = ?1 WHERE product_id = ?2",
                    params![keeper, dupe],
                )
                .map_err(|e| e.to_string())?;
                conn.execute("DELETE FROM products WHERE id = ?1", params![dupe])
                    .map_err(|e| e.to_string())?;
                Ok(())
            },
        )?;

        Self::dedupe_by_natural_key(
            conn,
            "stock",
            "rolls",
            "SELECT natural_key, MIN(id) AS keeper, COUNT(*) AS c FROM stock WHERE natural_key IS NOT NULL AND natural_key != '' GROUP BY natural_key HAVING c > 1",
            |conn, keeper, dupe| {
                conn.execute(
                    "UPDATE stock SET
                        rolls = rolls + COALESCE((SELECT rolls FROM stock WHERE id = ?2), 0),
                        total_metres = total_metres + COALESCE((SELECT total_metres FROM stock WHERE id = ?2), 0),
                        metres_used = metres_used + COALESCE((SELECT metres_used FROM stock WHERE id = ?2), 0),
                        updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?1",
                    params![keeper, dupe],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE sales SET stock_id = ?1 WHERE stock_id = ?2",
                    params![keeper, dupe],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE service_transactions SET stock_id = ?1 WHERE stock_id = ?2",
                    params![keeper, dupe],
                )
                .map_err(|e| e.to_string())?;
                conn.execute("DELETE FROM stock WHERE id = ?1", params![dupe])
                    .map_err(|e| e.to_string())?;
                Ok(())
            },
        )?;

        Self::dedupe_by_natural_key(
            conn,
            "printing_materials",
            "rolls",
            "SELECT natural_key, MIN(id) AS keeper, COUNT(*) AS c FROM printing_materials WHERE natural_key IS NOT NULL AND natural_key != '' GROUP BY natural_key HAVING c > 1",
            |conn, keeper, dupe| {
                conn.execute(
                    "UPDATE printing_materials SET
                        rolls = rolls + COALESCE((SELECT rolls FROM printing_materials WHERE id = ?2), 0),
                        total_metres = total_metres + COALESCE((SELECT total_metres FROM printing_materials WHERE id = ?2), 0),
                        metres_used = metres_used + COALESCE((SELECT metres_used FROM printing_materials WHERE id = ?2), 0),
                        updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?1",
                    params![keeper, dupe],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE service_transactions SET printing_material_id = ?1 WHERE printing_material_id = ?2",
                    params![keeper, dupe],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "DELETE FROM printing_materials WHERE id = ?1",
                    params![dupe],
                )
                .map_err(|e| e.to_string())?;
                Ok(())
            },
        )?;

        // Full unique indexes (all rows backfilled) so INSERT … ON CONFLICT(natural_key) works.
        conn.execute_batch(
            "
            CREATE UNIQUE INDEX IF NOT EXISTS idx_products_natural_key ON products(natural_key);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_natural_key ON stock(natural_key);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_printing_materials_natural_key ON printing_materials(natural_key);
            ",
        )
        .map_err(|e| e.to_string())?;

        println!("Multi-PC hardening applied (natural keys + unique indexes)");
        Ok(())
    }

    /// Merge rows that share a natural_key, keeping the lowest id.
    fn dedupe_by_natural_key<F>(
        conn: &Connection,
        table: &str,
        _merge_hint: &str,
        group_sql: &str,
        mut merge_one: F,
    ) -> Result<(), String>
    where
        F: FnMut(&Connection, i64, i64) -> Result<(), String>,
    {
        let groups: Vec<(String, i64)> = {
            let mut stmt = conn.prepare(group_sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        };

        for (natural_key, keeper) in groups {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT id FROM {} WHERE natural_key = ?1 AND id != ?2",
                    table
                ))
                .map_err(|e| e.to_string())?;
            let dupes: Vec<i64> = stmt
                .query_map(params![natural_key, keeper], |row| row.get(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            drop(stmt);
            for dupe in dupes {
                merge_one(conn, keeper, dupe)?;
                println!("Merged duplicate {} row {} into {}", table, dupe, keeper);
            }
        }
        Ok(())
    }

    // ==================== Products CRUD ====================

    pub fn get_all_products(&self) -> Result<Vec<Product>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM products ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Product {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    product_type: row.get(2)?,
                    color: row.get(3)?,
                    size: row.get(4)?,
                    selling_price: row.get(5)?,
                    stock: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_product(&self, id: i64) -> Result<Option<Product>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM products WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(Product {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    product_type: row.get(2)?,
                    color: row.get(3)?,
                    size: row.get(4)?,
                    selling_price: row.get(5)?,
                    stock: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn get_products_page(&self, query: ProductsPageQuery) -> Result<ProductsPageData, String> {
        let conn = self.synced_conn()?;
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let total_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at FROM products ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![per_page as i64, offset], |row| {
                Ok(Product {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    product_type: row.get(2)?,
                    color: row.get(3)?,
                    size: row.get(4)?,
                    selling_price: row.get(5)?,
                    stock: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let items = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let (total_stock_units, life_saver_stock, chevron_stock, stripes_stock, stock_value): (
            i64,
            i64,
            i64,
            i64,
            f64,
        ) = conn
            .query_row(
                "SELECT
                COALESCE(SUM(stock), 0),
                COALESCE(SUM(CASE WHEN product_type = 'life_saver' THEN stock ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN product_type = 'chevron' THEN stock ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN product_type = 'stripes' THEN stock ELSE 0 END), 0),
                COALESCE(SUM(stock * selling_price), 0)
             FROM products",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;

        Ok(ProductsPageData {
            items,
            total_count,
            total_stock_units,
            life_saver_stock,
            chevron_stock,
            stripes_stock,
            stock_value,
        })
    }

    /// Atomically adjust product stock by a relative delta (safe across concurrent PCs).
    pub fn adjust_product_stock(&self, id: i64, delta: i64) -> Result<(), String> {
        if delta == 0 {
            return Ok(());
        }
        let conn = self.synced_conn()?;
        let updated = if delta > 0 {
            conn.execute(
                "UPDATE products SET stock = stock + ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![delta, id],
            )
            .map_err(|e| e.to_string())?
        } else {
            let need = -delta;
            conn.execute(
                "UPDATE products SET stock = stock + ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND stock >= ?3",
                params![delta, id, need],
            )
            .map_err(|e| e.to_string())?
        };
        if updated == 0 {
            return Err(if delta < 0 {
                "Insufficient product stock or product was not found.".into()
            } else {
                "Product was not found.".into()
            });
        }
        self.finish_write(())
    }

    pub fn add_product(&self, product: NewProduct) -> Result<Product, String> {
        let conn = self.synced_conn()?;
        let natural_key = product_natural_key(&product.product_type, &product.color, &product.size);
        let candidate_id = new_distributed_id();

        // Upsert by natural key: concurrent PCs adding the same variant merge stock instead of duplicating
        conn.execute(
            "INSERT INTO products (id, name, product_type, color, size, selling_price, stock, natural_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(natural_key) DO UPDATE SET
                stock = stock + excluded.stock,
                name = CASE WHEN length(excluded.name) > 0 THEN excluded.name ELSE name END,
                selling_price = CASE WHEN excluded.selling_price > 0 THEN excluded.selling_price ELSE selling_price END,
                updated_at = CURRENT_TIMESTAMP",
            params![
                candidate_id,
                product.name,
                product.product_type,
                product.color,
                product.size,
                product.selling_price,
                product.stock,
                natural_key
            ],
        )
        .map_err(|e| e.to_string())?;

        // IMPORTANT: resolve by natural_key (conflict keeps the existing id)
        let product = conn
            .query_row(
                "SELECT id, name, product_type, color, size, selling_price, stock, created_at, updated_at
                 FROM products WHERE natural_key = ?1",
                params![natural_key],
                |row| {
                    Ok(Product {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        product_type: row.get(2)?,
                        color: row.get(3)?,
                        size: row.get(4)?,
                        selling_price: row.get(5)?,
                        stock: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;
        self.finish_write(product)
    }

    pub fn update_product(&self, id: i64, updates: ProductUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let mut sql = String::from("UPDATE products SET ");
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut first = true;
        let mut touches_identity = false;

        if let Some(v) = updates.name {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("name = ?");
            params.push(Box::new(v));
            first = false;
        }
        if let Some(v) = updates.product_type {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("product_type = ?");
            params.push(Box::new(v));
            first = false;
            touches_identity = true;
        }
        if let Some(v) = updates.color {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("color = ?");
            params.push(Box::new(v));
            first = false;
            touches_identity = true;
        }
        if let Some(v) = updates.size {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("size = ?");
            params.push(Box::new(v));
            first = false;
            touches_identity = true;
        }
        if let Some(v) = updates.selling_price {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("selling_price = ?");
            params.push(Box::new(v));
            first = false;
        }
        if let Some(v) = updates.stock {
            if !first {
                sql.push_str(", ");
            }
            sql.push_str("stock = ?");
            params.push(Box::new(v));
            first = false;
        }

        if first {
            return Ok(());
        }
        sql.push_str(", updated_at = CURRENT_TIMESTAMP WHERE id = ?");
        params.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;

        if touches_identity {
            // Keep natural_key in sync so multi-PC upserts still match
            conn.execute(
                "UPDATE products SET natural_key =
                    lower(trim(product_type)) || '|' ||
                    lower(trim(ifnull(color, ''))) || '|' ||
                    lower(trim(ifnull(size, '')))
                 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| {
                format!(
                    "Could not update product identity (another product may already use that type/color/size): {}",
                    e
                )
            })?;
        }
        self.finish_write(())
    }

    pub fn delete_product(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM products WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Stock CRUD ====================

    pub fn get_all_stock(&self) -> Result<Vec<StockItem>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(StockItem {
                    id: row.get(0)?,
                    color: row.get(1)?,
                    size: row.get(2)?,
                    sticker_type: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_stock(&self, id: i64) -> Result<Option<StockItem>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(StockItem {
                    id: row.get(0)?,
                    color: row.get(1)?,
                    size: row.get(2)?,
                    sticker_type: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn get_stock_by_color_size_type(
        &self,
        color: &str,
        size: &str,
        sticker_type: &str,
    ) -> Result<Option<StockItem>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock WHERE LOWER(color) = LOWER(?1) AND size = ?2 AND sticker_type = ?3")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![color, size, sticker_type], |row| {
                Ok(StockItem {
                    id: row.get(0)?,
                    color: row.get(1)?,
                    size: row.get(2)?,
                    sticker_type: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn get_stock_page(&self, query: StockPageQuery) -> Result<StockPageData, String> {
        let conn = self.synced_conn()?;
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let total_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stock", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at FROM stock ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![per_page as i64, offset], |row| {
                Ok(StockItem {
                    id: row.get(0)?,
                    color: row.get(1)?,
                    size: row.get(2)?,
                    sticker_type: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let items = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let (total_rolls, total_metres, remaining_metres): (i64, f64, f64) = conn
            .query_row(
                "SELECT
                COALESCE(SUM(rolls), 0),
                COALESCE(SUM(total_metres), 0),
                COALESCE(SUM(total_metres - COALESCE(metres_used, 0)), 0)
             FROM stock",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| e.to_string())?;

        Ok(StockPageData {
            items,
            total_count,
            total_rolls,
            total_metres,
            remaining_metres,
        })
    }

    /// Atomically add rolls (and metres) to an existing stock row.
    pub fn add_stock_rolls(&self, id: i64, rolls: i64) -> Result<(), String> {
        if rolls <= 0 {
            return Err("Rolls to add must be greater than zero.".into());
        }
        let conn = self.synced_conn()?;
        let updated = conn
            .execute(
                "UPDATE stock SET
                    rolls = rolls + ?1,
                    total_metres = total_metres + (?1 * metres_per_roll),
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?2",
                params![rolls, id],
            )
            .map_err(|e| e.to_string())?;
        if updated == 0 {
            return Err("Stock item was not found.".into());
        }
        self.finish_write(())
    }

    pub fn add_stock(&self, item: NewStockItem) -> Result<StockItem, String> {
        let conn = self.synced_conn()?;

        let base_metres_per_roll = 50.0_f64;
        let (metres_per_roll, add_metres) = if item.sticker_type == "reflective" {
            if let Some(custom) = item.custom_metres_per_roll {
                (custom, item.rolls as f64 * custom)
            } else {
                (
                    base_metres_per_roll,
                    item.rolls as f64 * base_metres_per_roll,
                )
            }
        } else {
            (
                base_metres_per_roll,
                item.rolls as f64 * base_metres_per_roll,
            )
        };

        let natural_key = stock_natural_key(&item.color, &item.size, &item.sticker_type);
        let candidate_id = new_distributed_id();

        // Upsert by natural key — concurrent multi-PC stock adds accumulate on one row
        conn.execute(
            "INSERT INTO stock (id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, natural_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)
             ON CONFLICT(natural_key) DO UPDATE SET
                rolls = rolls + excluded.rolls,
                total_metres = total_metres + excluded.total_metres,
                updated_at = CURRENT_TIMESTAMP",
            params![
                candidate_id,
                item.color,
                item.size,
                item.sticker_type,
                item.rolls,
                metres_per_roll,
                add_metres,
                natural_key
            ],
        )
        .map_err(|e| e.to_string())?;

        let stock_item = conn
            .query_row(
                "SELECT id, color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used, created_at, updated_at
                 FROM stock WHERE natural_key = ?1",
                params![natural_key],
                |row| {
                    Ok(StockItem {
                        id: row.get(0)?,
                        color: row.get(1)?,
                        size: row.get(2)?,
                        sticker_type: row.get(3)?,
                        rolls: row.get(4)?,
                        metres_per_roll: row.get(5)?,
                        total_metres: row.get(6)?,
                        metres_used: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;
        self.finish_write(stock_item)
    }

    pub fn update_stock(&self, id: i64, updates: StockUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let touches_identity =
            updates.color.is_some() || updates.size.is_some() || updates.sticker_type.is_some();

        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.color {
            set_clauses.push("color = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.size {
            set_clauses.push("size = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.sticker_type {
            set_clauses.push("sticker_type = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.rolls {
            set_clauses.push("rolls = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.metres_per_roll {
            set_clauses.push("metres_per_roll = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.total_metres {
            set_clauses.push("total_metres = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.metres_used {
            set_clauses.push("metres_used = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!("UPDATE stock SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;

        if touches_identity {
            conn.execute(
                "UPDATE stock SET natural_key =
                    lower(trim(color)) || '|' || trim(size) || '|' || lower(trim(sticker_type))
                 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| {
                format!(
                    "Could not update stock identity (another row may already use that color/size/type): {}",
                    e
                )
            })?;
        }
        self.finish_write(())
    }

    pub fn delete_stock(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM stock WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Sales CRUD ====================

    pub fn get_all_sales(&self) -> Result<Vec<Sale>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp,
                CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount
                     ELSE COALESCE((SELECT d.paid_amount FROM debts d WHERE d.sale_id = sales.id LIMIT 1), 0)
                END AS amount_paid
             FROM sales ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Sale {
                    id: row.get(0)?,
                    r#type: row.get(1)?,
                    product_id: row.get(2)?,
                    stock_id: row.get(3)?,
                    product_name: row.get(4)?,
                    product_type: row.get(5)?,
                    sticker_type: row.get(6)?,
                    quantity: row.get(7)?,
                    amount: row.get(8)?,
                    payment_method: row.get(9)?,
                    customer_name: row.get(10)?,
                    is_debt: row.get(11)?,
                    timestamp: row.get(12)?,
                    amount_paid: row.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_today_sales(&self) -> Result<Vec<Sale>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp,
                CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount
                     ELSE COALESCE((SELECT d.paid_amount FROM debts d WHERE d.sale_id = sales.id LIMIT 1), 0)
                END AS amount_paid
             FROM sales WHERE DATE(timestamp) = DATE('now', 'localtime') ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Sale {
                    id: row.get(0)?,
                    r#type: row.get(1)?,
                    product_id: row.get(2)?,
                    stock_id: row.get(3)?,
                    product_name: row.get(4)?,
                    product_type: row.get(5)?,
                    sticker_type: row.get(6)?,
                    quantity: row.get(7)?,
                    amount: row.get(8)?,
                    payment_method: row.get(9)?,
                    customer_name: row.get(10)?,
                    is_debt: row.get(11)?,
                    timestamp: row.get(12)?,
                    amount_paid: row.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_sale(&self, sale: NewSale) -> Result<Sale, String> {
        let conn = self.synced_conn()?;
        let timestamp = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();

        conn.execute_batch("BEGIN IMMEDIATE;")
            .map_err(|e| e.to_string())?;

        let insert_result = (|| -> Result<i64, String> {
            if let (Some(product_id), Some(quantity)) = (sale.product_id, sale.product_quantity) {
                if quantity > 0 {
                    let updated = conn
                        .execute(
                            "UPDATE products SET stock = stock - ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND stock >= ?1",
                            params![quantity, product_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if updated == 0 {
                        return Err("Insufficient product stock or product was not found.".into());
                    }
                }
            }

            if let (Some(stock_id), Some(metres_used)) = (sale.stock_id, sale.stock_metres_used) {
                if metres_used > 0.0 {
                    let updated = conn
                        .execute(
                            "UPDATE stock SET metres_used = metres_used + ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND (total_metres - metres_used) >= ?1",
                            params![metres_used, stock_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if updated == 0 {
                        return Err("Insufficient stock metres or stock item was not found.".into());
                    }
                }
            }

            let id = new_distributed_id();
            conn.execute(
                "INSERT INTO sales (id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![id, sale.r#type, sale.product_id, sale.stock_id, sale.product_name, sale.product_type, sale.sticker_type, sale.quantity, sale.amount, sale.payment_method, sale.customer_name, sale.is_debt, timestamp],
            ).map_err(|e| e.to_string())?;

            Ok(id)
        })();

        let id = match insert_result {
            Ok(id) => {
                conn.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
                id
            }
            Err(err) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(err);
            }
        };

        drop(conn);
        self.finish_write(Sale {
            id,
            r#type: sale.r#type,
            product_id: sale.product_id,
            stock_id: sale.stock_id,
            product_name: sale.product_name,
            product_type: sale.product_type,
            sticker_type: sale.sticker_type,
            quantity: sale.quantity,
            amount: sale.amount,
            payment_method: sale.payment_method,
            customer_name: sale.customer_name,
            is_debt: sale.is_debt,
            timestamp: Some(timestamp),
            amount_paid: if sale.is_debt == 0 { sale.amount } else { 0.0 },
        })
    }

    pub fn get_today_total_sales(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        let total: f64 = conn
            .query_row(
                "SELECT
                    COALESCE((SELECT SUM(amount) FROM sales
                              WHERE DATE(timestamp) = DATE('now', 'localtime')
                                AND COALESCE(is_debt, 0) = 0), 0)
                  + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                              INNER JOIN debts d ON d.id = dp.debt_id
                              WHERE d.sale_id IS NOT NULL
                                AND DATE(dp.payment_date) = DATE('now', 'localtime')), 0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(total)
    }

    pub fn get_sales_page(&self, query: SalesPageQuery) -> Result<SalesPageData, String> {
        let conn = self.synced_conn()?;

        let search = query.search.unwrap_or_default().trim().to_lowercase();
        let sort_by = query.sort_by.unwrap_or_else(|| "newest".to_string());
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let search_filter = if search.is_empty() {
            String::new()
        } else {
            "WHERE (
                LOWER(COALESCE(product_name, '')) LIKE ?1 OR
                LOWER(customer_name) LIKE ?1 OR
                LOWER(payment_method) LIKE ?1 OR
                LOWER(COALESCE(quantity, '')) LIKE ?1 OR
                LOWER(type) LIKE ?1 OR
                LOWER(COALESCE(timestamp, '')) LIKE ?1
            )"
            .to_string()
        };

        let order_by = match sort_by.as_str() {
            "oldest" => "timestamp ASC",
            "amount_desc" => "amount DESC, timestamp DESC",
            "amount_asc" => "amount ASC, timestamp DESC",
            _ => "timestamp DESC",
        };

        let total_count: i64 = if search.is_empty() {
            conn.query_row("SELECT COUNT(*) FROM sales", [], |row| row.get(0))
        } else {
            let term = format!("%{}%", search);
            conn.query_row(
                &format!("SELECT COUNT(*) FROM sales {}", search_filter),
                params![term],
                |row| row.get(0),
            )
        }
        .map_err(|e| e.to_string())?;

        let sql = format!(
            "SELECT id, type, product_id, stock_id, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, is_debt, timestamp,
                CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount
                     ELSE COALESCE((SELECT d.paid_amount FROM debts d WHERE d.sale_id = sales.id LIMIT 1), 0)
                END AS amount_paid
             FROM sales
             {}
             ORDER BY {}
             LIMIT ?{} OFFSET ?{}",
            search_filter,
            order_by,
            if search.is_empty() { 1 } else { 2 },
            if search.is_empty() { 2 } else { 3 },
        );

        let items = if search.is_empty() {
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![per_page as i64, offset], |row| {
                    Ok(Sale {
                        id: row.get(0)?,
                        r#type: row.get(1)?,
                        product_id: row.get(2)?,
                        stock_id: row.get(3)?,
                        product_name: row.get(4)?,
                        product_type: row.get(5)?,
                        sticker_type: row.get(6)?,
                        quantity: row.get(7)?,
                        amount: row.get(8)?,
                        payment_method: row.get(9)?,
                        customer_name: row.get(10)?,
                        is_debt: row.get(11)?,
                        timestamp: row.get(12)?,
                        amount_paid: row.get(13)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        } else {
            let term = format!("%{}%", search);
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![term, per_page as i64, offset], |row| {
                    Ok(Sale {
                        id: row.get(0)?,
                        r#type: row.get(1)?,
                        product_id: row.get(2)?,
                        stock_id: row.get(3)?,
                        product_name: row.get(4)?,
                        product_type: row.get(5)?,
                        sticker_type: row.get(6)?,
                        quantity: row.get(7)?,
                        amount: row.get(8)?,
                        payment_method: row.get(9)?,
                        customer_name: row.get(10)?,
                        is_debt: row.get(11)?,
                        timestamp: row.get(12)?,
                        amount_paid: row.get(13)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        // Recognized cash only: non-debt sales + debt collections linked to sales.
        let today_total: f64 = conn
            .query_row(
                "SELECT
                COALESCE((SELECT SUM(amount) FROM sales
                          WHERE DATE(timestamp) = DATE('now', 'localtime')
                            AND COALESCE(is_debt, 0) = 0), 0)
              + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                          INNER JOIN debts d ON d.id = dp.debt_id
                          WHERE d.sale_id IS NOT NULL
                            AND DATE(dp.payment_date) = DATE('now', 'localtime')), 0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let all_revenue: f64 = conn
            .query_row(
                "SELECT
                    COALESCE((SELECT SUM(amount) FROM sales WHERE COALESCE(is_debt, 0) = 0), 0)
                  + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                              INNER JOIN debts d ON d.id = dp.debt_id
                              WHERE d.sale_id IS NOT NULL), 0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let product_sales_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sales WHERE type = 'product'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        Ok(SalesPageData {
            items,
            total_count,
            today_total,
            all_revenue,
            product_sales_count,
        })
    }

    pub fn update_sale(&self, id: i64, updates: SaleUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.r#type {
            set_clauses.push("type = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.amount {
            set_clauses.push("amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.payment_method {
            set_clauses.push("payment_method = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.customer_name {
            set_clauses.push("customer_name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.is_debt {
            set_clauses.push("is_debt = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE sales SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_sale(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM sales WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Debts CRUD ====================

    pub fn get_all_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN COALESCE(
                    NULLIF(TRIM(s.product_name), ''),
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Life Saver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE NULL
                    END,
                    CASE
                      WHEN s.type = 'stock' THEN TRIM(COALESCE(sk.color, '') || ' ' || CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Sticker' END)
                      WHEN s.type = 'service' THEN 'Service sale'
                      WHEN s.type = 'product' THEN 'Product sale'
                      ELSE 'Sale'
                    END
                  )
                  WHEN d.service_transaction_id IS NOT NULL THEN COALESCE(NULLIF(TRIM(st.service_name), ''), 'Printing job')
                  ELSE COALESCE(NULLIF(TRIM(d.description), ''), 'Manual debt')
                END AS source_label,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN 'sale'
                  WHEN d.service_transaction_id IS NOT NULL THEN 'printing'
                  ELSE 'manual'
                END AS source_kind,
                CASE
                  WHEN d.sale_id IS NOT NULL AND s.type = 'product' THEN TRIM(
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Lifesaver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE COALESCE(s.product_type, 'Product')
                    END ||
                    CASE WHEN COALESCE(p.color, '') != '' THEN ' · ' || CASE p.color
                      WHEN 'white_red' THEN 'White / Red'
                      WHEN 'yellow_red' THEN 'Yellow / Red'
                      WHEN 'white' THEN 'White'
                      WHEN 'yellow' THEN 'Yellow'
                      ELSE p.color
                    END ELSE '' END ||
                    CASE WHEN COALESCE(s.quantity, '') != '' THEN ' · qty ' || s.quantity ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL AND s.type = 'stock' THEN TRIM(
                    CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Colored' END ||
                    CASE WHEN COALESCE(sk.color, '') != '' THEN ' · ' || sk.color ELSE '' END ||
                    CASE WHEN COALESCE(sk.size, s.quantity, '') != '' THEN ' · ' || COALESCE(sk.size, s.quantity) ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL THEN TRIM(COALESCE(s.quantity, ''))
                  WHEN d.service_transaction_id IS NOT NULL THEN TRIM(
                    CASE WHEN st.stock_metres_used > 0 THEN printf('%.1fm printed', st.stock_metres_used) ELSE '' END ||
                    CASE WHEN st.material_type IS NOT NULL AND TRIM(st.material_type) != '' THEN ' · ' || st.material_type ELSE '' END ||
                    CASE WHEN st.material_size IS NOT NULL AND TRIM(st.material_size) != '' THEN ' · ' || st.material_size || 'm wide' ELSE '' END
                  )
                  ELSE NULL
                END AS source_detail,
                s.type AS source_sale_type,
                s.product_type AS source_product_type,
                COALESCE(p.color, sk.color) AS source_color,
                COALESCE(s.sticker_type, sk.sticker_type) AS source_sticker_type
             FROM debts d
             LEFT JOIN sales s ON s.id = d.sale_id
             LEFT JOIN products p ON p.id = s.product_id
             LEFT JOIN stock sk ON sk.id = s.stock_id
             LEFT JOIN service_transactions st ON st.id = d.service_transaction_id ORDER BY d.created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                    source_label: row.get(14)?,
                    source_kind: row.get(15)?,
                    source_detail: row.get(16)?,
                    source_sale_type: row.get(17)?,
                    source_product_type: row.get(18)?,
                    source_color: row.get(19)?,
                    source_sticker_type: row.get(20)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_pending_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN COALESCE(
                    NULLIF(TRIM(s.product_name), ''),
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Life Saver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE NULL
                    END,
                    CASE
                      WHEN s.type = 'stock' THEN TRIM(COALESCE(sk.color, '') || ' ' || CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Sticker' END)
                      WHEN s.type = 'service' THEN 'Service sale'
                      WHEN s.type = 'product' THEN 'Product sale'
                      ELSE 'Sale'
                    END
                  )
                  WHEN d.service_transaction_id IS NOT NULL THEN COALESCE(NULLIF(TRIM(st.service_name), ''), 'Printing job')
                  ELSE COALESCE(NULLIF(TRIM(d.description), ''), 'Manual debt')
                END AS source_label,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN 'sale'
                  WHEN d.service_transaction_id IS NOT NULL THEN 'printing'
                  ELSE 'manual'
                END AS source_kind,
                CASE
                  WHEN d.sale_id IS NOT NULL AND s.type = 'product' THEN TRIM(
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Lifesaver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE COALESCE(s.product_type, 'Product')
                    END ||
                    CASE WHEN COALESCE(p.color, '') != '' THEN ' · ' || CASE p.color
                      WHEN 'white_red' THEN 'White / Red'
                      WHEN 'yellow_red' THEN 'Yellow / Red'
                      WHEN 'white' THEN 'White'
                      WHEN 'yellow' THEN 'Yellow'
                      ELSE p.color
                    END ELSE '' END ||
                    CASE WHEN COALESCE(s.quantity, '') != '' THEN ' · qty ' || s.quantity ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL AND s.type = 'stock' THEN TRIM(
                    CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Colored' END ||
                    CASE WHEN COALESCE(sk.color, '') != '' THEN ' · ' || sk.color ELSE '' END ||
                    CASE WHEN COALESCE(sk.size, s.quantity, '') != '' THEN ' · ' || COALESCE(sk.size, s.quantity) ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL THEN TRIM(COALESCE(s.quantity, ''))
                  WHEN d.service_transaction_id IS NOT NULL THEN TRIM(
                    CASE WHEN st.stock_metres_used > 0 THEN printf('%.1fm printed', st.stock_metres_used) ELSE '' END ||
                    CASE WHEN st.material_type IS NOT NULL AND TRIM(st.material_type) != '' THEN ' · ' || st.material_type ELSE '' END ||
                    CASE WHEN st.material_size IS NOT NULL AND TRIM(st.material_size) != '' THEN ' · ' || st.material_size || 'm wide' ELSE '' END
                  )
                  ELSE NULL
                END AS source_detail,
                s.type AS source_sale_type,
                s.product_type AS source_product_type,
                COALESCE(p.color, sk.color) AS source_color,
                COALESCE(s.sticker_type, sk.sticker_type) AS source_sticker_type
             FROM debts d
             LEFT JOIN sales s ON s.id = d.sale_id
             LEFT JOIN products p ON p.id = s.product_id
             LEFT JOIN stock sk ON sk.id = s.stock_id
             LEFT JOIN service_transactions st ON st.id = d.service_transaction_id WHERE d.status = 'pending' ORDER BY d.created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                    source_label: row.get(14)?,
                    source_kind: row.get(15)?,
                    source_detail: row.get(16)?,
                    source_sale_type: row.get(17)?,
                    source_product_type: row.get(18)?,
                    source_color: row.get(19)?,
                    source_sticker_type: row.get(20)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_debt(&self, debt: NewDebt) -> Result<Debt, String> {
        let conn = self.synced_conn()?;
        let paid_amount = debt.paid_amount.unwrap_or(0.0);
        let remaining_amount = debt.remaining_amount.unwrap_or(debt.amount - paid_amount);
        let created_at = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();
        let status = if remaining_amount <= 0.0 {
            "paid"
        } else {
            "pending"
        };
        let paid_at = if paid_amount > 0.0 && remaining_amount <= 0.0 {
            Some(created_at.clone())
        } else {
            None
        };

        let id = new_distributed_id();
        conn.execute(
            "INSERT INTO debts (id, customer_name, phone, amount, paid_amount, remaining_amount, due_date, description, status, sale_id, service_transaction_id, paid_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![id, debt.customer_name, debt.phone, debt.amount, paid_amount, remaining_amount, debt.due_date, debt.description, status, debt.sale_id, debt.service_transaction_id, paid_at, created_at],
        ).map_err(|e| e.to_string())?;

        if paid_amount > 0.0 {
            let payment_method =
                infer_debt_payment_method(&conn, debt.sale_id, debt.service_transaction_id);
            let payment_id = new_distributed_id();
            conn.execute(
                "INSERT INTO debt_payments (id, debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![payment_id, id, paid_amount, payment_method, Some("Initial payment".to_string()), created_at.clone()],
            ).map_err(|e| e.to_string())?;
        }

        // Convert-to-debt: mark source row in the same write so the UI does not
        // need a second update_sale / update_service_transaction round-trip.
        if let Some(sid) = debt.sale_id {
            conn.execute("UPDATE sales SET is_debt = 1 WHERE id = ?1", params![sid])
                .map_err(|e| e.to_string())?;
        }
        if let Some(tid) = debt.service_transaction_id {
            conn.execute(
                "UPDATE service_transactions SET is_debt = 1 WHERE id = ?1",
                params![tid],
            )
            .map_err(|e| e.to_string())?;
        }

        // Prefer joined source metadata for previews (same shape as list queries).
        let mut source_label = debt.description.clone().filter(|s| !s.trim().is_empty());
        let mut source_kind = if debt.sale_id.is_some() {
            Some("sale".into())
        } else if debt.service_transaction_id.is_some() {
            Some("printing".into())
        } else {
            Some("manual".into())
        };
        let mut source_sale_type = None::<String>;
        let mut source_product_type = None::<String>;
        let mut source_color = None::<String>;
        let mut source_sticker_type = None::<String>;

        if let Some(sid) = debt.sale_id {
            if let Ok((stype, pname, ptype, sticker, pcolor, skcolor, sksticker)) = conn.query_row(
                "SELECT s.type, s.product_name, s.product_type, s.sticker_type,
                            p.color, sk.color, sk.sticker_type
                     FROM sales s
                     LEFT JOIN products p ON p.id = s.product_id
                     LEFT JOIN stock sk ON sk.id = s.stock_id
                     WHERE s.id = ?1",
                params![sid],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            ) {
                source_sale_type = stype.clone();
                source_product_type = ptype;
                source_sticker_type = sticker.or(sksticker);
                source_color = pcolor.or(skcolor);
                if source_label.is_none() {
                    source_label =
                        pname
                            .filter(|s| !s.trim().is_empty())
                            .or_else(|| match stype.as_deref() {
                                Some("stock") => Some("Sticker sale".into()),
                                Some("product") => Some("Product sale".into()),
                                Some("service") => Some("Service sale".into()),
                                _ => Some("Sale".into()),
                            });
                }
                source_kind = Some("sale".into());
            }
        } else if let Some(tid) = debt.service_transaction_id {
            if let Ok(sname) = conn.query_row(
                "SELECT service_name FROM service_transactions WHERE id = ?1",
                params![tid],
                |row| row.get::<_, Option<String>>(0),
            ) {
                if source_label.is_none() {
                    source_label = sname.filter(|s| !s.trim().is_empty());
                }
                source_kind = Some("printing".into());
            }
        }

        self.finish_write(Debt {
            id,
            customer_name: debt.customer_name,
            phone: debt.phone,
            amount: debt.amount,
            paid_amount,
            remaining_amount,
            due_date: debt.due_date,
            description: debt.description,
            status: status.to_string(),
            sale_id: debt.sale_id,
            service_transaction_id: debt.service_transaction_id,
            paid_at: paid_at.clone(),
            last_payment_at: if paid_amount > 0.0 {
                Some(created_at.clone())
            } else {
                None
            },
            created_at: Some(created_at),
            source_label,
            source_kind,
            source_detail: None,
            source_sale_type,
            source_product_type,
            source_color,
            source_sticker_type,
        })
    }

    pub fn update_debt(&self, id: i64, updates: DebtUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.customer_name {
            set_clauses.push("customer_name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.phone {
            set_clauses.push("phone = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.amount {
            set_clauses.push("amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.paid_amount {
            set_clauses.push("paid_amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.remaining_amount {
            set_clauses.push("remaining_amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.due_date {
            set_clauses.push("due_date = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.description {
            set_clauses.push("description = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.status {
            set_clauses.push("status = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE debts SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn mark_debt_paid(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let (sale_id, transaction_id, amount, paid_amount, remaining_amount): (Option<i64>, Option<i64>, f64, f64, f64) = conn
            .query_row(
                "SELECT sale_id, service_transaction_id, amount, paid_amount, remaining_amount FROM debts WHERE id = ?1",
                params![id],
                |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
                }
            )
            .map_err(|e| e.to_string())?;

        let paid_at = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();
        let settlement_amount = remaining_amount.max(0.0);

        if settlement_amount > 0.0 {
            let payment_method = infer_debt_payment_method(&conn, sale_id, transaction_id);
            let payment_id = new_distributed_id();
            conn.execute(
                "INSERT INTO debt_payments (id, debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![payment_id, id, settlement_amount, payment_method, Some("Marked as paid".to_string()), paid_at.clone()],
            ).map_err(|e| e.to_string())?;
        }

        conn.execute(
            "UPDATE debts SET status = 'paid', paid_amount = ?1, paid_at = ?2, remaining_amount = 0 WHERE id = ?3",
            params![amount.max(paid_amount), paid_at.clone(), id],
        ).map_err(|e| e.to_string())?;

        if let Some(sid) = sale_id {
            conn.execute("UPDATE sales SET is_debt = 2 WHERE id = ?1", params![sid])
                .map_err(|e| e.to_string())?;
        }
        if let Some(tid) = transaction_id {
            conn.execute(
                "UPDATE service_transactions SET is_debt = 2 WHERE id = ?1",
                params![tid],
            )
            .map_err(|e| e.to_string())?;
        }

        self.finish_write(())
    }

    pub fn get_debt_by_sale_id(&self, sale_id: i64) -> Result<Option<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN COALESCE(
                    NULLIF(TRIM(s.product_name), ''),
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Life Saver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE NULL
                    END,
                    CASE
                      WHEN s.type = 'stock' THEN TRIM(COALESCE(sk.color, '') || ' ' || CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Sticker' END)
                      WHEN s.type = 'service' THEN 'Service sale'
                      WHEN s.type = 'product' THEN 'Product sale'
                      ELSE 'Sale'
                    END
                  )
                  WHEN d.service_transaction_id IS NOT NULL THEN COALESCE(NULLIF(TRIM(st.service_name), ''), 'Printing job')
                  ELSE COALESCE(NULLIF(TRIM(d.description), ''), 'Manual debt')
                END AS source_label,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN 'sale'
                  WHEN d.service_transaction_id IS NOT NULL THEN 'printing'
                  ELSE 'manual'
                END AS source_kind,
                CASE
                  WHEN d.sale_id IS NOT NULL AND s.type = 'product' THEN TRIM(
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Lifesaver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE COALESCE(s.product_type, 'Product')
                    END ||
                    CASE WHEN COALESCE(p.color, '') != '' THEN ' · ' || CASE p.color
                      WHEN 'white_red' THEN 'White / Red'
                      WHEN 'yellow_red' THEN 'Yellow / Red'
                      WHEN 'white' THEN 'White'
                      WHEN 'yellow' THEN 'Yellow'
                      ELSE p.color
                    END ELSE '' END ||
                    CASE WHEN COALESCE(s.quantity, '') != '' THEN ' · qty ' || s.quantity ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL AND s.type = 'stock' THEN TRIM(
                    CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Colored' END ||
                    CASE WHEN COALESCE(sk.color, '') != '' THEN ' · ' || sk.color ELSE '' END ||
                    CASE WHEN COALESCE(sk.size, s.quantity, '') != '' THEN ' · ' || COALESCE(sk.size, s.quantity) ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL THEN TRIM(COALESCE(s.quantity, ''))
                  WHEN d.service_transaction_id IS NOT NULL THEN TRIM(
                    CASE WHEN st.stock_metres_used > 0 THEN printf('%.1fm printed', st.stock_metres_used) ELSE '' END ||
                    CASE WHEN st.material_type IS NOT NULL AND TRIM(st.material_type) != '' THEN ' · ' || st.material_type ELSE '' END ||
                    CASE WHEN st.material_size IS NOT NULL AND TRIM(st.material_size) != '' THEN ' · ' || st.material_size || 'm wide' ELSE '' END
                  )
                  ELSE NULL
                END AS source_detail,
                s.type AS source_sale_type,
                s.product_type AS source_product_type,
                COALESCE(p.color, sk.color) AS source_color,
                COALESCE(s.sticker_type, sk.sticker_type) AS source_sticker_type
             FROM debts d
             LEFT JOIN sales s ON s.id = d.sale_id
             LEFT JOIN products p ON p.id = s.product_id
             LEFT JOIN stock sk ON sk.id = s.stock_id
             LEFT JOIN service_transactions st ON st.id = d.service_transaction_id WHERE d.sale_id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![sale_id], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                    source_label: row.get(14)?,
                    source_kind: row.get(15)?,
                    source_detail: row.get(16)?,
                    source_sale_type: row.get(17)?,
                    source_product_type: row.get(18)?,
                    source_color: row.get(19)?,
                    source_sticker_type: row.get(20)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn get_debt_by_transaction_id(&self, transaction_id: i64) -> Result<Option<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN COALESCE(
                    NULLIF(TRIM(s.product_name), ''),
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Life Saver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE NULL
                    END,
                    CASE
                      WHEN s.type = 'stock' THEN TRIM(COALESCE(sk.color, '') || ' ' || CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Sticker' END)
                      WHEN s.type = 'service' THEN 'Service sale'
                      WHEN s.type = 'product' THEN 'Product sale'
                      ELSE 'Sale'
                    END
                  )
                  WHEN d.service_transaction_id IS NOT NULL THEN COALESCE(NULLIF(TRIM(st.service_name), ''), 'Printing job')
                  ELSE COALESCE(NULLIF(TRIM(d.description), ''), 'Manual debt')
                END AS source_label,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN 'sale'
                  WHEN d.service_transaction_id IS NOT NULL THEN 'printing'
                  ELSE 'manual'
                END AS source_kind,
                CASE
                  WHEN d.sale_id IS NOT NULL AND s.type = 'product' THEN TRIM(
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Lifesaver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE COALESCE(s.product_type, 'Product')
                    END ||
                    CASE WHEN COALESCE(p.color, '') != '' THEN ' · ' || CASE p.color
                      WHEN 'white_red' THEN 'White / Red'
                      WHEN 'yellow_red' THEN 'Yellow / Red'
                      WHEN 'white' THEN 'White'
                      WHEN 'yellow' THEN 'Yellow'
                      ELSE p.color
                    END ELSE '' END ||
                    CASE WHEN COALESCE(s.quantity, '') != '' THEN ' · qty ' || s.quantity ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL AND s.type = 'stock' THEN TRIM(
                    CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Colored' END ||
                    CASE WHEN COALESCE(sk.color, '') != '' THEN ' · ' || sk.color ELSE '' END ||
                    CASE WHEN COALESCE(sk.size, s.quantity, '') != '' THEN ' · ' || COALESCE(sk.size, s.quantity) ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL THEN TRIM(COALESCE(s.quantity, ''))
                  WHEN d.service_transaction_id IS NOT NULL THEN TRIM(
                    CASE WHEN st.stock_metres_used > 0 THEN printf('%.1fm printed', st.stock_metres_used) ELSE '' END ||
                    CASE WHEN st.material_type IS NOT NULL AND TRIM(st.material_type) != '' THEN ' · ' || st.material_type ELSE '' END ||
                    CASE WHEN st.material_size IS NOT NULL AND TRIM(st.material_size) != '' THEN ' · ' || st.material_size || 'm wide' ELSE '' END
                  )
                  ELSE NULL
                END AS source_detail,
                s.type AS source_sale_type,
                s.product_type AS source_product_type,
                COALESCE(p.color, sk.color) AS source_color,
                COALESCE(s.sticker_type, sk.sticker_type) AS source_sticker_type
             FROM debts d
             LEFT JOIN sales s ON s.id = d.sale_id
             LEFT JOIN products p ON p.id = s.product_id
             LEFT JOIN stock sk ON sk.id = s.stock_id
             LEFT JOIN service_transactions st ON st.id = d.service_transaction_id WHERE d.service_transaction_id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![transaction_id], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                    source_label: row.get(14)?,
                    source_kind: row.get(15)?,
                    source_detail: row.get(16)?,
                    source_sale_type: row.get(17)?,
                    source_product_type: row.get(18)?,
                    source_color: row.get(19)?,
                    source_sticker_type: row.get(20)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn delete_debt(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM debts WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn get_total_outstanding(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        conn.query_row(
            "SELECT COALESCE(SUM(remaining_amount), 0) FROM debts WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    }

    pub fn get_debts_page(&self, query: DebtsPageQuery) -> Result<DebtsPageData, String> {
        let conn = self.synced_conn()?;

        let search = query.search.unwrap_or_default().trim().to_lowercase();
        let sort_by = query.sort_by.unwrap_or_else(|| "newest".to_string());
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        // Qualify debt columns — joins to products also have created_at (ambiguous otherwise).
        let search_filter = if search.is_empty() {
            String::new()
        } else {
            "WHERE (
                LOWER(d.customer_name) LIKE ?1 OR
                LOWER(COALESCE(d.phone, '')) LIKE ?1 OR
                LOWER(COALESCE(d.description, '')) LIKE ?1 OR
                LOWER(d.status) LIKE ?1 OR
                LOWER(COALESCE(d.due_date, '')) LIKE ?1 OR
                LOWER(COALESCE(s.product_name, '')) LIKE ?1 OR
                LOWER(COALESCE(st.service_name, '')) LIKE ?1
            )"
            .to_string()
        };

        let order_by = match sort_by.as_str() {
            "oldest" => "d.created_at ASC",
            "amount_desc" => "d.remaining_amount DESC, d.created_at DESC",
            "amount_asc" => "d.remaining_amount ASC, d.created_at DESC",
            _ => "d.created_at DESC",
        };

        let total_count: i64 = if search.is_empty() {
            conn.query_row("SELECT COUNT(*) FROM debts", [], |row| row.get(0))
        } else {
            let term = format!("%{}%", search);
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM debts d
                     LEFT JOIN sales s ON s.id = d.sale_id
                     LEFT JOIN service_transactions st ON st.id = d.service_transaction_id
                     {}",
                    search_filter
                ),
                params![term],
                |row| row.get(0),
            )
        }
        .map_err(|e| e.to_string())?;

        let sql = format!(
            "SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN COALESCE(
                    NULLIF(TRIM(s.product_name), ''),
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Life Saver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE NULL
                    END,
                    CASE
                      WHEN s.type = 'stock' THEN TRIM(COALESCE(sk.color, '') || ' ' || CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Sticker' END)
                      WHEN s.type = 'service' THEN 'Service sale'
                      WHEN s.type = 'product' THEN 'Product sale'
                      ELSE 'Sale'
                    END
                  )
                  WHEN d.service_transaction_id IS NOT NULL THEN COALESCE(NULLIF(TRIM(st.service_name), ''), 'Printing job')
                  ELSE COALESCE(NULLIF(TRIM(d.description), ''), 'Manual debt')
                END AS source_label,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN 'sale'
                  WHEN d.service_transaction_id IS NOT NULL THEN 'printing'
                  ELSE 'manual'
                END AS source_kind,
                CASE
                  WHEN d.sale_id IS NOT NULL AND s.type = 'product' THEN TRIM(
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Lifesaver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE COALESCE(s.product_type, 'Product')
                    END ||
                    CASE WHEN COALESCE(p.color, '') != '' THEN ' · ' || CASE p.color
                      WHEN 'white_red' THEN 'White / Red'
                      WHEN 'yellow_red' THEN 'Yellow / Red'
                      WHEN 'white' THEN 'White'
                      WHEN 'yellow' THEN 'Yellow'
                      ELSE p.color
                    END ELSE '' END ||
                    CASE WHEN COALESCE(s.quantity, '') != '' THEN ' · qty ' || s.quantity ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL AND s.type = 'stock' THEN TRIM(
                    CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Colored' END ||
                    CASE WHEN COALESCE(sk.color, '') != '' THEN ' · ' || sk.color ELSE '' END ||
                    CASE WHEN COALESCE(sk.size, s.quantity, '') != '' THEN ' · ' || COALESCE(sk.size, s.quantity) ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL THEN TRIM(COALESCE(s.quantity, ''))
                  WHEN d.service_transaction_id IS NOT NULL THEN TRIM(
                    CASE WHEN st.stock_metres_used > 0 THEN printf('%.1fm printed', st.stock_metres_used) ELSE '' END ||
                    CASE WHEN st.material_type IS NOT NULL AND TRIM(st.material_type) != '' THEN ' · ' || st.material_type ELSE '' END ||
                    CASE WHEN st.material_size IS NOT NULL AND TRIM(st.material_size) != '' THEN ' · ' || st.material_size || 'm wide' ELSE '' END
                  )
                  ELSE NULL
                END AS source_detail,
                s.type AS source_sale_type,
                s.product_type AS source_product_type,
                COALESCE(p.color, sk.color) AS source_color,
                COALESCE(s.sticker_type, sk.sticker_type) AS source_sticker_type
             FROM debts d
             LEFT JOIN sales s ON s.id = d.sale_id
             LEFT JOIN products p ON p.id = s.product_id
             LEFT JOIN stock sk ON sk.id = s.stock_id
             LEFT JOIN service_transactions st ON st.id = d.service_transaction_id
             {}
             ORDER BY {}
             LIMIT ?{} OFFSET ?{}",
            search_filter,
            order_by,
            if search.is_empty() { 1 } else { 2 },
            if search.is_empty() { 2 } else { 3 },
        );

        let items = if search.is_empty() {
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![per_page as i64, offset], |row| {
                    Ok(Debt {
                        id: row.get(0)?,
                        customer_name: row.get(1)?,
                        phone: row.get(2)?,
                        amount: row.get(3)?,
                        paid_amount: row.get(4)?,
                        remaining_amount: row.get(5)?,
                        due_date: row.get(6)?,
                        description: row.get(7)?,
                        status: row.get(8)?,
                        sale_id: row.get(9)?,
                        service_transaction_id: row.get(10)?,
                        paid_at: row.get(11)?,
                        last_payment_at: row.get(12)?,
                        created_at: row.get(13)?,
                        source_label: row.get(14)?,
                        source_kind: row.get(15)?,
                        source_detail: row.get(16)?,
                        source_sale_type: row.get(17)?,
                        source_product_type: row.get(18)?,
                        source_color: row.get(19)?,
                        source_sticker_type: row.get(20)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        } else {
            let term = format!("%{}%", search);
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![term, per_page as i64, offset], |row| {
                    Ok(Debt {
                        id: row.get(0)?,
                        customer_name: row.get(1)?,
                        phone: row.get(2)?,
                        amount: row.get(3)?,
                        paid_amount: row.get(4)?,
                        remaining_amount: row.get(5)?,
                        due_date: row.get(6)?,
                        description: row.get(7)?,
                        status: row.get(8)?,
                        sale_id: row.get(9)?,
                        service_transaction_id: row.get(10)?,
                        paid_at: row.get(11)?,
                        last_payment_at: row.get(12)?,
                        created_at: row.get(13)?,
                        source_label: row.get(14)?,
                        source_kind: row.get(15)?,
                        source_detail: row.get(16)?,
                        source_sale_type: row.get(17)?,
                        source_product_type: row.get(18)?,
                        source_color: row.get(19)?,
                        source_sticker_type: row.get(20)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let total_outstanding: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(remaining_amount), 0) FROM debts WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let paid_this_month: f64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM debt_payments WHERE strftime('%Y-%m', payment_date) = strftime('%Y-%m', 'now')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        let overdue_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
             FROM debts
             WHERE status = 'pending' AND due_date IS NOT NULL AND DATE(due_date) < DATE('now')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let mut all_stmt = conn.prepare(
            "SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN COALESCE(
                    NULLIF(TRIM(s.product_name), ''),
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Life Saver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE NULL
                    END,
                    CASE
                      WHEN s.type = 'stock' THEN TRIM(COALESCE(sk.color, '') || ' ' || CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Sticker' END)
                      WHEN s.type = 'service' THEN 'Service sale'
                      WHEN s.type = 'product' THEN 'Product sale'
                      ELSE 'Sale'
                    END
                  )
                  WHEN d.service_transaction_id IS NOT NULL THEN COALESCE(NULLIF(TRIM(st.service_name), ''), 'Printing job')
                  ELSE COALESCE(NULLIF(TRIM(d.description), ''), 'Manual debt')
                END AS source_label,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN 'sale'
                  WHEN d.service_transaction_id IS NOT NULL THEN 'printing'
                  ELSE 'manual'
                END AS source_kind,
                CASE
                  WHEN d.sale_id IS NOT NULL AND s.type = 'product' THEN TRIM(
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Lifesaver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE COALESCE(s.product_type, 'Product')
                    END ||
                    CASE WHEN COALESCE(p.color, '') != '' THEN ' · ' || CASE p.color
                      WHEN 'white_red' THEN 'White / Red'
                      WHEN 'yellow_red' THEN 'Yellow / Red'
                      WHEN 'white' THEN 'White'
                      WHEN 'yellow' THEN 'Yellow'
                      ELSE p.color
                    END ELSE '' END ||
                    CASE WHEN COALESCE(s.quantity, '') != '' THEN ' · qty ' || s.quantity ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL AND s.type = 'stock' THEN TRIM(
                    CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Colored' END ||
                    CASE WHEN COALESCE(sk.color, '') != '' THEN ' · ' || sk.color ELSE '' END ||
                    CASE WHEN COALESCE(sk.size, s.quantity, '') != '' THEN ' · ' || COALESCE(sk.size, s.quantity) ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL THEN TRIM(COALESCE(s.quantity, ''))
                  WHEN d.service_transaction_id IS NOT NULL THEN TRIM(
                    CASE WHEN st.stock_metres_used > 0 THEN printf('%.1fm printed', st.stock_metres_used) ELSE '' END ||
                    CASE WHEN st.material_type IS NOT NULL AND TRIM(st.material_type) != '' THEN ' · ' || st.material_type ELSE '' END ||
                    CASE WHEN st.material_size IS NOT NULL AND TRIM(st.material_size) != '' THEN ' · ' || st.material_size || 'm wide' ELSE '' END
                  )
                  ELSE NULL
                END AS source_detail,
                s.type AS source_sale_type,
                s.product_type AS source_product_type,
                COALESCE(p.color, sk.color) AS source_color,
                COALESCE(s.sticker_type, sk.sticker_type) AS source_sticker_type
             FROM debts d
             LEFT JOIN sales s ON s.id = d.sale_id
             LEFT JOIN products p ON p.id = s.product_id
             LEFT JOIN stock sk ON sk.id = s.stock_id
             LEFT JOIN service_transactions st ON st.id = d.service_transaction_id
             ORDER BY d.created_at DESC"
        ).map_err(|e| e.to_string())?;

        let all_rows = all_stmt
            .query_map([], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                    source_label: row.get(14)?,
                    source_kind: row.get(15)?,
                    source_detail: row.get(16)?,
                    source_sale_type: row.get(17)?,
                    source_product_type: row.get(18)?,
                    source_color: row.get(19)?,
                    source_sticker_type: row.get(20)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let all_debts = all_rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(DebtsPageData {
            items,
            total_count,
            total_outstanding,
            paid_this_month,
            overdue_count,
            all_debts,
        })
    }

    pub fn get_paid_this_month(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM debt_payments WHERE strftime('%Y-%m', payment_date) = strftime('%Y-%m', 'now')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn get_overdue_debts(&self) -> Result<Vec<Debt>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT d.id, d.customer_name, d.phone, d.amount, d.paid_amount, d.remaining_amount, d.due_date, d.description, d.status, d.sale_id, d.service_transaction_id, d.paid_at, (SELECT MAX(dp.payment_date) FROM debt_payments dp WHERE dp.debt_id = d.id) AS last_payment_at, d.created_at,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN COALESCE(
                    NULLIF(TRIM(s.product_name), ''),
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Life Saver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE NULL
                    END,
                    CASE
                      WHEN s.type = 'stock' THEN TRIM(COALESCE(sk.color, '') || ' ' || CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Sticker' END)
                      WHEN s.type = 'service' THEN 'Service sale'
                      WHEN s.type = 'product' THEN 'Product sale'
                      ELSE 'Sale'
                    END
                  )
                  WHEN d.service_transaction_id IS NOT NULL THEN COALESCE(NULLIF(TRIM(st.service_name), ''), 'Printing job')
                  ELSE COALESCE(NULLIF(TRIM(d.description), ''), 'Manual debt')
                END AS source_label,
                CASE
                  WHEN d.sale_id IS NOT NULL THEN 'sale'
                  WHEN d.service_transaction_id IS NOT NULL THEN 'printing'
                  ELSE 'manual'
                END AS source_kind,
                CASE
                  WHEN d.sale_id IS NOT NULL AND s.type = 'product' THEN TRIM(
                    CASE s.product_type
                      WHEN 'life_saver' THEN 'Lifesaver'
                      WHEN 'chevron' THEN 'Chevron'
                      WHEN 'stripes' THEN 'Stripes'
                      ELSE COALESCE(s.product_type, 'Product')
                    END ||
                    CASE WHEN COALESCE(p.color, '') != '' THEN ' · ' || CASE p.color
                      WHEN 'white_red' THEN 'White / Red'
                      WHEN 'yellow_red' THEN 'Yellow / Red'
                      WHEN 'white' THEN 'White'
                      WHEN 'yellow' THEN 'Yellow'
                      ELSE p.color
                    END ELSE '' END ||
                    CASE WHEN COALESCE(s.quantity, '') != '' THEN ' · qty ' || s.quantity ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL AND s.type = 'stock' THEN TRIM(
                    CASE WHEN COALESCE(s.sticker_type, sk.sticker_type, '') = 'reflective' THEN 'Reflective' ELSE 'Colored' END ||
                    CASE WHEN COALESCE(sk.color, '') != '' THEN ' · ' || sk.color ELSE '' END ||
                    CASE WHEN COALESCE(sk.size, s.quantity, '') != '' THEN ' · ' || COALESCE(sk.size, s.quantity) ELSE '' END
                  )
                  WHEN d.sale_id IS NOT NULL THEN TRIM(COALESCE(s.quantity, ''))
                  WHEN d.service_transaction_id IS NOT NULL THEN TRIM(
                    CASE WHEN st.stock_metres_used > 0 THEN printf('%.1fm printed', st.stock_metres_used) ELSE '' END ||
                    CASE WHEN st.material_type IS NOT NULL AND TRIM(st.material_type) != '' THEN ' · ' || st.material_type ELSE '' END ||
                    CASE WHEN st.material_size IS NOT NULL AND TRIM(st.material_size) != '' THEN ' · ' || st.material_size || 'm wide' ELSE '' END
                  )
                  ELSE NULL
                END AS source_detail,
                s.type AS source_sale_type,
                s.product_type AS source_product_type,
                COALESCE(p.color, sk.color) AS source_color,
                COALESCE(s.sticker_type, sk.sticker_type) AS source_sticker_type
             FROM debts d
             LEFT JOIN sales s ON s.id = d.sale_id
             LEFT JOIN products p ON p.id = s.product_id
             LEFT JOIN stock sk ON sk.id = s.stock_id
             LEFT JOIN service_transactions st ON st.id = d.service_transaction_id WHERE d.status = 'pending' AND d.due_date IS NOT NULL AND DATE(d.due_date) < DATE('now')")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Debt {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    phone: row.get(2)?,
                    amount: row.get(3)?,
                    paid_amount: row.get(4)?,
                    remaining_amount: row.get(5)?,
                    due_date: row.get(6)?,
                    description: row.get(7)?,
                    status: row.get(8)?,
                    sale_id: row.get(9)?,
                    service_transaction_id: row.get(10)?,
                    paid_at: row.get(11)?,
                    last_payment_at: row.get(12)?,
                    created_at: row.get(13)?,
                    source_label: row.get(14)?,
                    source_kind: row.get(15)?,
                    source_detail: row.get(16)?,
                    source_sale_type: row.get(17)?,
                    source_product_type: row.get(18)?,
                    source_color: row.get(19)?,
                    source_sticker_type: row.get(20)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    // ==================== Debt Payments CRUD ====================

    pub fn get_debt_payments(&self, debt_id: i64) -> Result<Vec<DebtPayment>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, debt_id, amount, payment_method, notes, payment_date FROM debt_payments WHERE debt_id = ?1 ORDER BY payment_date DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![debt_id], |row| {
                Ok(DebtPayment {
                    id: row.get(0)?,
                    debt_id: row.get(1)?,
                    amount: row.get(2)?,
                    payment_method: row.get(3)?,
                    notes: row.get(4)?,
                    payment_date: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_debt_payment(&self, payment: NewDebtPayment) -> Result<DebtPayment, String> {
        let conn = self.synced_conn()?;
        let payment_date = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();

        let id = new_distributed_id();
        conn.execute(
            "INSERT INTO debt_payments (id, debt_id, amount, payment_method, notes, payment_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, payment.debt_id, payment.amount, payment.payment_method, payment.notes, payment_date],
        ).map_err(|e| e.to_string())?;

        // Update debt paid_amount, remaining_amount, status
        let debt: (f64, f64) = conn
            .query_row(
                "SELECT paid_amount, amount FROM debts WHERE id = ?1",
                params![payment.debt_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let new_paid_amount = debt.0 + payment.amount;
        let new_remaining = (debt.1 - new_paid_amount).max(0.0);
        let new_status = if new_remaining <= 0.0 {
            "paid"
        } else {
            "pending"
        };
        let paid_at_value = if new_remaining <= 0.0 {
            Some(payment_date.clone())
        } else {
            None
        };

        conn.execute(
            "UPDATE debts SET paid_amount = ?1, remaining_amount = ?2, status = ?3, paid_at = COALESCE(?4, paid_at) WHERE id = ?5",
            params![new_paid_amount, new_remaining, new_status, paid_at_value, payment.debt_id],
        ).map_err(|e| e.to_string())?;

        self.finish_write(DebtPayment {
            id,
            debt_id: payment.debt_id,
            amount: payment.amount,
            payment_method: payment.payment_method,
            notes: payment.notes,
            payment_date: Some(payment_date),
        })
    }

    pub fn delete_debt_payment(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let payment: (i64, f64) = conn
            .query_row(
                "SELECT debt_id, amount FROM debt_payments WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let debt: (f64, f64) = conn
            .query_row(
                "SELECT paid_amount, amount FROM debts WHERE id = ?1",
                params![payment.0],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let new_paid_amount = (debt.0 - payment.1).max(0.0);
        let new_remaining = debt.1 - new_paid_amount;
        let new_status = if new_remaining > 0.0 {
            "pending"
        } else {
            "paid"
        };

        conn.execute(
            "UPDATE debts SET paid_amount = ?1, remaining_amount = ?2, status = ?3 WHERE id = ?4",
            params![new_paid_amount, new_remaining, new_status, payment.0],
        )
        .map_err(|e| e.to_string())?;

        conn.execute("DELETE FROM debt_payments WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;

        self.finish_write(())
    }

    // ==================== Services CRUD ====================

    pub fn get_all_services(&self) -> Result<Vec<Service>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Service {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    price: row.get(3)?,
                    unit: row.get(4)?,
                    uses_stock: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_active_services(&self) -> Result<Vec<Service>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE is_active = 1 ORDER BY name")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Service {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    price: row.get(3)?,
                    unit: row.get(4)?,
                    uses_stock: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_service(&self, id: i64) -> Result<Option<Service>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(Service {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    price: row.get(3)?,
                    unit: row.get(4)?,
                    uses_stock: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn add_service(&self, service: NewService) -> Result<Service, String> {
        let conn = self.synced_conn()?;
        let created_at = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();
        let id = new_distributed_id();

        conn.execute(
            "INSERT INTO services (id, name, description, price, unit, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, service.name, service.description, service.price.unwrap_or(0.0), service.unit, service.is_active, created_at],
        ).map_err(|e| e.to_string())?;

        // IMPORTANT: do not call self.get_service(id) here while holding the mutex,
        // because get_service() also locks the same mutex and deadlocks the UI.
        let service = conn.query_row(
            "SELECT id, name, description, price, unit, uses_stock, is_active, created_at, updated_at FROM services WHERE id = ?1",
            params![id],
            |row| {
                Ok(Service {
                    id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                    price: row.get(3)?, unit: row.get(4)?, uses_stock: row.get(5)?,
                    is_active: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
                })
            },
        ).map_err(|e| e.to_string())?;
        self.finish_write(service)
    }

    pub fn update_service(&self, id: i64, updates: ServiceUpdate) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.name {
            set_clauses.push("name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.description {
            set_clauses.push("description = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.price {
            set_clauses.push("price = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.unit {
            set_clauses.push("unit = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.is_active {
            set_clauses.push("is_active = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!(
            "UPDATE services SET {} WHERE id = ?",
            set_clauses.join(", ")
        );
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_service(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM services WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Service Transactions CRUD ====================

    pub fn get_all_service_transactions(&self) -> Result<Vec<ServiceTransaction>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp,
                CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount
                     ELSE COALESCE((SELECT d.paid_amount FROM debts d WHERE d.service_transaction_id = service_transactions.id LIMIT 1), 0)
                END AS amount_paid
             FROM service_transactions ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ServiceTransaction {
                    id: row.get(0)?,
                    service_id: row.get(1)?,
                    service_name: row.get(2)?,
                    quantity: row.get(3)?,
                    price: row.get(4)?,
                    amount: row.get(5)?,
                    payment_method: row.get(6)?,
                    customer_name: row.get(7)?,
                    notes: row.get(8)?,
                    stock_id: row.get(9)?,
                    stock_metres_used: row.get(10)?,
                    material_size: row.get(11)?,
                    material_type: row.get(12)?,
                    printing_material_id: row.get(13)?,
                    is_debt: row.get(14)?,
                    timestamp: row.get(15)?,
                    amount_paid: row.get(16)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_today_service_transactions(&self) -> Result<Vec<ServiceTransaction>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp,
                CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount
                     ELSE COALESCE((SELECT d.paid_amount FROM debts d WHERE d.service_transaction_id = service_transactions.id LIMIT 1), 0)
                END AS amount_paid
             FROM service_transactions WHERE DATE(timestamp) = DATE('now', 'localtime') ORDER BY timestamp DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ServiceTransaction {
                    id: row.get(0)?,
                    service_id: row.get(1)?,
                    service_name: row.get(2)?,
                    quantity: row.get(3)?,
                    price: row.get(4)?,
                    amount: row.get(5)?,
                    payment_method: row.get(6)?,
                    customer_name: row.get(7)?,
                    notes: row.get(8)?,
                    stock_id: row.get(9)?,
                    stock_metres_used: row.get(10)?,
                    material_size: row.get(11)?,
                    material_type: row.get(12)?,
                    printing_material_id: row.get(13)?,
                    is_debt: row.get(14)?,
                    timestamp: row.get(15)?,
                    amount_paid: row.get(16)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_service_transaction(
        &self,
        tx: NewServiceTransaction,
    ) -> Result<ServiceTransaction, String> {
        let conn = self.synced_conn()?;
        let timestamp = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();
        let amount = tx.amount.unwrap_or(tx.quantity * tx.price.unwrap_or(0.0));

        conn.execute_batch("BEGIN IMMEDIATE;")
            .map_err(|e| e.to_string())?;

        let insert_result = (|| -> Result<i64, String> {
            // Auto-deduct stock / materials first so concurrent PCs cannot oversell metres
            if let (Some(stock_id), stock_metres) = (tx.stock_id, tx.stock_metres_used) {
                if stock_metres > 0.0 {
                    let updated = conn
                        .execute(
                            "UPDATE stock SET metres_used = metres_used + ?1, updated_at = CURRENT_TIMESTAMP
                             WHERE id = ?2 AND (total_metres - metres_used) >= ?1",
                            params![stock_metres, stock_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if updated == 0 {
                        return Err("Insufficient stock metres or stock item was not found.".into());
                    }
                }
            }

            if let Some(material_id) = tx.printing_material_id {
                if tx.stock_metres_used > 0.0 {
                    let updated = conn
                        .execute(
                            "UPDATE printing_materials SET metres_used = metres_used + ?1, updated_at = CURRENT_TIMESTAMP
                             WHERE id = ?2 AND (total_metres - metres_used) >= ?1",
                            params![tx.stock_metres_used, material_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if updated == 0 {
                        return Err(
                            "Insufficient printing material or material was not found.".into()
                        );
                    }
                }
            }

            let id = new_distributed_id();
            conn.execute(
                "INSERT INTO service_transactions (id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![id, tx.service_id, tx.service_name, tx.quantity, tx.price.unwrap_or(0.0), amount, tx.payment_method, tx.customer_name, tx.notes, tx.stock_id, tx.stock_metres_used, tx.material_size, tx.material_type, tx.printing_material_id, tx.is_debt, timestamp],
            ).map_err(|e| e.to_string())?;

            Ok(id)
        })();

        let id = match insert_result {
            Ok(id) => {
                conn.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
                id
            }
            Err(err) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(err);
            }
        };

        drop(conn);
        self.finish_write(ServiceTransaction {
            id,
            service_id: tx.service_id,
            service_name: tx.service_name,
            quantity: tx.quantity,
            price: tx.price.unwrap_or(0.0),
            amount,
            payment_method: tx.payment_method,
            customer_name: tx.customer_name,
            notes: tx.notes,
            stock_id: tx.stock_id,
            stock_metres_used: tx.stock_metres_used,
            material_size: tx.material_size,
            material_type: tx.material_type,
            printing_material_id: tx.printing_material_id,
            is_debt: tx.is_debt,
            timestamp: Some(timestamp),
            amount_paid: if tx.is_debt == 0 { amount } else { 0.0 },
        })
    }

    pub fn get_today_total_service_earnings(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        conn.query_row(
            "SELECT
                COALESCE((SELECT SUM(amount) FROM service_transactions
                          WHERE DATE(timestamp) = DATE('now', 'localtime')
                            AND COALESCE(is_debt, 0) = 0), 0)
              + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                          INNER JOIN debts d ON d.id = dp.debt_id
                          WHERE d.service_transaction_id IS NOT NULL
                            AND DATE(dp.payment_date) = DATE('now', 'localtime')), 0)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    }

    pub fn get_total_service_earnings(&self) -> Result<f64, String> {
        let conn = self.synced_conn()?;
        conn.query_row(
            "SELECT
                COALESCE((SELECT SUM(amount) FROM service_transactions WHERE COALESCE(is_debt, 0) = 0), 0)
              + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                          INNER JOIN debts d ON d.id = dp.debt_id
                          WHERE d.service_transaction_id IS NOT NULL), 0)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    }

    pub fn get_printing_page(&self, query: PrintingPageQuery) -> Result<PrintingPageData, String> {
        let conn = self.synced_conn()?;

        let search = query.search.unwrap_or_default().trim().to_lowercase();
        let sort_by = query.sort_by.unwrap_or_else(|| "newest".to_string());
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(10).clamp(1, 200);
        let offset = ((page - 1) * per_page) as i64;

        let base_filter = "stock_metres_used > 0";
        let search_filter = if search.is_empty() {
            base_filter.to_string()
        } else {
            format!(
                "{} AND (
                    LOWER(service_name) LIKE ?1 OR
                    LOWER(customer_name) LIKE ?1 OR
                    LOWER(payment_method) LIKE ?1 OR
                    LOWER(COALESCE(material_type, '')) LIKE ?1 OR
                    LOWER(COALESCE(material_size, '')) LIKE ?1 OR
                    LOWER(COALESCE(timestamp, '')) LIKE ?1
                )",
                base_filter
            )
        };

        let order_by = match sort_by.as_str() {
            "oldest" => "timestamp ASC",
            "amount_desc" => "amount DESC, timestamp DESC",
            "amount_asc" => "amount ASC, timestamp DESC",
            _ => "timestamp DESC",
        };

        let total_count: i64 = if search.is_empty() {
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM service_transactions WHERE {}",
                    base_filter
                ),
                [],
                |row| row.get(0),
            )
        } else {
            let term = format!("%{}%", search);
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM service_transactions WHERE {}",
                    search_filter
                ),
                params![term],
                |row| row.get(0),
            )
        }
        .map_err(|e| e.to_string())?;

        let sql = format!(
            "SELECT id, service_id, service_name, quantity, price, amount, payment_method, customer_name, notes, stock_id, stock_metres_used, material_size, material_type, printing_material_id, is_debt, timestamp,
                CASE WHEN COALESCE(is_debt, 0) = 0 THEN amount
                     ELSE COALESCE((SELECT d.paid_amount FROM debts d WHERE d.service_transaction_id = service_transactions.id LIMIT 1), 0)
                END AS amount_paid
             FROM service_transactions
             WHERE {}
             ORDER BY {}
             LIMIT ?{} OFFSET ?{}",
            search_filter,
            order_by,
            if search.is_empty() { 1 } else { 2 },
            if search.is_empty() { 2 } else { 3 },
        );

        let items = if search.is_empty() {
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![per_page as i64, offset], |row| {
                    Ok(ServiceTransaction {
                        id: row.get(0)?,
                        service_id: row.get(1)?,
                        service_name: row.get(2)?,
                        quantity: row.get(3)?,
                        price: row.get(4)?,
                        amount: row.get(5)?,
                        payment_method: row.get(6)?,
                        customer_name: row.get(7)?,
                        notes: row.get(8)?,
                        stock_id: row.get(9)?,
                        stock_metres_used: row.get(10)?,
                        material_size: row.get(11)?,
                        material_type: row.get(12)?,
                        printing_material_id: row.get(13)?,
                        is_debt: row.get(14)?,
                        timestamp: row.get(15)?,
                        amount_paid: row.get(16)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        } else {
            let term = format!("%{}%", search);
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![term, per_page as i64, offset], |row| {
                    Ok(ServiceTransaction {
                        id: row.get(0)?,
                        service_id: row.get(1)?,
                        service_name: row.get(2)?,
                        quantity: row.get(3)?,
                        price: row.get(4)?,
                        amount: row.get(5)?,
                        payment_method: row.get(6)?,
                        customer_name: row.get(7)?,
                        notes: row.get(8)?,
                        stock_id: row.get(9)?,
                        stock_metres_used: row.get(10)?,
                        material_size: row.get(11)?,
                        material_type: row.get(12)?,
                        printing_material_id: row.get(13)?,
                        is_debt: row.get(14)?,
                        timestamp: row.get(15)?,
                        amount_paid: row.get(16)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let today_earnings: f64 = conn
            .query_row(
                "SELECT
                    COALESCE((SELECT SUM(amount) FROM service_transactions
                              WHERE stock_metres_used > 0
                                AND DATE(timestamp) = DATE('now', 'localtime')
                                AND COALESCE(is_debt, 0) = 0), 0)
                  + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                              INNER JOIN debts d ON d.id = dp.debt_id
                              INNER JOIN service_transactions st ON st.id = d.service_transaction_id
                              WHERE st.stock_metres_used > 0
                                AND DATE(dp.payment_date) = DATE('now', 'localtime')), 0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let total_jobs_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM service_transactions WHERE stock_metres_used > 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let material_used: f64 = conn.query_row(
            "SELECT COALESCE(SUM(stock_metres_used), 0) FROM service_transactions WHERE stock_metres_used > 0",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        let total_revenue: f64 = conn
            .query_row(
                "SELECT
                COALESCE((SELECT SUM(amount) FROM service_transactions
                          WHERE stock_metres_used > 0 AND COALESCE(is_debt, 0) = 0), 0)
              + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                          INNER JOIN debts d ON d.id = dp.debt_id
                          INNER JOIN service_transactions st ON st.id = d.service_transaction_id
                          WHERE st.stock_metres_used > 0), 0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        Ok(PrintingPageData {
            items,
            total_count,
            today_earnings,
            total_jobs_count,
            material_used,
            total_revenue,
        })
    }

    pub fn update_service_transaction(
        &self,
        id: i64,
        updates: ServiceTransactionUpdate,
    ) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.service_name {
            set_clauses.push("service_name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.quantity {
            set_clauses.push("quantity = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.price {
            set_clauses.push("price = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.amount {
            set_clauses.push("amount = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.payment_method {
            set_clauses.push("payment_method = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.customer_name {
            set_clauses.push("customer_name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.notes {
            set_clauses.push("notes = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.stock_id {
            set_clauses.push("stock_id = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.stock_metres_used {
            set_clauses.push("stock_metres_used = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.material_size {
            set_clauses.push("material_size = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.material_type {
            set_clauses.push("material_type = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.is_debt {
            set_clauses.push("is_debt = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        let sql = format!(
            "UPDATE service_transactions SET {} WHERE id = ?",
            set_clauses.join(", ")
        );
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn delete_service_transaction(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;

        let txn: Option<(Option<i64>, f64)> = conn
            .query_row(
                "SELECT printing_material_id, stock_metres_used FROM service_transactions WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((Some(material_id), used_metres)) = txn {
            let current_metres_used: Option<f64> = conn
                .query_row(
                    "SELECT metres_used FROM printing_materials WHERE id = ?1",
                    params![material_id],
                    |row| row.get(0),
                )
                .ok();
            if let Some(current_used) = current_metres_used {
                let new_used = (current_used - used_metres).max(0.0);
                conn.execute(
                    "UPDATE printing_materials SET metres_used = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                    params![new_used, material_id],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        conn.execute(
            "DELETE FROM service_transactions WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Printing Materials CRUD ====================

    pub fn get_all_printing_materials(&self) -> Result<Vec<PrintingMaterial>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM printing_materials ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(PrintingMaterial {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    material_type: row.get(2)?,
                    width: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    color: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_printing_material(&self, id: i64) -> Result<Option<PrintingMaterial>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at FROM printing_materials WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(PrintingMaterial {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    material_type: row.get(2)?,
                    width: row.get(3)?,
                    rolls: row.get(4)?,
                    metres_per_roll: row.get(5)?,
                    total_metres: row.get(6)?,
                    metres_used: row.get(7)?,
                    color: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn add_printing_material(
        &self,
        material: NewPrintingMaterial,
    ) -> Result<PrintingMaterial, String> {
        let conn = self.synced_conn()?;
        let total_metres = material
            .total_metres
            .unwrap_or(material.rolls as f64 * material.metres_per_roll);
        let natural_key = material_natural_key(
            &material.name,
            &material.material_type,
            material.width,
            &material.color,
        );
        let candidate_id = new_distributed_id();

        conn.execute(
            "INSERT INTO printing_materials (id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, natural_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9)
             ON CONFLICT(natural_key) DO UPDATE SET
                rolls = rolls + excluded.rolls,
                total_metres = total_metres + excluded.total_metres,
                metres_per_roll = excluded.metres_per_roll,
                updated_at = CURRENT_TIMESTAMP",
            params![
                candidate_id,
                material.name,
                material.material_type,
                material.width,
                material.rolls,
                material.metres_per_roll,
                total_metres,
                material.color,
                natural_key
            ],
        )
        .map_err(|e| e.to_string())?;

        let printing_material = conn
            .query_row(
                "SELECT id, name, material_type, width, rolls, metres_per_roll, total_metres, metres_used, color, created_at, updated_at
                 FROM printing_materials WHERE natural_key = ?1",
                params![natural_key],
                |row| {
                    Ok(PrintingMaterial {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        material_type: row.get(2)?,
                        width: row.get(3)?,
                        rolls: row.get(4)?,
                        metres_per_roll: row.get(5)?,
                        total_metres: row.get(6)?,
                        metres_used: row.get(7)?,
                        color: row.get(8)?,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;
        self.finish_write(printing_material)
    }

    /// Atomically add rolls (and metres) to an existing printing material row.
    pub fn add_printing_material_rolls(&self, id: i64, rolls: i64) -> Result<(), String> {
        if rolls <= 0 {
            return Err("Rolls to add must be greater than zero.".into());
        }
        let conn = self.synced_conn()?;
        let updated = conn
            .execute(
                "UPDATE printing_materials SET
                    rolls = rolls + ?1,
                    total_metres = total_metres + (?1 * metres_per_roll),
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?2",
                params![rolls, id],
            )
            .map_err(|e| e.to_string())?;
        if updated == 0 {
            return Err("Printing material was not found.".into());
        }
        self.finish_write(())
    }

    pub fn update_printing_material(
        &self,
        id: i64,
        updates: PrintingMaterialUpdate,
    ) -> Result<(), String> {
        let conn = self.synced_conn()?;
        let touches_identity = updates.name.is_some()
            || updates.material_type.is_some()
            || updates.width.is_some()
            || updates.color.is_some();

        let mut set_clauses = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(v) = updates.name {
            set_clauses.push("name = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.material_type {
            set_clauses.push("material_type = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.width {
            set_clauses.push("width = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.rolls {
            set_clauses.push("rolls = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.metres_per_roll {
            set_clauses.push("metres_per_roll = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.total_metres {
            set_clauses.push("total_metres = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.metres_used {
            set_clauses.push("metres_used = ?".to_string());
            params_vec.push(Box::new(v));
        }
        if let Some(v) = updates.color {
            set_clauses.push("color = ?".to_string());
            params_vec.push(Box::new(v));
        }

        if set_clauses.is_empty() {
            return Ok(());
        }

        set_clauses.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!(
            "UPDATE printing_materials SET {} WHERE id = ?",
            set_clauses.join(", ")
        );
        params_vec.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;

        if touches_identity {
            conn.execute(
                "UPDATE printing_materials SET natural_key =
                    lower(trim(name)) || '|' ||
                    lower(trim(material_type)) || '|' ||
                    printf('%.4f', width) || '|' ||
                    lower(trim(ifnull(color, '')))
                 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| {
                format!(
                    "Could not update material identity (another material may already use that name/type/width/color): {}",
                    e
                )
            })?;
        }
        self.finish_write(())
    }

    pub fn delete_printing_material(&self, id: i64) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM printing_materials WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Users (for auth) ====================

    pub fn get_user_by_username(&self, username: &str) -> Result<Option<UserRow>, String> {
        // Local-only during auth bootstrap so startup never waits on the network.
        let conn = self.local_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, username, password_hash, role, permissions, created_at, updated_at FROM users WHERE username = ?1")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![username], |row| {
                Ok(UserRow {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    role: row.get(3)?,
                    permissions: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn add_user(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
        permissions: &str,
    ) -> Result<(), String> {
        let conn = self.local_conn()?;
        let id = new_distributed_id();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, permissions) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, username, password_hash, role, permissions],
        ).map_err(|e| e.to_string())?;
        // Defer Turso push to background / later writes so first-launch setup cannot hang.
        self.finish_write(())
    }

    pub fn update_user_password(&self, username: &str, new_hash: &str) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute(
            "UPDATE users SET password_hash = ?1, updated_at = CURRENT_TIMESTAMP WHERE username = ?2",
            params![new_hash, username],
        ).map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn update_username(&self, old_username: &str, new_username: &str) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute(
            "UPDATE users SET username = ?1, updated_at = CURRENT_TIMESTAMP WHERE username = ?2",
            params![new_username, old_username],
        )
        .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    pub fn get_all_users(&self) -> Result<Vec<User>, String> {
        let conn = self.synced_conn()?;
        let mut stmt = conn
            .prepare("SELECT id, username, role, created_at FROM users ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    role: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn delete_user(&self, username: &str) -> Result<(), String> {
        let conn = self.synced_conn()?;
        conn.execute("DELETE FROM users WHERE username = ?1", params![username])
            .map_err(|e| e.to_string())?;
        self.finish_write(())
    }

    // ==================== Clear All Data ====================

    pub fn clear_all_data(&self) -> Result<(), String> {
        let conn = self.synced_conn()?;
        Self::clear_business_tables(&conn)?;
        println!("All business data cleared");
        self.finish_write(())
    }

    fn clear_business_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            DELETE FROM debt_payments;
            DELETE FROM debts;
            DELETE FROM sales;
            DELETE FROM service_transactions;
            DELETE FROM services;
            DELETE FROM printing_materials;
            DELETE FROM stock;
            DELETE FROM products;
            DELETE FROM sqlite_sequence WHERE name IN (
                'debt_payments', 'debts', 'sales', 'service_transactions',
                'services', 'printing_materials', 'stock', 'products'
            );
            ",
        )
        .map_err(|e| e.to_string())
    }

    // ==================== Backup export / import ====================

    pub fn export_database_backup(&self) -> Result<DatabaseBackup, String> {
        let conn = self.synced_conn()?;
        Ok(DatabaseBackup {
            format: "multiprints-backup".into(),
            version: 1,
            exported_at: chrono::Local::now().to_rfc3339(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            products: Self::select_table_json(&conn, "products")?,
            stock: Self::select_table_json(&conn, "stock")?,
            services: Self::select_table_json(&conn, "services")?,
            printing_materials: Self::select_table_json(&conn, "printing_materials")?,
            sales: Self::select_table_json(&conn, "sales")?,
            service_transactions: Self::select_table_json(&conn, "service_transactions")?,
            debts: Self::select_table_json(&conn, "debts")?,
            debt_payments: Self::select_table_json(&conn, "debt_payments")?,
        })
    }

    pub fn import_database_backup(&self, backup: DatabaseBackup) -> Result<(), String> {
        if backup.format != "multiprints-backup" {
            return Err(
                "Not a valid MULTIPRINTS backup file (missing or wrong format marker)".into(),
            );
        }
        if backup.version == 0 {
            return Err("Backup file is missing a version field".into());
        }
        if backup.version > 1 {
            return Err(format!(
                "Backup version {} is newer than this app supports",
                backup.version
            ));
        }

        let conn = self.synced_conn()?;
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;

        let result = (|| -> Result<(), String> {
            // Temporarily relax FKs for bulk replace with preserved ids
            let _ = conn.execute_batch("PRAGMA foreign_keys = OFF");
            Self::clear_business_tables(&conn)?;

            Self::insert_json_rows(
                &conn,
                "products",
                &[
                    "id",
                    "name",
                    "product_type",
                    "color",
                    "size",
                    "selling_price",
                    "stock",
                    "natural_key",
                    "created_at",
                    "updated_at",
                ],
                &backup.products,
            )?;
            Self::insert_json_rows(
                &conn,
                "stock",
                &[
                    "id",
                    "color",
                    "size",
                    "sticker_type",
                    "rolls",
                    "metres_per_roll",
                    "total_metres",
                    "metres_used",
                    "natural_key",
                    "created_at",
                    "updated_at",
                ],
                &backup.stock,
            )?;
            Self::insert_json_rows(
                &conn,
                "services",
                &[
                    "id",
                    "name",
                    "description",
                    "price",
                    "unit",
                    "uses_stock",
                    "is_active",
                    "created_at",
                    "updated_at",
                ],
                &backup.services,
            )?;
            Self::insert_json_rows(
                &conn,
                "printing_materials",
                &[
                    "id",
                    "name",
                    "material_type",
                    "width",
                    "rolls",
                    "metres_per_roll",
                    "total_metres",
                    "metres_used",
                    "color",
                    "natural_key",
                    "created_at",
                    "updated_at",
                ],
                &backup.printing_materials,
            )?;
            Self::insert_json_rows(
                &conn,
                "sales",
                &[
                    "id",
                    "type",
                    "product_id",
                    "stock_id",
                    "product_name",
                    "product_type",
                    "sticker_type",
                    "quantity",
                    "amount",
                    "payment_method",
                    "customer_name",
                    "is_debt",
                    "timestamp",
                ],
                &backup.sales,
            )?;
            Self::insert_json_rows(
                &conn,
                "service_transactions",
                &[
                    "id",
                    "service_id",
                    "service_name",
                    "quantity",
                    "price",
                    "amount",
                    "payment_method",
                    "customer_name",
                    "notes",
                    "stock_id",
                    "stock_metres_used",
                    "material_size",
                    "material_type",
                    "printing_material_id",
                    "is_debt",
                    "timestamp",
                ],
                &backup.service_transactions,
            )?;
            Self::insert_json_rows(
                &conn,
                "debts",
                &[
                    "id",
                    "customer_name",
                    "phone",
                    "amount",
                    "paid_amount",
                    "remaining_amount",
                    "due_date",
                    "description",
                    "status",
                    "sale_id",
                    "service_transaction_id",
                    "paid_at",
                    "created_at",
                ],
                &backup.debts,
            )?;
            Self::insert_json_rows(
                &conn,
                "debt_payments",
                &[
                    "id",
                    "debt_id",
                    "amount",
                    "payment_method",
                    "notes",
                    "payment_date",
                ],
                &backup.debt_payments,
            )?;

            let _ = conn.execute_batch("PRAGMA foreign_keys = ON");
            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
                println!(
                    "Imported backup (products={}, sales={}, debts={})",
                    backup.products.len(),
                    backup.sales.len(),
                    backup.debts.len()
                );
                self.finish_write(())
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                let _ = conn.execute_batch("PRAGMA foreign_keys = ON");
                Err(e)
            }
        }
    }

    fn select_table_json(conn: &Connection, table: &str) -> Result<Vec<serde_json::Value>, String> {
        // Table names are hardcoded callers only — never user input.
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {table}"))
            .map_err(|e| e.to_string())?;
        let col_count = stmt.column_count();
        let col_names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("col").to_string())
            .collect();

        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let mut map = serde_json::Map::new();
            for (i, name) in col_names.iter().enumerate() {
                let val = match row.get_ref(i).map_err(|e| e.to_string())? {
                    rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                    rusqlite::types::ValueRef::Integer(n) => {
                        // Keep large distributed ids exact in JSON
                        serde_json::Value::String(n.to_string())
                    }
                    rusqlite::types::ValueRef::Real(f) => serde_json::json!(f),
                    rusqlite::types::ValueRef::Text(t) => {
                        serde_json::Value::String(String::from_utf8_lossy(t).into_owned())
                    }
                    rusqlite::types::ValueRef::Blob(b) => serde_json::Value::String(hex::encode(b)),
                };
                map.insert(name.clone(), val);
            }
            out.push(serde_json::Value::Object(map));
        }
        Ok(out)
    }

    fn insert_json_rows(
        conn: &Connection,
        table: &str,
        columns: &[&str],
        rows: &[serde_json::Value],
    ) -> Result<(), String> {
        if rows.is_empty() {
            return Ok(());
        }
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "INSERT INTO {table} ({cols}) VALUES ({vals})",
            cols = columns.join(", "),
            vals = placeholders.join(", ")
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        for row in rows {
            let obj = row
                .as_object()
                .ok_or_else(|| format!("Invalid row in {table} backup"))?;
            let params: Vec<rusqlite::types::Value> = columns
                .iter()
                .map(|col| Self::json_to_sql_value(obj.get(*col)))
                .collect();
            stmt.execute(rusqlite::params_from_iter(params.iter()))
                .map_err(|e| format!("Failed inserting into {table}: {e}"))?;
        }
        Ok(())
    }

    fn json_to_sql_value(v: Option<&serde_json::Value>) -> rusqlite::types::Value {
        match v {
            None | Some(serde_json::Value::Null) => rusqlite::types::Value::Null,
            Some(serde_json::Value::Bool(b)) => {
                rusqlite::types::Value::Integer(if *b { 1 } else { 0 })
            }
            Some(serde_json::Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    rusqlite::types::Value::Integer(i)
                } else if let Some(u) = n.as_u64() {
                    rusqlite::types::Value::Integer(u as i64)
                } else if let Some(f) = n.as_f64() {
                    rusqlite::types::Value::Real(f)
                } else {
                    rusqlite::types::Value::Null
                }
            }
            // Keep strings as text so phone numbers / sizes keep leading zeros and format.
            // SQLite column affinity still coerces pure digit strings into INTEGER/REAL columns.
            Some(serde_json::Value::String(s)) => rusqlite::types::Value::Text(s.clone()),
            Some(serde_json::Value::Array(_)) | Some(serde_json::Value::Object(_)) => {
                rusqlite::types::Value::Text(v.unwrap().to_string())
            }
        }
    }

    // ==================== Migration from localStorage ====================

    pub fn get_dashboard_summary(&self) -> Result<DashboardSummary, String> {
        let conn = self.synced_conn()?;

        // Cash recognized: non-debt sales/jobs + all debt repayments (partial + full).
        let total_revenue: f64 = conn
            .query_row(
                "SELECT
                COALESCE((SELECT SUM(amount) FROM sales WHERE COALESCE(is_debt, 0) = 0), 0) +
                COALESCE((SELECT SUM(amount) FROM service_transactions WHERE COALESCE(is_debt, 0) = 0), 0) +
                COALESCE((SELECT SUM(amount) FROM debt_payments), 0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let (today_sales_count, today_sales_revenue): (i64, f64) = conn
            .query_row(
                "SELECT COUNT(*),
                    COALESCE((SELECT SUM(amount) FROM sales
                              WHERE DATE(timestamp) = DATE('now', 'localtime')
                                AND COALESCE(is_debt, 0) = 0), 0)
                  + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                              INNER JOIN debts d ON d.id = dp.debt_id
                              WHERE d.sale_id IS NOT NULL
                                AND DATE(dp.payment_date) = DATE('now', 'localtime')), 0)
             FROM sales
             WHERE DATE(timestamp) = DATE('now', 'localtime')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let (today_service_count, today_service_revenue): (i64, f64) = conn
            .query_row(
                "SELECT COUNT(*),
                    COALESCE((SELECT SUM(amount) FROM service_transactions
                              WHERE DATE(timestamp) = DATE('now', 'localtime')
                                AND COALESCE(is_debt, 0) = 0), 0)
                  + COALESCE((SELECT SUM(dp.amount) FROM debt_payments dp
                              INNER JOIN debts d ON d.id = dp.debt_id
                              WHERE d.service_transaction_id IS NOT NULL
                                AND DATE(dp.payment_date) = DATE('now', 'localtime')), 0)
             FROM service_transactions
             WHERE DATE(timestamp) = DATE('now', 'localtime')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let (outstanding_debts, pending_debts_count): (f64, i64) = conn
            .query_row(
                "SELECT COALESCE(SUM(remaining_amount), 0), COUNT(*)
             FROM debts
             WHERE status = 'pending'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let recent_transactions = {
            let mut stmt = conn.prepare(
                "SELECT COALESCE(product_name, type), COALESCE(SUBSTR(timestamp, 1, 10), ''), amount, is_debt, 'Sale', COALESCE(timestamp, '')
                 FROM sales
                 UNION ALL
                 SELECT service_name, COALESCE(SUBSTR(timestamp, 1, 10), ''), amount, is_debt, 'Printing', COALESCE(timestamp, '')
                 FROM service_transactions
                 WHERE stock_metres_used > 0
                 ORDER BY 6 DESC
                 LIMIT 5"
            ).map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(DashboardRecentTransaction {
                        name: row.get(0)?,
                        date: row.get(1)?,
                        amount: row.get(2)?,
                        is_debt: row.get::<_, i64>(3)? > 0,
                        type_label: row.get(4)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let activity_items = {
            let mut stmt = conn
                .prepare(
                    "SELECT item_type, text, time FROM (
                    SELECT 'sale' AS item_type,
                           printf('%s — KSh %.0f', COALESCE(product_name, type), amount) AS text,
                           COALESCE(SUBSTR(timestamp, 12, 5), '') AS time,
                           COALESCE(timestamp, '') AS sort_ts
                    FROM sales
                    UNION ALL
                    SELECT 'debt' AS item_type,
                           printf('Debt: %s — KSh %.0f', customer_name, amount) AS text,
                           COALESCE(SUBSTR(created_at, 12, 5), '') AS time,
                           COALESCE(created_at, '') AS sort_ts
                    FROM debts
                )
                ORDER BY sort_ts DESC
                LIMIT 8",
                )
                .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(DashboardActivityItem {
                        item_type: row.get(0)?,
                        text: row.get(1)?,
                        time: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        let top_products = {
            let mut stmt = conn.prepare(
                "SELECT s.product_id,
                        COALESCE(p.name, COALESCE(s.product_name, printf('Product #%d', s.product_id))),
                        CAST(SUM(COALESCE(CAST(s.quantity AS INTEGER), 1)) AS INTEGER) AS qty
                 FROM sales s
                 LEFT JOIN products p ON p.id = s.product_id
                 WHERE s.product_id IS NOT NULL
                 GROUP BY s.product_id, COALESCE(p.name, COALESCE(s.product_name, printf('Product #%d', s.product_id)))
                 ORDER BY qty DESC
                 LIMIT 4"
            ).map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(DashboardTopProduct {
                        product_id: row.get(0)?,
                        name: row.get(1)?,
                        quantity: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };

        Ok(DashboardSummary {
            total_revenue,
            today_sales_count: today_sales_count + today_service_count,
            today_revenue: today_sales_revenue + today_service_revenue,
            outstanding_debts,
            pending_debts_count,
            recent_transactions,
            activity_items,
            top_products,
        })
    }

    pub fn get_dashboard_chart(&self, period: &str) -> Result<Vec<DashboardChartPoint>, String> {
        let conn = self.synced_conn()?;

        // Helper: map (label, amount, sales_count, debt_amount) rows into chart points.
        let map_points = |rows: Vec<(String, f64, f64, f64)>| -> Vec<DashboardChartPoint> {
            rows.into_iter()
                .map(
                    |(label, amount, sales_count, debt_amount)| DashboardChartPoint {
                        label,
                        amount,
                        sales_count,
                        debt_amount,
                    },
                )
                .collect()
        };

        let points = match period {
            "month" => {
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE weeks(idx, start_date, end_date) AS (
                        SELECT 0, DATE('now', 'localtime', '-6 days'), DATE('now', 'localtime')
                        UNION ALL
                        SELECT idx + 1,
                               DATE(start_date, '-7 days'),
                               DATE(end_date, '-7 days')
                        FROM weeks
                        WHERE idx < 3
                    ), revenues AS (
                        SELECT DATE(timestamp) AS tx_date, amount FROM sales WHERE COALESCE(is_debt, 0) = 0
                        UNION ALL
                        SELECT DATE(timestamp) AS tx_date, amount FROM service_transactions WHERE COALESCE(is_debt, 0) = 0
                        UNION ALL
                        SELECT DATE(payment_date) AS tx_date, amount FROM debt_payments
                    ), tx_counts AS (
                        SELECT DATE(timestamp) AS tx_date FROM sales
                        UNION ALL
                        SELECT DATE(timestamp) AS tx_date FROM service_transactions
                    )
                    SELECT strftime('%d', start_date) || '–' || strftime('%d', end_date) || ' ' ||
                           CASE strftime('%m', end_date)
                               WHEN '01' THEN 'Jan'
                               WHEN '02' THEN 'Feb'
                               WHEN '03' THEN 'Mar'
                               WHEN '04' THEN 'Apr'
                               WHEN '05' THEN 'May'
                               WHEN '06' THEN 'Jun'
                               WHEN '07' THEN 'Jul'
                               WHEN '08' THEN 'Aug'
                               WHEN '09' THEN 'Sep'
                               WHEN '10' THEN 'Oct'
                               WHEN '11' THEN 'Nov'
                               WHEN '12' THEN 'Dec'
                           END AS label,
                           COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.tx_date BETWEEN w.start_date AND w.end_date), 0) AS amount,
                           COALESCE((SELECT COUNT(*) FROM tx_counts t WHERE t.tx_date BETWEEN w.start_date AND w.end_date), 0) AS sales_count,
                           COALESCE((
                               SELECT SUM(remaining_amount) FROM debts d
                               WHERE d.status = 'pending'
                                 AND DATE(d.created_at) BETWEEN w.start_date AND w.end_date
                           ), 0) AS debt_amount,
                           idx
                    FROM weeks w
                    ORDER BY idx DESC"
                ).map_err(|e| e.to_string())?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, f64>(1)?,
                            row.get::<_, f64>(2)?,
                            row.get::<_, f64>(3)?,
                        ))
                    })
                    .map_err(|e| e.to_string())?;

                map_points(
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?,
                )
            }
            "year" => {
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE months(idx, month_start) AS (
                        SELECT 0, DATE('now', 'localtime', 'start of month')
                        UNION ALL
                        SELECT idx + 1, DATE(month_start, '-1 month')
                        FROM months
                        WHERE idx < 11
                    ), revenues AS (
                        SELECT strftime('%Y-%m', timestamp) AS ym, amount FROM sales WHERE COALESCE(is_debt, 0) = 0
                        UNION ALL
                        SELECT strftime('%Y-%m', timestamp) AS ym, amount FROM service_transactions WHERE COALESCE(is_debt, 0) = 0
                        UNION ALL
                        SELECT strftime('%Y-%m', payment_date) AS ym, amount FROM debt_payments
                    ), tx_counts AS (
                        SELECT strftime('%Y-%m', timestamp) AS ym FROM sales
                        UNION ALL
                        SELECT strftime('%Y-%m', timestamp) AS ym FROM service_transactions
                    )
                    SELECT CASE strftime('%m', month_start)
                               WHEN '01' THEN 'Jan'
                               WHEN '02' THEN 'Feb'
                               WHEN '03' THEN 'Mar'
                               WHEN '04' THEN 'Apr'
                               WHEN '05' THEN 'May'
                               WHEN '06' THEN 'Jun'
                               WHEN '07' THEN 'Jul'
                               WHEN '08' THEN 'Aug'
                               WHEN '09' THEN 'Sep'
                               WHEN '10' THEN 'Oct'
                               WHEN '11' THEN 'Nov'
                               WHEN '12' THEN 'Dec'
                           END AS label,
                           COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.ym = strftime('%Y-%m', m.month_start)), 0) AS amount,
                           COALESCE((SELECT COUNT(*) FROM tx_counts t WHERE t.ym = strftime('%Y-%m', m.month_start)), 0) AS sales_count,
                           COALESCE((
                               SELECT SUM(remaining_amount) FROM debts d
                               WHERE d.status = 'pending'
                                 AND strftime('%Y-%m', d.created_at) = strftime('%Y-%m', m.month_start)
                           ), 0) AS debt_amount,
                           idx
                    FROM months m
                    ORDER BY idx DESC"
                ).map_err(|e| e.to_string())?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, f64>(1)?,
                            row.get::<_, f64>(2)?,
                            row.get::<_, f64>(3)?,
                        ))
                    })
                    .map_err(|e| e.to_string())?;

                map_points(
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?,
                )
            }
            _ => {
                // week (default): last 7 days
                let mut stmt = conn.prepare(
                    "WITH RECURSIVE days(idx, day_date) AS (
                        SELECT 0, DATE('now', 'localtime')
                        UNION ALL
                        SELECT idx + 1, DATE(day_date, '-1 day')
                        FROM days
                        WHERE idx < 6
                    ), revenues AS (
                        SELECT DATE(timestamp) AS tx_date, amount FROM sales WHERE COALESCE(is_debt, 0) = 0
                        UNION ALL
                        SELECT DATE(timestamp) AS tx_date, amount FROM service_transactions WHERE COALESCE(is_debt, 0) = 0
                        UNION ALL
                        SELECT DATE(payment_date) AS tx_date, amount FROM debt_payments
                    ), tx_counts AS (
                        SELECT DATE(timestamp) AS tx_date FROM sales
                        UNION ALL
                        SELECT DATE(timestamp) AS tx_date FROM service_transactions
                    )
                    SELECT CASE strftime('%w', day_date)
                               WHEN '0' THEN 'Sun'
                               WHEN '1' THEN 'Mon'
                               WHEN '2' THEN 'Tue'
                               WHEN '3' THEN 'Wed'
                               WHEN '4' THEN 'Thu'
                               WHEN '5' THEN 'Fri'
                               WHEN '6' THEN 'Sat'
                           END AS label,
                           COALESCE((SELECT SUM(amount) FROM revenues r WHERE r.tx_date = d.day_date), 0) AS amount,
                           COALESCE((SELECT COUNT(*) FROM tx_counts t WHERE t.tx_date = d.day_date), 0) AS sales_count,
                           COALESCE((
                               SELECT SUM(remaining_amount) FROM debts dbt
                               WHERE dbt.status = 'pending'
                                 AND DATE(dbt.created_at) = d.day_date
                           ), 0) AS debt_amount,
                           idx
                    FROM days d
                    ORDER BY idx DESC"
                ).map_err(|e| e.to_string())?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, f64>(1)?,
                            row.get::<_, f64>(2)?,
                            row.get::<_, f64>(3)?,
                        ))
                    })
                    .map_err(|e| e.to_string())?;

                map_points(
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?,
                )
            }
        };

        Ok(points)
    }

    pub fn migrate_from_localstorage(&self, data: LocalStorageData) -> Result<(), String> {
        let conn = self.synced_conn()?;

        println!("Migrating data from localStorage...");

        // Migrate products
        if let Some(products) = &data.products {
            for product in products {
                conn.execute(
                    "INSERT INTO products (name, product_type, color, size, selling_price, stock) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        product["name"].as_str().unwrap_or(""),
                        product["product_type"].as_str().unwrap_or("life_saver"),
                        product["color"].as_str(),
                        product["size"].as_str(),
                        product["selling_price"].as_f64().unwrap_or(0.0),
                        product["stock"].as_i64().unwrap_or(0),
                    ],
                ).map_err(|e| e.to_string())?;
            }
            println!("Migrated {} products", products.len());
        }

        // Migrate stock
        if let Some(stock) = &data.stock {
            for item in stock {
                conn.execute(
                    "INSERT INTO stock (color, size, sticker_type, rolls, metres_per_roll, total_metres, metres_used) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        item["color"].as_str().unwrap_or(""),
                        item["size"].as_str().unwrap_or("1"),
                        item["sticker_type"].as_str().unwrap_or("colored"),
                        item["rolls"].as_i64().unwrap_or(0),
                        item["metres_per_roll"].as_f64().unwrap_or(50.0),
                        item["total_metres"].as_f64().unwrap_or(0.0),
                        item["metres_used"].as_f64().unwrap_or(0.0),
                    ],
                ).map_err(|e| e.to_string())?;
            }
            println!("Migrated {} stock items", stock.len());
        }

        // Migrate sales
        if let Some(sales) = &data.sales {
            for sale in sales {
                conn.execute(
                    "INSERT INTO sales (type, product_name, product_type, sticker_type, quantity, amount, payment_method, customer_name, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        sale["type"].as_str().unwrap_or("product"),
                        sale["product_name"].as_str(),
                        sale["product_type"].as_str(),
                        sale["sticker_type"].as_str(),
                        sale["quantity"].as_str(),
                        sale["amount"].as_f64().unwrap_or(0.0),
                        sale["payment_method"].as_str().unwrap_or("cash"),
                        sale["customer_name"].as_str().unwrap_or("Walk-in"),
                        sale["timestamp"].as_str(),
                    ],
                ).map_err(|e| e.to_string())?;
            }
            println!("Migrated {} sales", sales.len());
        }

        // Migrate debts
        if let Some(debts) = &data.debts {
            for debt in debts {
                conn.execute(
                    "INSERT INTO debts (customer_name, phone, amount, due_date, description, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        debt["customer_name"].as_str().unwrap_or(""),
                        debt["phone"].as_str(),
                        debt["amount"].as_f64().unwrap_or(0.0),
                        debt["due_date"].as_str(),
                        debt["description"].as_str(),
                        debt["status"].as_str().unwrap_or("pending"),
                    ],
                ).map_err(|e| e.to_string())?;
            }
            println!("Migrated {} debts", debts.len());
        }

        println!("Migration completed!");
        self.finish_write(())
    }
}
