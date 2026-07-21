//! Cloudflare Workers remote API client.
//!
//! When configured (compile-time embed), multi-PC data ops prefer this HTTP path
//! (Worker → Turso). On failure, callers fall back to local Tauri so the app
//! never bricks offline.

#![allow(dead_code)]

use serde::{de::DeserializeOwned, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_api.rs"));
}

/// Bumped after successful remote mutations so open pages can refresh immediately.
static DATA_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Poll interval when remote multi-PC API is active (near real-time).
pub const REMOTE_LIVE_REFRESH_MS: u32 = 4_000;

/// Poll interval for local-only / Turso-replica mode (unchanged behaviour).
pub const LOCAL_LIVE_REFRESH_MS: u32 = 12_000;

pub fn is_enabled() -> bool {
    embedded::EMBEDDED_API_PRESENT
        && !embedded::EMBEDDED_API_BASE_URL.is_empty()
        && !embedded::EMBEDDED_API_SECRET.is_empty()
}

pub fn base_url() -> &'static str {
    embedded::EMBEDDED_API_BASE_URL
}

pub fn live_refresh_ms() -> u32 {
    if is_enabled() {
        REMOTE_LIVE_REFRESH_MS
    } else {
        LOCAL_LIVE_REFRESH_MS
    }
}

pub fn data_epoch() -> u64 {
    DATA_EPOCH.load(Ordering::Relaxed)
}

pub fn notify_data_changed() {
    DATA_EPOCH.fetch_add(1, Ordering::Relaxed);
}

fn auth_header() -> String {
    format!("Bearer {}", embedded::EMBEDDED_API_SECRET)
}

fn full_url(path: &str) -> String {
    format!("{}{}", embedded::EMBEDDED_API_BASE_URL, path)
}

async fn parse_response<T: DeserializeOwned>(
    method: &str,
    path: &str,
    resp: gloo_net::http::Response,
) -> Result<T, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("remote read body {path}: {e}"))?;

    if !(200..300).contains(&status) {
        return Err(format!(
            "remote {method} {path} → HTTP {status}: {}",
            text.chars().take(200).collect::<String>()
        ));
    }

    serde_json::from_str(&text).map_err(|e| {
        format!(
            "remote deserialize {path}: {e} — {}",
            text.chars().take(200).collect::<String>()
        )
    })
}

pub async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    if !is_enabled() {
        return Err("remote API not configured".into());
    }
    let resp = gloo_net::http::Request::get(&full_url(path))
        .header("Authorization", &auth_header())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("remote GET {path}: {e}"))?;
    parse_response("GET", path, resp).await
}

pub async fn post_json<B: Serialize, T: DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    if !is_enabled() {
        return Err("remote API not configured".into());
    }
    let resp = gloo_net::http::Request::post(&full_url(path))
        .header("Authorization", &auth_header())
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(body)
        .map_err(|e| format!("remote POST body {path}: {e}"))?
        .send()
        .await
        .map_err(|e| format!("remote POST {path}: {e}"))?;
    let out = parse_response("POST", path, resp).await?;
    notify_data_changed();
    Ok(out)
}

pub async fn patch_json<B: Serialize, T: DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    if !is_enabled() {
        return Err("remote API not configured".into());
    }
    let resp = gloo_net::http::Request::patch(&full_url(path))
        .header("Authorization", &auth_header())
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(body)
        .map_err(|e| format!("remote PATCH body {path}: {e}"))?
        .send()
        .await
        .map_err(|e| format!("remote PATCH {path}: {e}"))?;
    let out = parse_response("PATCH", path, resp).await?;
    notify_data_changed();
    Ok(out)
}

pub async fn delete_json<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    if !is_enabled() {
        return Err("remote API not configured".into());
    }
    let resp = gloo_net::http::Request::delete(&full_url(path))
        .header("Authorization", &auth_header())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("remote DELETE {path}: {e}"))?;
    let out = parse_response("DELETE", path, resp).await?;
    notify_data_changed();
    Ok(out)
}

/// Prefer remote when enabled; otherwise run local. On remote error, fall back to local.
pub async fn prefer_remote_then_local<T, FR, FL>(
    label: &str,
    remote: FR,
    local: FL,
) -> Result<T, String>
where
    FR: std::future::Future<Output = Result<T, String>>,
    FL: std::future::Future<Output = Result<T, String>>,
{
    if is_enabled() {
        match remote.await {
            Ok(v) => return Ok(v),
            Err(e) => {
                log::warn!("{label}: remote failed ({e}); using local Tauri fallback");
            }
        }
    }
    local.await
}
