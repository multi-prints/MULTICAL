use crate::api::{self, Debt, DebtPayment, DebtsPageQuery, NewDebt, NewDebtPayment};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;

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
    // Payment history
    let (payments, set_payments) = signal(Vec::<DebtPayment>::new());
    let (loading, set_loading) = signal(true);

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
            let n = cust.get();
            let a: f64 = amt.get().parse().unwrap_or(0.0);
            if n.is_empty() || a <= 0.0 {
                return;
            }
            set_show_add.set(false);
            leptos::task::spawn_local(async move {
                let _ = api::add_debt(&NewDebt {
                    customer_name: n,
                    phone: Some(phone.get()).filter(|p| !p.is_empty()),
                    amount: a,
                    paid_amount: Some(0.0),
                    remaining_amount: Some(a),
                    due_date: Some(due.get()).filter(|d| !d.is_empty()),
                    description: Some(desc.get()).filter(|d| !d.is_empty()),
                    sale_id: None,
                    service_transaction_id: None,
                })
                .await;
                set_cust.set(String::new());
                set_phone.set(String::new());
                set_amt.set(String::new());
                set_due.set(String::new());
                set_due_label.set(String::new());
                set_desc.set(String::new());
                l();
            });
        }
    };

    let submit_payment = {
        let l = reload;
        move |_| {
            let debt = show_pay.get();
            if let Some(d) = debt {
                let a: f64 = pay_amt.get().parse().unwrap_or(0.0);
                if a <= 0.0 || a > d.remaining_amount {
                    return;
                }
                set_show_pay.set(None);
                leptos::task::spawn_local(async move {
                    let _ = api::add_debt_payment(&NewDebtPayment {
                        debt_id: d.id,
                        amount: a,
                        payment_method: pay_method.get(),
                        notes: Some(pay_notes.get()).filter(|n| !n.is_empty()),
                    })
                    .await;
                    set_pay_amt.set(String::new());
                    set_pay_notes.set(String::new());
                    l();
                });
            }
        }
    };

    let open_history = move |debt: &Debt| {
        let did = debt.id;
        set_show_history.set(Some(debt.clone()));
        leptos::task::spawn_local(async move {
            if let Ok(ps) = api::get_debt_payments(did).await {
                set_payments.set(ps);
            }
        });
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

            <div class="dash-card dash-table-card">
                <div class="sales-toolbar">
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
                <table class="dash-table debts-table">
                    <thead>
                        <tr>
                            <th>"Customer"</th>
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
                                        <td colspan="8" class="dash-table-empty">"No debts recorded."</td>
                                    </tr>
                                }.into_any();
                            }
                            items.into_iter().map(|d| {
                                let id = d.id;
                                let is_pending = d.status == "pending";
                                let name = d.customer_name.clone();
                                let ph = d.phone.clone().unwrap_or_else(|| "—".into());
                                let dd = d.due_date.clone().unwrap_or_else(|| "—".into());
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
                                view! {
                                    <tr class="sales-row">
                                        <td class="dash-td-strong">{name}</td>
                                        <td class="dash-td-muted tnum">{ph}</td>
                                        <td class="dash-td-muted tnum">{format!("KSh {:.0}", amount)}</td>
                                        <td class="dash-td-muted tnum" style="color:#059669">{format!("KSh {:.0}", paid)}</td>
                                        <td class=remaining_cls>{format!("KSh {:.0}", remaining)}</td>
                                        <td class="dash-td-muted tnum">{dd}</td>
                                        <td><span class=status_cls>{status_label}</span></td>
                                        <td>
                                            <div class="prod-actions">
                                                {if is_pending {
                                                    view! {
                                                        <>
                                                            <button
                                                                type="button"
                                                                class="prod-btn-add"
                                                                on:click=move |_| {
                                                                    set_pay_amt.set(String::new());
                                                                    set_pay_notes.set(String::new());
                                                                    set_pay_method.set("cash".into());
                                                                    set_show_pay.set(Some(debt_pay.clone()));
                                                                }
                                                            >"Pay"</button>
                                                            <button
                                                                type="button"
                                                                class="sales-btn-secondary debts-btn-sm"
                                                                on:click={
                                                                    let did = id;
                                                                    let l = reload;
                                                                    move |_| {
                                                                        leptos::task::spawn_local(async move {
                                                                            let _ = api::mark_debt_paid(did).await;
                                                                            l();
                                                                        });
                                                                    }
                                                                }
                                                            >"Mark Paid"</button>
                                                        </>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                                <button
                                                    type="button"
                                                    class="prod-btn-icon"
                                                    title="Payment history"
                                                    aria-label="Payment history"
                                                    on:click=move |_| open_history(&debt_hist)
                                                >
                                                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                                    </svg>
                                                </button>
                                                <button
                                                    type="button"
                                                    class="prod-btn-icon is-danger"
                                                    title="Delete"
                                                    aria-label="Delete debt"
                                                    on:click={
                                                        let did = id;
                                                        let l = reload;
                                                        move |_| {
                                                            leptos::task::spawn_local(async move {
                                                                let _ = api::delete_debt(did).await;
                                                                l();
                                                            });
                                                        }
                                                    }
                                                >
                                                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                                    </svg>
                                                </button>
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
        {move || if show_add.get() { view!{<div class="modal-overlay open"><div class="modal-container"><div class="modal-header"><h3 class="modal-title">"Add New Debt"</h3><button class="modal-close-btn" on:click=move |_| set_show_add.set(false)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body">
            <div class="grid grid-cols-2 gap-4">
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Name"</label><input type="text" class="w-full" placeholder="Enter customer name" prop:value=move || cust.get() on:input=move |e| set_cust.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Phone Number"</label><input type="tel" class="w-full" placeholder="Enter phone number" prop:value=move || phone.get() on:input=move |e| set_phone.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Amount Owed"</label><input type="number" step="0.01" class="w-full" placeholder="0.00" prop:value=move || amt.get() on:input=move |e| set_amt.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Due Date"</label><MiniCalendar date_r=due date_w=set_due label=set_due_label/></div>
                <div class="col-span-2"><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Description"</label><textarea rows="2" class="w-full" placeholder="What was purchased?" prop:value=move || desc.get() on:input=move |e| set_desc.set(event_target_value(&e))></textarea></div>
            </div>
        </div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" on:click=move |_| set_show_add.set(false)>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" on:click=add_debt>"Save Debt"</button></div></div></div>}.into_any() } else { ().into_any() }}

        // Record Payment Modal
        {move || show_pay.get().map(|d| {
            let total = d.amount; let paid = d.paid_amount; let rem = d.remaining_amount;
            view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 600px;"><div class="modal-header"><h3 class="modal-title">"Record Debt Payment"</h3><button class="modal-close-btn" on:click=move |_| set_show_pay.set(None)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body">
                <div class="bg-gray-50 p-4 mb-4">
                    <div class="flex justify-between items-start mb-2"><div><p class="text-xs text-gray-500">"Customer"</p><p class="font-semibold text-gray-900">{d.customer_name.clone()}</p></div><div class="text-right"><p class="text-xs text-gray-500">"Total Debt"</p><p class="font-semibold text-gray-900">{format!("KSh {:.0}", total)}</p></div></div>
                    <div class="flex justify-between items-center pt-2 border-t border-gray-200"><div><p class="text-xs text-gray-500">"Already Paid"</p><p class="font-medium text-green-600">{format!("KSh {:.0}", paid)}</p></div><div class="text-right"><p class="text-xs text-gray-500">"Remaining"</p><p class="font-bold text-red-600">{format!("KSh {:.0}", rem)}</p></div></div>
                </div>
                <div class="grid grid-cols-2 gap-4">
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Payment Amount *"</label><input type="number" step="0.01" min="0.01" class="w-full" placeholder="Enter amount paid" prop:value=move || pay_amt.get() on:input=move |e| set_pay_amt.set(event_target_value(&e))/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Payment Method *"</label><div>
                        <CustomDropdown items=payment_items placeholder="Cash".to_string() on_select=Callback::new(move |v: String| set_pay_method.set(v))/>
                    </div></div>
                    <div class="col-span-2"><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Notes"</label><textarea rows="2" class="w-full" placeholder="Optional payment notes" prop:value=move || pay_notes.get() on:input=move |e| set_pay_notes.set(event_target_value(&e))></textarea></div>
                </div>
            </div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" on:click=move |_| set_show_pay.set(None)>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" on:click=submit_payment>"Record Payment"</button></div></div></div>}.into_any()
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
    </div>
        </Show>
    }
}
