#![allow(deprecated)]
#![allow(dead_code)]

use leptos::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use chrono::{Timelike, Datelike};
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use crate::api::{self, UserInfo, LoginResponse};
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
    Reflect::get(&notification, &JsValue::from_str("permission")).ok()?.as_string()
}

fn request_notification_permission() {
    let global = js_sys::global();
    if let Ok(notification) = Reflect::get(&global, &JsValue::from_str("Notification")) {
        if let Ok(func) = Reflect::get(&notification, &JsValue::from_str("requestPermission")).and_then(|v| v.dyn_into::<Function>().map_err(|e| e)) {
            let _ = func.call0(&notification);
        }
    }
}

fn show_desktop_notification(title: &str, body: &str) {
    let global = js_sys::global();
    let Ok(notification) = Reflect::get(&global, &JsValue::from_str("Notification")) else { return; };
    let Ok(ctor) = notification.dyn_into::<Function>() else { return; };

    let options = Object::new();
    let _ = Reflect::set(&options, &JsValue::from_str("body"), &JsValue::from_str(body));
    let _ = Reflect::set(&options, &JsValue::from_str("tag"), &JsValue::from_str("overdue-debts"));

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

    let load_overdue = {
        let set_overdue_debts = set_overdue_debts;
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
                let load_overdue = load_overdue.clone();
                Interval::new(60_000, move || load_overdue()).forget();
            }
        } else {
            set_overdue_debts.set(Vec::new());
        }
    });

    view! {
        {move || {
            if loading.get() {
                view! { <p>"Loading..."</p> }.into_any()
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
                view! {
                    <div class="flex h-screen" style="background:var(--color-bg-base);font-family:var(--font-sans)">
                        <Sidebar user_role=role.clone() current_page=p set_page=sp on_logout=logout />
                        <div class="flex-1 flex flex-col overflow-hidden">
                            <Header overdue_debts=Signal::derive(move || overdue_debts.get()) show_notifications=role == "admin" />
                            <main class="flex-1 overflow-y-auto p-6">
                                {move || match p.get() {
                                    Page::Dashboard => {
                                        if role == "admin" {
                                            view! { <DashboardPage set_page=sp /> }.into_any()
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
                }.into_any()
            } else {
                view! { <LoginPage set_user=set_user set_token=set_token /> }.into_any()
            }
        }}
    }
}

#[component]
fn Sidebar(
    user_role: String,
    current_page: ReadSignal<Page>,
    set_page: WriteSignal<Page>,
    on_logout: impl Fn(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    let nav_class = move |p: Page| -> &'static str {
        if current_page.get() == p { "flex items-center gap-2.5 px-3 py-2.5 mx-0.5 my-0.5 rounded text-[13px] font-medium bg-[#2563EB] text-white cursor-pointer transition-all duration-100" }
        else { "flex items-center gap-2.5 px-3 py-2.5 mx-0.5 my-0.5 rounded text-[13px] font-medium text-[#737373] hover:bg-[rgba(255,255,255,0.04)] hover:text-[#E5E5E5] cursor-pointer transition-all duration-100" }
    };
    let nav_item = |p: Page, label: &'static str, icon: &'static str| {
        view! {
            <span class=move || nav_class(p.clone()) on:click=move |_| set_page.set(p.clone())>
                <svg class="w-[18px] h-[18px] shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d=icon/>
                </svg>
                <span>{label}</span>
            </span>
        }
    };
    view! {
        <aside class="w-[var(--sidebar-width,220px)] bg-[#0F0F0F] flex flex-col shrink-0 h-screen">
            <div class="px-4 py-5 border-b border-[rgba(255,255,255,0.06)]">
                <div class="flex items-center gap-2.5">
                    <svg class="w-7 h-7 text-[#2563EB] shrink-0" viewBox="0 0 32 32" fill="none">
                        <rect x="2" y="2" width="28" height="28" rx="4" stroke="currentColor" stroke-width="1.5"/>
                        <path d="M9 11h14M9 16h10M9 21h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                    </svg>
                    <span class="text-sm font-semibold text-white tracking-wide">"MULTIPRINTS"</span>
                </div>
            </div>
            <nav class="flex-1 flex flex-col px-2 py-3">
                <div class="flex-1">
                    {if user_role == "admin" { view! {
                        {nav_item(Page::Dashboard, "Dashboard", "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z")}
                        {nav_item(Page::Products, "Products", "M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4")}
                        {nav_item(Page::Stock, "Stock", "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10")}
                    }.into_any()} else { ().into_any() }}
                    {nav_item(Page::Sales, "Sales", "M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 11-4 0 2 2 0 014 0z")}
                    {nav_item(Page::Printing, "Printing", "M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z")}
                    {nav_item(Page::Debts, "Debts", "M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z")}
                </div>
                <div class="pt-3 mt-auto border-t border-[rgba(255,255,255,0.06)]">
                    {nav_item(Page::Settings, "Settings", "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z")}
                    <span class="flex items-center gap-2.5 px-3 py-2.5 mx-0.5 my-0.5 rounded text-[13px] font-medium text-[#737373] hover:bg-[rgba(239,68,68,0.1)] hover:text-[#F87171] cursor-pointer transition-all duration-100" on:click=on_logout>
                        <svg class="w-[18px] h-[18px] shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"/>
                        </svg>
                        <span>"Log out"</span>
                    </span>
                </div>
            </nav>
        </aside>
    }
}

#[component]
fn Header(overdue_debts: Signal<Vec<crate::api::Debt>>, show_notifications: bool) -> impl IntoView {
    let (open, set_open) = signal(false);
    let (initial_notified, set_initial_notified) = signal(false);

    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            let set_open = set_open;
            let listener = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                set_open.set(false);
            }));
            let _ = window.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref());
            listener.forget();
        }
    });

    Effect::new(move |_| {
        if !show_notifications {
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
                    );
                    set_initial_notified.set(true);
                }
                Some("denied") => {}
                _ => request_notification_permission(),
            }
        }
    });

    view! {
        <header class="h-14 bg-white border-b border-[#E5E5E5] flex items-center justify-between px-5 shrink-0">
            <div class="flex items-center gap-2"></div>
            <div class="flex items-center gap-2">
                {move || if show_notifications { view! {
                    <div class="relative" on:click=move |e| e.stop_propagation()>
                        <button class="relative p-2 rounded hover:bg-[#F5F5F5] text-[#525252] hover:text-[#0A0A0A] transition-all duration-100" on:click=move |_| set_open.update(|v| *v = !*v)>
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"/>
                            </svg>
                            {move || if !overdue_debts.get().is_empty() { view! {
                                <span class="absolute -top-1 -right-1 min-w-5 h-5 px-1 bg-red-500 text-white text-[10px] rounded-full flex items-center justify-center font-medium">
                                    {move || {
                                        let count = overdue_debts.get().len();
                                        if count > 99 { "99+".to_string() } else { count.to_string() }
                                    }}
                                </span>
                            }.into_any() } else { ().into_any() }}
                        </button>
                        {move || if open.get() { view! {
                            <div class="absolute right-0 mt-2 w-[360px] bg-white border border-[#E5E5E5] shadow-lg z-50 overflow-hidden">
                                <div class="px-4 py-3 border-b border-[#F0F0F0] flex items-center justify-between">
                                    <h3 class="text-sm font-semibold text-[#0A0A0A]">"Notifications"</h3>
                                    <span class="text-xs text-[#737373]">
                                        {move || {
                                            let count = overdue_debts.get().len();
                                            if count == 0 { "No overdue debts".to_string() }
                                            else if count == 1 { "1 overdue debt".to_string() }
                                            else { format!("{} overdue debts", count) }
                                        }}
                                    </span>
                                </div>
                                <div class="max-h-[320px] overflow-y-auto">
                                    {move || {
                                        let items = overdue_debts.get();
                                        if items.is_empty() {
                                            view! {
                                                <div class="p-6 text-center">
                                                    <div class="w-12 h-12 mx-auto mb-3 bg-green-100 flex items-center justify-center rounded-full">
                                                        <svg class="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
                                                    </div>
                                                    <p class="text-sm text-gray-500">"All debts are up to date!"</p>
                                                </div>
                                            }.into_any()
                                        } else {
                                            items.into_iter().map(|debt| {
                                                let due = debt.due_date.clone().unwrap_or_default();
                                                let days_overdue = chrono::NaiveDate::parse_from_str(&due, "%Y-%m-%d")
                                                    .ok()
                                                    .map(|d| (chrono::Local::now().date_naive() - d).num_days().max(0))
                                                    .unwrap_or(0);
                                                let urgency_cls = if days_overdue > 7 {
                                                    "bg-red-50 border-l-4 border-red-500"
                                                } else if days_overdue > 3 {
                                                    "bg-amber-50 border-l-4 border-amber-500"
                                                } else {
                                                    "bg-yellow-50 border-l-4 border-yellow-500"
                                                };
                                                let badge_cls = if days_overdue > 7 {
                                                    "bg-red-100 text-red-800"
                                                } else {
                                                    "bg-amber-100 text-amber-800"
                                                };
                                                view! {
                                                    <div class=move || format!("p-3 {}", urgency_cls)>
                                                        <div class="flex items-start justify-between gap-3">
                                                            <div class="min-w-0 flex-1">
                                                                <p class="text-sm font-medium text-gray-900 truncate">{debt.customer_name}</p>
                                                                <p class="text-xs text-gray-600 mt-0.5">{format!("KSh {:.0}", debt.remaining_amount)}</p>
                                                                {debt.description.clone().map(|desc| view! {
                                                                    <p class="text-xs text-gray-500 mt-1 truncate">{desc}</p>
                                                                })}
                                                            </div>
                                                            <span class=move || format!("inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {}", badge_cls)>
                                                                {format!("{}d overdue", days_overdue)}
                                                            </span>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>().into_any()
                                        }
                                    }}
                                </div>
                            </div>
                        }.into_any() } else { ().into_any() }}
                    </div>
                }.into_any() } else { ().into_any() }}
            </div>
        </header>
    }
}

#[component]
fn LoginPage(
    set_user: WriteSignal<Option<UserInfo>>,
    set_token: WriteSignal<Option<String>>,
) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (show_pw, set_show_pw) = signal(false);
    let (_pw_focused, set_pw_focused) = signal(false);
    let (_user_focused, set_user_focused) = signal(false);
    let (_pw_has_val, set_pw_has_val) = signal(false);

    let do_login = move |_| {
        let u = username.get();
        let p = password.get();
        if u.is_empty() || p.is_empty() {
            set_error.set("Please enter username and password".into());
            return;
        }
        set_error.set(String::new());
        set_loading.set(true);
        leptos::task::spawn_local(async move {
            match api::login(&u, &p).await {
                Ok(LoginResponse { success: true, token: Some(tok), user: Some(info), .. }) => {
                    LocalStorage::set("sessionToken", &tok).ok();
                    LocalStorage::set("currentUser", &serde_json::to_string(&info).unwrap_or_default()).ok();
                    set_token.set(Some(tok));
                    set_user.set(Some(info));
                }
                Ok(r) => set_error.set(r.error.unwrap_or("Invalid username or password".into())),
                Err(e) => set_error.set(e),
            }
            set_loading.set(false);
        });
    };

    view! {
        <div style="display:flex;justify-content:center;align-items:center;min-height:100vh;background:#FAFAFA;font-family:var(--font-sans);-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale">
            <div style="width:100%;max-width:400px;padding:24px">
                <div style="background:white;border:1px solid var(--color-border,#E5E5E5);border-radius:var(--radius-xl,12px);padding:40px;box-shadow:var(--shadow-sm,0 1px 3px rgba(0,0,0,0.04),0 1px 2px rgba(0,0,0,0.03))">
                    <div style="text-align:center;margin-bottom:32px">
                        <div style="display:inline-flex;align-items:center;gap:10px;margin-bottom:8px">
                            <svg style="width:32px;height:32px;color:#2563EB" viewBox="0 0 32 32" fill="none">
                                <rect x="2" y="2" width="28" height="28" rx="6" stroke="currentColor" stroke-width="2"/>
                                <path d="M9 11h14M9 16h10M9 21h6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                            <h1 style="font-size:22px;font-weight:600;color:#0A0A0A;letter-spacing:-0.02em;margin:0">"MULTIPRINTS"</h1>
                        </div>
                        <p style="font-size:14px;color:#525252;margin:6px 0 0 0">"Sign in to your account"</p>
                    </div>

                    <div style="margin-bottom:20px">
                        <label style="display:block;font-size:14px;font-weight:500;color:#0A0A0A;margin-bottom:6px">"Username"</label>
                        <input type="text" placeholder="Enter username"
                            autocomplete="username" required
                            style="width:100%;padding:10px 12px;border:1px solid var(--color-border,#E5E5E5);border-radius:var(--radius-md,6px);font-size:14px;font-family:var(--font-sans);transition:all 150ms ease;background:white;color:#0A0A0A;outline:none"
                            on:focus=move |_| set_user_focused.set(true)
                            on:blur=move |_| set_user_focused.set(false)
                            on:input=move |e| set_username.set(event_target_value(&e)) />
                    </div>

                    <div style="margin-bottom:20px">
                        <label style="display:block;font-size:14px;font-weight:500;color:#0A0A0A;margin-bottom:6px">"Password"</label>
                        <div style="position:relative;display:flex;align-items:center">
                            <input type={move || if show_pw.get() { "text" } else { "password" }}
                                placeholder="Enter password" autocomplete="current-password" required
                                style="width:100%;padding:10px 40px 10px 12px;border:1px solid var(--color-border,#E5E5E5);border-radius:var(--radius-md,6px);font-size:14px;font-family:var(--font-sans);transition:all 150ms ease;background:white;color:#0A0A0A;outline:none"
                                on:focus=move |_| set_pw_focused.set(true)
                                on:blur=move |_| set_pw_focused.set(false)
                                on:input=move |e| { set_password.set(event_target_value(&e)); set_pw_has_val.set(!event_target_value(&e).is_empty()); } />
                            <button type="button"
                                style="position:absolute;right:10px;background:none;border:none;cursor:pointer;color:var(--color-text-muted,#A3A3A3);padding:4px;display:flex;align-items:center;justify-content:center;transition:color 150ms ease;border-radius:4px"
                                on:click=move |_| set_show_pw.update(|v| *v = !*v)>
                                {move || if show_pw.get() {
                                    view! {
                                        <svg style="width:18px;height:18px" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                                d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"/>
                                        </svg>
                                    }.into_any()
                                } else {
                                    view! {
                                        <svg style="width:18px;height:18px" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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

                    <Show when=move || !error.get().is_empty()>
                        <div style="color:var(--color-error,#EF4444);font-size:13px;margin-bottom:16px;padding:8px 12px;background:var(--color-error-bg,#FEF2F2);border:1px solid var(--color-error-border,#FECACA);border-radius:var(--radius-md,6px)">
                            {move || error.get()}
                        </div>
                    </Show>

                    <button
                        style="width:100%;padding:10px 16px;background:#2563EB;color:white;border:none;border-radius:var(--radius-md,6px);font-size:14px;font-weight:500;cursor:pointer;transition:all 150ms ease;margin-top:8px;display:flex;align-items:center;justify-content:center"
                        disabled=move || loading.get()
                        on:click=do_login>
                        {move || if loading.get() {
                            view! {
                                <div style="display:flex;align-items:center;gap:8px">
                                    <div style="width:16px;height:16px;border:2px solid rgba(255,255,255,0.3);border-top:2px solid white;border-radius:50%" class="animate-spin"></div>
                                    <span>"Signing in..."</span>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span>"Sign in"</span> }.into_any()
                        }}
                    </button>

                    <div style="text-align:center;margin-top:24px;font-size:12px;color:var(--color-text-muted,#A3A3A3)">
                        "© 2026 MULTIPRINTS"
                    </div>
                </div>
            </div>
        </div>
            }
}

#[component]
fn DashboardPage(set_page: WriteSignal<Page>) -> impl IntoView {
    let (all_sales, set_all_sales) = signal(Vec::<crate::api::Sale>::new());
    let (all_svc, set_all_svc) = signal(Vec::<crate::api::ServiceTransaction>::new());
    let (all_debts, set_all_debts) = signal(Vec::<crate::api::Debt>::new());
    let (all_products, set_all_products) = signal(Vec::<crate::api::Product>::new());
    let (chart_period, set_chart_period) = signal("week".to_string());
    let (hovered_bar, set_hovered_bar) = signal(None::<usize>);
    let (loading, set_loading) = signal(true);

    let hour = chrono::Local::now().hour();
    let greeting = if hour < 12 { "Good morning" } else if hour < 18 { "Good afternoon" } else { "Good evening" };

    leptos::task::spawn_local(async move {
        if let Ok(s) = api::get_all_sales().await { set_all_sales.set(s); }
        if let Ok(t) = api::get_all_service_transactions().await { set_all_svc.set(t); }
        if let Ok(d) = api::get_all_debts().await { set_all_debts.set(d); }
        if let Ok(p) = api::get_all_products().await { set_all_products.set(p); }
        set_loading.set(false);
    });

    let fmt_f = |a: f64| format!("KSh {:.0}", a);

    // ---- derived stats ----
    let total_revenue = move || {
        let sr: f64 = all_sales.get().iter().map(|s| s.amount).sum();
        let sv: f64 = all_svc.get().iter().map(|t| t.amount).sum();
        sr + sv
    };

    let today_stats = {
        let sales = all_sales;
        let svc = all_svc;
        move || {
            let today = chrono::Local::now().date_naive().to_string();
            let s = sales.get(); let t = svc.get();
            let sc = s.iter().filter(|x| x.timestamp.as_deref().unwrap_or("").starts_with(&today)).count();
            let tc = t.iter().filter(|x| x.timestamp.as_deref().unwrap_or("").starts_with(&today)).count();
            let sr: f64 = s.iter().filter(|x| x.timestamp.as_deref().unwrap_or("").starts_with(&today)).map(|x| x.amount).sum();
            let tr: f64 = t.iter().filter(|x| x.timestamp.as_deref().unwrap_or("").starts_with(&today)).map(|x| x.amount).sum();
            ((sc + tc) as i64, sr + tr)
        }
    };

    let debt_stats = {
        let debts = all_debts;
        move || {
            let p: Vec<_> = debts.get().into_iter().filter(|d| d.status == "pending").collect();
            (p.iter().map(|d| d.remaining_amount).sum::<f64>(), p.len() as i64)
        }
    };

    // ---- chart data ----
    let chart_data = {
        let sales = all_sales;
        let svc = all_svc;
        let period = chart_period;
        move || {
            let p = period.get();
            let s = sales.get();
            let t = svc.get();
            let today = chrono::Local::now().date_naive();
            let mut labels: Vec<String> = Vec::new();
            let mut data: Vec<f64> = Vec::new();

            fn day_rev(sales: &[crate::api::Sale], svc: &[crate::api::ServiceTransaction], ds: &str) -> f64 {
                let sr: f64 = sales.iter().filter(|x| x.timestamp.as_deref().unwrap_or("").starts_with(ds)).map(|x| x.amount).sum();
                let tr: f64 = svc.iter().filter(|x| x.timestamp.as_deref().unwrap_or("").starts_with(ds)).map(|x| x.amount).sum();
                sr + tr
            }

            if p == "week" {
                for i in (0..7).rev() {
                    let d = today - chrono::Duration::days(i);
                    labels.push(d.format("%a").to_string());
                    data.push(day_rev(&s, &t, &d.to_string()));
                }
            } else if p == "month" {
                for w in 0..4 {
                    let end = today - chrono::Duration::days((w * 7) as i64);
                    let start = end - chrono::Duration::days(6);
                    labels.push(format!("{}–{}", start.format("%d"), end.format("%d %b")));
                    let mut total = 0.0;
                    for d in 0..7 {
                        let day = start + chrono::Duration::days(d);
                        total += day_rev(&s, &t, &day.to_string());
                    }
                    data.push(total);
                }
                labels.reverse(); data.reverse();
            } else {
                for i in (0..12).rev() {
                    let m = today - chrono::Duration::days(i * 30);
                    let ms = format!("{}-{:02}", m.year(), m.month());
                    labels.push(m.format("%b").to_string());
                    let total: f64 = s.iter().filter(|x| x.timestamp.as_deref().unwrap_or("").starts_with(&ms)).map(|x| x.amount).sum::<f64>()
                        + t.iter().filter(|x| x.timestamp.as_deref().unwrap_or("").starts_with(&ms)).map(|x| x.amount).sum::<f64>();
                    data.push(total);
                }
            }
            (labels, data)
        }
    };

    // ---- chart summary ----
    let chart_stats = move || {
        let (_, data) = chart_data();
        if data.is_empty() { return ("KSh 0".to_string(), "KSh 0".to_string(), "KSh 0".to_string()); }
        let max = data.iter().cloned().fold(0.0_f64, f64::max);
        let sum: f64 = data.iter().sum();
        let avg = sum / data.len() as f64;
        (fmt_f(max), fmt_f(avg as i64 as f64), fmt_f(sum))
    };

    let chart_period_label = move || match chart_period.get().as_str() {
        "month" => "Last 4 weeks",
        "year" => "Last 12 months",
        _ => "Last 7 days",
    };

    // ---- recent transactions (combined) ----
    let recent_txns = move || {
        let mut items: Vec<(String, String, f64, bool, String)> = Vec::new(); // name, date, amount, is_debt, type_label
        for s in all_sales.get().iter() {
            items.push((
                s.product_name.clone().unwrap_or(s.r#type.clone()),
                s.timestamp.as_deref().unwrap_or("").split('T').next().unwrap_or("").to_string(),
                s.amount, s.is_debt > 0,
                "Sale".into(),
            ));
        }
        for t in all_svc.get().iter() {
            if t.stock_metres_used > 0.0 {
                items.push((
                    t.service_name.clone(),
                    t.timestamp.as_deref().unwrap_or("").split('T').next().unwrap_or("").to_string(),
                    t.amount, t.is_debt > 0,
                    "Printing".into(),
                ));
            }
        }
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.truncate(5);
        items
    };

    // ---- activity feed ----
    let activity_items = move || {
        let mut items: Vec<(String, String, String)> = Vec::new(); // (type, text, time)
        for s in all_sales.get().iter().take(20) {
            let time = s.timestamp.as_deref().and_then(|t| t.get(11..16)).unwrap_or("").to_string();
            let name = s.product_name.clone().unwrap_or(s.r#type.clone());
            items.push(("sale".into(), format!("{} — KSh {:.0}", name, s.amount), time));
        }
        for d in all_debts.get().iter().take(20) {
            let time = d.created_at.as_deref().and_then(|t| t.get(11..16)).unwrap_or("").to_string();
            items.push(("debt".into(), format!("Debt: {} — KSh {:.0}", d.customer_name, d.amount), time));
        }
        items.sort_by(|a, b| b.2.cmp(&a.2));
        items.truncate(8);
        items
    };

    // ---- top products ----
    let top_products = move || {
        let products = all_products.get();
        let sales = all_sales.get();
        let mut counts: Vec<(i64, i64, String)> = Vec::new(); // (id, qty, name)
        for s in sales.iter() {
            if let Some(pid) = s.product_id {
                if let Some(pos) = counts.iter().position(|(id, _, _)| *id == pid) {
                    let qty: i64 = s.quantity.as_deref().and_then(|q| q.parse().ok()).unwrap_or(1);
                    counts[pos].1 += qty;
                } else {
                    let qty: i64 = s.quantity.as_deref().and_then(|q| q.parse().ok()).unwrap_or(1);
                    let name = products.iter().find(|p| p.id == pid).map(|p| p.name.clone()).unwrap_or_else(|| format!("Product #{}", pid));
                    counts.push((pid, qty, name));
                }
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        counts.truncate(4);
        counts
    };

    // ---- chart SVG constants ----
    let chart_w = 640.0_f64;
    let chart_h = 260.0_f64;
    let pad_l = 60.0;
    let pad_r = 20.0;
    let pad_t = 20.0;
    let pad_b = 36.0;

    view! {
        <Show when=move || !loading.get() fallback=|| view! { <div class="page-content"><p class="text-gray-500">"Loading dashboard..."</p></div> }>
        <div class="page-content">
            <div class="mb-6">
                <h1 class="text-[22px] font-semibold text-[#0A0A0A] tracking-[-0.02em] mb-1">{greeting} ", Admin"</h1>
                <p class="text-sm text-[#525252]">"Here's what's happening today."</p>
            </div>

            <div class="grid grid-cols-3 gap-4 mb-6">
                <div class="bg-white border border-[#E5E5E5] p-5">
                    <div class="flex items-start justify-between">
                        <div>
                            <p class="text-xs text-gray-500 font-medium mb-1">"Total Revenue"</p>
                            <h3 class="text-xl font-semibold text-[#0A0A0A]">{move || fmt_f(total_revenue())}</h3>
                            <div class="flex items-center gap-1 mt-2 text-xs font-medium text-green-600">
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"/></svg>
                                <span>"All time"</span>
                            </div>
                        </div>
                        <div class="w-9 h-9 flex items-center justify-center bg-[#EFF6FF] text-[#2563EB]">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                        </div>
                    </div>
                </div>
                <div class="bg-white border border-[#E5E5E5] p-5">
                    <div class="flex items-start justify-between">
                        <div>
                            <p class="text-xs text-gray-500 font-medium mb-1">"Today's Sales"</p>
                            <h3 class="text-xl font-semibold text-[#0A0A0A]">{move || { let (c, _) = today_stats(); c }}</h3>
                            <div class="flex items-center gap-1 mt-2 text-xs font-medium text-gray-500">
                                <span>{move || { let (_, r) = today_stats(); format!("Today: KSh {:.0}", r) }}</span>
                            </div>
                        </div>
                        <div class="w-9 h-9 flex items-center justify-center bg-[#EFF6FF] text-[#2563EB]">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M16 11V7a4 4 0 00-8 0v4M5 9h14l1 12H4L5 9z"/></svg>
                        </div>
                    </div>
                </div>
                <div class="bg-white border border-[#E5E5E5] p-5">
                    <div class="flex items-start justify-between">
                        <div>
                            <p class="text-xs text-gray-500 font-medium mb-1">"Outstanding Debts"</p>
                            <h3 class="text-xl font-semibold text-[#EF4444]">{move || { let (o, _) = debt_stats(); fmt_f(o) }}</h3>
                            <div class="flex items-center gap-1 mt-2 text-xs font-medium text-red-500">
                                <span>{move || { let (_, c) = debt_stats(); format!("{} pending", c) }}</span>
                            </div>
                        </div>
                        <div class="w-9 h-9 flex items-center justify-center bg-[#EFF6FF] text-[#2563EB]">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                        </div>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-3 gap-6">
                <div class="col-span-2 space-y-6">
                    // ---- Revenue Chart ----
                    <div class="bg-white border border-[#E5E5E5] p-5">
                        <div class="flex items-center justify-between mb-5">
                            <div>
                                <h3 class="text-base font-semibold text-[#0A0A0A] tracking-[-0.01em]">"Revenue"</h3>
                                <p class="text-xs text-gray-500 mt-0.5">{move || chart_period_label()}</p>
                            </div>
                            <div class="flex border border-gray-200 rounded overflow-hidden">
                                {let cp_rd = chart_period;
                                let cp_wr = set_chart_period;
                                let periods = [("week", "Week"), ("month", "Month"), ("year", "Year")];
                                periods.iter().map(|(val, label)| {
                                    let v = *val;
                                    view! {
                                        <button
                                            on:click=move |_| cp_wr.set(v.to_string())
                                            class={move || if cp_rd.get() == v { "px-3 py-1.5 text-xs font-medium bg-[#2563EB] text-white" } else { "px-3 py-1.5 text-xs font-medium text-gray-500 hover:bg-gray-50" }}
                                        >{*label}</button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                        <div class="relative w-full" style="overflow-x:auto">
                            <svg viewBox=move || format!("0 0 {} {}", chart_w, chart_h) class="w-full" style="min-height:260px;max-height:320px">
                                // Y-axis gridlines + labels
                                {move || {
                                    let (_, data) = chart_data();
                                    let max_val = data.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
                                    let step = if max_val <= 1000.0 { 200.0 } else if max_val <= 5000.0 { 1000.0 } else if max_val <= 20000.0 { 5000.0 } else if max_val <= 100_000.0 { 20000.0 } else { (max_val / 5.0).max(1000.0) };
                                    let n_ticks = ((max_val / step).ceil() as usize).max(1);
                                    let plot_h = chart_h - pad_t - pad_b;
                                    let mut lines = Vec::new();
                                    for i in 0..=n_ticks {
                                        let val = i as f64 * step;
                                        if val > max_val + step * 0.5 { continue; }
                                        let y = pad_t + plot_h - (val / max_val.max(step)) * plot_h;
                                        let label = if val >= 1_000_000.0 { format!("{:.1}M", val/1_000_000.0) } else if val >= 1000.0 { format!("{}k", (val/1000.0) as i64) } else { format!("{:.0}", val) };
                                        lines.push(view! {
                                            <g>
                                                <line x1=pad_l y1=y x2=chart_w-pad_r y2=y stroke="#f3f4f6" stroke-width="1"/>
                                                <text x=pad_l-8.0 y=y+4.0 text-anchor="end" fill="#9ca3af" font-size="11" font-family="Inter,system-ui,sans-serif">"KSh " {label}</text>
                                            </g>
                                        }.into_any());
                                    }
                                    lines.into_any()
                                }}
                                // Bars
                                {move || {
                                    let (_labels, data) = chart_data();
                                    if data.is_empty() { return view! { <text x=chart_w/2.0 y=chart_h/2.0 text-anchor="middle" fill="#9ca3af" font-size="13">"No data"</text> }.into_any(); }
                                    let max_val = data.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
                                    let plot_h = chart_h - pad_t - pad_b;
                                    let plot_w = chart_w - pad_l - pad_r;
                                    let bar_gap = 4.0;
                                    let bar_w = ((plot_w - bar_gap * (data.len() - 1) as f64) / data.len() as f64).max(6.0);
                                    let hovered = hovered_bar;
                                    data.iter().enumerate().map(|(i, &val)| {
                                        let bar_h = (val / max_val * plot_h).max(if val > 0.0 { 3.0 } else { 0.0 });
                                        let x = pad_l + i as f64 * (bar_w + bar_gap);
                                        let y = pad_t + plot_h - bar_h;
                                        let is_hovered = move || hovered.get() == Some(i);
                                        view! {
                                            <g>
                                                <rect
                                                    x=x y=y width=bar_w height=bar_h
                                                    rx="4" ry="4"
                                                    fill={move || if is_hovered() { "#374151" } else { "#111827" }}
                                                    style="cursor:pointer;transition:fill 0.15s"
                                                    on:mouseenter=move |_| set_hovered_bar.set(Some(i))
                                                    on:mouseleave=move |_| set_hovered_bar.set(None)
                                                ></rect>
                                                // tooltip
                                                {move || if hovered.get() == Some(i) {
                                                    view! {
                                                        <g>
                                                            <rect x=x-20.0 y=y-28.0 width=bar_w+40.0 height=22.0 rx="6" fill="#111827"></rect>
                                                            <text x=x+bar_w/2.0 y=y-13.0 text-anchor="middle" fill="white" font-size="12" font-weight="bold" font-family="Inter,system-ui,sans-serif">{fmt_f(val)}</text>
                                                        </g>
                                                    }.into_any()
                                                } else { ().into_any() }}
                                            </g>
                                        }.into_any()
                                    }).collect::<Vec<_>>().into_any()
                                }}
                                // X-axis labels
                                {move || {
                                    let (labels, data) = chart_data();
                                    if data.is_empty() { return ().into_any(); }
                                    let plot_w = chart_w - pad_l - pad_r;
                                    let bar_gap = 4.0;
                                    let bar_w = ((plot_w - bar_gap * (data.len() - 1) as f64) / data.len() as f64).max(6.0);
                                    labels.iter().enumerate().map(|(i, lbl)| {
                                        let x = pad_l + i as f64 * (bar_w + bar_gap) + bar_w / 2.0;
                                        view! {
                                            <text x=x y=chart_h-6.0 text-anchor="middle" fill="#9ca3af" font-size="11" font-family="Inter,system-ui,sans-serif">{lbl.clone()}</text>
                                        }.into_any()
                                    }).collect::<Vec<_>>().into_any()
                                }}
                            </svg>
                        </div>
                        // Summary stats
                        <div class="grid grid-cols-3 gap-4 mt-5 pt-5 border-t border-gray-100">
                            <div class="text-center">
                                <p class="text-xs text-gray-500 mb-1">"Highest Day"</p>
                                <p class="text-sm font-semibold text-[#0A0A0A]">{move || { let (h, _, _) = chart_stats(); h }}</p>
                            </div>
                            <div class="text-center">
                                <p class="text-xs text-gray-500 mb-1">"Average"</p>
                                <p class="text-sm font-semibold text-[#0A0A0A]">{move || { let (_, a, _) = chart_stats(); a }}</p>
                            </div>
                            <div class="text-center">
                                <p class="text-xs text-gray-500 mb-1">"Period Total"</p>
                                <p class="text-sm font-semibold text-[#0A0A0A]">{move || { let (_, _, t) = chart_stats(); t }}</p>
                            </div>
                        </div>
                    </div>

                    // ---- Recent Transactions ----
                    <div class="bg-white border border-[#E5E5E5] p-5">
                        <div class="flex items-center justify-between mb-4">
                            <h3 class="text-base font-semibold text-[#0A0A0A] tracking-[-0.01em]">"Recent Transactions"</h3>
                            <span class="text-xs font-medium text-[#2563EB] cursor-pointer" on:click=move |_| set_page.set(Page::Sales)>"View all"</span>
                        </div>
                        <table class="w-full text-left">
                            <thead>
                                <tr>
                                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5]">"Item"</th>
                                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5]">"Date"</th>
                                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5]">"Amount"</th>
                                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5]">"Status"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    let items = recent_txns();
                                    if items.is_empty() {
                                        return view! { <tr><td colspan="4" class="px-6 py-8 text-center text-gray-400 italic">"No transactions yet"</td></tr> }.into_any();
                                    }
                                    items.into_iter().map(|(name, date, amount, is_debt, typ)| {
                                        let status = if is_debt { ("Debt", "bg-[#FFFBEB] text-[#F59E0B]") } else { ("Completed", "bg-[#ECFDF5] text-[#10B981]") };
                                        view! {
                                            <tr class="border-b border-[#F0F0F0] hover:bg-[#F5F5F5] transition-all duration-100">
                                                <td class="px-4 py-[14px] text-sm text-[#0A0A0A]">{name} <span class="text-xs text-gray-400 ml-1">"(" {typ} ")"</span></td>
                                                <td class="px-4 py-[14px] text-sm text-[#0A0A0A]">{date}</td>
                                                <td class="px-4 py-[14px] text-sm text-[#0A0A0A]">"KSh " {amount}</td>
                                                <td class="px-4 py-[14px]"><span class={format!("inline-flex items-center gap-1 px-2.5 py-1 text-xs font-medium rounded {}", status.1)}>{status.0}</span></td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>().into_any()
                                }}
                            </tbody>
                        </table>
                    </div>
                </div>

                // ---- Right Column ----
                <div class="space-y-6">
                    // Activity Feed
                    <div class="bg-white border border-[#E5E5E5] p-5">
                        <h3 class="text-base font-semibold text-[#0A0A0A] tracking-[-0.01em] mb-5">"Activity"</h3>
                        <div class="relative">
                            {move || {
                                let items = activity_items();
                                if items.is_empty() {
                                    return view! { <p class="text-sm text-[#A3A3A3] text-center py-8">"No recent activity"</p> }.into_any();
                                }
                                let len = items.len();
                                items.into_iter().enumerate().map(|(i, (typ, text, time))| {
                                    let is_last = i == len - 1;
                                    let dot_color = if typ == "debt" { "bg-[#F59E0B]" } else { "bg-[#2563EB]" };
                                    view! {
                                        <div class="relative pl-5 pb-5">
                                            {if !is_last { view! { <div class="absolute top-1.5 left-[5px] bottom-[-4px] w-px bg-[#E5E5E5]"></div> }.into_any() } else { ().into_any() }}
                                            <div class={format!("absolute top-1 left-0 w-2.5 h-2.5 rounded-full {}", dot_color)}></div>
                                            <p class="text-sm font-medium text-[#0A0A0A]">{text}</p>
                                            <p class="text-xs text-[#525252] mt-0.5">{time}</p>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }}
                        </div>
                    </div>

                    // Top Products
                    <div class="bg-white border border-[#E5E5E5] p-5">
                        <h3 class="text-base font-semibold text-[#0A0A0A] tracking-[-0.01em] mb-5">"Top Products"</h3>
                        <div class="space-y-4">
                            {move || {
                                let items = top_products();
                                if items.is_empty() {
                                    return view! { <p class="text-sm text-[#A3A3A3] text-center py-8">"No sales data available"</p> }.into_any();
                                }
                                let max_qty = items.first().map(|(_, q, _)| *q as f64).unwrap_or(1.0);
                                items.into_iter().map(|(_, qty, name)| {
                                    let pct = (qty as f64 / max_qty * 100.0).min(100.0);
                                    view! {
                                        <div>
                                            <div class="flex justify-between text-sm mb-1">
                                                <span class="font-medium text-[#0A0A0A]">{name}</span>
                                                <span class="text-gray-500">{qty} " sold"</span>
                                            </div>
                                            <div class="w-full bg-gray-100 rounded-full h-1.5">
                                                <div class="bg-[#111827] h-1.5 rounded-full" style=move || format!("width:{}%", pct)></div>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }}
                        </div>
                    </div>
                </div>
            </div>
        </div>
        </Show>
    }
}

#[component]
fn StockPage() -> impl IntoView {
    let (stock, set_stock) = signal(Vec::<crate::api::StockItem>::new());
    let (color, set_color) = signal(String::new());
    let (rolls, set_rolls) = signal(0_i64);
    let (stype, set_stype) = signal("colored".to_string());

    let load = { let s = set_stock; move || { leptos::task::spawn_local(async move {
        if let Ok(d) = api::get_all_stock().await { s.set(d); }
    });}};
    load();

    let add = move |_| {
        let c = color.get(); if c.is_empty() { return; }
        let l = load.clone();
        leptos::task::spawn_local(async move {
            let _ = api::add_stock(&crate::api::NewStockItem {
                color: c, size: "1".into(), sticker_type: stype.get(), rolls: rolls.get(),
                metres_per_roll: None, total_metres: None, metres_used: 0.0, custom_metres_per_roll: None,
            }).await;
            set_color.set(String::new()); set_rolls.set(0); l();
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
                            let l = load.clone();
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

    let load = { let s = set_sales; move || { leptos::task::spawn_local(async move {
        if let Ok(d) = api::get_all_sales().await { s.set(d); }
    });}};
    load();

    let add = move |_| {
        if amount.get() <= 0.0 { return; }
        let l = load.clone();
        leptos::task::spawn_local(async move {
            let _ = api::add_sale(&crate::api::NewSale {
                r#type: "product".into(), product_id: None, stock_id: None,
                product_name: Some("Sale".into()), product_type: None, sticker_type: None,
                quantity: None, amount: amount.get(), payment_method: pm.get(),
                customer_name: cust.get(), is_debt: 0,
            }).await;
            set_amount.set(0.0); l();
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

    let load = { let s = set_txns; move || { leptos::task::spawn_local(async move {
        if let Ok(t) = api::get_all_service_transactions().await { s.set(t); }
    });}};
    load();

    let add = move |_| {
        if amount.get() <= 0.0 { return; }
        let n = name.get(); let l = load.clone();
        leptos::task::spawn_local(async move {
            let _ = api::add_service_transaction(&crate::api::NewServiceTransaction {
                service_id: None, service_name: n, quantity: 1.0, price: None, amount: Some(amount.get()),
                payment_method: "cash".into(), customer_name: "Walk-in".into(), notes: None,
                stock_id: None, stock_metres_used: 0.0, material_size: None, material_type: None,
                printing_material_id: None, is_debt: 0,
            }).await;
            set_amount.set(0.0); set_name.set(String::new()); l();
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

    let load = { let s = set_debts; move || { leptos::task::spawn_local(async move {
        if let Ok(d) = api::get_all_debts().await { s.set(d); }
    });}};
    load();

    let add = move |_| {
        let c = cust.get(); let a = amt.get(); if c.is_empty() || a <= 0.0 { return; }
        let l = load.clone();
        leptos::task::spawn_local(async move {
            let _ = api::add_debt(&crate::api::NewDebt {
                customer_name: c, phone: None, amount: a,
                paid_amount: Some(0.0), remaining_amount: Some(a),
                due_date: None, description: None, sale_id: None, service_transaction_id: None,
            }).await;
            set_cust.set(String::new()); set_amt.set(0.0); l();
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
                    let l = load.clone();
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
    let _is_admin = move || user.get().map(|u| u.role.as_str() == "admin").unwrap_or(false);

    let change_pw = move |_| {
        let o = old_pw.get(); let n = new_pw.get(); let u = cur();
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
