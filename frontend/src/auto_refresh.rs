//! Live multi-PC refresh: poll while a page is mounted so Turso-synced
//! changes from other PCs show up without a manual reload.
//!
//! Keep this interval well above the backend read-sync throttle so polls
//! mostly hit the local replica instead of blocking on the network.

use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Poll open pages for multi-PC updates (ms).
/// 12s keeps the UI snappy while still feeling near-realtime across tills.
pub const LIVE_REFRESH_MS: u32 = 12_000;

/// Call `callback` every `interval_ms` until the current component unmounts.
/// Skips overlapping ticks if a previous callback is still marked busy by the page.
pub fn use_auto_refresh(interval_ms: u32, callback: impl Fn() + 'static) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);

    leptos::task::spawn_local(async move {
        loop {
            TimeoutFuture::new(interval_ms).await;
            if flag.load(Ordering::Relaxed) {
                break;
            }
            callback();
        }
    });

    on_cleanup(move || {
        cancelled.store(true, Ordering::Relaxed);
    });
}
