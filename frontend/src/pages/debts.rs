use leptos::prelude::*;
use crate::api::{self, Debt, NewDebt, NewDebtPayment, DebtPayment};

#[path = "../components/dropdown.rs"]
mod dropdown_comp;
use dropdown_comp::{CustomDropdown, DropdownItem};
#[path = "../components/calendar.rs"]
mod calendar_comp;
use calendar_comp::{CalendarModal, MiniCalendar};

#[component]
pub fn DebtsPage() -> impl IntoView {
    let (debts, set_debts) = signal(Vec::<Debt>::new());
    let (show_add, set_show_add) = signal(false);
    let (show_calendar, set_show_calendar) = signal(false);
    let (show_pay, set_show_pay) = signal(None::<Debt>);
    let (show_history, set_show_history) = signal(None::<Debt>);
    let (current_page, set_current_page) = signal(1u32);
    let items_per_page = 10u32;
    // Add debt form
    let (cust, set_cust) = signal(String::new());
    let (phone, set_phone) = signal(String::new());
    let (amt, set_amt) = signal(String::new());
    let (due, set_due) = signal(String::new());
    let (due_label, set_due_label) = signal(String::new());
    let (desc, set_desc) = signal(String::new());
    // Payment form
    let (pay_amt, set_pay_amt) = signal(String::new());
    let (pay_method, set_pay_method) = signal("cash".to_string());
    let (pay_notes, set_pay_notes) = signal(String::new());
    // Payment history
    let (payments, set_payments) = signal(Vec::<DebtPayment>::new());

    let reload = {
        let sd = set_debts;
        move || leptos::task::spawn_local({ let sd = sd; async move {
            if let Ok(d) = api::get_all_debts().await { sd.set(d); }
        }})
    };
    reload();

    let payment_items = Signal::derive(move || vec![
        DropdownItem::new("cash", "Cash"),
        DropdownItem::new("mpesa", "M-Pesa"),
        DropdownItem::new("till", "Till Number"),
    ]);

    let total_outstanding = move || debts.get().iter().filter(|d| d.status == "pending").map(|d| d.remaining_amount).sum::<f64>();
    let paid_month = move || {
        let now = chrono::Local::now();
        let month_key = now.format("%Y-%m").to_string();
        debts.get().iter().filter(|d| d.paid_at.as_ref().map_or(false, |p| p.starts_with(&month_key))).map(|d| d.paid_amount).sum::<f64>()
    };
    let overdue_count = move || {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        debts.get().iter().filter(|d| d.status == "pending" && d.due_date.as_ref().map_or(false, |dd| dd < &today)).count()
    };

    let total_items = move || debts.get().len() as u32;
    let total_pages = move || { let n = total_items(); if n == 0 { 1 } else { (n + items_per_page - 1) / items_per_page } };
    let paginated = move || {
        let items = debts.get();
        let start = ((current_page.get() - 1) * items_per_page) as usize;
        items.into_iter().skip(start).take(items_per_page as usize).collect::<Vec<_>>()
    };

    let add_debt = {
        let l = reload.clone();
        move |_| {
            let n = cust.get(); let a: f64 = amt.get().parse().unwrap_or(0.0);
            if n.is_empty() || a <= 0.0 { return; }
            set_show_add.set(false);
            leptos::task::spawn_local(async move {
                let _ = api::add_debt(&NewDebt {
                    customer_name: n, phone: Some(phone.get()).filter(|p| !p.is_empty()),
                    amount: a, paid_amount: Some(0.0), remaining_amount: Some(a),
                    due_date: Some(due.get()).filter(|d| !d.is_empty()),
                    description: Some(desc.get()).filter(|d| !d.is_empty()),
                    sale_id: None, service_transaction_id: None,
                }).await;
                set_cust.set(String::new()); set_phone.set(String::new()); set_amt.set(String::new());
                set_due.set(String::new()); set_due_label.set(String::new()); set_desc.set(String::new()); l();
            });
        }
    };

    let submit_payment = {
        let l = reload.clone();
        move |_| {
            let debt = show_pay.get();
            if let Some(d) = debt {
                let a: f64 = pay_amt.get().parse().unwrap_or(0.0);
                if a <= 0.0 || a > d.remaining_amount { return; }
                set_show_pay.set(None);
                leptos::task::spawn_local(async move {
                    let _ = api::add_debt_payment(&NewDebtPayment {
                        debt_id: d.id, amount: a, payment_method: pay_method.get(), notes: Some(pay_notes.get()).filter(|n| !n.is_empty()),
                    }).await;
                    set_pay_amt.set(String::new()); set_pay_notes.set(String::new()); l();
                });
            }
        }
    };

    let open_history = move |debt: &Debt| {
        let did = debt.id;
        set_show_history.set(Some(debt.clone()));
        leptos::task::spawn_local(async move {
            if let Ok(ps) = api::get_debt_payments(did).await { set_payments.set(ps); }
        });
    };

    view! { <div id="page-debts" class="page-content">
        <div class="flex items-center justify-between mb-6">
            <div><h1 class="page-title">"Debts Management"</h1><p class="page-subtitle">"Track customer debts and payments"</p></div>
            <div class="flex gap-3">
                <button class="flex items-center gap-2 bg-white border border-gray-200 text-gray-700 px-4 py-2 text-sm font-medium hover:bg-gray-50 transition-all"
                    on:click=move |_| set_show_calendar.set(true)>
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/></svg>
                    "Calendar View"</button>
                <button class="flex items-center gap-2 bg-brand-500 text-white px-4 py-2 text-sm font-medium hover:bg-brand-600 transition-all"
                    on:click=move |_| { set_cust.set(String::new()); set_phone.set(String::new()); set_amt.set(String::new()); set_due.set(String::new()); set_due_label.set(String::new()); set_desc.set(String::new()); set_show_add.set(true); }>
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
                    "Add Debt"</button>
            </div>
        </div>

        // Stats
        <div class="grid grid-cols-3 gap-4 mb-6">
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Total Outstanding"</p><p class="text-xl font-bold text-red-600">{move || format!("KSh {:.2}", total_outstanding())}</p></div>
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Paid This Month"</p><p class="text-xl font-bold text-green-600">{move || format!("KSh {:.2}", paid_month())}</p></div>
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Overdue Debts"</p><p class="text-xl font-bold text-gray-900">{move || overdue_count()}</p></div>
        </div>

        // Table
        <div class="dashboard-panel overflow-hidden">
            <table class="w-full data-table">
                <thead><tr><th>"Customer"</th><th>"Phone"</th><th>"Total Amount"</th><th>"Paid"</th><th>"Remaining"</th><th>"Due Date"</th><th>"Status"</th><th>"Actions"</th></tr></thead>
                <tbody>
                    {move || {
                        let items = paginated();
                        if items.is_empty() {
                            view!{<tr><td colspan="8" class="px-5 py-8 text-center text-gray-500">"No debts recorded."</td></tr>}.into_any()
                        } else {
                            items.into_iter().map(|d| {
                                let id = d.id; let is_pending = d.status == "pending";
                                let name = d.customer_name.clone();
                                let ph = d.phone.clone().unwrap_or_default();
                                let dd = d.due_date.clone().unwrap_or_default();
                                let debt_clone = d.clone(); let debt_clone2 = d.clone();
                                view!{
                                    <tr class="hover:bg-gray-50 transition-colors">
                                        <td class="px-5 py-4"><span class="text-sm font-medium text-gray-900">{name}</span></td>
                                        <td class="px-5 py-4 text-sm text-gray-600">{ph}</td>
                                        <td class="px-5 py-4 text-sm text-gray-900">{format!("KSh {:.2}", d.amount)}</td>
                                        <td class="px-5 py-4 text-sm text-green-600">{format!("KSh {:.2}", d.paid_amount)}</td>
                                        <td class="px-5 py-4 text-sm font-medium text-red-600">{format!("KSh {:.2}", d.remaining_amount)}</td>
                                        <td class="px-5 py-4 text-sm text-gray-600">{dd}</td>
                                        <td class="px-5 py-4"><span class=format!("status-badge {}", if is_pending {"status-badge--warning"} else {"status-badge--success"})>{if is_pending {"Pending"} else {"Paid"}}</span></td>
                                        <td class="px-5 py-4"><div class="flex items-center gap-2">
                                            {if is_pending { view!{<>
                                                <button on:click=move |_| { set_pay_amt.set(String::new()); set_pay_notes.set(String::new()); set_pay_method.set("cash".into()); set_show_pay.set(Some(debt_clone.clone())); } class="px-3 py-1 text-xs font-medium bg-brand-600 text-white rounded-md hover:bg-brand-700 transition-colors">"Pay"</button>
                                                <button on:click={let did=id;let l=reload.clone();move|_|{leptos::task::spawn_local(async move{let _=api::mark_debt_paid(did).await;l();});}} class="btn-primary px-3 py-1 text-xs font-medium rounded-md">"Mark Paid"</button>
                                            </>}.into_any() } else { ().into_any() }}
                                            <button on:click=move |_| open_history(&debt_clone2) class="text-gray-400 hover:text-gray-600 transition-colors" title="Payment history">
                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                                            </button>
                                            <button on:click={let did=id;let l=reload.clone();move|_|{leptos::task::spawn_local(async move{let _=api::delete_debt(did).await;l();});}} class="text-gray-400 hover:text-red-600 transition-colors" title="Delete">
                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                                            </button>
                                        </div></td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>().into_any()
                        }
                    }}
                </tbody>
            </table>
            {move || { let n = total_items(); if n == 0 { ().into_any() } else {
                let cp = current_page.get(); let tp = total_pages();
                let si = (cp-1)*items_per_page+1; let ei = (cp*items_per_page).min(n);
                view!{<div class="flex items-center justify-between px-5 py-3 bg-gray-50 border-t border-gray-200">
                    <div class="text-sm text-gray-600">"Showing "<span class="font-medium">{si}</span>" to "<span class="font-medium">{ei}</span>" of "<span class="font-medium">{n}</span>" debts"</div>
                    <div class="flex gap-2">
                        <button on:click=move |_| { if cp>1 {set_current_page.set(cp-1)} } class=format!("px-3 py-1 text-sm font-medium rounded-md {}", if cp==1 {"bg-gray-200 text-gray-400 cursor-not-allowed"} else {"bg-black text-white hover:bg-gray-800"}) disabled=move || cp==1>"Previous"</button>
                        <span class="px-3 py-1 text-sm font-medium text-gray-700">{format!("Page {} of {}", cp, tp)}</span>
                        <button on:click=move |_| { if cp<tp {set_current_page.set(cp+1)} } class=format!("px-3 py-1 text-sm font-medium rounded-md {}", if cp>=tp {"bg-gray-200 text-gray-400 cursor-not-allowed"} else {"bg-black text-white hover:bg-gray-800"}) disabled=move || tp <= cp>"Next"</button>
                    </div>
                </div>}.into_any()
            }}}
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
                    <div class="flex justify-between items-center pt-2 border-t border-gray-200"><div><p class="text-xs text-gray-500">"Total Paid"</p><p class="font-medium text-green-600">{format!("KSh {:.0}", paid)}</p></div><div class="text-right"><p class="text-xs text-gray-500">"Remaining"</p><p class="font-bold text-red-600">{format!("KSh {:.0}", rem)}</p></div></div>
                </div>
                <div class="overflow-hidden border border-gray-200"><table class="w-full data-table">
                    <thead><tr><th>"Date & Time"</th><th>"Amount"</th><th>"Payment Method"</th></tr></thead>
                    <tbody>
                        {move || {
                            let ps = payments.get();
                            if ps.is_empty() { view!{<tr><td colspan="3" class="px-5 py-8 text-center text-gray-500">"No payments recorded yet."</td></tr>}.into_any() }
                            else { ps.into_iter().map(|p| {
                                let dt = p.payment_date.unwrap_or_default();
                                view!{<tr class="border-b border-gray-50"><td class="px-4 py-3 text-sm text-gray-600">{dt}</td><td class="px-4 py-3 text-sm font-medium">{format!("KSh {:.2}", p.amount)}</td><td class="px-4 py-3 text-sm text-gray-600 capitalize">{p.payment_method}</td></tr>}
                            }).collect::<Vec<_>>().into_any() }
                        }}
                    </tbody>
                </table></div>
            </div></div></div>}.into_any()
        }).unwrap_or_else(|| ().into_any())}

        // Calendar Modal
        <CalendarModal show=Signal::derive(move || show_calendar.get()) set_show=set_show_calendar debts=Signal::derive(move || debts.get())/>
    </div> }
}
