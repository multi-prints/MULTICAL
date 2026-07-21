//! Refresh open pages **only after a data change** (add / edit / delete), not on a timer.
//!
//! `remote::notify_data_changed()` is called after successful mutations; this hook
//! watches that epoch and re-runs the page load. No periodic HTTP polling.

use crate::remote;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Kept for call-site compatibility. Interval polling is disabled.
pub const LIVE_REFRESH_MS: u32 = 0;

/// Re-run `callback` when app data changes (mutations), until the page unmounts.
///
/// `interval_ms` is ignored — we do **not** poll the API on a timer.
pub fn use_auto_refresh(_interval_ms: u32, callback: impl Fn() + 'static) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);

    leptos::task::spawn_local(async move {
        let mut last_epoch = remote::data_epoch();
        // Light sleep only to notice epoch bumps from other pages on this PC.
        loop {
            TimeoutFuture::new(400).await;
            if flag.load(Ordering::Relaxed) {
                break;
            }
            let epoch = remote::data_epoch();
            if epoch != last_epoch {
                last_epoch = epoch;
                callback();
            }
        }
    });

    on_cleanup(move || {
        cancelled.store(true, Ordering::Relaxed);
    });
}
