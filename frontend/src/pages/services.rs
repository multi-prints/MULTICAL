use leptos::prelude::*;
use crate::api::{self, NewService, NewServiceTransaction, Service, ServiceTransaction};

#[component]
pub fn ServicesPage() -> impl IntoView {
    let (services, set_services) = signal(Vec::<Service>::new());
    let (transactions, set_transactions) = signal(Vec::<ServiceTransaction>::new());
    let (show_service_form, set_show_service_form) = signal(false);
    let (show_transaction_form, set_show_transaction_form) = signal(false);

    let (service_name, set_service_name) = signal(String::new());
    let (service_desc, set_service_desc) = signal(String::new());
    let (service_price, set_service_price) = signal(String::new());
    let (service_unit, set_service_unit) = signal(String::new());

    let (selected_service, set_selected_service) = signal(None::<i64>);
    let (tx_qty, set_tx_qty) = signal("1".to_string());
    let (tx_price, set_tx_price) = signal(String::new());
    let (tx_payment, set_tx_payment) = signal("cash".to_string());
    let (tx_customer, set_tx_customer) = signal("Walk-in".to_string());
    let (tx_notes, set_tx_notes) = signal(String::new());

    let reload = {
        let set_services = set_services;
        let set_transactions = set_transactions;
        move || leptos::task::spawn_local(async move {
            if let Ok(items) = api::get_all_services().await {
                set_services.set(items);
            }
            if let Ok(items) = api::get_all_service_transactions().await {
                set_transactions.set(items);
            }
        })
    };
    reload();

    let reset_service_form = move || {
        set_service_name.set(String::new());
        set_service_desc.set(String::new());
        set_service_price.set(String::new());
        set_service_unit.set(String::new());
        set_show_service_form.set(false);
    };

    let reset_transaction_form = move || {
        set_selected_service.set(None);
        set_tx_qty.set("1".to_string());
        set_tx_price.set(String::new());
        set_tx_payment.set("cash".to_string());
        set_tx_customer.set("Walk-in".to_string());
        set_tx_notes.set(String::new());
        set_show_transaction_form.set(false);
    };

    let active_count = move || services.get().iter().filter(|s| s.is_active == 1).count();
    let service_transactions = move || {
        transactions
            .get()
            .into_iter()
            .filter(|t| t.printing_material_id.is_none() && t.stock_metres_used <= 0.0)
            .collect::<Vec<_>>()
    };
    let total_revenue = move || service_transactions().iter().map(|t| t.amount).sum::<f64>();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let today_revenue = move || {
        service_transactions()
            .iter()
            .filter(|t| t.timestamp.as_ref().map_or(false, |ts| ts.starts_with(&today)))
            .map(|t| t.amount)
            .sum::<f64>()
    };

    let save_service = {
        let reload = reload.clone();
        move |_| {
            let name = service_name.get().trim().to_string();
            let price = service_price.get().parse::<f64>().unwrap_or(0.0);
            let unit = service_unit.get().trim().to_string();
            let desc = service_desc.get().trim().to_string();
            if name.is_empty() { return; }
            reset_service_form();
            leptos::task::spawn_local(async move {
                let _ = api::add_service(&NewService {
                    name,
                    description: Some(desc).filter(|v| !v.is_empty()),
                    price: Some(price),
                    unit: Some(unit).filter(|v| !v.is_empty()),
                    is_active: 1,
                }).await;
                reload();
            });
        }
    };

    let save_transaction = {
        let reload = reload.clone();
        move |_| {
            let service_id = selected_service.get();
            let service = service_id.and_then(|id| services.get().into_iter().find(|s| s.id == id));
            let Some(service) = service else { return; };
            let qty = tx_qty.get().parse::<f64>().unwrap_or(1.0);
            let price = tx_price.get().parse::<f64>().unwrap_or(service.price);
            let amount = qty * price;
            let payment_method = tx_payment.get();
            let customer_name = tx_customer.get();
            let notes = tx_notes.get();
            reset_transaction_form();
            leptos::task::spawn_local(async move {
                let _ = api::add_service_transaction(&NewServiceTransaction {
                    service_id: Some(service.id),
                    service_name: service.name,
                    quantity: qty,
                    price: Some(price),
                    amount: Some(amount),
                    payment_method,
                    customer_name,
                    notes: Some(notes).filter(|v| !v.is_empty()),
                    stock_id: None,
                    stock_metres_used: 0.0,
                    material_size: None,
                    material_type: None,
                    printing_material_id: None,
                    is_debt: 0,
                }).await;
                reload();
            });
        }
    };

    let toggle_service = move |service: Service| {
        let reload = reload.clone();
        leptos::task::spawn_local(async move {
            let next = if service.is_active == 1 { 0 } else { 1 };
            let _ = api::update_service(service.id, &serde_json::json!({"is_active": next})).await;
            reload();
        });
    };

    let delete_service = move |id: i64| {
        let reload = reload.clone();
        leptos::task::spawn_local(async move {
            let _ = api::delete_service(id).await;
            reload();
        });
    };

    let delete_transaction = move |id: i64| {
        let reload = reload.clone();
        leptos::task::spawn_local(async move {
            let _ = api::delete_service_transaction(id).await;
            reload();
        });
    };

    view! {
        <div id="page-services" class="page-content">
            <div class="flex items-center justify-between mb-6">
                <div>
                    <h1 class="page-title">"Services"</h1>
                    <p class="page-subtitle">"Manage services and track earnings"</p>
                </div>
                <div class="flex gap-3">
                    <button class="btn-secondary px-4 py-2 text-sm" on:click=move |_| set_show_service_form.set(true)>"Add Service"</button>
                    <button class="btn-primary px-4 py-2 text-sm" on:click=move |_| set_show_transaction_form.set(true)>"Record Transaction"</button>
                </div>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
                <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Active Services"</p><p class="text-xl font-bold text-gray-900">{move || active_count()}</p></div>
                <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Transactions"</p><p class="text-xl font-bold text-gray-900">{move || service_transactions().len()}</p></div>
                <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Today's Earnings"</p><p class="text-xl font-bold text-gray-900">{move || format!("KSh {:.0}", today_revenue())}</p></div>
                <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Total Earnings"</p><p class="text-xl font-bold text-gray-900">{move || format!("KSh {:.0}", total_revenue())}</p></div>
            </div>

            <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
                <div class="dashboard-panel overflow-hidden">
                    <div class="p-5 border-b border-gray-100">
                        <h2 class="dashboard-panel-title">"Service Catalog"</h2>
                    </div>
                    <div class="divide-y divide-gray-100">
                        {move || {
                            let items = services.get();
                            if items.is_empty() {
                                view! { <div class="p-8 text-center text-gray-400">"No services added yet"</div> }.into_any()
                            } else {
                                items.into_iter().map(|service| {
                                    let service_for_toggle = service.clone();
                                    let sid = service.id;
                                    view! {
                                        <div class="p-4 flex items-center justify-between gap-4">
                                            <div class="min-w-0">
                                                <div class="flex items-center gap-2">
                                                    <h3 class="font-medium text-gray-900 truncate">{service.name.clone()}</h3>
                                                    <span class=if service.is_active == 1 { "status-badge status-badge--success" } else { "status-badge bg-gray-100 text-gray-500" }>
                                                        {if service.is_active == 1 { "Active" } else { "Inactive" }}
                                                    </span>
                                                </div>
                                                <p class="text-sm text-gray-500 mt-1">{service.description.clone().unwrap_or_default()}</p>
                                                <p class="text-sm font-medium text-gray-900 mt-1">"KSh " {service.price} {service.unit.clone().map(|u| format!("/{}", u)).unwrap_or_default()}</p>
                                            </div>
                                            <div class="flex items-center gap-2 shrink-0">
                                                <button class="text-xs text-brand-600 hover:underline" on:click=move |_| toggle_service(service_for_toggle.clone())>
                                                    {if service.is_active == 1 { "Disable" } else { "Enable" }}
                                                </button>
                                                <button class="text-xs text-red-600 hover:underline" on:click=move |_| delete_service(sid)>"Delete"</button>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }
                        }}
                    </div>
                </div>

                <div class="dashboard-panel overflow-hidden">
                    <div class="p-5 border-b border-gray-100">
                        <h2 class="dashboard-panel-title">"Recent Service Transactions"</h2>
                    </div>
                    <div class="overflow-x-auto">
                        <table class="w-full data-table">
                            <thead><tr><th>"Service"</th><th>"Customer"</th><th>"Payment"</th><th class="text-right">"Amount"</th><th class="text-right">"Actions"</th></tr></thead>
                            <tbody>
                                {move || {
                                    let items = service_transactions();
                                    if items.is_empty() {
                                        view! { <tr><td colspan="5" class="px-5 py-8 text-center text-gray-500">"No service transactions recorded."</td></tr> }.into_any()
                                    } else {
                                        items.into_iter().take(10).map(|tx| {
                                            let tid = tx.id;
                                            view! {
                                                <tr class="border-b border-gray-50">
                                                    <td class="px-4 py-3 text-sm font-medium">{tx.service_name}</td>
                                                    <td class="px-4 py-3 text-sm text-gray-600">{tx.customer_name}</td>
                                                    <td class="px-4 py-3 text-sm text-gray-600 capitalize">{tx.payment_method}</td>
                                                    <td class="px-4 py-3 text-sm font-medium text-right">"KSh " {tx.amount}</td>
                                                    <td class="px-4 py-3 text-right"><button class="text-xs text-red-600 hover:underline" on:click=move |_| delete_transaction(tid)>"Delete"</button></td>
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>().into_any()
                                    }
                                }}
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>

            {move || if show_service_form.get() {
                view! {
                    <div class="modal-overlay open">
                        <div class="modal-container">
                            <div class="modal-header"><h3 class="modal-title">"Add Service"</h3><button class="modal-close-btn" on:click=move |_| reset_service_form()>"x"</button></div>
                            <div class="modal-body space-y-4">
                                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Name"</label><input class="w-full" prop:value=move || service_name.get() on:input=move |e| set_service_name.set(event_target_value(&e))/></div>
                                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Description"</label><textarea class="w-full" rows="2" prop:value=move || service_desc.get() on:input=move |e| set_service_desc.set(event_target_value(&e))></textarea></div>
                                <div class="grid grid-cols-2 gap-4">
                                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Price"</label><input type="number" min="0" step="0.01" class="w-full" prop:value=move || service_price.get() on:input=move |e| set_service_price.set(event_target_value(&e))/></div>
                                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Unit"</label><input class="w-full" placeholder="item, hour, job" prop:value=move || service_unit.get() on:input=move |e| set_service_unit.set(event_target_value(&e))/></div>
                                </div>
                            </div>
                            <div class="modal-footer"><button class="btn-secondary px-4 py-2" on:click=move |_| reset_service_form()>"Cancel"</button><button class="btn-primary px-4 py-2" on:click=save_service>"Save"</button></div>
                        </div>
                    </div>
                }.into_any()
            } else { ().into_any() }}

            {move || if show_transaction_form.get() {
                view! {
                    <div class="modal-overlay open">
                        <div class="modal-container">
                            <div class="modal-header"><h3 class="modal-title">"Record Service Transaction"</h3><button class="modal-close-btn" on:click=move |_| reset_transaction_form()>"x"</button></div>
                            <div class="modal-body space-y-4">
                                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Service"</label>
                                    <select class="w-full" on:change=move |e| {
                                        let id = event_target_value(&e).parse::<i64>().ok();
                                        if let Some(id) = id {
                                            if let Some(s) = services.get().into_iter().find(|s| s.id == id) {
                                                set_tx_price.set(s.price.to_string());
                                            }
                                        }
                                        set_selected_service.set(id);
                                    }>
                                        <option value="">"Select service"</option>
                                        {move || services.get().into_iter().filter(|s| s.is_active == 1).map(|s| view! { <option value=s.id.to_string()>{s.name}</option> }).collect::<Vec<_>>()}
                                    </select>
                                </div>
                                <div class="grid grid-cols-2 gap-4">
                                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Quantity"</label><input type="number" min="0.01" step="0.01" class="w-full" prop:value=move || tx_qty.get() on:input=move |e| set_tx_qty.set(event_target_value(&e))/></div>
                                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Unit Price"</label><input type="number" min="0" step="0.01" class="w-full" prop:value=move || tx_price.get() on:input=move |e| set_tx_price.set(event_target_value(&e))/></div>
                                </div>
                                <div class="grid grid-cols-2 gap-4">
                                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Payment"</label><select class="w-full" on:change=move |e| set_tx_payment.set(event_target_value(&e))><option value="cash">"Cash"</option><option value="mpesa">"M-Pesa"</option><option value="till">"Till Number"</option></select></div>
                                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer"</label><input class="w-full" prop:value=move || tx_customer.get() on:input=move |e| set_tx_customer.set(event_target_value(&e))/></div>
                                </div>
                                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Notes"</label><textarea class="w-full" rows="2" prop:value=move || tx_notes.get() on:input=move |e| set_tx_notes.set(event_target_value(&e))></textarea></div>
                            </div>
                            <div class="modal-footer"><button class="btn-secondary px-4 py-2" on:click=move |_| reset_transaction_form()>"Cancel"</button><button class="btn-primary px-4 py-2" on:click=save_transaction>"Record"</button></div>
                        </div>
                    </div>
                }.into_any()
            } else { ().into_any() }}
        </div>
    }
}
