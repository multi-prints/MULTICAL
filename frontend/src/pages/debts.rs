use crate::api::{self, Debt, DebtPayment, DebtsPageQuery, NewDebt, NewDebtPayment};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast};

#[path = "../components/dropdown.rs"]
mod dropdown_comp;
use dropdown_comp::{CustomDropdown, DropdownItem};
#[path = "../components/calendar.rs"]
mod calendar_comp;
use calendar_comp::{CalendarModal, MiniCalendar};
#[path = "../components/loading.rs"]
mod loading_comp;
use loading_comp::PageLoading;

fn format_debt_datetime(ts: &Option<String>) -> String {
    ts.as_ref()
        .and_then(|t| {
            chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.3f")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S"))
                .ok()
        })
        .map(|dt| {
            let today = chrono::Local::now().date_naive();
            let payment_day = dt.date();
            let time = dt.format("%I:%M %p").to_string();

            if payment_day == today {
                format!("Today {}", time)
            } else if payment_day == today.pred_opt().unwrap_or(today) {
                format!("Yesterday {}", time)
            } else {
                dt.format("%d/%m/%Y %I:%M %p").to_string()
            }
        })
        .or_else(|| ts.clone())
        .unwrap_or_else(|| "-".to_string())
}

fn debt_product_preview(product_type: &str, color: Option<&str>) -> AnyView {
    match product_type {
        "life_saver" => view! {
            <svg viewBox="0 0 24 24" class="debts-item-preview-icon" aria-hidden="true">
                <polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/>
                <text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">"!"</text>
            </svg>
        }
        .into_any(),
        "chevron" => {
            let style = match color {
                Some("white_red") => "background:linear-gradient(135deg,#ffffff 50%,#ef4444 50%)",
                Some("yellow_red") => "background:linear-gradient(135deg,#eab308 50%,#ef4444 50%)",
                _ => "background:linear-gradient(135deg,#ffffff 50%,#ef4444 50%)",
            };
            view! { <div class="debts-item-preview-swatch" style=style></div> }.into_any()
        }
        "stripes" => {
            let (style, extra) = match color {
                Some("white") => ("background:#ffffff", " is-bordered"),
                Some("yellow") => ("background:#eab308", ""),
                _ => ("background:#ffffff", " is-bordered"),
            };
            let class = format!("debts-item-preview-swatch{extra}");
            view! { <div class=class style=style></div> }.into_any()
        }
        _ => view! { <div class="debts-item-preview-swatch is-bordered is-muted"></div> }.into_any(),
    }
}

fn debt_stock_preview(color: Option<&str>, sticker_type: Option<&str>) -> AnyView {
    let reflective = sticker_type == Some("reflective");
    let bg = match color.map(|c| c.to_lowercase()).as_deref() {
        Some("yellow") | Some("gold") => "#eab308",
        Some("white") => "#ffffff",
        Some("red") => "#ef4444",
        Some("blue") => "#3b82f6",
        Some("green") => "#22c55e",
        Some("black") => "#18181b",
        Some("orange") => "#f97316",
        Some("silver") | Some("grey") | Some("gray") => "#9ca3af",
        _ => "#d4d4d8",
    };
    let class = if color == Some("white") || color.is_none() {
        "debts-item-preview-swatch is-bordered"
    } else {
        "debts-item-preview-swatch"
    };
    let style = if reflective {
        format!(
            "background:linear-gradient(135deg,{} 0%,#f8fafc 45%,{} 100%)",
            bg, bg
        )
    } else {
        format!("background:{}", bg)
    };
    view! { <div class=class style=style></div> }.into_any()
}

fn debt_print_preview() -> AnyView {
    view! {
        <div class="debts-item-preview-print" aria-hidden="true">
            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M6 9V2h12v7M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2m-4 0H10v4h4v-4z"/>
            </svg>
        </div>
    }
    .into_any()
}

fn debt_manual_preview() -> AnyView {
    view! {
        <div class="debts-item-preview-manual" aria-hidden="true">
            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
            </svg>
        </div>
    }
    .into_any()
}

