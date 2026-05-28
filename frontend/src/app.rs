use leptos::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use chrono::{Timelike, Datelike};
use crate::api::{self, UserInfo, LoginResponse, SuccessResponse};
#[path = "pages/stock.rs"]
mod stock_page;
use stock_page::StockPage as ElectronStockPage;
#[path = "pages/sales.rs"]
mod sales_page;
use sales_page::SalesPage as ElectronSalesPage;
#[path = "pages/printing.rs"]
mod printing_page;
use printing_page::PrintingPage as ElectronPrintingPage;
#[path = "pages/debts.rs"]
mod debts_page;
use debts_page::DebtsPage as ElectronDebtsPage;
#[path = "pages/settings.rs"]
mod settings_page;
use settings_page::SettingsPage as ElectronSettingsPage;

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

#[component]
pub fn App() -> impl IntoView {
    let (user, set_user) = signal(None::<UserInfo>);
    let (token, set_token) = signal(None::<String>);
    let (loading, set_loading) = signal(true);
    let (page, set_page) = signal(Page::Dashboard);

    leptos::task::spawn_local(async move {
        if let Ok(tok) = LocalStorage::get::<String>("sessionToken") {
            match api::validate_session(&tok).await {
                Ok(true) => {
                    set_token.set(Some(tok));
                    if let Ok(u) = LocalStorage::get::<String>("currentUser") {
                        if let Ok(info) = serde_json::from_str::<UserInfo>(&u) {
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
                            <Header />
                            <main class="flex-1 overflow-y-auto p-6">
                                {move || match p.get() {
                                    Page::Dashboard => view! { <DashboardPage set_page=sp /> }.into_any(),
                                    Page::Products => view! { <ProductsPage /> }.into_any(),
                                    Page::Stock => view! { <ElectronStockPage /> }.into_any(),
                                    Page::Sales => view! { <ElectronSalesPage /> }.into_any(),
                                    Page::Printing => view! { <ElectronPrintingPage /> }.into_any(),
                                    Page::Debts => view! { <ElectronDebtsPage /> }.into_any(),
                                    Page::Settings => view! { <ElectronSettingsPage user=user set_user=set_user /> }.into_any(),
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
                    {nav_item(Page::Dashboard, "Dashboard", "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z")}
                    {if user_role == "admin" { view! {
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
fn Header() -> impl IntoView {
    view! {
        <header class="h-14 bg-white border-b border-[#E5E5E5] flex items-center justify-between px-5 shrink-0">
            <div class="flex items-center gap-2"></div>
            <div class="flex items-center gap-2">
                <button class="relative p-2 rounded hover:bg-[#F5F5F5] text-[#525252] hover:text-[#0A0A0A] transition-all duration-100">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"/>
                    </svg>
                </button>
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
    let (pw_focused, set_pw_focused) = signal(false);
    let (user_focused, set_user_focused) = signal(false);
    let (pw_has_val, set_pw_has_val) = signal(false);

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

    let fmt_k = |a: f64| if a >= 1_000_000.0 { format!("KSh {:.1}M", a / 1_000_000.0) } else if a >= 1000.0 { format!("KSh {}k", (a / 1000.0) as i64) } else { format!("KSh {:.0}", a) };
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
                                    let (labels, data) = chart_data();
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
                                                <span class="text-gray-500">"{qty} sold"</span>
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
fn ProductsPage() -> impl IntoView {
    let (products, set_products) = signal(Vec::<crate::api::Product>::new());
    let (page, set_page) = signal(1u32);
    let (show_add, set_show_add) = signal(false);
    let (show_stock, set_show_stock) = signal(false);
    let (del_id, set_del_id) = signal(None::<i64>);
    let (stock_pid, set_stock_pid) = signal(None::<i64>);
    let (stock_pname, set_stock_pname) = signal(String::new());
    let (stock_pstock, set_stock_pstock) = signal(0i64);
    let (stock_qty, set_stock_qty) = signal(0i64);
    let (add_qty, set_add_qty) = signal(0i64);
    let (add_error, set_add_error) = signal(None::<String>);
    let (sel_type, set_sel_type) = signal("life_saver".to_string());
    let (sel_color, set_sel_color) = signal("white_red".to_string());
    let (sel_size, set_sel_size) = signal("1x1".to_string());
    let per_page = 10u32;

    // Shared refetch helper
    let refetch = {
        let sp = set_products;
        move || { leptos::task::spawn_local(async move { if let Ok(p) = api::get_all_products().await { sp.set(p); } }); }
    };
    refetch();

    // ---- Actions ----
    let add_action: Action<(String, Option<String>, Option<String>, i64), Result<api::Product, String>, SyncStorage> = Action::new_unsync(move |input: &(String, Option<String>, Option<String>, i64)| {
        let (pt, color_opt, size_opt, qty) = input.clone();
        let pname = if pt == "life_saver" { "Life Saver".into() }
            else if pt == "stripes" { format!("{} Stripes", if color_opt.as_deref() == Some("white") {"White"} else {"Yellow"}) }
            else { let cn = if color_opt.as_deref() == Some("white_red") {"White / Red"} else {"Yellow / Red"}; format!("{} Chevron ({})", cn, size_opt.as_deref().unwrap_or("1x1")) };
        async move {
            api::add_product(&crate::api::NewProduct {
                name: pname, product_type: pt, color: color_opt, size: size_opt, selling_price: 0.0, stock: qty,
            }).await
        }
    });
    let delete_action: Action<i64, Result<SuccessResponse, String>, SyncStorage> = Action::new_unsync(move |id: &i64| { let id = *id; async move { api::delete_product(id).await } });
    let stock_action: Action<(i64, i64, i64), Result<SuccessResponse, String>, SyncStorage> = Action::new_unsync(move |(pid, add_qty, current): &(i64, i64, i64)| {
        let (pid, add_qty, current) = (*pid, *add_qty, *current);
        async move {
            api::update_product(pid, &crate::api::ProductUpdate {
                stock: Some(current + add_qty), name: None, product_type: None, color: None, size: None, selling_price: None,
            }).await
        }
    });

    // Watch actions for completion -> refetch & close modals
    let add_ver = add_action.version();
    create_effect(move |_| {
        let _ = add_ver.get();
        if let Some(result) = add_action.value().get() {
            match result {
                Ok(_) => { set_show_add.set(false); set_add_qty.set(0); set_add_error.set(None); refetch(); }
                Err(e) => { set_add_error.set(Some(e)); }
            }
        }
    });
    let del_ver = delete_action.version();
    create_effect(move |_| {
        let _ = del_ver.get();
        if let Some(Ok(_)) = delete_action.value().get() { set_del_id.set(None); refetch(); }
    });
    let stock_ver = stock_action.version();
    create_effect(move |_| {
        let _ = stock_ver.get();
        if let Some(Ok(_)) = stock_action.value().get() { set_show_stock.set(false); set_stock_qty.set(0); refetch(); }
    });

    // ---- Actions: trigger via signals to avoid moving Action into view closures ----
    // Add product trigger
    let (add_trigger, set_add_trigger) = signal(false);
    let add_payload = store_value((String::new(), String::new(), String::new(), 0i64));
    create_effect(move |_| {
        if add_trigger.get() {
            let (pt, col, sz, qty) = add_payload.get_value();
            let pname = if pt == "life_saver" { "Life Saver".into() }
                else if pt == "stripes" { format!("{} Stripes", if col == "white" {"White"} else {"Yellow"}) }
                else { let cn = if col == "white_red" {"White / Red"} else {"Yellow / Red"}; format!("{} Chevron ({})", cn, sz) };
            let color_opt = if pt == "life_saver" { None } else { Some(col.clone()) };
            let size_opt = if pt == "chevron" { Some(sz.clone()) } else { None };
            add_action.dispatch((pt, color_opt, size_opt, qty));
            set_add_trigger.set(false);
        }
    });

    // Delete product trigger
    let (del_trigger, set_del_trigger) = signal(None::<i64>);
    create_effect(move |_| {
        if let Some(id) = del_trigger.get() {
            delete_action.dispatch(id);
            set_del_trigger.set(None);
        }
    });

    // Add stock trigger
    let (stock_trigger, set_stock_trigger) = signal(false);
    let stock_payload = store_value((0i64, 0i64, 0i64));
    create_effect(move |_| {
        if stock_trigger.get() {
            stock_action.dispatch(stock_payload.get_value());
            set_stock_trigger.set(false);
        }
    });

    // ---- Derived stats ----
    let total = move || products.get().len();
    let ls_s = move || products.get().iter().filter(|p| p.product_type == "life_saver").map(|p| p.stock).sum::<i64>();
    let ch_s = move || products.get().iter().filter(|p| p.product_type == "chevron").map(|p| p.stock).sum::<i64>();
    let st_s = move || products.get().iter().filter(|p| p.product_type == "stripes").map(|p| p.stock).sum::<i64>();
    let sv = move || products.get().iter().map(|p| p.stock as f64 * p.selling_price).sum::<f64>();

    let cls = move |base: &str, active: bool| {
        if active { format!("{} border-2 border-gray-900 bg-gray-50 rounded", base) }
        else { format!("{} border border-gray-200 bg-white hover:border-gray-300 rounded", base) }
    };
    let tx = move |active: bool| if active { "font-medium text-gray-900" } else { "font-medium text-gray-500" };
    let sx = move |active: bool| if active { "text-xs text-gray-500" } else { "text-xs text-gray-400" };

    // Extract pending signals before the view (avoid moving Action into closures)
    let add_pending = add_action.pending();
    let del_pending = delete_action.pending();
    let stock_pending = stock_action.pending();

    view! {
        <div class="page-content">
            <div class="flex items-center justify-between mb-6">
                <div>
                    <h1 class="text-[22px] font-semibold text-[#0A0A0A] tracking-[-0.02em]">"Products"</h1>
                    <p class="text-sm text-[#525252] mt-0.5">"Life Savers, Chevrons, and Stripes inventory"</p>
                </div>
                <button on:click=move |_| set_show_add.set(true) class="flex items-center gap-2 bg-[#2563EB] text-white px-4 py-2 text-sm font-medium border-none cursor-pointer transition-all duration-100 hover:bg-[#1D4ED8]">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
                    "Add Product"
                </button>
            </div>

            <div class="grid grid-cols-5 gap-4 mb-6">
                <div class="bg-white border border-[#E5E5E5] p-5"><p class="text-xs text-gray-500 font-medium mb-1">"Total"</p><p class="text-xl font-semibold text-[#0A0A0A]">{move || total()}</p></div>
                <div class="bg-white border border-[#E5E5E5] p-5"><p class="text-xs text-gray-500 font-medium mb-1">"Life Savers"</p><p class="text-xl font-semibold text-[#0A0A0A]">{move || ls_s()}</p></div>
                <div class="bg-white border border-[#E5E5E5] p-5"><p class="text-xs text-gray-500 font-medium mb-1">"Chevrons"</p><p class="text-xl font-semibold text-[#0A0A0A]">{move || ch_s()}</p></div>
                <div class="bg-white border border-[#E5E5E5] p-5"><p class="text-xs text-gray-500 font-medium mb-1">"Stripes"</p><p class="text-xl font-semibold text-[#0A0A0A]">{move || st_s()}</p></div>
                <div class="bg-white border border-[#E5E5E5] p-5"><p class="text-xs text-gray-500 font-medium mb-1">"Stock Value"</p><p class="text-xl font-semibold text-[#0A0A0A]">{move || format!("KSh {:.0}", sv())}</p></div>
            </div>

            <div class="bg-white border border-[#E5E5E5] overflow-hidden">
                <table class="w-full table-fixed data-table"><thead><tr>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Type"</th>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Color"</th>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Size"</th>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Stock"</th>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Actions"</th>
                </tr></thead><tbody>
                    {move || {
                        let all = products.get();
                        if all.is_empty() {
                            return view! { <tr><td colspan="5" class="px-6 py-12 text-center text-gray-500">
                                <div class="flex flex-col items-center justify-center gap-2">
                                    <svg class="w-12 h-12 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/></svg>
                                    <p>"No products added yet"</p>
                                    <button on:click=move |_| set_show_add.set(true) class="text-black font-semibold hover:underline text-sm">"Add your first product"</button>
                                </div>
                            </td></tr>
                            }.into_any()
                        }
                        let total = all.len();
                        let tp = ((total as f64) / (per_page as f64)).ceil() as u32;
                        let cur = page.get().min(tp.max(1));
                        let start = ((cur - 1) * per_page) as usize;
                        let end = (start + per_page as usize).min(total);
                        all.into_iter().skip(start).take(end - start).map(|p| {
                            let is_ls = p.product_type == "life_saver";
                            let is_ch = p.product_type == "chevron";
                            let badged = if is_ls { "bg-green-100 text-green-800" } else if is_ch { "bg-orange-100 text-orange-800" } else { "bg-blue-100 text-blue-800" };
                            let color_cell = if let Some(ref col) = p.color {
                                let swatch = if is_ch {
                                    let (c1, c2) = if col == "white_red" { ("#ffffff", "#ef4444") } else { ("#eab308", "#ef4444") };
                                    format!("background:linear-gradient(135deg,{} 50%,{} 50%)", c1, c2)
                                } else {
                                    let b = if col == "white" { ";border:1px solid #d1d5db" } else { "" };
                                    format!("background-color:{}{}", if col == "white" { "#ffffff" } else { "#eab308" }, b)
                                };
                                let label: String = match col.as_str() { "white_red" => "White / Red", "yellow_red" => "Yellow / Red", "white" => "White", "yellow" => "Yellow", _ => col.as_str() }.into();
                                view! {
                                    <div class="flex items-center gap-2">
                                        <div class="w-6 h-6 rounded-sm shadow-sm flex-shrink-0" style=swatch></div>
                                        <span class="text-sm text-[#525252]">{label}</span>
                                    </div>
                                }.into_any()
                            } else if is_ls {
                                view! {
                                    <div class="flex items-center gap-2">
                                        <svg viewBox="0 0 24 24" class="w-6 h-6 flex-shrink-0"><polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/><text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">!</text></svg>
                                        <span class="text-sm text-gray-400">"Standard"</span>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span class="text-sm text-gray-400">"-"</span> }.into_any()
                            };
                            view! {
                                <tr class="border-b border-[#F0F0F0] hover:bg-[#F5F5F5] transition-all duration-100">
                                    <td class="px-4 py-[14px] text-sm">
                                        {if is_ls { view! {
                                            <div class="flex items-center gap-2">
                                                <svg viewBox="0 0 24 24" class="w-5 h-5"><polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/><text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">!</text></svg>
                                                <span class="text-sm font-medium text-[#0A0A0A]">"Life Saver"</span>
                                            </div>
                                        }.into_any() } else { view! {
                                            <span class=move || format!("inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium {}", badged)>
                                                {if p.product_type == "chevron" { "Chevron" } else { "Stripes" }}
                                            </span>
                                        }.into_any() }}
                                    </td>
                                    <td class="px-4 py-[14px] text-sm">{color_cell}</td>
                                    <td class="px-4 py-[14px] text-sm text-[#525252] font-medium">{p.size.unwrap_or_else(|| "-".into())}</td>
                                    <td class="px-4 py-[14px]">
                                        <span class="text-sm font-medium text-[#0A0A0A]">{p.stock} " units"</span>
                                    </td>
                                    <td class="px-4 py-[14px]">
                                        <div class="flex items-center gap-2">
                                            <button on:click=move |_| { set_stock_pid.set(Some(p.id)); set_stock_pname.set(p.name.clone()); set_stock_pstock.set(p.stock); set_stock_qty.set(0); set_show_stock.set(true); } class="px-2.5 py-1 text-xs font-medium bg-[#2563EB] text-white rounded hover:bg-[#1D4ED8] transition-colors">"+ Add"</button>
                                            <button on:click=move |_| { set_del_id.set(Some(p.id)); } class="text-gray-400 hover:text-red-600 transition-colors">
                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                                            </button>
                                        </div>
                                    </td>
                                </tr>
                            }
                        }).collect::<Vec<_>>().into_any()
                    }}
                </tbody></table>
                {move || {
                    let all = products.get();
                    if all.is_empty() { return ().into_any(); }
                    let total = all.len();
                    let tp = ((total as f64) / (per_page as f64)).ceil() as u32;
                    let cur = page.get().min(tp.max(1));
                    let start = ((cur - 1) * per_page) as usize;
                    let end = (start + per_page as usize).min(total);
                    view! {
                        <div class="flex items-center justify-between px-5 py-3 bg-gray-50 border-t border-gray-200 text-sm">
                            <div class="text-gray-600">"Showing " <span class="font-medium">{start + 1}</span> " to " <span class="font-medium">{end}</span> " of " <span class="font-medium">{total}</span> " products"</div>
                            <div class="flex gap-2 items-center">
                                <button on:click=move |_| set_page.set(((page.get() as i32 - 1).max(1)) as u32) class={if cur <= 1 { "px-3 py-1 text-sm font-medium rounded bg-gray-200 text-gray-400 cursor-not-allowed" } else { "px-3 py-1 text-sm font-medium rounded bg-black text-white hover:bg-gray-800" }} disabled=move || cur <= 1>"Previous"</button>
                                <span class="px-3 py-1 text-sm font-medium text-gray-700">"Page " {cur} " of " {tp}</span>
                                <button on:click=move |_| set_page.set((page.get() + 1).min(tp)) class={if cur >= tp { "px-3 py-1 text-sm font-medium rounded bg-gray-200 text-gray-400 cursor-not-allowed" } else { "px-3 py-1 text-sm font-medium rounded bg-black text-white hover:bg-gray-800" }} disabled=move || {cur >= tp}>"Next"</button>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>
        </div>

        <Show when=move || show_add.get()>
            <div class="fixed inset-0 bg-black/50 z-[200] flex items-center justify-center" on:click=move |e| { if e.target() == e.current_target() { set_show_add.set(false); } }>
                <div class="bg-white w-full max-w-[520px] max-h-[90vh] overflow-y-auto shadow-[0_20px_40px_rgba(0,0,0,0.08),0_8px_16px_rgba(0,0,0,0.04)]">
                    <div class="flex items-center justify-between p-5 border-b border-[#F0F0F0]">
                        <h3 class="text-base font-semibold text-[#0A0A0A]">"Add New Product"</h3>
                        <button on:click=move |_| set_show_add.set(false) class="w-7 h-7 flex items-center justify-center text-[#A3A3A3] border-none bg-transparent cursor-pointer rounded transition-colors hover:text-[#0A0A0A]">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                        </button>
                    </div>
                    <div class="p-6">
                        <div class="space-y-5">
                            <div>
                                <label class="block text-[13px] font-medium text-[#0A0A0A] mb-1.5">"Product Type"</label>
                                <div class="flex gap-2 mt-1">
                                    <button on:click=move |_| set_sel_type.set("life_saver".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all flex items-center gap-3", sel_type.get() == "life_saver")>
                                        <svg viewBox="0 0 24 24" class="w-6 h-6 flex-shrink-0"><polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/><text x="12" y="16" text-anchor="middle" font-size="10" font-weight="bold" fill="#1a1a1a">!</text></svg>
                                        <div class=move || tx(sel_type.get() == "life_saver")>"Life Saver"</div>
                                    </button>
                                    <button on:click=move |_| set_sel_type.set("chevron".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all flex items-center gap-3", sel_type.get() == "chevron")>
                                        <div class=move || tx(sel_type.get() == "chevron")>"Chevron"</div>
                                    </button>
                                    <button on:click=move |_| set_sel_type.set("stripes".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all flex items-center gap-3", sel_type.get() == "stripes")>
                                        <div class=move || tx(sel_type.get() == "stripes")>"Stripes"</div>
                                    </button>
                                </div>
                            </div>

                            <Show when=move || sel_type.get() == "chevron" || sel_type.get() == "stripes">
                                <div>
                                    <label class="block text-[13px] font-medium text-[#0A0A0A] mb-1.5">"Color"</label>
                                    <div class="flex gap-2 mt-1">
                                        <Show when=move || sel_type.get() == "chevron" fallback=move || {
                                            view! {
                                                <button on:click=move |_| set_sel_color.set("white".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all flex items-center gap-3", sel_color.get() == "white")>
                                                    <div class="w-8 h-8 rounded-sm shadow-sm flex-shrink-0 border border-gray-200" style="background:#ffffff"></div>
                                                    <div><div class=move || tx(sel_color.get() == "white")>"White"</div><div class=move || sx(sel_color.get() == "white")>"Stripe"</div></div>
                                                </button>
                                                <button on:click=move |_| set_sel_color.set("yellow".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all flex items-center gap-3", sel_color.get() == "yellow")>
                                                    <div class="w-8 h-8 rounded-sm shadow-sm flex-shrink-0" style="background:#eab308"></div>
                                                    <div><div class=move || tx(sel_color.get() == "yellow")>"Yellow"</div><div class=move || sx(sel_color.get() == "yellow")>"Stripe"</div></div>
                                                </button>
                                            }.into_any()
                                        }>
                                            <button on:click=move |_| set_sel_color.set("white_red".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all flex items-center gap-3", sel_color.get() == "white_red")>
                                                <div class="w-8 h-8 rounded-sm shadow-sm flex-shrink-0" style="background:linear-gradient(135deg,#ffffff 50%,#ef4444 50%)"></div>
                                                <div><div class=move || tx(sel_color.get() == "white_red")>"White / Red"</div><div class=move || sx(sel_color.get() == "white_red")>"Chevron"</div></div>
                                            </button>
                                            <button on:click=move |_| set_sel_color.set("yellow_red".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all flex items-center gap-3", sel_color.get() == "yellow_red")>
                                                <div class="w-8 h-8 rounded-sm shadow-sm flex-shrink-0" style="background:linear-gradient(135deg,#eab308 50%,#ef4444 50%)"></div>
                                                <div><div class=move || tx(sel_color.get() == "yellow_red")>"Yellow / Red"</div><div class=move || sx(sel_color.get() == "yellow_red")>"Chevron"</div></div>
                                            </button>
                                        </Show>
                                    </div>
                                </div>
                            </Show>

                            <Show when=move || sel_type.get() == "chevron">
                                <div>
                                    <label class="block text-[13px] font-medium text-[#0A0A0A] mb-1.5">"Size"</label>
                                    <div class="flex gap-2 mt-1">
                                        <button on:click=move |_| set_sel_size.set("1x1".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all", sel_size.get() == "1x1")>
                                            <div><div class=move || tx(sel_size.get() == "1x1")>"1x1"</div><div class=move || sx(sel_size.get() == "1x1")>"Standard"</div></div>
                                        </button>
                                        <button on:click=move |_| set_sel_size.set("1x2".into()) class=move || cls("flex-1 px-4 py-3 text-sm transition-all", sel_size.get() == "1x2")>
                                            <div><div class=move || tx(sel_size.get() == "1x2")>"1x2"</div><div class=move || sx(sel_size.get() == "1x2")>"Large"</div></div>
                                        </button>
                                    </div>
                                </div>
                            </Show>

                            <div>
                                <label class="block text-[13px] font-medium text-[#0A0A0A] mb-1.5">"Stock Quantity"</label>
                                <input type="number" min="1" required class="w-full border border-[#E5E5E5] rounded px-3 py-2 text-sm font-sans text-[#0A0A0A] bg-white outline-none mt-1" placeholder="0" on:input=move |e| set_add_qty.set(event_target_value(&e).parse().unwrap_or(0)) />
                            </div>
                        </div>
                    </div>
                    <div class="flex justify-end gap-2.5 px-6 py-4 bg-[#F5F5F5] border-t border-[#F0F0F0]">
                        <button on:click=move |_| { set_show_add.set(false); set_add_qty.set(0); set_sel_type.set("life_saver".into()); set_sel_color.set("white_red".into()); set_sel_size.set("1x1".into()); } class="px-4 py-2 text-sm font-medium bg-white text-[#0A0A0A] border border-[#E5E5E5] cursor-pointer transition-colors hover:bg-[#F5F5F5]">"Cancel"</button>
                        <button
on:click=move |_| {
add_payload.set_value((sel_type.get(), sel_color.get(), sel_size.get(), add_qty.get()));
set_add_trigger.set(true);
}
disabled=move || add_pending.get()
class="px-4 py-2 text-sm font-medium bg-[#2563EB] text-white border-none cursor-pointer transition-colors hover:bg-[#1D4ED8] disabled:opacity-50 disabled:cursor-not-allowed"
>{move || if add_pending.get() { "Saving..." } else { "Save" }}</button>
                    </div>
                </div>
            </div>
        </Show>

        <Show when=move || show_stock.get()>
            <div class="fixed inset-0 bg-black/50 z-[200] flex items-center justify-center" on:click=move |e| { if e.target() == e.current_target() { set_show_stock.set(false); } }>
                <div class="bg-white w-full max-w-[500px] shadow-xl">
                    <div class="flex items-center justify-between px-6 py-5 border-b border-[#F0F0F0]">
                        <h3 class="text-base font-semibold text-[#0A0A0A]">"Add Product Stock"</h3>
                        <button on:click=move |_| set_show_stock.set(false) class="w-7 h-7 flex items-center justify-center text-[#A3A3A3] border-none bg-transparent cursor-pointer rounded transition-colors hover:text-[#0A0A0A]">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                        </button>
                    </div>
                    <div class="p-6">
                        <div class="bg-gray-50 p-4 mb-4">
                            <p class="text-xs text-gray-500 uppercase tracking-wide">"Product"</p>
                            <p class="font-semibold text-gray-900">{move || stock_pname.get()}</p>
                            <p class="text-sm text-gray-600 mt-1">"Current stock: " {move || stock_pstock.get()} " units"</p>
                        </div>
                        <div>
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Quantity to Add *"</label>
                            <input type="number" min="1" class="w-full border border-[#E5E5E5] rounded px-3 py-2 text-sm outline-none focus:border-[#2563EB] focus:shadow-[0_0_0_2px_#EFF6FF]" placeholder="Enter units to add" autofocus on:input=move |e| set_stock_qty.set(event_target_value(&e).parse().unwrap_or(0)) />
                        </div>
                        <div class="bg-blue-50 border border-blue-200 p-3 mt-4">
                            <p class="text-sm text-gray-700"><span class="font-medium">"New Total Stock: "</span><span>{move || (stock_pstock.get() + stock_qty.get())}</span> " units"</p>
                        </div>
                    </div>
                    <div class="flex justify-end gap-2.5 px-6 py-4 bg-[#F5F5F5] border-t border-[#F0F0F0]">
                        <button on:click=move |_| set_show_stock.set(false) class="px-4 py-2 text-sm font-medium bg-white text-[#0A0A0A] border border-[#E5E5E5] hover:bg-[#F5F5F5]">"Cancel"</button>
                        <button
on:click=move |_| { if let (Some(pid), qty, cur) = (stock_pid.get(), stock_qty.get(), stock_pstock.get()) { if qty > 0 { stock_payload.set_value((pid, qty, cur));
set_stock_trigger.set(true); } } }
disabled=move || stock_pending.get()
class="px-4 py-2 text-sm font-medium bg-[#2563EB] text-white border-none hover:bg-[#1D4ED8] disabled:opacity-50 disabled:cursor-not-allowed"
>{move || if stock_pending.get() { "Adding..." } else { "Add Stock" }}</button>
                    </div>
                </div>
            </div>
        </Show>

        <Show when=move || del_id.get().is_some()>
            <div class="fixed inset-0 bg-black/50 z-[200] flex items-center justify-center" on:click=move |e| { if e.target() == e.current_target() { set_del_id.set(None); } }>
                <div class="bg-white w-full max-w-md shadow-xl p-6">
                    <h3 class="text-lg font-semibold mb-2">"Delete Product?"</h3>
                    <p class="text-sm text-gray-600 mb-6">"Are you sure you want to delete this product? This action cannot be undone."</p>
                    <div class="flex justify-end gap-3">
                        <button on:click=move |_| set_del_id.set(None) class="px-4 py-2 text-sm font-medium bg-white text-[#0A0A0A] border border-[#E5E5E5] hover:bg-[#F5F5F5]">"Cancel"</button>
                        <button
on:click=move |_| { if let Some(id) = del_id.get() { set_del_trigger.set(Some(id)); } }
disabled=move || del_pending.get()
class="px-4 py-2 text-sm font-medium bg-red-600 text-white border-none hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed"
>{move || if del_pending.get() { "Deleting..." } else { "Delete" }}</button>
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
    su: WriteSignal<Option<UserInfo>>,
) -> impl IntoView {
    let (old_pw, set_old_pw) = signal(String::new());
    let (new_pw, set_new_pw) = signal(String::new());
    let (msg, set_msg) = signal(String::new());
    let cur = move || user.get().map(|u| u.username).unwrap_or_default();
    let is_admin = move || user.get().map(|u| u.role.as_str() == "admin").unwrap_or(false);

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
