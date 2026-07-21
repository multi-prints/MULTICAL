//! Refresh open pages **only after a data change** (add / edit / delete), never on a timer.
//!
//! Successful mutations call `remote::notify_data_changed()` (local Tauri writes and
//! Cloudflare Worker writes). This hook watches that in-memory epoch and re-runs the
//! page load. No periodic HTTP polling and no paid push infrastructure.
//!
//! Other tills see shared data after they load/navigate a page (or make their own write).

use crate::remote;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Kept for call-site compatibility. Interval polling is disabled (always change-driven).
pub const LIVE_REFRESH_MS: u32 = 0;

/// How often to notice local epoch bumps (in-process only — no network).
const EPOCH_SLICE_MS: u32 = 400;

/// When the document is hidden, check less often (CPU / Windows idle safety).
const HIDDEN_SLICE_MS: u32 = 2_000;

fn document_is_visible() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .map(|doc| {
            js_sys::Reflect::get(
                doc.as_ref(),
                &wasm_bindgen::JsValue::from_str("visibilityState"),
            )
            .ok()
            .and_then(|v| v.as_string())
            .map(|s| s != "hidden")
            .unwrap_or(true)
        })
        .unwrap_or(true)
}

/// Re-run `callback` when app data changes (mutations on this PC), until unmount.
///
/// `interval_ms` is ignored — we never poll the API on a timer.
pub fn use_auto_refresh(_interval_ms: u32, callback: impl Fn() + 'static) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);

    leptos::task::spawn_local(async move {
        let mut last_epoch = remote::data_epoch();

        loop {
            let hidden = !document_is_visible();
            let slice = if hidden {
                HIDDEN_SLICE_MS
            } else {
                EPOCH_SLICE_MS
            };
            TimeoutFuture::new(slice).await;

            if flag.load(Ordering::Relaxed) {
                break;
            }

            let epoch = remote::data_epoch();
            if epoch == last_epoch {
                continue;
            }

            // While hidden: leave last_epoch stale so we fire once when shown again.
            if hidden {
                continue;
            }

            last_epoch = epoch;
            callback();
        }
    });

    on_cleanup(move || {
        cancelled.store(true, Ordering::Relaxed);
    });
}
