#![allow(deprecated)]
#![allow(dead_code)]
#![allow(clippy::duplicate_mod)]

use crate::api::{self, LoginResponse, UserInfo};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use js_sys::{Array, Function, Object, Reflect};
use leptos::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
#[path = "pages/products.rs"]
mod products_page;
use products_page::ProductsPage as ProductsPageView;
#[path = "pages/stock.rs"]
mod stock_page;
use stock_page::StockPage as StockPageView;
#[path = "pages/sales.rs"]
mod sales_page;
use sales_page::SalesPage as SalesPageView;
#[path = "pages/printing.rs"]
mod printing_page;
use printing_page::PrintingPage as PrintingPageView;
#[path = "pages/debts.rs"]
mod debts_page;
use debts_page::DebtsPage as DebtsPageView;
#[path = "pages/settings.rs"]
mod settings_page;
use settings_page::SettingsPage as SettingsPageView;
#[path = "components/loading.rs"]
mod loading_comp;
use loading_comp::PageLoading;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Dashboard,
    Products,
    Stock,
    Sales,
    Printing,
    Debts,
    Settings,
}

fn default_page_for_role(role: &str) -> Page {
    if role == "admin" {
        Page::Dashboard
    } else {
        Page::Sales
    }
}

fn notification_permission() -> Option<String> {
    let global = js_sys::global();
    let notification = Reflect::get(&global, &JsValue::from_str("Notification")).ok()?;
    Reflect::get(&notification, &JsValue::from_str("permission"))
        .ok()?
        .as_string()
}

fn request_notification_permission() {
    let global = js_sys::global();
    if let Ok(notification) = Reflect::get(&global, &JsValue::from_str("Notification")) {
        if let Ok(func) = Reflect::get(&notification, &JsValue::from_str("requestPermission"))
            .and_then(|v| v.dyn_into::<Function>())
        {
            let _ = func.call0(&notification);
        }
    }
}

fn show_desktop_notification(title: &str, body: &str, tag: &str) {
    let global = js_sys::global();
    let Ok(notification) = Reflect::get(&global, &JsValue::from_str("Notification")) else {
        return;
    };
    let Ok(ctor) = notification.dyn_into::<Function>() else {
        return;
    };

    let options = Object::new();
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("body"),
        &JsValue::from_str(body),
    );
    let _ = Reflect::set(&options, &JsValue::from_str("tag"), &JsValue::from_str(tag));

    let args = Array::new();
    args.push(&JsValue::from_str(title));
    args.push(&options);
    let _ = Reflect::construct(&ctor, &args);
}

