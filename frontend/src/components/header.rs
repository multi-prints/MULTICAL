use leptos::prelude::*;

/// Lightweight header stub (main Header lives in `app.rs`).
#[component]
pub fn Header(overdue_count: ReadSignal<i32>) -> impl IntoView {
    view! {
        <header class="h-14 border-b border-[var(--color-border)] bg-white flex items-center justify-between px-5 shrink-0">
            <div></div>
            <div class="relative">
                <button type="button" class="notif-bell" aria-label="Notifications">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"/>
                    </svg>
                    {move || {
                        if overdue_count.get() > 0 {
                            view! {
                                <span class="notification-badge">
                                    {overdue_count.get().min(99)}
                                </span>
                            }.into_any()
                        } else {
                            ().into_any()
                        }
                    }}
                </button>
            </div>
        </header>
    }
}
