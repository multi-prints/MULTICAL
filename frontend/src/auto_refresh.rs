//! Live multi-PC refresh: poll while a page is mounted so changes from other
//! PCs show up without a manual reload.
//!
//! - Local / Turso-replica mode: ~12s (existing behaviour)
//! - Cloudflare remote API mode: ~4s for near real-time multi-till updates
//! - Also watches a local data-epoch so same-PC mutations refresh all open pages

use crate::remote;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Default poll when remote API is not configured (legacy multi-PC via Turso).
pub const LIVE_REFRESH_MS: u32 = remote::LOCAL_LIVE_REFRESH_MS;

/// Call `callback` on an interval (and when the remote data-epoch bumps) until unmount.
pub fn use_auto_refresh(interval_ms: u32, callback: impl Fn() + 'static) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);

    // Use remote-aware interval when multi-PC API is embedded (faster live load).
    let interval = if remote::is_enabled() {
        remote::live_refresh_ms().min(interval_ms)
    } else {
        interval_ms
    };

    leptos::task::spawn_local(async move {
        let mut last_epoch = remote::data_epoch();
        // Slice long polls so epoch bumps are noticed within ~500ms
        let slice = 500u32;
        let mut elapsed = 0u32;
        loop {
            TimeoutFuture::new(slice).await;
            if flag.load(Ordering::Relaxed) {
                break;
            }
            let epoch = remote::data_epoch();
            if epoch != last_epoch {
                last_epoch = epoch;
                elapsed = 0;
                callback();
                continue;
            }
            elapsed = elapsed.saturating_add(slice);
            if elapsed >= interval {
                elapsed = 0;
                callback();
            }
        }
    });

    on_cleanup(move || {
        cancelled.store(true, Ordering::Relaxed);
    });
}