/// Item cell: product/print preview + name + subtitle (sales-style).
fn render_debt_item_cell(debt: &Debt) -> AnyView {
    let kind = debt.source_kind.as_deref().unwrap_or("manual");
    let label = debt
        .source_label
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| debt.description.clone().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| match kind {
            "sale" => "Sale".into(),
            "printing" => "Printing job".into(),
            _ => "Manual debt".into(),
        });
    let detail = debt
        .source_detail
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let preview = match kind {
        "sale" => {
            let sale_type = debt.source_sale_type.as_deref().unwrap_or("product");
            match sale_type {
                "product" => debt_product_preview(
                    debt.source_product_type.as_deref().unwrap_or(""),
                    debt.source_color.as_deref(),
                ),
                "stock" => debt_stock_preview(
                    debt.source_color.as_deref(),
                    debt.source_sticker_type.as_deref(),
                ),
                _ => view! { <div class="debts-item-preview-swatch is-bordered is-muted"></div> }
                    .into_any(),
            }
        }
        "printing" => debt_print_preview(),
        _ => debt_manual_preview(),
    };

    let kind_label = match kind {
        "sale" => match debt.source_sale_type.as_deref() {
            Some("stock") => "Sticker",
            Some("service") => "Service",
            _ => "Product",
        },
        "printing" => "Printing",
        _ => "Manual",
    };

    let subtitle = detail.unwrap_or_else(|| kind_label.to_string());

    view! {
        <div class="debts-item-cell">
            {preview}
            <div class="debts-item-copy">
                <p class="debts-item-title">{label}</p>
                <p class="debts-item-sub">{subtitle}</p>
            </div>
        </div>
    }
    .into_any()
}

