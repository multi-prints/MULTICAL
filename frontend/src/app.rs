use leptos::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use chrono::Timelike;
use crate::api::{self, UserInfo, LoginResponse};
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
                                    Page::Dashboard => view! { <DashboardPage /> }.into_any(),
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
fn DashboardPage() -> impl IntoView {
    let (total_revenue, set_total_revenue) = signal(0.0_f64);
    let (today_total, set_today_total) = signal(0.0_f64);
    let (sales_count, set_sales_count) = signal(0i64);
    let (outstanding, set_outstanding) = signal(0.0_f64);
    let (pending_count, set_pending_count) = signal(0i64);
    let (recent_sales, set_recent_sales) = signal(Vec::<crate::api::Sale>::new());
    let (loading, set_loading) = signal(true);

    let hour = chrono::Local::now().hour();
    let greeting = if hour < 12 { "Good morning" } else if hour < 18 { "Good afternoon" } else { "Good evening" };

    leptos::task::spawn_local(async move {
        let mut rev = 0.0;
        let mut today = 0.0;
        let mut count = 0i64;
        let mut out = 0.0;
        let mut pc = 0i64;
        let mut recent = Vec::new();

        if let Ok(sales) = api::get_all_sales().await {
            count = sales.len() as i64;
            rev = sales.iter().map(|s| s.amount).sum();
            recent = sales.into_iter().take(10).collect();
        }
        if let Ok(t) = api::get_today_total_sales().await { today = t; }
        if let Ok(d) = api::get_all_debts().await {
            let pending: Vec<_> = d.into_iter().filter(|d| d.status == "pending").collect();
            pc = pending.len() as i64;
            out = pending.iter().map(|d| d.remaining_amount).sum();
        }
        set_total_revenue.set(rev);
        set_today_total.set(today);
        set_sales_count.set(count);
        set_outstanding.set(out);
        set_pending_count.set(pc);
        set_recent_sales.set(recent);
        set_loading.set(false);
    });

    let fmt = |amount: f64| format!("KSh {:.2}", amount);

    view! {
        <div class="page-content">
            <div class="mb-6">
                <h1 class="text-[22px] font-semibold text-[#0A0A0A] tracking-[-0.02em] mb-1">{greeting} "Admin"</h1>
                <p class="text-sm text-[#525252]">"Here's what's happening today."</p>
            </div>

            <div class="grid grid-cols-3 gap-4 mb-6">
                <div class="bg-white border border-[#E5E5E5] p-5">
                    <div class="flex items-start justify-between">
                        <div>
                            <p class="text-xs text-gray-500 font-medium mb-1">"Total Revenue"</p>
                            <h3 class="text-xl font-semibold text-[#0A0A0A]">{move || fmt(total_revenue.get())}</h3>
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
                            <h3 class="text-xl font-semibold text-[#0A0A0A]">{move || sales_count.get()}</h3>
                            <div class="flex items-center gap-1 mt-2 text-xs font-medium text-gray-500">
                                <span>{move || format!("Today: KSh {:.2}", today_total.get())}</span>
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
                            <h3 class="text-xl font-semibold text-[#EF4444]">{move || fmt(outstanding.get())}</h3>
                            <div class="flex items-center gap-1 mt-2 text-xs font-medium text-red-500">
                                <span>{move || format!("{} pending", pending_count.get())}</span>
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
                    <div class="bg-white border border-[#E5E5E5] p-5">
                        <div class="flex items-center justify-between mb-5">
                            <div>
                                <h3 class="text-base font-semibold text-[#0A0A0A] tracking-[-0.01em]">"Revenue"</h3>
                                <p class="text-xs text-gray-500 mt-0.5">"Last 7 days"</p>
                            </div>
                        </div>
                        <div class="relative w-full h-[280px] flex items-center justify-center text-gray-400 text-sm">
                            "Chart will appear here"
                        </div>
                    </div>

                    <div class="bg-white border border-[#E5E5E5] p-5">
                        <div class="flex items-center justify-between mb-4">
                            <h3 class="text-base font-semibold text-[#0A0A0A] tracking-[-0.01em]">"Recent Transactions"</h3>
                            <span class="text-xs font-medium text-[#2563EB] cursor-pointer">"View all"</span>
                        </div>
                        <div class="overflow-x-auto">
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
                                    {move || recent_sales.get().into_iter().map(|s| {
                                        let is_debt = s.is_debt > 0;
                                        let status_text = if is_debt && s.is_debt == 1 { "Pending" } else if is_debt { "Paid" } else { "Completed" };
                                        let is_pending = is_debt && s.is_debt == 1;
                                        let date = s.timestamp.as_deref().unwrap_or("").split('T').next().unwrap_or("").to_string();
                                        view! {
                                            <tr class="border-b border-[#F0F0F0] hover:bg-[#F5F5F5] transition-all duration-100">
                                                <td class="px-4 py-[14px] text-sm text-[#0A0A0A]">{s.product_name.unwrap_or(s.r#type)}</td>
                                                <td class="px-4 py-[14px] text-sm text-[#0A0A0A]">{date}</td>
                                                <td class="px-4 py-[14px] text-sm text-[#0A0A0A]">"KSh " {s.amount}</td>
                                                <td class="px-4 py-[14px]">
                                                    <span class={if is_pending { "inline-flex items-center gap-1 px-2.5 py-1 text-xs font-medium rounded bg-[#FFFBEB] text-[#F59E0B]" } else { "inline-flex items-center gap-1 px-2.5 py-1 text-xs font-medium rounded bg-[#ECFDF5] text-[#10B981]" }}>
                                                        {status_text}
                                                    </span>
                                                </td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>

                <div class="space-y-6">
                    <div class="bg-white border border-[#E5E5E5] p-5">
                        <h3 class="text-base font-semibold text-[#0A0A0A] tracking-[-0.01em] mb-5">"Activity"</h3>
                        <div class="relative">
                            {move || {
                                let items = recent_sales.get();
                                if items.is_empty() {
                                    return view! { <p class="text-sm text-[#A3A3A3] text-center py-8">"No recent activity"</p> }.into_any();
                                }
                                items.into_iter().take(8).enumerate().map(|(i, s)| {
                                    let is_last = i == 7;
                                    let time = s.timestamp.as_deref().and_then(|t| t.get(11..16)).map(|ts| ts.to_string()).unwrap_or_default();
                                    view! {
                                        <div class="relative pl-5 pb-5">
                                            {if !is_last { view! { <div class="absolute top-1.5 left-[5px] bottom-[-4px] w-px bg-[#E5E5E5]"></div> }.into_any() } else { ().into_any() }}
                                            <div class="absolute top-1 left-0 w-2.5 h-2.5 rounded-full bg-[#2563EB]"></div>
                                            <p class="text-sm font-medium text-[#0A0A0A]">{s.product_name.clone().unwrap_or(s.r#type.clone())}</p>
                                            <p class="text-xs text-[#525252] mt-0.5">{s.customer_name.clone()} " - " {time}</p>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }}
                        </div>
                    </div>
                </div>
            </div>
        </div>
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
    let (sel_type, set_sel_type) = signal("life_saver".to_string());
    let (sel_color, set_sel_color) = signal("white_red".to_string());
    let (sel_size, set_sel_size) = signal("1x1".to_string());
    let per_page = 10u32;

    let load = { let sp = set_products; move || { leptos::task::spawn_local(async move {
        if let Ok(p) = api::get_all_products().await { sp.set(p); }
    });}};
    load();

    let total = move || products.get().len();
    let ls_s = move || products.get().iter().filter(|p| p.product_type == "life_saver").map(|p| p.stock).sum::<i64>();
    let ch_s = move || products.get().iter().filter(|p| p.product_type == "chevron").map(|p| p.stock).sum::<i64>();
    let st_s = move || products.get().iter().filter(|p| p.product_type == "stripes").map(|p| p.stock).sum::<i64>();
    let sv = move || products.get().iter().map(|p| p.stock as f64 * p.selling_price).sum::<f64>();

    let handle_add = move |_| {
            let pt = sel_type.get(); let col = sel_color.get(); let sz = sel_size.get(); let q = add_qty.get();
            if q <= 0 { return; }
            let (color_opt, size_opt, pname) = if pt == "life_saver" {
                (None, None, "Life Saver".to_string())
            } else if pt == "stripes" {
                (Some(col.clone()), None, format!("{} Stripes", if col == "white" {"White"} else {"Yellow"}))
            } else {
                let cn = if col == "white_red" {"White / Red"} else {"Yellow / Red"};
                (Some(col.clone()), Some(sz.clone()), format!("{} Chevron ({})", cn, sz))
            };
            leptos::task::spawn_local(async move {
                let _ = api::add_product(&crate::api::NewProduct {
                    name: pname, product_type: pt, color: color_opt, size: size_opt, selling_price: 0.0, stock: q,
                }).await;
                set_show_add.set(false);
                let sp = set_products;
                leptos::task::spawn_local(async move { if let Ok(p) = api::get_all_products().await { sp.set(p); } });
            });
        };

    let handle_delete = move |id: i64| {
        let l = load.clone();
        leptos::task::spawn_local(async move {
            let _ = api::delete_product(id).await;
            set_del_id.set(None); l();
        });
    };

    let handle_stock = move |_| {
        if let Some(pid) = stock_pid.get() {
            let q = stock_qty.get(); if q <= 0 { return; }
            let l = load.clone();
            let s = stock_pstock.get();
            leptos::task::spawn_local(async move {
                let _ = api::update_product(pid, &crate::api::ProductUpdate {
                    stock: Some(s + q), name: None, product_type: None, color: None, size: None, selling_price: None,
                }).await;
                set_show_stock.set(false); l();
            });
        }
    };

    let cls = move |base: &str, active: bool| {
        if active { format!("{} border-2 border-gray-900 bg-gray-50 rounded", base) }
        else { format!("{} border border-gray-200 bg-white hover:border-gray-300 rounded", base) }
    };
    let tx = move |active: bool| if active { "font-medium text-gray-900" } else { "font-medium text-gray-500" };
    let sx = move |active: bool| if active { "text-xs text-gray-500" } else { "text-xs text-gray-400" };

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
                <table class="w-full data-table"><thead><tr>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Type"</th>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Color"</th>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Size"</th>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-left">"Stock"</th>
                    <th class="px-4 py-3 text-[11px] font-medium text-[#A3A3A3] uppercase tracking-[0.05em] bg-[#F0F0F0] border-b border-[#E5E5E5] text-right">"Actions"</th>
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
                            let stock_badge = if p.stock > 10 { "bg-[#ECFDF5] text-[#10B981]" } else if p.stock > 0 { "bg-[#FFFBEB] text-[#F59E0B]" } else { "bg-[#FEF2F2] text-[#EF4444]" };
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
                                        <div class="flex items-center gap-2">
                                            <span class=move || format!("inline-flex items-center gap-1 px-2.5 py-1 text-xs font-medium rounded {}", stock_badge)>{p.stock} " Units"</span>
                                            <button on:click=move |_| { set_stock_pid.set(Some(p.id)); set_stock_pname.set(p.name.clone()); set_stock_pstock.set(p.stock); set_stock_qty.set(0); set_show_stock.set(true); } class="px-2 py-1 text-xs font-medium bg-black text-white rounded hover:bg-gray-800 transition-colors">"+ Add"</button>
                                        </div>
                                    </td>
                                    <td class="px-4 py-[14px] text-right">
                                        <button on:click=move |_| { set_del_id.set(Some(p.id)); } class="text-gray-400 hover:text-red-600 transition-colors">
                                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                                        </button>
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
                                <button on:click=move |_| set_page.set((page.get() + 1).min(tp)) class={if cur >= tp { "px-3 py-1 text-sm font-medium rounded bg-gray-200 text-gray-400 cursor-not-allowed" } else { "px-3 py-1 text-sm font-medium rounded bg-black text-white hover:bg-gray-800" }} disabled=move || cur >= tp>"Next"</button>
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
                        <button on:click=move |e| { handle_add(e); set_add_qty.set(0); } class="px-4 py-2 text-sm font-medium bg-[#2563EB] text-white border-none cursor-pointer transition-colors hover:bg-[#1D4ED8]">"Save"</button>
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
                        <button on:click=handle_stock class="px-4 py-2 text-sm font-medium bg-[#2563EB] text-white border-none hover:bg-[#1D4ED8]">"Add Stock"</button>
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
                        <button on:click=move |_| { if let Some(id) = del_id.get() { handle_delete(id); } } class="px-4 py-2 text-sm font-medium bg-red-600 text-white border-none hover:bg-red-700">"Delete"</button>
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