#[component]
pub fn App() -> impl IntoView {
    let (user, set_user) = signal(None::<UserInfo>);
    let (token, set_token) = signal(None::<String>);
    let (loading, set_loading) = signal(true);
    let (page, set_page) = signal(Page::Sales);

    leptos::task::spawn_local(async move {
        if let Ok(tok) = LocalStorage::get::<String>("sessionToken") {
            match api::validate_session(&tok).await {
                Ok(true) => {
                    set_token.set(Some(tok));
                    if let Ok(u) = LocalStorage::get::<String>("currentUser") {
                        if let Ok(info) = serde_json::from_str::<UserInfo>(&u) {
                            set_page.set(default_page_for_role(&info.role));
                            set_user.set(Some(info));
                        }
                    }
                }
                _ => {
                    LocalStorage::delete("sessionToken");
                    LocalStorage::delete("currentUser");
                }
            }
        }
        set_loading.set(false);
    });

    let user_role = move || user.get().map(|u| u.role.clone()).unwrap_or_default();
    let (overdue_debts, set_overdue_debts) = signal(Vec::<crate::api::Debt>::new());
    let (overdue_polling_started, set_overdue_polling_started) = signal(false);
    let (available_update, set_available_update) = signal(None::<api::UpdateResult>);
    let (update_polling_started, set_update_polling_started) = signal(false);

    let load_overdue = {
        move || {
            leptos::task::spawn_local(async move {
                if let Ok(items) = api::get_overdue_debts().await {
                    set_overdue_debts.set(items);
                }
            });
        }
    };

    create_effect(move |_| {
        let role = user_role();
        if role == "admin" {
            load_overdue();
            if !overdue_polling_started.get() {
                set_overdue_polling_started.set(true);
                let load_overdue = load_overdue;
                Interval::new(60_000, load_overdue).forget();
            }
        } else {
            set_overdue_debts.set(Vec::new());
        }
    });

    let check_for_update = move || {
        leptos::task::spawn_local(async move {
            if let Ok(result) = api::check_for_update().await {
                if result.available {
                    set_available_update.set(Some(result));
                } else {
                    set_available_update.set(None);
                }
            }
        });
    };

    create_effect(move |_| {
        if user.get().is_some() {
            check_for_update();
            if !update_polling_started.get() {
                set_update_polling_started.set(true);
                let check_for_update = check_for_update;
                Interval::new(6 * 60 * 60 * 1000, check_for_update).forget();
            }
        } else {
            set_available_update.set(None);
        }
    });

    let show_notifications = Signal::derive(move || user.get().is_some());
    let show_debt_notifications =
        Signal::derive(move || user.get().map(|u| u.role == "admin").unwrap_or(false));

    view! {
        <div class="app-frame">
            <WindowResizeHandles/>
            {move || {
                if loading.get() {
                    view! {
                        <div class="app-shell app-shell--full">
                            <div class="app-main">
                                <TitleBar
                                    overdue_debts=Signal::derive(move || overdue_debts.get())
                                    available_update=Signal::derive(move || available_update.get())
                                    show_notifications=show_notifications
                                    show_debt_notifications=show_debt_notifications
                                    set_page=set_page
                                />
                                <div class="app-main-body">
                                    <PageLoading message="Starting MULTIPRINTS..."/>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else if user.get().is_some() {
                    let role = user_role();
                    let p = page;
                    let sp = set_page;
                    let tok = token.get().unwrap_or_default();
                    let tok2 = tok.clone();
                    let logout = {
                        let su = set_user;
                        let st = set_token;
                        move |_| {
                            let t = tok2.clone();
                            leptos::task::spawn_local(async move { let _ = api::logout(&t).await; });
                            LocalStorage::delete("sessionToken");
                            LocalStorage::delete("currentUser");
                            su.set(None);
                            st.set(None);
                        }
                    };
                    let username = user
                        .get()
                        .map(|u| u.username.clone())
                        .unwrap_or_else(|| "User".to_string());
                    // Sidebar is full-height; titlebar only spans the main column
                    view! {
                        <div class="app-shell">
                            <Sidebar
                                user_role=role.clone()
                                current_page=p
                                set_page=sp
                                on_logout=logout
                            />
                            <div class="app-main">
                                <TitleBar
                                    overdue_debts=Signal::derive(move || overdue_debts.get())
                                    available_update=Signal::derive(move || available_update.get())
                                    show_notifications=show_notifications
                                    show_debt_notifications=show_debt_notifications
                                    set_page=set_page
                                />
                                <div class="app-main-body">
                                    <main class="app-page">
                                        {move || match p.get() {
                                            Page::Dashboard => {
                                                if role == "admin" {
                                                    view! { <DashboardPage set_page=sp username=username.clone() /> }.into_any()
                                                } else {
                                                    view! { <SalesPageView show_revenue_stats=false /> }.into_any()
                                                }
                                            },
                                            Page::Products => view! { <ProductsPageView /> }.into_any(),
                                            Page::Stock => view! { <StockPageView /> }.into_any(),
                                            Page::Sales => view! { <SalesPageView show_revenue_stats=role == "admin" /> }.into_any(),
                                            Page::Printing => view! { <PrintingPageView show_revenue_stats=role == "admin" can_manage_materials=role == "admin" /> }.into_any(),
                                            Page::Debts => view! { <DebtsPageView /> }.into_any(),
                                            Page::Settings => view! { <SettingsPageView user=user set_user=set_user /> }.into_any(),
                                        }}
                                    </main>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="app-shell app-shell--full">
                            <div class="app-main">
                                <TitleBar
                                    overdue_debts=Signal::derive(move || overdue_debts.get())
                                    available_update=Signal::derive(move || available_update.get())
                                    show_notifications=show_notifications
                                    show_debt_notifications=show_debt_notifications
                                    set_page=set_page
                                />
                                <div class="app-main-body">
                                    <LoginPage set_user=set_user set_token=set_token set_page=set_page />
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

fn window_control(action: &str) {
    let global = js_sys::global();
    let Ok(api) = Reflect::get(&global, &JsValue::from_str("multiprintsWindow")) else {
        return;
    };
    let Ok(func) =
        Reflect::get(&api, &JsValue::from_str(action)).and_then(|v| v.dyn_into::<Function>())
    else {
        return;
    };
    let _ = func.call0(&api);
}

fn window_start_resize(direction: &str) {
    let global = js_sys::global();
    let Ok(api) = Reflect::get(&global, &JsValue::from_str("multiprintsWindow")) else {
        return;
    };
    let Ok(func) = Reflect::get(&api, &JsValue::from_str("startResizeDragging"))
        .and_then(|v| v.dyn_into::<Function>())
    else {
        return;
    };
    let _ = func.call1(&api, &JsValue::from_str(direction));
}

/// Edge/corner resize grips for the undecorated window.
#[component]
fn WindowResizeHandles() -> impl IntoView {
    let on_resize = move |direction: &'static str| {
        move |ev: leptos::ev::MouseEvent| {
            // Only primary button; ignore when maximized (handles are CSS-hidden too).
            if ev.button() != 0 {
                return;
            }
            ev.prevent_default();
            ev.stop_propagation();
            window_start_resize(direction);
        }
    };

    view! {
        <div class="win-resize-layer" aria-hidden="true">
            <div class="win-resize win-resize--n" on:mousedown=on_resize("North")></div>
            <div class="win-resize win-resize--s" on:mousedown=on_resize("South")></div>
            <div class="win-resize win-resize--e" on:mousedown=on_resize("East")></div>
            <div class="win-resize win-resize--w" on:mousedown=on_resize("West")></div>
            <div class="win-resize win-resize--ne" on:mousedown=on_resize("NorthEast")></div>
            <div class="win-resize win-resize--nw" on:mousedown=on_resize("NorthWest")></div>
            <div class="win-resize win-resize--se" on:mousedown=on_resize("SouthEast")></div>
            <div class="win-resize win-resize--sw" on:mousedown=on_resize("SouthWest")></div>
        </div>
    }
}

const THEME_STORAGE_KEY: &str = "multiprints-theme";

fn read_stored_theme() -> String {
    // Prefer raw localStorage so the early HTML boot script can share the same key/value.
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(Some(raw)) = storage.get_item(THEME_STORAGE_KEY) {
            let cleaned = raw.trim().trim_matches('"');
            if cleaned == "dark" || cleaned == "light" {
                return cleaned.to_string();
            }
        }
    }
    LocalStorage::get::<String>(THEME_STORAGE_KEY)
        .ok()
        .filter(|t| t == "dark" || t == "light")
        .unwrap_or_else(|| "light".to_string())
}

fn apply_theme(theme: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = el.set_attribute("data-theme", theme);
    }
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(THEME_STORAGE_KEY, theme);
    }
}

/// Sync `html.is-maximized` so CSS can drop border-radius when maximized.
fn sync_maximized_class() {
    leptos::task::spawn_local(async {
        let global = js_sys::global();
        let Ok(api) = Reflect::get(&global, &JsValue::from_str("multiprintsWindow")) else {
            return;
        };
        let Ok(func) = Reflect::get(&api, &JsValue::from_str("isMaximized"))
            .and_then(|v| v.dyn_into::<Function>())
        else {
            return;
        };
        let Ok(ret) = func.call0(&api) else {
            return;
        };
        let Ok(val) = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&ret)).await
        else {
            return;
        };
        let maximized = val.is_truthy();
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let classes = el.class_list();
            let _ = if maximized {
                classes.add_1("is-maximized")
            } else {
                classes.remove_1("is-maximized")
            };
        }
    });
}

/// Custom undecorated titlebar — drag region + theme + notifications + window controls only.
/// Brand / sidebar toggle live in the sidebar so they move with collapse.
#[component]
fn TitleBar(
    overdue_debts: Signal<Vec<crate::api::Debt>>,
    available_update: Signal<Option<api::UpdateResult>>,
    show_notifications: Signal<bool>,
    show_debt_notifications: Signal<bool>,
    set_page: WriteSignal<Page>,
) -> impl IntoView {
    let (open, set_open) = signal(false);
    let (initial_notified, set_initial_notified) = signal(false);
    let (update_notified, set_update_notified) = signal(false);
    let (installing_update, set_installing_update) = signal(false);
    let (theme, set_theme) = signal(read_stored_theme());

    // Apply theme on mount and whenever it changes
    Effect::new(move |_| {
        let t = theme.get();
        apply_theme(&t);
    });

    Effect::new(move |_| {
        sync_maximized_class();
        if let Some(window) = web_sys::window() {
            let listener = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                gloo_timers::callback::Timeout::new(50, || {
                    sync_maximized_class();
                })
                .forget();
            }));
            let _ = window
                .add_event_listener_with_callback("resize", listener.as_ref().unchecked_ref());
            listener.forget();
        }
    });

    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            let set_open = set_open;
            let listener = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                set_open.set(false);
            }));
            let _ =
                window.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref());
            listener.forget();
        }
    });

    let notification_count = move || {
        if !show_notifications.get() {
            return 0;
        }
        let debt_count = if show_debt_notifications.get() {
            overdue_debts.get().len()
        } else {
            0
        };
        debt_count + usize::from(available_update.get().is_some())
    };

    let overdue_total = move || {
        if !show_debt_notifications.get() {
            return 0.0;
        }
        overdue_debts
            .get()
            .iter()
            .map(|d| d.remaining_amount)
            .sum::<f64>()
    };

    let install_update = move |_| {
        if installing_update.get() {
            return;
        }
        set_installing_update.set(true);
        leptos::task::spawn_local(async move {
            if api::check_and_install_update().await.is_err() {
                set_installing_update.set(false);
            }
        });
    };

    let go_to_debts = move |_| {
        set_open.set(false);
        set_page.set(Page::Debts);
    };

    Effect::new(move |_| {
        if !show_debt_notifications.get() {
            return;
        }
        let overdue = overdue_debts.get();
        if !initial_notified.get() && !overdue.is_empty() {
            let total: f64 = overdue.iter().map(|d| d.remaining_amount).sum();
            match notification_permission().as_deref() {
                Some("granted") => {
                    show_desktop_notification(
                        "Overdue Debts Reminder",
                        &format!(
                            "You have {} overdue debt{} totaling KSh {:.0}",
                            overdue.len(),
                            if overdue.len() > 1 { "s" } else { "" },
                            total
                        ),
                        "overdue-debts",
                    );
                    set_initial_notified.set(true);
                }
                Some("denied") => {}
                _ => request_notification_permission(),
            }
        }
    });

    Effect::new(move |_| {
        if !show_notifications.get() {
            return;
        }
        let Some(update) = available_update.get() else {
            return;
        };
        if update_notified.get() {
            return;
        }
        match notification_permission().as_deref() {
            Some("granted") => {
                show_desktop_notification(
                    "Update Available",
                    &update.message,
                    "app-update-available",
                );
                set_update_notified.set(true);
            }
            Some("denied") => {}
            _ => request_notification_permission(),
        }
    });

    let on_minimize = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        window_control("minimize");
    };
    let on_maximize = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        window_control("toggleMaximize");
        gloo_timers::callback::Timeout::new(80, || {
            sync_maximized_class();
        })
        .forget();
    };
    let on_close = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        window_control("close");
    };
    let on_drag_dblclick = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        window_control("toggleMaximize");
        gloo_timers::callback::Timeout::new(80, || {
            sync_maximized_class();
        })
        .forget();
    };

    // Manual drag fallback — more reliable than data-tauri-drag-region alone on some Linux WMs.
    let on_drag_mousedown = move |ev: leptos::ev::MouseEvent| {
        if ev.button() != 0 {
            return;
        }
        // Leave double-click maximize free; only start drag on single primary press.
        if ev.detail() > 1 {
            return;
        }
        window_control("startDragging");
    };

    let toggle_theme = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        set_theme.update(|t| {
            *t = if *t == "dark" {
                "light".to_string()
            } else {
                "dark".to_string()
            };
        });
    };

    view! {
        <header class="titlebar">
            <div
                class="titlebar-drag"
                attr:data-tauri-drag-region=""
                on:mousedown=on_drag_mousedown
                on:dblclick=on_drag_dblclick
            ></div>

            <div class="titlebar-controls" on:mousedown=|ev| ev.stop_propagation()>
                <button
                    type="button"
                    class="titlebar-theme-btn titlebar-theme"
                    aria-label=move || {
                        if theme.get() == "dark" {
                            "Switch to light mode"
                        } else {
                            "Switch to dark mode"
                        }
                    }
                    title=move || {
                        if theme.get() == "dark" {
                            "Light mode"
                        } else {
                            "Dark mode"
                        }
                    }
                    on:mousedown=|ev| ev.stop_propagation()
                    on:click=toggle_theme
                >
                    {move || if theme.get() == "dark" {
                        // Sun icon — click to go light
                        view! {
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                    d="M12 3v2.25m6.364.386l-1.591 1.591M21 12h-2.25m-.386 6.364l-1.591-1.591M12 18.75V21m-4.773-4.227l-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0z"/>
                            </svg>
                        }.into_any()
                    } else {
                        // Moon icon — click to go dark
                        view! {
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                    d="M21.752 15.002A9.718 9.718 0 0118 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 003 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 009.002-5.998z"/>
                            </svg>
                        }.into_any()
                    }}
                </button>
                {move || if show_notifications.get() {
                    view! {
                        <div class="titlebar-notif" on:click=move |e| e.stop_propagation()>
                            <button
                                type="button"
                                class=move || if open.get() { "notif-bell is-open" } else { "notif-bell" }
                                aria-label="Notifications"
                                aria-expanded=move || open.get().to_string()
                                title="Notifications"
                                on:mousedown=|ev| ev.stop_propagation()
                                on:click=move |_| set_open.update(|v| *v = !*v)
                            >
                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"/>
                                </svg>
                                {move || if notification_count() > 0 {
                                    view! {
                                        <span class="notification-badge">
                                            {move || {
                                                let count = notification_count();
                                                if count > 99 { "99+".to_string() } else { count.to_string() }
                                            }}
                                        </span>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
                            </button>
                            {move || if open.get() {
                                view! {
                                    <div class="notification-dropdown" role="menu" aria-label="Notifications">
                                        <div class="notification-dropdown-header">
                                            <h3>"Notifications"</h3>
                                            <span class=move || {
                                                if notification_count() > 0 {
                                                    "notif-count-pill has-items".to_string()
                                                } else {
                                                    "notif-count-pill".to_string()
                                                }
                                            }>
                                                {move || {
                                                    let count = notification_count();
                                                    if count == 0 {
                                                        "All clear".to_string()
                                                    } else if count == 1 {
                                                        "1 new".to_string()
                                                    } else {
                                                        format!("{} new", count)
                                                    }
                                                }}
                                            </span>
                                        </div>
                                        <div class="notification-dropdown-list">
                                            {move || {
                                                let items = if show_debt_notifications.get() {
                                                    overdue_debts.get()
                                                } else {
                                                    Vec::new()
                                                };
                                                let has_update = available_update.get().is_some();
                                                if items.is_empty() && !has_update {
                                                    view! {
                                                        <div class="notification-empty">
                                                            <div class="notification-empty-icon">
                                                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"/>
                                                                </svg>
                                                            </div>
                                                            <p class="notification-empty-title">"You're all caught up"</p>
                                                            <p class="notification-empty-sub">"Overdue debts and app updates will show up here."</p>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    let mut views = Vec::new();

                                                    if show_debt_notifications.get() && !items.is_empty() {
                                                        let total = overdue_total();
                                                        views.push(view! {
                                                            <div class="notif-summary">
                                                                <span class="notif-summary-label">"Overdue total"</span>
                                                                <span class="notif-summary-value">{format!("KSh {:.0}", total)}</span>
                                                            </div>
                                                        }.into_any());
                                                    }

                                                    if let Some(update) = available_update.get() {
                                                        let version_text = update
                                                            .version
                                                            .clone()
                                                            .map(|version| format!("Version {} is ready to install", version))
                                                            .unwrap_or_else(|| update.message.clone());
                                                        views.push(view! {
                                                            <button
                                                                type="button"
                                                                class="notif-item is-action"
                                                                prop:disabled=move || installing_update.get()
                                                                on:click=install_update
                                                            >
                                                                <div class="notif-item-icon update">
                                                                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                                                                    </svg>
                                                                </div>
                                                                <div class="notif-item-body">
                                                                    <div class="notif-item-top">
                                                                        <p class="notif-item-title">"Update available"</p>
                                                                    </div>
                                                                    <p class="notif-item-meta">{version_text}</p>
                                                                    <p class="notif-item-cta">
                                                                        {move || if installing_update.get() {
                                                                            "Installing…"
                                                                        } else {
                                                                            "Install now"
                                                                        }}
                                                                    </p>
                                                                </div>
                                                            </button>
                                                        }.into_any());
                                                    }

                                                    views.extend(items.into_iter().map(|debt| {
                                                        let due = debt.due_date.clone().unwrap_or_default();
                                                        let days_overdue = chrono::NaiveDate::parse_from_str(&due, "%Y-%m-%d")
                                                            .ok()
                                                            .map(|d| (chrono::Local::now().date_naive() - d).num_days().max(0))
                                                            .unwrap_or(0);
                                                        let pill_cls = if days_overdue > 7 {
                                                            "notif-pill hot"
                                                        } else {
                                                            "notif-pill mild"
                                                        };
                                                        let due_label = if due.is_empty() {
                                                            "No due date".to_string()
                                                        } else {
                                                            format!("Due {}", due)
                                                        };
                                                        let overdue_label = if days_overdue == 0 {
                                                            "Due today".to_string()
                                                        } else if days_overdue == 1 {
                                                            "1 day".to_string()
                                                        } else {
                                                            format!("{} days", days_overdue)
                                                        };
                                                        view! {
                                                            <div class="notif-item">
                                                                <div class="notif-item-icon debt">
                                                                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                                                    </svg>
                                                                </div>
                                                                <div class="notif-item-body">
                                                                    <div class="notif-item-top">
                                                                        <p class="notif-item-title">{debt.customer_name.clone()}</p>
                                                                        <span class=pill_cls.to_string()>{overdue_label}</span>
                                                                    </div>
                                                                    <p class="notif-item-amount">{format!("KSh {:.0}", debt.remaining_amount)}</p>
                                                                    <p class="notif-item-meta">{due_label}</p>
                                                                    {debt.description.clone().filter(|d| !d.trim().is_empty()).map(|desc| view! {
                                                                        <p class="notif-item-desc">{desc}</p>
                                                                    })}
                                                                </div>
                                                            </div>
                                                        }
                                                        .into_any()
                                                    }));
                                                    views.into_any()
                                                }
                                            }}
                                        </div>
                                        {move || {
                                            let has_debts = show_debt_notifications.get() && !overdue_debts.get().is_empty();
                                            if has_debts {
                                                view! {
                                                    <div class="notification-dropdown-footer">
                                                        <button type="button" on:click=go_to_debts>
                                                            "View all debts"
                                                        </button>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                ().into_any()
                                            }
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                ().into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}

                <button
                    type="button"
                    class="titlebar-btn"
                    aria-label="Minimize"
                    title="Minimize"
                    on:mousedown=|ev| ev.stop_propagation()
                    on:click=on_minimize
                >
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.75" aria-hidden="true">
                        <path stroke-linecap="round" d="M3.5 8h9"/>
                    </svg>
                </button>
                <button
                    type="button"
                    class="titlebar-btn"
                    aria-label="Maximize"
                    title="Maximize"
                    on:mousedown=|ev| ev.stop_propagation()
                    on:click=on_maximize
                >
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.75" aria-hidden="true">
                        <rect x="3.75" y="3.75" width="8.5" height="8.5" rx="1.25"/>
                    </svg>
                </button>
                <button
                    type="button"
                    class="titlebar-btn titlebar-btn--close"
                    aria-label="Close"
                    title="Close"
                    on:mousedown=|ev| ev.stop_propagation()
                    on:click=on_close
                >
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.75" aria-hidden="true">
                        <path stroke-linecap="round" d="M4.5 4.5l7 7M11.5 4.5l-7 7"/>
                    </svg>
                </button>
            </div>
        </header>
    }
}

#[component]
fn Sidebar(
    user_role: String,
    current_page: ReadSignal<Page>,
    set_page: WriteSignal<Page>,
    on_logout: impl Fn(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    let is_admin = user_role == "admin";
    let (collapsed, set_collapsed) = signal(false);

    let nav_item = move |p: Page, label: &'static str, icon: &'static str| {
        view! {
            <button
                type="button"
                title=label
                class=move || {
                    if current_page.get() == p {
                        "sidebar-nav-item active"
                    } else {
                        "sidebar-nav-item"
                    }
                }
                on:click=move |_| set_page.set(p)
            >
                <svg class="sidebar-nav-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d=icon/>
                </svg>
                <span class="sidebar-nav-label">{label}</span>
            </button>
        }
    };

    view! {
        <aside
            class=move || {
                if collapsed.get() {
                    "sidebar is-collapsed"
                } else {
                    "sidebar"
                }
            }
            aria-label="Main navigation"
            aria-expanded=move || (!collapsed.get()).to_string()
        >
            <div class="sidebar-top">
                <div class="sidebar-brand">
                    <div class="sidebar-logo">
                        <span class="sidebar-logo-text">"MULTIPRINTS"</span>
                    </div>
                    <button
                        type="button"
                        class="sidebar-collapse-btn"
                        aria-label=move || {
                            if collapsed.get() {
                                "Expand sidebar"
                            } else {
                                "Collapse sidebar"
                            }
                        }
                        title=move || {
                            if collapsed.get() {
                                "Expand"
                            } else {
                                "Collapse"
                            }
                        }
                        on:click=move |ev| {
                            ev.stop_propagation();
                            set_collapsed.update(|v| *v = !*v);
                        }
                    >
                        <svg viewBox="0 0 24 24" aria-hidden="true">
                            <rect
                                x="3"
                                y="3"
                                width="18"
                                height="18"
                                rx="2.5"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                            />
                            <path
                                d="M3 5.5C3 4.12 4.12 3 5.5 3H9v18H5.5C4.12 21 3 19.88 3 18.5V5.5z"
                                fill="currentColor"
                            />
                        </svg>
                    </button>
                </div>
            </div>

            <nav class="sidebar-nav">
                {if is_admin {
                    view! {
                        <div class="sidebar-section">
                            {nav_item(
                                Page::Dashboard,
                                "Dashboard",
                                "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-4 0h4",
                            )}
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}

                <div class="sidebar-section">
                    <div class="sidebar-section-label">"Work"</div>
                    {if is_admin {
                        view! {
                            <>
                                {nav_item(Page::Products, "Products", "M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4")}
                                {nav_item(Page::Stock, "Stock", "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10")}
                            </>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    {nav_item(Page::Sales, "Sales", "M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 11-4 0 2 2 0 014 0z")}
                    {nav_item(Page::Printing, "Printing", "M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z")}
                    {nav_item(Page::Debts, "Debts", "M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z")}
                </div>

                <div class="sidebar-section">
                    <div class="sidebar-section-label">"Others"</div>
                    {nav_item(
                        Page::Settings,
                        "Settings",
                        "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z",
                    )}
                </div>
            </nav>

            <div class="sidebar-footer">
                <button
                    type="button"
                    class="sidebar-signout"
                    title="Sign out"
                    on:click=on_logout
                >
                    <svg class="sidebar-nav-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"/>
                    </svg>
                    <span class="sidebar-nav-label">"Sign out"</span>
                </button>
            </div>
        </aside>
    }
}

#[component]
fn LoginPage(
    set_user: WriteSignal<Option<UserInfo>>,
    set_token: WriteSignal<Option<String>>,
    set_page: WriteSignal<Page>,
) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (show_pw, set_show_pw) = signal(false);

    let do_login = move || {
        if loading.get() {
            return;
        }
        let u = username.get().trim().to_string();
        let p = password.get();
        if u.is_empty() || p.is_empty() {
            set_error.set("Please enter username and password".into());
            return;
        }
        set_error.set(String::new());
        set_loading.set(true);
        leptos::task::spawn_local(async move {
            match api::login(&u, &p).await {
                Ok(LoginResponse {
                    success: true,
                    token: Some(tok),
                    user: Some(info),
                    ..
                }) => {
                    LocalStorage::set("sessionToken", &tok).ok();
                    LocalStorage::set(
                        "currentUser",
                        serde_json::to_string(&info).unwrap_or_default(),
                    )
                    .ok();
                    let landing_page = default_page_for_role(&info.role);
                    set_token.set(Some(tok));
                    set_user.set(Some(info));
                    set_page.set(landing_page);
                }
                Ok(r) => set_error.set(r.error.unwrap_or("Invalid username or password".into())),
                Err(e) => set_error.set(e),
            }
            set_loading.set(false);
        });
    };

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        do_login();
    };

    view! {
        <div class="login-page">
            <div class="login-shell">
                <div class="login-card">
                    <div class="login-brand">
                        <h1 class="login-title">"Sign in to your account"</h1>
                    </div>

                    <form class="login-form" on:submit=on_submit>
                        <Show when=move || !error.get().is_empty()>
                            <div class="login-error" role="alert">{move || error.get()}</div>
                        </Show>

                        <div class="login-fields">
                            <div class="login-field">
                                <label class="login-label" for="login-username">"Username"</label>
                                <input
                                    id="login-username"
                                    type="text"
                                    class="login-input"
                                    placeholder="Username"
                                    autocomplete="username"
                                    prop:value=move || username.get()
                                    on:input=move |e| set_username.set(event_target_value(&e))
                                />
                            </div>

                            <div class="login-field">
                                <label class="login-label" for="login-password">"Password"</label>
                                <div class="login-password-wrap">
                                    <input
                                        id="login-password"
                                        type=move || if show_pw.get() { "text" } else { "password" }
                                        class="login-input login-input--password"
                                        placeholder="Password"
                                        autocomplete="current-password"
                                        prop:value=move || password.get()
                                        on:input=move |e| set_password.set(event_target_value(&e))
                                    />
                                    <button
                                        type="button"
                                        class="login-pw-toggle"
                                        aria-label=move || if show_pw.get() { "Hide password" } else { "Show password" }
                                        on:click=move |_| set_show_pw.update(|v| *v = !*v)
                                    >
                                        {move || if show_pw.get() {
                                            view! {
                                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                                        d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"/>
                                                </svg>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                                        d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                                                </svg>
                                            }.into_any()
                                        }}
                                    </button>
                                </div>
                            </div>
                        </div>

                        <button
                            type="submit"
                            class="login-submit"
                            prop:disabled=move || loading.get()
                        >
                            {move || if loading.get() {
                                view! {
                                    <span class="login-submit-inner">
                                        <span class="login-spinner" aria-hidden="true"></span>
                                        "Signing in..."
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span>"Sign In"</span> }.into_any()
                            }}
                        </button>
                    </form>

                    <p class="login-footer">"© 2026 MULTIPRINTS"</p>
                </div>
            </div>
        </div>
    }
}

fn short_chart_label(raw: &str, period: &str) -> String {
    let s = raw.trim();
    match period {
        "week" => s.chars().take(3).collect(),
        "year" => s.chars().take(3).collect(),
        "month" => {
            // Backend: "22–28 Jun" or "22-28 Jun" → keep month token + last day
            if let Some(month) = s.split_whitespace().last() {
                if let Some(range) = s.split_whitespace().next() {
                    let end = range
                        .split(['–', '-', '—'])
                        .next_back()
                        .unwrap_or(range)
                        .trim();
                    return format!("{} {}", month, end);
                }
            }
            s.to_string()
        }
        _ => s.to_string(),
    }
}

/// Catmull–Rom → cubic Bézier smooth path through the points.
fn smooth_line_through(pts: &[(f64, f64)]) -> String {
    if pts.is_empty() {
        return String::new();
    }
    if pts.len() == 1 {
        return format!("M{:.2},{:.2}", pts[0].0, pts[0].1);
    }
    if pts.len() == 2 {
        return format!(
            "M{:.2},{:.2} L{:.2},{:.2}",
            pts[0].0, pts[0].1, pts[1].0, pts[1].1
        );
    }

    // Low tension keeps the curve inside the plot box when the SVG scales.
    let t = 0.14;
    let mut d = format!("M{:.2},{:.2}", pts[0].0, pts[0].1);

    for i in 0..pts.len() - 1 {
        let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = if i + 2 < pts.len() {
            pts[i + 2]
        } else {
            pts[i + 1]
        };

        let mut cp1x = p1.0 + (p2.0 - p0.0) * t;
        let mut cp1y = p1.1 + (p2.1 - p0.1) * t;
        let mut cp2x = p2.0 - (p3.0 - p1.0) * t;
        let mut cp2y = p2.1 - (p3.1 - p1.1) * t;

        // Keep control points inside the point bounds so scaled SVGs don't clip spikes.
        let min_x = p1.0.min(p2.0);
        let max_x = p1.0.max(p2.0);
        let min_y = p0.1.min(p1.1).min(p2.1).min(p3.1);
        let max_y = p0.1.max(p1.1).max(p2.1).max(p3.1);
        cp1x = cp1x.clamp(min_x, max_x);
        cp2x = cp2x.clamp(min_x, max_x);
        cp1y = cp1y.clamp(min_y, max_y);
        cp2y = cp2y.clamp(min_y, max_y);

        d.push_str(&format!(
            " C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
            cp1x, cp1y, cp2x, cp2y, p2.0, p2.1
        ));
    }
    d
}

/// Build SVG line + area paths for a compact metric sparkline (smooth curves).
fn sparkline_paths(values: &[f64], width: f64, height: f64) -> Option<(String, String)> {
    if values.len() < 2 {
        return None;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // Soft floor so a single spike doesn't pin everything to the bottom edge
    let range = (max - min).max(1.0);
    // Inset so stroke ends and smooth curves stay inside the viewBox when scaled.
    let pad_x = 4.0;
    let pad_y = 5.0;
    let plot_w = (width - pad_x * 2.0).max(1.0);
    let plot_h = (height - pad_y * 2.0).max(1.0);
    let n = values.len();
    let step = plot_w / (n - 1) as f64;

    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(n);
    for (i, v) in values.iter().enumerate() {
        let x = pad_x + i as f64 * step;
        let t = ((v - min) / range).clamp(0.0, 1.0);
        let y = pad_y + plot_h * (1.0 - t);
        pts.push((x, y));
    }

    let line = smooth_line_through(&pts);
    let last = pts.last()?;
    let first = pts.first()?;
    let fill = format!(
        "{} L{:.2},{:.2} L{:.2},{:.2} Z",
        line, last.0, height, first.0, height
    );
    Some((line, fill))
}

#[component]
fn MetricSparkline(
    /// Series values (time order).
    values: Vec<f64>,
    /// CSS color for the stroke (and gradient).
    color: &'static str,
    /// Unique gradient id so multiple sparklines don't clash.
    #[prop(into)]
    grad_id: String,
) -> impl IntoView {
    // Logical viewBox; CSS scales the SVG fluidly to the card width.
    const W: f64 = 100.0;
    const H: f64 = 36.0;
    let paths = sparkline_paths(&values, W, H);
    view! {
        <div class="dash-metric-spark" aria-hidden="true">
            {match paths {
                Some((line, fill)) => view! {
                    <svg
                        class="dash-spark-svg"
                        viewBox=format!("0 0 {} {}", W, H)
                        preserveAspectRatio="none"
                        width="100%"
                        height="100%"
                    >
                        <defs>
                            <linearGradient id=grad_id.clone() x1="0" y1="0" x2="0" y2="1">
                                <stop offset="0%" stop-color=color stop-opacity="0.28"/>
                                <stop offset="100%" stop-color=color stop-opacity="0"/>
                            </linearGradient>
                        </defs>
                        <path d=fill fill=format!("url(#{})", grad_id) stroke="none"/>
                        <path
                            class="dash-spark-line"
                            d=line
                            fill="none"
                            stroke=color
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            vector-effect="non-scaling-stroke"
                        />
                    </svg>
                }.into_any(),
                None => ().into_any(),
            }}
        </div>
    }
}

#[component]
fn DashboardPage(set_page: WriteSignal<Page>, username: String) -> impl IntoView {
    let (summary, set_summary) = signal(None::<crate::api::DashboardSummary>);
    let (chart_data, set_chart_data) = signal(Vec::<crate::api::DashboardChartPoint>::new());
    let (chart_period, set_chart_period) = signal("year".to_string());
    let (hovered_bar, set_hovered_bar) = signal(None::<usize>);
    let (txn_query, set_txn_query) = signal(String::new());
    let (loading, set_loading) = signal(true);

    let display_name = {
        let n = username.trim();
        if n.is_empty() {
            "Admin".to_string()
        } else {
            let mut chars = n.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => "Admin".to_string(),
            }
        }
    };

    let load_summary = {
        move || {
            leptos::task::spawn_local(async move {
                if let Ok(s) = api::get_dashboard_summary().await {
                    set_summary.set(Some(s));
                }
                set_loading.set(false);
            });
        }
    };

    load_summary();
    // Period change always reloads chart (tracks chart_period)
    create_effect(move |_| {
        let period = chart_period.get();
        set_hovered_bar.set(None);
        leptos::task::spawn_local(async move {
            if let Ok(points) = api::get_dashboard_chart(&period).await {
                set_chart_data.set(points);
            }
        });
    });
    let (live_tick, set_live_tick) = signal(0u64);
    create_effect(move |_| {
        let tick = live_tick.get();
        if tick == 0 {
            return;
        }
        load_summary();
    });
    use_auto_refresh(LIVE_REFRESH_MS, move || {
        set_live_tick.update(|t| *t = t.wrapping_add(1));
    });

    let fmt_money = |a: f64| format!("KSh {:.0}", a);

    let chart_y_ticks = move || {
        let data = chart_data.get();
        let max_val = data
            .iter()
            .map(|p| p.amount)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let rough_step = max_val / 4.0;
        let magnitude = 10_f64.powf(rough_step.max(1.0).log10().floor());
        let normalized = rough_step / magnitude;
        let nice = if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };
        let step = (nice * magnitude).max(1.0);
        let max_tick = (max_val / step).ceil() * step;
        let mut ticks = Vec::new();
        let mut val = 0.0;
        while val <= max_tick + step * 0.5 {
            ticks.push(val);
            val += step;
        }
        if ticks.len() < 2 {
            ticks = vec![0.0, max_val];
        }
        ticks
    };

    let filtered_txns = move || {
        let q = txn_query.get().trim().to_lowercase();
        let items = summary
            .get()
            .map(|s| s.recent_transactions)
            .unwrap_or_default();
        if q.is_empty() {
            return items;
        }
        items
            .into_iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&q)
                    || t.type_label.to_lowercase().contains(&q)
                    || t.date.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    };

    view! {
        {move || if loading.get() {
            view! {
                <div class="dash dash--loading" role="status" aria-live="polite" aria-busy="true">
                    <PageLoading message="Loading dashboard..."/>
                </div>
            }.into_any()
        } else {
            view! {
        <div class="dash">
            <div class="dash-top">
                <h2 class="dash-welcome">"Welcome back, " {display_name.clone()} "!"</h2>
                <div class="dash-period" role="group" aria-label="Chart period">
                    {let cp = chart_period;
                    let set_cp = set_chart_period;
                    [("week", "This Week"), ("month", "This Month"), ("year", "This Year")]
                        .into_iter()
                        .map(|(val, label)| {
                            let v = val;
                            view! {
                                <button
                                    type="button"
                                    class=move || {
                                        if cp.get() == v {
                                            "dash-period-btn is-active"
                                        } else {
                                            "dash-period-btn"
                                        }
                                    }
                                    on:click=move |_| set_cp.set(v.to_string())
                                >{label}</button>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>
            </div>

            <div class="dash-hero">
                <div class="dash-card dash-chart-card">
                    <div class="dash-chart-head">
                        <h3 class="dash-chart-title">"Revenue"</h3>
                        <span class="dash-chart-sub">
                            {move || match chart_period.get().as_str() {
                                "week" => "Last 7 days",
                                "month" => "Last 4 weeks",
                                _ => "Last 12 months",
                            }}
                        </span>
                    </div>
                    // CSS bars fill 100% width — no SVG letterboxing / side gaps
                    <div class="dash-chart-body">
                        {move || {
                            let data = chart_data.get();
                            if data.is_empty() {
                                return view! {
                                    <p class="dash-chart-empty">"No data for this period"</p>
                                }.into_any();
                            }
                            let period = chart_period.get();
                            let ticks = chart_y_ticks();
                            let max_tick = ticks.last().copied().unwrap_or(1.0).max(1.0);
                            // Only highlight / tip while actually hovering — never sticky peak
                            let hot = hovered_bar.get();

                            // Y labels top → bottom
                            let y_labels: Vec<String> = ticks
                                .iter()
                                .rev()
                                .map(|val| {
                                    if *val >= 1_000_000.0 {
                                        format!("{:.1}M", val / 1_000_000.0)
                                    } else if *val >= 1000.0 {
                                        format!("{}k", (*val / 1000.0) as i64)
                                    } else {
                                        format!("{:.0}", val)
                                    }
                                })
                                .collect();

                            view! {
                                <div class="dash-bars-layout">
                                    <div class="dash-y-axis" aria-hidden="true">
                                        {y_labels.into_iter().map(|lab| view! {
                                            <span class="dash-y-tick">{lab}</span>
                                        }).collect::<Vec<_>>()}
                                    </div>
                                    <div class="dash-plot">
                                        <div class="dash-plot-grid" aria-hidden="true">
                                            {(0..ticks.len()).map(|_| view! {
                                                <span class="dash-plot-grid-line"></span>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                        <div class="dash-bars">
                                            {data.into_iter().enumerate().map(|(i, point)| {
                                                let val = point.amount;
                                                let label = short_chart_label(&point.label, &period);
                                                let pct = if max_tick > 0.0 {
                                                    ((val / max_tick) * 100.0).clamp(0.0, 100.0)
                                                } else {
                                                    0.0
                                                };
                                                // Tiny visible stub for zeros so columns still read
                                                let height_pct = if val > 0.0 { pct.max(2.0) } else { 1.5 };
                                                let active = hot == Some(i);
                                                let zero = val <= 0.0;
                                                let tip_money = fmt_money(val);
                                                let tip_label = label.clone();
                                                view! {
                                                    <div
                                                        class=if active { "dash-bar-col is-active" } else { "dash-bar-col" }
                                                        on:mouseenter=move |_| set_hovered_bar.set(Some(i))
                                                        on:mouseleave=move |_| set_hovered_bar.set(None)
                                                    >
                                                        <div class="dash-bar-track">
                                                            <div
                                                                class=if zero {
                                                                    "dash-bar-fill is-zero"
                                                                } else if active {
                                                                    "dash-bar-fill is-active"
                                                                } else {
                                                                    "dash-bar-fill"
                                                                }
                                                                style=format!("height:{:.2}%", height_pct)
                                                            >
                                                                {if active && !zero {
                                                                    view! {
                                                                        <div class="dash-bar-tip">
                                                                            <span class="dash-bar-tip-label">{tip_label.clone()}</span>
                                                                            <span class="dash-bar-tip-value">{tip_money}</span>
                                                                        </div>
                                                                    }.into_any()
                                                                } else {
                                                                    ().into_any()
                                                                }}
                                                            </div>
                                                        </div>
                                                        <span class="dash-bar-x">{label}</span>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }}
                    </div>
                </div>

                <div class="dash-card dash-metrics">
                    <div class="dash-metric">
                        <div class="dash-metric-main">
                            <div class="dash-metric-icon" aria-hidden="true">
                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                            </div>
                            <div class="dash-metric-body">
                                <p class="dash-metric-label">"Total Revenue"</p>
                                <p class="dash-metric-value">
                                    {move || fmt_money(summary.get().map(|s| s.total_revenue).unwrap_or(0.0))}
                                </p>
                                <p class="dash-metric-hint is-muted">"All time"</p>
                            </div>
                        </div>
                        {move || {
                            // Revenue trend for the selected chart period
                            let vals: Vec<f64> =
                                chart_data.get().into_iter().map(|p| p.amount).collect();
                            view! {
                                <MetricSparkline values=vals color="#6565EC" grad_id="spark-rev"/>
                            }
                        }}
                    </div>
                    <div class="dash-metric">
                        <div class="dash-metric-main">
                            <div class="dash-metric-icon" aria-hidden="true">
                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                            </div>
                            <div class="dash-metric-body">
                                <p class="dash-metric-label">"Outstanding Debts"</p>
                                <p class=move || {
                                    let amt = summary.get().map(|s| s.outstanding_debts).unwrap_or(0.0);
                                    if amt > 0.0 { "dash-metric-value is-danger" } else { "dash-metric-value" }
                                }>
                                    {move || fmt_money(summary.get().map(|s| s.outstanding_debts).unwrap_or(0.0))}
                                </p>
                                <p class=move || {
                                    let n = summary.get().map(|s| s.pending_debts_count).unwrap_or(0);
                                    if n > 0 { "dash-metric-hint is-warn" } else { "dash-metric-hint is-muted" }
                                }>
                                    {move || format!("{} pending", summary.get().map(|s| s.pending_debts_count).unwrap_or(0))}
                                </p>
                            </div>
                        </div>
                        {move || {
                            // Pending debt remaining amounts created in each period bucket
                            let vals: Vec<f64> =
                                chart_data.get().into_iter().map(|p| p.debt_amount).collect();
                            view! {
                                <MetricSparkline values=vals color="#EF4444" grad_id="spark-debt"/>
                            }
                        }}
                    </div>
                    <div class="dash-metric">
                        <div class="dash-metric-main">
                            <div class="dash-metric-icon" aria-hidden="true">
                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M16 11V7a4 4 0 00-8 0v4M5 9h14l1 12H4L5 9z"/></svg>
                            </div>
                            <div class="dash-metric-body">
                                <p class="dash-metric-label">"Today's Sales"</p>
                                <p class="dash-metric-value">
                                    {move || summary.get().map(|s| s.today_sales_count).unwrap_or(0)}
                                </p>
                                <p class="dash-metric-hint is-ok">
                                    {move || format!("KSh {:.0} today", summary.get().map(|s| s.today_revenue).unwrap_or(0.0))}
                                </p>
                            </div>
                        </div>
                        {move || {
                            // Transaction count trend (sales + printing jobs) for the period
                            let vals: Vec<f64> =
                                chart_data.get().into_iter().map(|p| p.sales_count).collect();
                            view! {
                                <MetricSparkline values=vals color="#F59E0B" grad_id="spark-sales"/>
                            }
                        }}
                    </div>
                    <div class="dash-metric">
                        <div class="dash-metric-main">
                            <div class="dash-metric-icon" aria-hidden="true">
                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/></svg>
                            </div>
                            <div class="dash-metric-body">
                                <p class="dash-metric-label">"Top Product"</p>
                                <p class="dash-metric-value dash-metric-value--sm">
                                    {move || {
                                        summary.get()
                                            .and_then(|s| s.top_products.first().map(|p| p.name.clone()))
                                            .unwrap_or_else(|| "—".to_string())
                                    }}
                                </p>
                                <p class="dash-metric-hint is-muted">
                                    {move || {
                                        summary.get()
                                            .and_then(|s| s.top_products.first().map(|p| format!("{} sold", p.quantity)))
                                            .unwrap_or_else(|| "No sales yet".to_string())
                                    }}
                                </p>
                            </div>
                        </div>
                        // Categorical metric — sparkline not applicable
                    </div>
                </div>
            </div>

            // ---- Transactions ----
            <div class="dash-table-section">
                <div class="dash-table-head">
                    <h3 class="dash-section-title">"Recent Transactions"</h3>
                    <div class="dash-table-actions">
                        <label class="dash-search">
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-4.35-4.35M11 18a7 7 0 100-14 7 7 0 000 14z"/>
                            </svg>
                            <input
                                type="search"
                                placeholder="Search..."
                                prop:value=move || txn_query.get()
                                on:input=move |ev| set_txn_query.set(event_target_value(&ev))
                                aria-label="Search transactions"
                            />
                        </label>
                        <button
                            type="button"
                            class="dash-btn-primary"
                            on:click=move |_| set_page.set(Page::Sales)
                        >
                            <span aria-hidden="true">"+"</span>
                            " New Sale"
                        </button>
                    </div>
                </div>

                <div class="dash-card dash-table-card">
                    <table class="dash-table">
                        <thead>
                            <tr>
                                <th>"Item"</th>
                                <th>"Type"</th>
                                <th>"Date"</th>
                                <th>"Amount"</th>
                                <th>"Status"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                let items = filtered_txns();
                                if items.is_empty() {
                                    return view! {
                                        <tr>
                                            <td colspan="5" class="dash-table-empty">"No transactions found"</td>
                                        </tr>
                                    }.into_any();
                                }
                                items.into_iter().map(|item| {
                                    let (status_label, status_cls) = if item.is_debt {
                                        ("Debt", "dash-status is-warn")
                                    } else {
                                        ("Completed", "dash-status is-ok")
                                    };
                                    view! {
                                        <tr>
                                            <td class="dash-td-strong">{item.name}</td>
                                            <td class="dash-td-muted">{item.type_label}</td>
                                            <td class="dash-td-muted">{item.date}</td>
                                            <td class="dash-td-strong tnum">{format!("KSh {:.0}", item.amount)}</td>
                                            <td><span class=status_cls>{status_label}</span></td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }}
                        </tbody>
                    </table>
                    <div class="dash-table-foot">
                        <span class="dash-table-count">
                            {move || {
                                let n = filtered_txns().len();
                                if n == 0 {
                                    "No rows".to_string()
                                } else if n == 1 {
                                    "Showing 1 transaction".to_string()
                                } else {
                                    format!("Showing {} transactions", n)
                                }
                            }}
                        </span>
                        <button
                            type="button"
                            class="dash-link"
                            on:click=move |_| set_page.set(Page::Sales)
                        >"View all sales"</button>
                    </div>
                </div>
            </div>
        </div>
            }.into_any()
        }}
    }
}

#[component]
fn StockPage() -> impl IntoView {
    let (stock, set_stock) = signal(Vec::<crate::api::StockItem>::new());
    let (color, set_color) = signal(String::new());
    let (rolls, set_rolls) = signal(0_i64);
    let (stype, set_stype) = signal("colored".to_string());

    let load = {
        let s = set_stock;
        move || {
            leptos::task::spawn_local(async move {
                if let Ok(d) = api::get_all_stock().await {
                    s.set(d);
                }
            });
        }
    };
    load();

    let add = move |_| {
        let c = color.get();
        if c.is_empty() {
            return;
        }
        let l = load;
        leptos::task::spawn_local(async move {
            let _ = api::add_stock(&crate::api::NewStockItem {
                color: c,
                size: "1".into(),
                sticker_type: stype.get(),
                rolls: rolls.get(),
                metres_per_roll: None,
                total_metres: None,
                metres_used: 0.0,
                custom_metres_per_roll: None,
            })
            .await;
            set_color.set(String::new());
            set_rolls.set(0);
            l();
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-6">"Stock"</h1>
            <div class="bg-white rounded-xl p-4 shadow-sm border mb-4 flex gap-3 items-end">
                <div><label class="text-xs">"Color"</label><input type="text" class="border rounded px-2 py-1 text-sm w-24" prop:value=move || color.get() on:input=move |e| set_color.set(event_target_value(&e)) /></div>
                <div><label class="text-xs">"Type"</label><select class="border rounded px-2 py-1 text-sm" on:change=move |e| set_stype.set(event_target_value(&e))><option value="colored">"Colored"</option><option value="clear">"Clear"</option><option value="reflective">"Reflective"</option></select></div>
                <div><label class="text-xs">"Rolls"</label><input type="number" class="border rounded px-2 py-1 text-sm w-20" prop:value=move || rolls.get() on:input=move |e| set_rolls.set(event_target_value(&e).parse().unwrap_or(0)) /></div>
                <button on:click=add class="bg-brand-600 text-white rounded px-4 py-1.5 text-sm">"+ Add"</button>
            </div>
            <div class="bg-white rounded-xl shadow-sm border overflow-hidden">
                <table class="w-full text-sm">
                    <thead class="bg-gray-50"><tr><th class="px-4 py-2 text-left">"Color"</th><th class="px-4 py-2">"Type"</th><th class="px-4 py-2 text-right">"Rolls"</th><th class="px-4 py-2 text-right">"Total m"</th><th class="px-4 py-2 text-right">"Used m"</th><th class="px-4 py-2 text-right">""</th></tr></thead>
                    <tbody>
                        {move || stock.get().into_iter().map(|s| {
                            let l = load;
                            view! { <tr class="border-t hover:bg-gray-50">
                                <td class="px-4 py-2 capitalize">{s.color}</td>
                                <td class="px-4 py-2 capitalize">{s.sticker_type}</td>
                                <td class="px-4 py-2 text-right">{s.rolls}</td>
                                <td class="px-4 py-2 text-right">{s.total_metres}</td>
                                <td class="px-4 py-2 text-right">{s.metres_used}</td>
                                <td class="px-4 py-2 text-right"><button on:click=move |_| { let sid = s.id; leptos::task::spawn_local(async move { let _ = api::delete_stock(sid).await; l(); }); } class="text-red-600 text-xs">"Del"</button></td>
                            </tr>}
                        }).collect::<Vec<_>>()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[component]
fn SalesPage() -> impl IntoView {
    let (sales, set_sales) = signal(Vec::<crate::api::Sale>::new());
    let (amount, set_amount) = signal(0.0_f64);
    let (cust, set_cust) = signal("Walk-in".to_string());
    let (pm, set_pm) = signal("cash".to_string());

    let load = {
        let s = set_sales;
        move || {
            leptos::task::spawn_local(async move {
                if let Ok(d) = api::get_all_sales().await {
                    s.set(d);
                }
            });
        }
    };
    load();

    let add = move |_| {
        if amount.get() <= 0.0 {
            return;
        }
        let l = load;
        leptos::task::spawn_local(async move {
            let _ = api::add_sale(&crate::api::NewSale {
                r#type: "product".into(),
                product_id: None,
                stock_id: None,
                product_name: Some("Sale".into()),
                product_type: None,
                sticker_type: None,
                quantity: None,
                amount: amount.get(),
                payment_method: pm.get(),
                customer_name: cust.get(),
                is_debt: 0,
                product_quantity: None,
                stock_metres_used: None,
            })
            .await;
            set_amount.set(0.0);
            l();
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-6">"Sales"</h1>
            <div class="bg-white rounded-xl p-4 shadow-sm border mb-4 flex gap-3 items-end">
                <div><label class="text-xs">"Amount"</label><input type="number" step="0.01" class="border rounded px-2 py-1 text-sm w-28" prop:value=move || amount.get() on:input=move |e| set_amount.set(event_target_value(&e).parse().unwrap_or(0.0)) /></div>
                <div><label class="text-xs">"Payment"</label><select class="border rounded px-2 py-1 text-sm" on:change=move |e| set_pm.set(event_target_value(&e))><option value="cash">"Cash"</option><option value="mpesa">"M-Pesa"</option></select></div>
                <div><label class="text-xs">"Customer"</label><input type="text" class="border rounded px-2 py-1 text-sm w-32" prop:value=move || cust.get() on:input=move |e| set_cust.set(event_target_value(&e)) /></div>
                <button on:click=add class="bg-brand-600 text-white rounded px-4 py-1.5 text-sm">"Record"</button>
            </div>
            <div class="bg-white rounded-xl shadow-sm border overflow-hidden">
                <table class="w-full text-sm"><thead class="bg-gray-50"><tr><th class="px-4 py-2 text-left">"Customer"</th><th class="px-4 py-2 text-right">"Amount"</th><th class="px-4 py-2">"Payment"</th></tr></thead>
                <tbody>
                    {move || sales.get().into_iter().take(30).map(|s| view! { <tr class="border-t hover:bg-gray-50">
                        <td class="px-4 py-2">{s.customer_name}</td>
                        <td class="px-4 py-2 text-right">"KSh " {s.amount}</td>
                        <td class="px-4 py-2 capitalize">{s.payment_method}</td>
                    </tr>}).collect::<Vec<_>>()}
                </tbody></table>
            </div>
        </div>
    }
}

#[component]
fn PrintingPage() -> impl IntoView {
    let (txns, set_txns) = signal(Vec::<crate::api::ServiceTransaction>::new());
    let (name, set_name) = signal(String::new());
    let (amount, set_amount) = signal(0.0_f64);

    let load = {
        let s = set_txns;
        move || {
            leptos::task::spawn_local(async move {
                if let Ok(t) = api::get_all_service_transactions().await {
                    s.set(t);
                }
            });
        }
    };
    load();

    let add = move |_| {
        if amount.get() <= 0.0 {
            return;
        }
        let n = name.get();
        let l = load;
        leptos::task::spawn_local(async move {
            let _ = api::add_service_transaction(&crate::api::NewServiceTransaction {
                service_id: None,
                service_name: n,
                quantity: 1.0,
                price: None,
                amount: Some(amount.get()),
                payment_method: "cash".into(),
                customer_name: "Walk-in".into(),
                notes: None,
                stock_id: None,
                stock_metres_used: 0.0,
                material_size: None,
                material_type: None,
                printing_material_id: None,
                is_debt: 0,
            })
            .await;
            set_amount.set(0.0);
            set_name.set(String::new());
            l();
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-6">"Printing"</h1>
            <div class="bg-white rounded-xl p-4 shadow-sm border mb-4 flex gap-3 items-end">
                <div><label class="text-xs">"Job"</label><input type="text" class="border rounded px-2 py-1 text-sm w-40" prop:value=move || name.get() on:input=move |e| set_name.set(event_target_value(&e)) /></div>
                <div><label class="text-xs">"Amount"</label><input type="number" step="0.01" class="border rounded px-2 py-1 text-sm w-28" prop:value=move || amount.get() on:input=move |e| set_amount.set(event_target_value(&e).parse().unwrap_or(0.0)) /></div>
                <button on:click=add class="bg-brand-600 text-white rounded px-4 py-1.5 text-sm">"Record"</button>
            </div>
            <div class="bg-white rounded-xl shadow-sm border overflow-hidden">
                <table class="w-full text-sm"><thead class="bg-gray-50"><tr><th class="px-4 py-2 text-left">"Job"</th><th class="px-4 py-2 text-right">"Amount"</th></tr></thead>
                <tbody>{move || txns.get().into_iter().take(20).map(|t| view! { <tr class="border-t"><td class="px-4 py-2">{t.service_name}</td><td class="px-4 py-2 text-right">"KSh " {t.amount}</td></tr>}).collect::<Vec<_>>()}</tbody></table>
            </div>
        </div>
    }
}

#[component]
fn DebtsPage() -> impl IntoView {
    let (debts, set_debts) = signal(Vec::<crate::api::Debt>::new());
    let (cust, set_cust) = signal(String::new());
    let (amt, set_amt) = signal(0.0_f64);

    let load = {
        let s = set_debts;
        move || {
            leptos::task::spawn_local(async move {
                if let Ok(d) = api::get_all_debts().await {
                    s.set(d);
                }
            });
        }
    };
    load();

    let add = move |_| {
        let c = cust.get();
        let a = amt.get();
        if c.is_empty() || a <= 0.0 {
            return;
        }
        let l = load;
        leptos::task::spawn_local(async move {
            let _ = api::add_debt(&crate::api::NewDebt {
                customer_name: c,
                phone: None,
                amount: a,
                paid_amount: Some(0.0),
                remaining_amount: Some(a),
                due_date: None,
                description: None,
                sale_id: None,
                service_transaction_id: None,
            })
            .await;
            set_cust.set(String::new());
            set_amt.set(0.0);
            l();
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-6">"Debts"</h1>
            <div class="bg-white rounded-xl p-4 shadow-sm border mb-4 flex gap-3 items-end">
                <div><label class="text-xs">"Customer"</label><input type="text" class="border rounded px-2 py-1 text-sm w-32" prop:value=move || cust.get() on:input=move |e| set_cust.set(event_target_value(&e)) /></div>
                <div><label class="text-xs">"Amount"</label><input type="number" step="0.01" class="border rounded px-2 py-1 text-sm w-28" prop:value=move || amt.get() on:input=move |e| set_amt.set(event_target_value(&e).parse().unwrap_or(0.0)) /></div>
                <button on:click=add class="bg-brand-600 text-white rounded px-4 py-1.5 text-sm">"+ Add"</button>
            </div>
            <div class="bg-white rounded-xl shadow-sm border overflow-hidden">
                <table class="w-full text-sm"><thead class="bg-gray-50"><tr><th class="px-4 py-2 text-left">"Customer"</th><th class="px-4 py-2 text-right">"Total"</th><th class="px-4 py-2 text-right">"Paid"</th><th class="px-4 py-2">"Status"</th><th class="px-4 py-2 text-right">""</th></tr></thead>
                <tbody>{move || debts.get().into_iter().map(|d| {
                    let l = load;
                    view! { <tr class="border-t hover:bg-gray-50">
                        <td class="px-4 py-2">{d.customer_name}</td><td class="px-4 py-2 text-right">{d.amount}</td>
                        <td class="px-4 py-2 text-right">{d.paid_amount}</td>
                        <td class="px-4 py-2">{if d.status=="pending"{"Pending"}else{"Paid"}}</td>
                        <td class="px-4 py-2 text-right">
                            {if d.status == "pending" { view! {
                                <button on:click=move |_| { let did = d.id; leptos::task::spawn_local(async move { let _ = api::mark_debt_paid(did).await; l(); }); } class="text-brand-600 text-xs mr-2">"Mark Paid"</button>
                            }.into_any() } else { view! { <span></span> }.into_any() }}
                            <button on:click=move |_| { let did = d.id; leptos::task::spawn_local(async move { let _ = api::delete_debt(did).await; l(); }); } class="text-red-600 text-xs">"Del"</button>
                        </td>
                    </tr>}
                }).collect::<Vec<_>>()}</tbody></table>
            </div>
        </div>
    }
}

#[component]
fn SettingsPage(
    user: ReadSignal<Option<UserInfo>>,
    _su: WriteSignal<Option<UserInfo>>,
) -> impl IntoView {
    let (old_pw, set_old_pw) = signal(String::new());
    let (new_pw, set_new_pw) = signal(String::new());
    let (msg, set_msg) = signal(String::new());
    let cur = move || user.get().map(|u| u.username).unwrap_or_default();
    let _is_admin = move || {
        user.get()
            .map(|u| u.role.as_str() == "admin")
            .unwrap_or(false)
    };

    let change_pw = move |_| {
        let o = old_pw.get();
        let n = new_pw.get();
        let u = cur();
        leptos::task::spawn_local(async move {
            match api::update_password(&u, &o, &n).await {
                Ok(r) if r.success => set_msg.set("Password updated".into()),
                Ok(r) => set_msg.set(r.error.unwrap_or("Error".into())),
                Err(e) => set_msg.set(e),
            }
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-6">"Settings"</h1>
            <p class="text-green-600 text-sm mb-4">{move || msg.get()}</p>
            <div class="bg-white rounded-xl p-5 shadow-sm border mb-6">
                <h2 class="font-semibold mb-4">"Change Password"</h2>
                <p class="text-sm text-gray-500 mb-4">"Logged in as: " <strong>{cur()}</strong></p>
                <div class="grid grid-cols-2 gap-4">
                    <input type="password" class="border rounded px-3 py-2 text-sm" placeholder="Current password" on:input=move |e| set_old_pw.set(event_target_value(&e)) />
                    <input type="password" class="border rounded px-3 py-2 text-sm" placeholder="New password" on:input=move |e| set_new_pw.set(event_target_value(&e)) />
                </div>
                <button on:click=change_pw class="mt-4 bg-brand-600 text-white rounded px-4 py-2 text-sm">"Update"</button>
            </div>
        </div>
    }
}