#[component]
pub fn DebtsPage() -> impl IntoView {
    let (debts, set_debts) = signal(Vec::<Debt>::new());
    let (all_debts, set_all_debts) = signal(Vec::<Debt>::new());
    let (total_outstanding, set_total_outstanding) = signal(0.0f64);
    let (paid_month, set_paid_month) = signal(0.0f64);
    let (overdue_count, set_overdue_count) = signal(0u32);
    let (total_count, set_total_count) = signal(0u32);
    let (show_add, set_show_add) = signal(false);
    let (show_calendar, set_show_calendar) = signal(false);
    let (show_pay, set_show_pay) = signal(None::<Debt>);
    let (show_history, set_show_history) = signal(None::<Debt>);
    let (current_page, set_current_page) = signal(1u32);
    let (search, set_search) = signal(String::new());
    let (sort_by, set_sort_by) = signal("newest".to_string());
    let items_per_page = 10u32;
    // Add debt form
    let (cust, set_cust) = signal(String::new());
    let (phone, set_phone) = signal(String::new());
    let (amt, set_amt) = signal(String::new());
    let (due, set_due) = signal(String::new());
    let (_due_label, set_due_label) = signal(String::new());
    let (desc, set_desc) = signal(String::new());
    // Payment form
    let (pay_amt, set_pay_amt) = signal(String::new());
    let (pay_method, set_pay_method) = signal("cash".to_string());
    let (pay_notes, set_pay_notes) = signal(String::new());
    let (pay_error, set_pay_error) = signal(None::<String>);
    let (paying, set_paying) = signal(false);
    let (adding_debt, set_adding_debt) = signal(false);
    let (marking_paid_id, set_marking_paid_id) = signal(None::<i64>);
    // Payment history
    let (payments, set_payments) = signal(Vec::<DebtPayment>::new());
    let (loading, set_loading) = signal(true);
    // Row action menu (one open at a time)
    let (open_menu_id, set_open_menu_id) = signal(None::<i64>);
    // Delete confirmation
    let (del_id, set_del_id) = signal(None::<i64>);
    let (del_label, set_del_label) = signal(String::new());
    let (deleting_debt, set_deleting_debt) = signal(false);

    // Close ⋯ menu on outside click (no full-screen overlay that steals Pay clicks)
    Effect::new(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let listener = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
            set_open_menu_id.set(None);
        }));
        let _ = window.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref());
        listener.forget();
    });

    let reload = {
        let sd = set_debts;
        let sad = set_all_debts;
        let sto = set_total_outstanding;
        let spm = set_paid_month;
        let soc = set_overdue_count;
        let stc = set_total_count;
        let sl = set_loading;
        let search_r = search;
        let sort_r = sort_by;
        let page_r = current_page;
        move || {
            leptos::task::spawn_local({
                let sd = sd;
                let sad = sad;
                let sto = sto;
                let spm = spm;
                let soc = soc;
                let stc = stc;
                let sl = sl;
                let query = DebtsPageQuery {
                    search: Some(search_r.get()),
                    sort_by: Some(sort_r.get()),
                    page: Some(page_r.get()),
                    per_page: Some(items_per_page),
                };
                async move {
                    if let Ok(page) = api::get_debts_page(&query).await {
                        sd.set(page.items);
                        sad.set(page.all_debts);
                        sto.set(page.total_outstanding);
                        spm.set(page.paid_this_month);
                        soc.set(page.overdue_count as u32);
                        stc.set(page.total_count as u32);
                    }
                    sl.set(false);
                }
            })
        }
    };

    let (live_tick, set_live_tick) = signal(0u64);
    create_effect(move |_| {
        let _ = search.get();
        let _ = sort_by.get();
        let _ = current_page.get();
        let _ = live_tick.get();
        reload();
    });
    use_auto_refresh(LIVE_REFRESH_MS, move || {
        set_live_tick.update(|t| *t = t.wrapping_add(1));
    });

    let payment_items = Signal::derive(move || {
        vec![
            DropdownItem::new("cash", "Cash"),
            DropdownItem::new("mpesa", "M-Pesa"),
            DropdownItem::new("till", "Till Number"),
        ]
    });
    let sort_items = Signal::derive(move || {
        vec![
            DropdownItem::new("newest", "Newest First"),
            DropdownItem::new("oldest", "Oldest First"),
            DropdownItem::new("amount_desc", "Highest Remaining"),
            DropdownItem::new("amount_asc", "Lowest Remaining"),
        ]
    });

    create_effect(move |_| {
        let _ = search.get();
        let _ = sort_by.get();
        set_current_page.set(1);
    });

    let total_items = move || total_count.get();
    let total_pages = move || {
        let n = total_items();
        if n == 0 {
            1
        } else {
            n.div_ceil(items_per_page)
        }
    };
    let paginated = move || debts.get();

    let add_debt = {
        let l = reload;
        move |_| {
            if adding_debt.get() {
                return;
            }
            let n = cust.get();
            let a: f64 = amt.get().parse().unwrap_or(0.0);
            if n.is_empty() || a <= 0.0 {
                return;
            }
            let phone_v = phone.get();
            let due_v = due.get();
            let desc_v = desc.get();
            set_adding_debt.set(true);
            leptos::task::spawn_local(async move {
                let ok = api::add_debt(&NewDebt {
                    customer_name: n,
                    phone: Some(phone_v).filter(|p| !p.is_empty()),
                    amount: a,
                    paid_amount: Some(0.0),
                    remaining_amount: Some(a),
                    due_date: Some(due_v).filter(|d| !d.is_empty()),
                    description: Some(desc_v).filter(|d| !d.is_empty()),
                    sale_id: None,
                    service_transaction_id: None,
                })
                .await
                .is_ok();
                if ok {
                    set_show_add.set(false);
                    set_cust.set(String::new());
                    set_phone.set(String::new());
                    set_amt.set(String::new());
                    set_due.set(String::new());
                    set_due_label.set(String::new());
                    set_desc.set(String::new());
                }
                set_adding_debt.set(false);
                l();
            });
        }
    };

    let submit_payment = {
        let l = reload;
        move |_| {
            if paying.get() {
                return;
            }
            let Some(d) = show_pay.get() else {
                return;
            };
            let a: f64 = pay_amt.get().trim().parse().unwrap_or(0.0);
            if a <= 0.0 {
                set_pay_error.set(Some("Enter a payment amount greater than zero.".into()));
                return;
            }
            // Allow tiny float noise when paying the full remaining balance
            if a > d.remaining_amount + 0.009 {
                set_pay_error.set(Some(format!(
                    "Amount exceeds remaining balance (KSh {:.2}).",
                    d.remaining_amount
                )));
                return;
            }
            let amount = a.min(d.remaining_amount);
            let method = pay_method.get();
            let notes = pay_notes.get();
            let debt_id = d.id;
            set_pay_error.set(None);
            set_paying.set(true);
            leptos::task::spawn_local(async move {
                match api::add_debt_payment(&NewDebtPayment {
                    debt_id,
                    amount,
                    payment_method: method,
                    notes: Some(notes).filter(|n| !n.is_empty()),
                })
                .await
                {
                    Ok(_) => {
                        set_show_pay.set(None);
                        set_pay_amt.set(String::new());
                        set_pay_notes.set(String::new());
                        set_pay_method.set("cash".into());
                        set_paying.set(false);
                        l();
                    }
                    Err(e) => {
                        set_paying.set(false);
                        set_pay_error.set(Some(format!("Payment failed: {e}")));
                    }
                }
            });
        }
    };

    let open_history = move |debt: &Debt| {
        let did = debt.id;
        set_open_menu_id.set(None);
        set_show_history.set(Some(debt.clone()));
        leptos::task::spawn_local(async move {
            if let Ok(ps) = api::get_debt_payments(did).await {
                set_payments.set(ps);
            }
        });
    };

    let mark_paid = {
        let l = reload;
        move |id: i64| {
            if marking_paid_id.get().is_some() {
                return;
            }
            set_open_menu_id.set(None);
            set_marking_paid_id.set(Some(id));
            leptos::task::spawn_local(async move {
                let _ = api::mark_debt_paid(id).await;
                set_marking_paid_id.set(None);
                l();
            });
        }
    };

    let confirm_delete_debt = {
        let l = reload;
        move |_| {
            let Some(id) = del_id.get() else {
                return;
            };
            if deleting_debt.get() {
                return;
            }
            set_deleting_debt.set(true);
            leptos::task::spawn_local(async move {
                let _ = api::delete_debt(id).await;
                set_del_id.set(None);
                set_del_label.set(String::new());
                set_deleting_debt.set(false);
                l();
            });
        }
    };

    view! {
        <Show when=move || !loading.get() fallback=|| view! {
            <div id="page-debts" class="dash">
                <PageLoading message="Loading debts..."/>
            </div>
        }>
        <div id="page-debts" class="dash">
            <div class="dash-table-head">
                <div>
                    <h2 class="dash-section-title">"Debt records"</h2>
                    <p class="prod-sub">"Customer balances and payment tracking"</p>
                </div>
                <div class="dash-table-actions">
                    <button
                        type="button"
                        class="sales-btn-secondary"
                        on:click=move |_| set_show_calendar.set(true)
                    >"Calendar"</button>
                    <button
                        type="button"
                        class="dash-btn-primary"
                        on:click=move |_| {
                            set_cust.set(String::new());
                            set_phone.set(String::new());
                            set_amt.set(String::new());
                            set_due.set(String::new());
                            set_due_label.set(String::new());
                            set_desc.set(String::new());
                            set_show_add.set(true);
                        }
                    >
                        <span aria-hidden="true">"+"</span>
                        " Add Debt"
                    </button>
                </div>
            </div>

            <div class="prod-metrics dash-card sales-metrics sales-metrics--3">
                <div class="prod-metric">
                    <p class="dash-metric-label">"Total outstanding"</p>
                    <p class=move || {
                        let v = total_outstanding.get();
                        if v > 0.0 { "dash-metric-value is-danger" } else { "dash-metric-value" }
                    }>
                        {move || format!("KSh {:.0}", total_outstanding.get())}
                    </p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Paid this month"</p>
                    <p class="dash-metric-value" style="color:#059669">
                        {move || format!("KSh {:.0}", paid_month.get())}
                    </p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Overdue debts"</p>
                    <p class="dash-metric-value">{move || overdue_count.get()}</p>
                </div>
            </div>

            // Search / sort sit outside the table card (not attached to the table chrome)
            <div class="sales-toolbar debts-toolbar">
                <label class="dash-search sales-search">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-4.35-4.35M11 18a7 7 0 100-14 7 7 0 000 14z"/>
                    </svg>
                    <input
                        type="search"
                        placeholder="Search customer, phone, note..."
                        prop:value=move || search.get()
                        on:input=move |e| set_search.set(event_target_value(&e))
                        aria-label="Search debts"
                    />
                </label>
                <div class="sales-sort">
                    <CustomDropdown
                        items=sort_items
                        placeholder="Newest First".to_string()
                        on_select=Callback::new(move |v: String| set_sort_by.set(v))
                    />
                </div>
            </div>

            <div class=move || {
                if open_menu_id.get().is_some() {
                    "dash-card dash-table-card debts-table-card is-menu-open"
                } else {
                    "dash-card dash-table-card debts-table-card"
                }
            }>
                <table class="dash-table debts-table">
                    <thead>
                        <tr>
                            <th>"Customer"</th>
                            <th>"Item"</th>
                            <th>"Phone"</th>
                            <th>"Total"</th>
                            <th>"Paid"</th>
                            <th>"Remaining"</th>
                            <th>"Due"</th>
                            <th>"Status"</th>
                            <th>"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let items = paginated();
                            if items.is_empty() {
                                return view! {
                                    <tr>
                                        <td colspan="9" class="dash-table-empty">"No debts recorded."</td>
                                    </tr>
                                }.into_any();
                            }
                            items.into_iter().map(|d| {
                                let id = d.id;
                                let is_pending = d.status == "pending";
                                let name = d.customer_name.clone();
                                let ph = d.phone.clone().unwrap_or_else(|| "—".into());
                                let dd = d.due_date.clone().unwrap_or_else(|| "—".into());
                                let item_label = d
                                    .source_label
                                    .clone()
                                    .filter(|s| !s.trim().is_empty())
                                    .or_else(|| d.description.clone().filter(|s| !s.trim().is_empty()))
                                    .unwrap_or_else(|| "Manual debt".into());
                                let item_cell = render_debt_item_cell(&d);
                                let debt_pay = d.clone();
                                let debt_hist = d.clone();
                                let amount = d.amount;
                                let paid = d.paid_amount;
                                let remaining = d.remaining_amount;
                                let status_cls = if is_pending {
                                    "dash-status is-warn"
                                } else {
                                    "dash-status is-ok"
                                };
                                let status_label = if is_pending { "Pending" } else { "Paid" };
                                let remaining_cls = if is_pending && remaining > 0.0 {
                                    "dash-td-strong tnum is-danger-text"
                                } else {
                                    "dash-td-strong tnum"
                                };
                                let del_label_text = format!("{} · {} · KSh {:.0}", name, item_label, remaining.max(amount));
                                let menu_open = move || open_menu_id.get() == Some(id);
                                view! {
                                    <tr class=move || {
                                        if menu_open() {
                                            "sales-row is-menu-open"
                                        } else {
                                            "sales-row"
                                        }
                                    }>
                                        <td class="dash-td-strong">{name.clone()}</td>
                                        <td class="debts-source-cell">{item_cell}</td>
                                        <td class="dash-td-muted tnum">{ph}</td>
                                        <td class="dash-td-muted tnum">{format!("KSh {:.0}", amount)}</td>
                                        <td class="dash-td-muted tnum" style="color:#059669">{format!("KSh {:.0}", paid)}</td>
                                        <td class=remaining_cls>{format!("KSh {:.0}", remaining)}</td>
                                        <td class="dash-td-muted tnum">{dd}</td>
                                        <td><span class=status_cls>{status_label}</span></td>
                                        <td class="debts-actions-cell">
                                            <div class="debts-actions">
                                                {if is_pending {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="prod-btn-add"
                                                            title="Record a payment"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                set_open_menu_id.set(None);
                                                                set_pay_error.set(None);
                                                                set_paying.set(false);
                                                                // Leave blank — installments are common
                                                                set_pay_amt.set(String::new());
                                                                set_pay_notes.set(String::new());
                                                                set_pay_method.set("cash".into());
                                                                set_show_pay.set(Some(debt_pay.clone()));
                                                            }
                                                        >"Pay"</button>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                                <div class="debts-more" on:click=move |ev| ev.stop_propagation()>
                                                    <button
                                                        type="button"
                                                        class=move || {
                                                            if menu_open() {
                                                                "prod-btn-icon debts-more-btn is-open"
                                                            } else {
                                                                "prod-btn-icon debts-more-btn"
                                                            }
                                                        }
                                                        title="More actions"
                                                        aria-label="More actions"
                                                        aria-haspopup="menu"
                                                        aria-expanded=move || menu_open().to_string()
                                                        on:click=move |ev| {
                                                            ev.prevent_default();
                                                            ev.stop_propagation();
                                                            set_open_menu_id.update(|cur| {
                                                                *cur = if *cur == Some(id) { None } else { Some(id) };
                                                            });
                                                        }
                                                    >
                                                        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M12 6.75a.75.75 0 110-1.5.75.75 0 010 1.5zM12 12.75a.75.75 0 110-1.5.75.75 0 010 1.5zM12 18.75a.75.75 0 110-1.5.75.75 0 010 1.5z"/>
                                                        </svg>
                                                    </button>
                                                    {move || if menu_open() {
                                                        let hist = debt_hist.clone();
                                                        let label = del_label_text.clone();
                                                        view! {
                                                            <div
                                                                class="debts-more-menu"
                                                                role="menu"
                                                                on:click=move |ev| ev.stop_propagation()
                                                            >
                                                                <button
                                                                    type="button"
                                                                    class="debts-more-item"
                                                                    role="menuitem"
                                                                    on:click=move |_| open_history(&hist)
                                                                >
                                                                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                                                    </svg>
                                                                    "Payment history"
                                                                </button>
                                                                {if is_pending {
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            class="debts-more-item"
                                                                            role="menuitem"
                                                                            on:click=move |_| mark_paid(id)
                                                                        >
                                                                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 13l4 4L19 7"/>
                                                                            </svg>
                                                                            "Mark as paid"
                                                                        </button>
                                                                    }.into_any()
                                                                } else {
                                                                    ().into_any()
                                                                }}
                                                                <button
                                                                    type="button"
                                                                    class="debts-more-item is-danger"
                                                                    role="menuitem"
                                                                    on:click=move |_| {
                                                                        set_open_menu_id.set(None);
                                                                        set_del_label.set(label.clone());
                                                                        set_del_id.set(Some(id));
                                                                    }
                                                                >
                                                                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                                                    </svg>
                                                                    "Delete"
                                                                </button>
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                </div>
                                            </div>
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>().into_any()
                        }}
                    </tbody>
                </table>
                {move || {
                    let n = total_items();
                    if n == 0 {
                        return ().into_any();
                    }
                    let cp = current_page.get();
                    let tp = total_pages();
                    let si = (cp - 1) * items_per_page + 1;
                    let ei = (cp * items_per_page).min(n);
                    let count_label = format!("Showing {}–{} of {}", si, ei, n);
                    let page_label = format!("Page {} of {}", cp, tp);
                    let prev_disabled = cp <= 1;
                    let next_disabled = cp >= tp;
                    let go_prev = move |_| {
                        set_current_page.update(|p| *p = p.saturating_sub(1).max(1));
                    };
                    let go_next = move |_| {
                        set_current_page.update(move |p| {
                            let next = *p + 1;
                            *p = if next > tp { tp } else { next };
                        });
                    };
                    view! {
                        <div class="dash-table-foot">
                            <span class="dash-table-count">{count_label}</span>
                            <div class="prod-pager">
                                <button
                                    type="button"
                                    class="prod-pager-btn"
                                    prop:disabled=prev_disabled
                                    on:click=go_prev
                                >"Previous"</button>
                                <span class="prod-pager-meta">{page_label}</span>
                                <button
                                    type="button"
                                    class="prod-pager-btn"
                                    prop:disabled=next_disabled
                                    on:click=go_next
                                >"Next"</button>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>

        // Add Debt Modal
        {move || if show_add.get() { view!{<div class="modal-overlay open"><div class="modal-container"><div class="modal-header"><h3 class="modal-title">"Add New Debt"</h3><button class="modal-close-btn" prop:disabled=move || adding_debt.get() on:click=move |_| { if !adding_debt.get() { set_show_add.set(false); } }><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body">
            <div class="grid grid-cols-2 gap-4">
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Name"</label><input type="text" class="w-full" placeholder="Enter customer name" prop:value=move || cust.get() prop:disabled=move || adding_debt.get() on:input=move |e| set_cust.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Phone Number"</label><input type="tel" class="w-full" placeholder="Enter phone number" prop:value=move || phone.get() prop:disabled=move || adding_debt.get() on:input=move |e| set_phone.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Amount Owed"</label><input type="number" step="0.01" class="w-full" placeholder="0.00" prop:value=move || amt.get() prop:disabled=move || adding_debt.get() on:input=move |e| set_amt.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Due Date"</label><MiniCalendar date_r=due date_w=set_due label=set_due_label/></div>
                <div class="col-span-2"><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Description"</label><textarea rows="2" class="w-full" placeholder="What was purchased?" prop:value=move || desc.get() prop:disabled=move || adding_debt.get() on:input=move |e| set_desc.set(event_target_value(&e))></textarea></div>
            </div>
        </div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" prop:disabled=move || adding_debt.get() on:click=move |_| { if !adding_debt.get() { set_show_add.set(false); } }>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" prop:disabled=move || adding_debt.get() on:click=add_debt>{move || if adding_debt.get() { "Saving..." } else { "Save Debt" }}</button></div></div></div>}.into_any() } else { ().into_any() }}

        // Record Payment Modal
        {move || show_pay.get().map(|d| {
            let total = d.amount; let paid = d.paid_amount; let rem = d.remaining_amount;
            view!{
                <div class="modal-overlay open" on:click=move |e| {
                    if e.target() == e.current_target() && !paying.get() {
                        set_show_pay.set(None);
                        set_pay_error.set(None);
                    }
                }>
                    <div class="modal-container" style="max-width: 600px;" on:click=move |e| e.stop_propagation()>
                        <div class="modal-header">
                            <h3 class="modal-title">"Record Debt Payment"</h3>
                            <button
                                type="button"
                                class="modal-close-btn"
                                prop:disabled=move || paying.get()
                                on:click=move |_| {
                                    if !paying.get() {
                                        set_show_pay.set(None);
                                        set_pay_error.set(None);
                                    }
                                }
                            >
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                            </button>
                        </div>
                        <div class="modal-body">
                            <div class="bg-gray-50 p-4 mb-4 rounded-lg">
                                <div class="flex justify-between items-start mb-2">
                                    <div>
                                        <p class="text-xs text-gray-500">"Customer"</p>
                                        <p class="font-semibold text-gray-900">{d.customer_name.clone()}</p>
                                    </div>
                                    <div class="text-right">
                                        <p class="text-xs text-gray-500">"Total Debt"</p>
                                        <p class="font-semibold text-gray-900">{format!("KSh {:.0}", total)}</p>
                                    </div>
                                </div>
                                <div class="flex justify-between items-center pt-2 border-t border-gray-200">
                                    <div>
                                        <p class="text-xs text-gray-500">"Already Paid"</p>
                                        <p class="font-medium text-green-600">{format!("KSh {:.0}", paid)}</p>
                                    </div>
                                    <div class="text-right">
                                        <p class="text-xs text-gray-500">"Remaining"</p>
                                        <p class="font-bold text-red-600">{format!("KSh {:.0}", rem)}</p>
                                    </div>
                                </div>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Payment Amount *"</label>
                                    <input
                                        type="number"
                                        step="0.01"
                                        min="0.01"
                                        class="w-full"
                                        placeholder="Amount (e.g. partial installment)"
                                        prop:value=move || pay_amt.get()
                                        prop:disabled=move || paying.get()
                                        on:input=move |e| {
                                            set_pay_error.set(None);
                                            set_pay_amt.set(event_target_value(&e));
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Payment Method *"</label>
                                    <CustomDropdown
                                        items=payment_items
                                        placeholder="Cash".to_string()
                                        on_select=Callback::new(move |v: String| set_pay_method.set(v))
                                    />
                                </div>
                                <div class="col-span-2">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Notes"</label>
                                    <textarea
                                        rows="2"
                                        class="w-full"
                                        placeholder="Optional payment notes"
                                        prop:value=move || pay_notes.get()
                                        prop:disabled=move || paying.get()
                                        on:input=move |e| set_pay_notes.set(event_target_value(&e))
                                    ></textarea>
                                </div>
                            </div>
                            {move || pay_error.get().map(|err| {
                                view! { <p class="text-sm text-red-600 mt-3">{err}</p> }
                            })}
                        </div>
                        <div class="modal-footer">
                            <button
                                type="button"
                                class="btn-secondary px-4 py-2"
                                prop:disabled=move || paying.get()
                                on:click=move |_| {
                                    if !paying.get() {
                                        set_show_pay.set(None);
                                        set_pay_error.set(None);
                                    }
                                }
                            >"Cancel"</button>
                            <button
                                type="button"
                                class="btn-primary px-4 py-2"
                                prop:disabled=move || paying.get()
                                on:click=submit_payment
                            >{move || if paying.get() { "Recording..." } else { "Record Payment" }}</button>
                        </div>
                    </div>
                </div>
            }.into_any()
        }).unwrap_or_else(|| ().into_any())}

        // Payment History Modal
        {move || show_history.get().map(|d| {
            let total = d.amount; let paid = d.paid_amount; let rem = d.remaining_amount;
            view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 700px;"><div class="modal-header"><h3 class="modal-title">"Payment History"</h3><button class="modal-close-btn" on:click=move |_| set_show_history.set(None)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body">
                <div class="bg-gray-50 p-4 mb-4">
                    <div class="flex justify-between items-start mb-2"><div><p class="text-xs text-gray-500">"Customer"</p><p class="font-semibold text-gray-900">{d.customer_name.clone()}</p></div><div class="text-right"><p class="text-xs text-gray-500">"Total Debt"</p><p class="font-semibold text-gray-900">{format!("KSh {:.0}", total)}</p></div></div>
                    <div class="grid grid-cols-2 gap-4 pt-2 border-t border-gray-200">
                        <div><p class="text-xs text-gray-500">"Total Paid"</p><p class="font-medium text-green-600">{format!("KSh {:.0}", paid)}</p></div>
                        <div class="text-right"><p class="text-xs text-gray-500">"Remaining"</p><p class="font-bold text-red-600">{format!("KSh {:.0}", rem)}</p></div>
                    </div>
                </div>
                <div class="overflow-hidden border border-gray-200"><table class="w-full data-table">
                    <thead><tr><th>"Date & Time"</th><th>"Amount"</th><th>"Payment Method"</th></tr></thead>
                    <tbody>
                        {move || {
                            let ps = payments.get();
                            if ps.is_empty() { view!{<tr><td colspan="3" class="px-5 py-8 text-center text-gray-500">"No payments recorded yet."</td></tr>}.into_any() }
                            else { ps.into_iter().map(|p| {
                                let dt = format_debt_datetime(&p.payment_date);
                                view!{<tr class="border-b border-gray-50"><td class="px-4 py-3 text-sm text-gray-600 whitespace-nowrap">{dt}</td><td class="px-4 py-3 text-sm font-medium">{format!("KSh {:.2}", p.amount)}</td><td class="px-4 py-3 text-sm text-gray-600 capitalize">{p.payment_method}</td></tr>}
                            }).collect::<Vec<_>>().into_any() }
                        }}
                    </tbody>
                </table></div>
            </div></div></div>}.into_any()
        }).unwrap_or_else(|| ().into_any())}

        // Calendar Modal
        <CalendarModal show=Signal::derive(move || show_calendar.get()) set_show=set_show_calendar debts=Signal::derive(move || all_debts.get())/>

        // Delete debt confirmation
        <Show when=move || del_id.get().is_some()>
            <div
                class="modal-overlay open"
                on:click=move |e| {
                    if e.target() == e.current_target() && !deleting_debt.get() {
                        set_del_id.set(None);
                        set_del_label.set(String::new());
                    }
                }
            >
                <div class="modal-container modal-sm">
                    <div class="modal-header">
                        <h3 class="modal-title">"Delete Debt?"</h3>
                        <button
                            type="button"
                            class="modal-close-btn"
                            prop:disabled=move || deleting_debt.get()
                            on:click=move |_| {
                                if !deleting_debt.get() {
                                    set_del_id.set(None);
                                    set_del_label.set(String::new());
                                }
                            }
                        >
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                            </svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <p class="modal-msg">
                            "Are you sure you want to delete debt for "
                            <span class="modal-entity">{move || del_label.get()}</span>
                            "? This action cannot be undone."
                        </p>
                    </div>
                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn-secondary"
                            prop:disabled=move || deleting_debt.get()
                            on:click=move |_| {
                                if !deleting_debt.get() {
                                    set_del_id.set(None);
                                    set_del_label.set(String::new());
                                }
                            }
                        >"Cancel"</button>
                        <button
                            type="button"
                            class="btn-danger"
                            prop:disabled=move || deleting_debt.get()
                            on:click=confirm_delete_debt
                        >{move || if deleting_debt.get() { "Deleting..." } else { "Delete" }}</button>
                    </div>
                </div>
            </div>
        </Show>
    </div>
        </Show>
    }
}
