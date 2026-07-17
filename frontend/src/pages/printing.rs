use crate::api::{
    self, NewDebt, NewPrintingMaterial, NewServiceTransaction, PrintingMaterial, PrintingPageQuery,
    ServiceTransaction,
};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;

#[path = "../components/dropdown.rs"]
mod dropdown_comp;
use dropdown_comp::{CustomDropdown, DropdownItem};

fn format_printing_timestamp(ts: &Option<String>) -> String {
    ts.as_ref()
        .and_then(|t| {
            chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.3f")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S"))
                .ok()
        })
        .map(|dt| {
            let today = chrono::Local::now().date_naive();
            let job_day = dt.date();
            let time = dt.format("%I:%M %p").to_string();

            if job_day == today {
                format!("Today {}", time)
            } else if job_day == today.pred_opt().unwrap_or(today) {
                format!("Yesterday {}", time)
            } else {
                dt.format("%d/%m/%Y %I:%M %p").to_string()
            }
        })
        .or_else(|| ts.clone())
        .unwrap_or_else(|| "-".to_string())
}

#[component]
pub fn PrintingPage(show_revenue_stats: bool, can_manage_materials: bool) -> impl IntoView {
    let (jobs, set_jobs) = signal(Vec::<ServiceTransaction>::new());
    let (materials, set_materials) = signal(Vec::<PrintingMaterial>::new());
    let (today_earnings, set_today_earnings) = signal(0.0f64);
    let (total_jobs_count, set_total_jobs_count) = signal(0u32);
    let (material_used, set_material_used) = signal(0.0f64);
    let (total_revenue, set_total_revenue) = signal(0.0f64);
    let (total_count, set_total_count) = signal(0u32);
    let (show_record, set_show_record) = signal(false);
    let (show_add_mat, set_show_add_mat) = signal(false);
    let (current_page, set_current_page) = signal(1u32);
    let (search, set_search) = signal(String::new());
    let (sort_by, set_sort_by) = signal("newest".to_string());
    let items_per_page = 10u32;

    // Record job state
    let (mat_id, set_mat_id) = signal(None::<i64>);
    let (metres_printed, set_metres_printed) = signal("1".to_string());
    let (total_price, set_total_price) = signal(String::new());
    let (payment, set_payment) = signal("cash".to_string());
    let (customer, set_customer) = signal("Walk-in".to_string());

    // Add material state
    let (mat_name, set_mat_name) = signal(String::new());
    let (mat_width, set_mat_width) = signal(String::new());
    let (mat_rolls, set_mat_rolls) = signal("1".to_string());
    let (mat_mpr, set_mat_mpr) = signal("50".to_string());

    // Add rolls modal
    let (show_add_rolls, set_show_add_rolls) = signal(None::<PrintingMaterial>);
    let (add_rolls_val, set_add_rolls_val) = signal(String::new());

    // Convert to debt
    let (show_convert, set_show_convert) = signal(None::<ServiceTransaction>);
    let (conv_cust, set_conv_cust) = signal(String::new());
    let (conv_phone, set_conv_phone) = signal(String::new());
    let (conv_paid, set_conv_paid) = signal(String::new());
    let (conv_due, set_conv_due) = signal(String::new());

    let reload = {
        let sj = set_jobs;
        let sm = set_materials;
        let ste = set_today_earnings;
        let stjc = set_total_jobs_count;
        let smu = set_material_used;
        let strv = set_total_revenue;
        let stc = set_total_count;
        let search_r = search;
        let sort_r = sort_by;
        let page_r = current_page;
        move || {
            leptos::task::spawn_local({
                let sj = sj;
                let sm = sm;
                let ste = ste;
                let stjc = stjc;
                let smu = smu;
                let strv = strv;
                let stc = stc;
                let query = PrintingPageQuery {
                    search: Some(search_r.get()),
                    sort_by: Some(sort_r.get()),
                    page: Some(page_r.get()),
                    per_page: Some(items_per_page),
                };
                async move {
                    if let Ok(page) = api::get_printing_page(&query).await {
                        ste.set(page.today_earnings);
                        stjc.set(page.total_jobs_count as u32);
                        smu.set(page.material_used);
                        strv.set(page.total_revenue);
                        stc.set(page.total_count as u32);
                        sj.set(page.items);
                    }
                    if let Ok(m) = api::get_all_printing_materials().await {
                        sm.set(m);
                    }
                }
            })
        }
    };

    // Lightweight poll: jobs page only (materials catalog reloads on full reload)
    let reload_list = {
        let sj = set_jobs;
        let ste = set_today_earnings;
        let stjc = set_total_jobs_count;
        let smu = set_material_used;
        let strv = set_total_revenue;
        let stc = set_total_count;
        let search_r = search;
        let sort_r = sort_by;
        let page_r = current_page;
        move || {
            leptos::task::spawn_local({
                let sj = sj;
                let ste = ste;
                let stjc = stjc;
                let smu = smu;
                let strv = strv;
                let stc = stc;
                let query = PrintingPageQuery {
                    search: Some(search_r.get()),
                    sort_by: Some(sort_r.get()),
                    page: Some(page_r.get()),
                    per_page: Some(items_per_page),
                };
                async move {
                    if let Ok(page) = api::get_printing_page(&query).await {
                        ste.set(page.today_earnings);
                        stjc.set(page.total_jobs_count as u32);
                        smu.set(page.material_used);
                        strv.set(page.total_revenue);
                        stc.set(page.total_count as u32);
                        sj.set(page.items);
                    }
                }
            })
        }
    };

    create_effect(move |_| {
        let _ = search.get();
        let _ = sort_by.get();
        let _ = current_page.get();
        reload();
    });
    let (live_tick, set_live_tick) = signal(0u64);
    create_effect(move |_| {
        let tick = live_tick.get();
        if tick == 0 {
            return;
        }
        reload_list();
    });
    use_auto_refresh(LIVE_REFRESH_MS, move || {
        set_live_tick.update(|t| *t = t.wrapping_add(1));
    });

    // Dropdowns
    let material_items = Signal::derive(move || {
        materials
            .get()
            .into_iter()
            .filter(|m| {
                let rem = m.total_metres
                    - if m.metres_used.is_nan() {
                        0.0
                    } else {
                        m.metres_used
                    };
                rem > 0.0
            })
            .map(|m| {
                let rem = m.total_metres
                    - if m.metres_used.is_nan() {
                        0.0
                    } else {
                        m.metres_used
                    };
                DropdownItem::new(
                    &m.id.to_string(),
                    &format!("{} - {}m width", m.name, m.width),
                )
                .with_badge(&format!("{:.1}m", rem))
            })
            .collect::<Vec<_>>()
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
            DropdownItem::new("amount_desc", "Highest Amount"),
            DropdownItem::new("amount_asc", "Lowest Amount"),
        ]
    });

    create_effect(move |_| {
        let _ = search.get();
        let _ = sort_by.get();
        set_current_page.set(1);
    });

    // Pagination
    let total_items = move || total_count.get();
    let total_pages = move || {
        let n = total_items();
        if n == 0 {
            1
        } else {
            n.div_ceil(items_per_page)
        }
    };
    let paginated = move || jobs.get();

    let remaining = |m: &PrintingMaterial| {
        m.total_metres
            - if m.metres_used.is_nan() {
                0.0
            } else {
                m.metres_used
            }
    };
    let rolls_remaining = move |m: &PrintingMaterial| {
        let rem = remaining(m);
        if m.metres_per_roll > 0.0 {
            rem / m.metres_per_roll
        } else {
            rem / 50.0
        }
    };

    // Submit record job
    let submit_job = {
        let l = reload;
        move |_| {
            let metres: f64 = metres_printed.get().parse().unwrap_or(0.0);
            let price: f64 = total_price.get().parse().unwrap_or(0.0);
            let mid = mat_id.get();
            if mid.is_none() || metres <= 0.0 || price <= 0.0 {
                return;
            }
            let m = materials
                .get()
                .iter()
                .find(|x| x.id == mid.unwrap())
                .cloned();
            let name = m
                .as_ref()
                .map(|m| format!("{} - {}m", m.name, metres as u64))
                .unwrap_or_default();
            set_show_record.set(false);
            leptos::task::spawn_local(async move {
                let _ = api::add_service_transaction(&NewServiceTransaction {
                    service_id: None,
                    service_name: name,
                    quantity: 1.0,
                    price: Some(price),
                    amount: Some(price),
                    payment_method: payment.get(),
                    customer_name: customer.get(),
                    notes: Some(format!("Printing - {}m", metres as u64)),
                    stock_id: None,
                    stock_metres_used: metres,
                    material_size: m.as_ref().map(|m| m.width.to_string()),
                    material_type: m.as_ref().map(|m| m.material_type.clone()),
                    printing_material_id: mid,
                    is_debt: 0,
                })
                .await;
                l();
            });
        }
    };

    // Submit add material
    let submit_add_mat = {
        let l = reload;
        move |_| {
            let name = mat_name.get();
            let width: f64 = mat_width.get().parse().unwrap_or(0.0);
            let rolls: i64 = mat_rolls.get().parse().unwrap_or(0);
            let mpr: f64 = mat_mpr.get().parse().unwrap_or(50.0);
            if name.is_empty() || width <= 0.0 || rolls <= 0 {
                return;
            }
            set_show_add_mat.set(false);
            leptos::task::spawn_local(async move {
                let _ = api::add_printing_material(&NewPrintingMaterial {
                    name,
                    material_type: "Custom".into(),
                    width,
                    rolls,
                    metres_per_roll: mpr,
                    total_metres: Some(rolls as f64 * mpr),
                    metres_used: 0.0,
                    color: None,
                })
                .await;
                l();
            });
        }
    };

    // Submit add rolls
    let submit_add_rolls = {
        let l = reload;
        move |_| {
            let mat = show_add_rolls.get();
            let added: i64 = add_rolls_val.get().parse().unwrap_or(0);
            if mat.is_none() || added <= 0 {
                return;
            }
            let m = mat.unwrap();
            set_show_add_rolls.set(None);
            leptos::task::spawn_local(async move {
                let _ = api::add_printing_material_rolls(m.id, added).await;
                l();
            });
        }
    };

    // Submit convert to debt
    let submit_convert = {
        let l = reload;
        move |_| {
            let txn = show_convert.get();
            if let Some(t) = txn {
                let name = conv_cust.get();
                if name.is_empty() {
                    return;
                }
                let paid: f64 = conv_paid.get().parse().unwrap_or(0.0);
                let remaining = t.amount - paid;
                if remaining <= 0.0 {
                    set_show_convert.set(None);
                    return;
                }
                let t_id = t.id;
                set_show_convert.set(None);
                leptos::task::spawn_local(async move {
                    let _ = api::add_debt(&NewDebt {
                        customer_name: name,
                        phone: Some(conv_phone.get()).filter(|p| !p.is_empty()),
                        amount: t.amount,
                        paid_amount: Some(paid),
                        remaining_amount: Some(remaining),
                        due_date: Some(conv_due.get()).filter(|d| !d.is_empty()),
                        description: Some(format!("Printing Job: {}", t.service_name)),
                        sale_id: None,
                        service_transaction_id: Some(t_id),
                    })
                    .await;
                    let _ =
                        api::update_service_transaction(t_id, &serde_json::json!({"is_debt": 1}))
                            .await;
                    l();
                });
            }
        }
    };

    let delete_job = move |id: i64| {
        let l = reload;
        leptos::task::spawn_local(async move {
            let _ = api::delete_service_transaction(id).await;
            l();
        });
    };

    let delete_material = move |id: i64| {
        let l = reload;
        leptos::task::spawn_local(async move {
            let _ = api::delete_printing_material(id).await;
            l();
        });
    };

    view! { <div id="page-printing" class="page-content">
        <div class="flex items-center justify-between mb-6">
            <div><h1 class="page-title">"Printing Services"</h1><p class="page-subtitle">"One-way vision, banners, satin, and reflective printing"</p></div>
            <div class="flex gap-3">
                {move || if can_manage_materials { view! {
                    <button class="flex items-center gap-2 bg-white border border-gray-200 text-gray-700 px-4 py-2 text-sm font-medium hover:bg-gray-50 transition-all"
                        on:click=move |_| { set_mat_name.set(String::new()); set_mat_width.set(String::new()); set_mat_rolls.set("1".into()); set_mat_mpr.set("50".into()); set_show_add_mat.set(true); }>
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
                        "Add Material"</button>
                }.into_any() } else { ().into_any() }}
                <button class="flex items-center gap-2 bg-brand-500 text-white px-4 py-2 text-sm font-medium hover:bg-brand-600 transition-all"
                    on:click=move |_| { set_mat_id.set(None); set_metres_printed.set("1".into()); set_total_price.set(String::new()); set_payment.set("cash".into()); set_customer.set("Walk-in".into()); set_show_record.set(true); }>
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
                    "Record Printing Job"</button>
            </div>
        </div>

        // Stats
        <div class=move || format!("grid grid-cols-1 gap-4 mb-6 {}", if show_revenue_stats { "md:grid-cols-4" } else { "md:grid-cols-3" })>
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Today's Earnings"</p><p class="text-xl font-bold text-gray-900">{move || format!("KSh {:.0}", today_earnings.get())}</p></div>
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Total Jobs"</p><p class="text-xl font-bold text-gray-900">{move || total_jobs_count.get()}</p></div>
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Material Used"</p><p class="text-xl font-bold text-gray-900">{move || format!("{}m", material_used.get() as u64)}</p></div>
            {move || if show_revenue_stats { view! {
                <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium uppercase tracking-wide mb-1">"Total Revenue"</p><p class="text-xl font-bold text-gray-900">{move || format!("KSh {:.0}", total_revenue.get())}</p></div>
            }.into_any() } else { ().into_any() }}
        </div>

        // Materials Inventory
        <div class="grid grid-cols-1 gap-6 mb-6">
            <div class="dashboard-panel overflow-hidden">
                <div class="p-5 border-b border-gray-100"><h2 class="dashboard-panel-title">"Printing Materials Inventory"</h2><p class="text-xs text-gray-500 mt-1">"Materials used for printing (Banner, Satin, Canvas, etc.)"</p></div>
                <div class="p-5">
                    <div class="space-y-3">
                        {move || {
                            let mats = materials.get();
                            if mats.is_empty() {
                                view!{<div class="text-center py-8 text-gray-400"><svg class="w-12 h-12 mx-auto mb-3 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/></svg><p>"No materials added yet"</p></div>}.into_any()
                            } else {
                                mats.into_iter().map(|m| {
                                    let rem = remaining(&m);
                                    let rr = rolls_remaining(&m);
                                    let pct = if m.total_metres > 0.0 { rem / m.total_metres * 100.0 } else { 0.0 };
                                    let (status_label, status_text, status_bg, progress_bg) = if pct > 20.0 {
                                        ("Healthy", "text-green-700", "bg-green-50", "bg-green-500")
                                    } else if pct > 10.0 {
                                        ("Low", "text-yellow-700", "bg-yellow-50", "bg-yellow-500")
                                    } else {
                                        ("Critical", "text-red-700", "bg-red-50", "bg-red-500")
                                    };
                                    let mid = m.id;
                                    view!{
                                        <div class="p-5 bg-white border border-gray-200 rounded-xl hover:border-gray-300 transition-colors">
                                            <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
                                                <div class="min-w-0 flex-1">
                                                    <div class="flex flex-wrap items-center gap-2 mb-3">
                                                        <h4 class="text-base font-semibold text-gray-900 truncate">{m.name.clone()}</h4>
                                                        <span class="inline-flex items-center px-2.5 py-1 rounded-full text-[11px] font-medium bg-blue-50 text-blue-700 border border-blue-100">{m.material_type.clone()}</span>
                                                        <span class=format!("inline-flex items-center px-2.5 py-1 rounded-full text-[11px] font-medium {} {} border border-transparent", status_bg, status_text)>{status_label}</span>
                                                    </div>

                                                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
                                                        <div class="rounded-lg border border-gray-100 bg-gray-50 px-3 py-3">
                                                            <p class="text-[11px] uppercase tracking-wide text-gray-500 font-medium">"Width"</p>
                                                            <p class="mt-1 text-sm font-semibold text-gray-900">{format!("{}m", m.width)}</p>
                                                        </div>
                                                        <div class="rounded-lg border border-gray-100 bg-gray-50 px-3 py-3">
                                                            <p class="text-[11px] uppercase tracking-wide text-gray-500 font-medium">"Total Rolls"</p>
                                                            <p class="mt-1 text-sm font-semibold text-gray-900">{m.rolls}</p>
                                                        </div>
                                                        <div class="rounded-lg border border-gray-100 bg-gray-50 px-3 py-3">
                                                            <p class="text-[11px] uppercase tracking-wide text-gray-500 font-medium">"Remaining"</p>
                                                            <p class=format!("mt-1 text-sm font-semibold {}", status_text)>{format!("{:.1} rolls", rr)}</p>
                                                            <p class="text-xs text-gray-500 mt-0.5">{format!("{:.1}m left", rem)}</p>
                                                        </div>
                                                    </div>

                                                    <div class="mt-4">
                                                        <div class="flex items-center justify-between text-xs mb-1.5">
                                                            <span class="text-gray-500 font-medium">"Stock Level"</span>
                                                            <span class=format!("font-semibold {}", status_text)>{format!("{:.0}% remaining", pct.max(0.0))}</span>
                                                        </div>
                                                        <div class="w-full h-2 rounded-full bg-gray-100 overflow-hidden">
                                                            <div class=progress_bg style=move || format!("width: {}%", pct.clamp(0.0, 100.0))></div>
                                                        </div>
                                                    </div>
                                                </div>

                                                <div class="flex gap-2 lg:ml-4">
                                                    <button on:click=move |_| { set_add_rolls_val.set(String::new()); set_show_add_rolls.set(Some(m.clone())); } class="px-3 py-2 text-xs font-medium bg-black text-white rounded-md hover:bg-gray-800 transition-colors whitespace-nowrap">"Add Rolls"</button>
                                                    <button on:click=move |_| delete_material(mid) class="h-9 w-9 inline-flex items-center justify-center text-gray-400 hover:text-red-600 transition-colors border border-gray-200 rounded-md hover:border-red-200">
                                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }
                        }}
                    </div>
                </div>
            </div>
        </div>

        // Jobs Table
        <div class="dashboard-panel overflow-hidden">
            <div class="p-5 border-b border-gray-100 flex flex-col md:flex-row md:items-center md:justify-between gap-3">
                <h2 class="dashboard-panel-title">"Printing Jobs"</h2>
                <div class="flex flex-col sm:flex-row gap-2">
                    <input
                        type="text"
                        class="min-w-[260px] border border-gray-200 rounded px-3 py-2 text-sm outline-none focus:border-[#2563EB] focus:shadow-[0_0_0_2px_#EFF6FF]"
                        placeholder="Search job, customer, material..."
                        prop:value=move || search.get()
                        on:input=move |e| set_search.set(event_target_value(&e))
                    />
                    <div class="min-w-[220px]">
                        <CustomDropdown
                            items=sort_items
                            placeholder="Newest First".to_string()
                            on_select=Callback::new(move |v: String| set_sort_by.set(v))
                        />
                    </div>
                </div>
            </div>
            <table class="w-full data-table">
                <thead><tr><th>"Date / Time"</th><th>"Job Details"</th><th>"Metres"</th><th>"Material"</th><th>"Amount"</th><th>"Payment"</th><th>"Customer"</th><th>"Actions"</th></tr></thead>
                <tbody>
                    {move || {
                        let items = paginated();
                        if items.is_empty() {
                            view!{<tr><td colspan="8" class="px-5 py-8 text-center text-gray-500">"No printing jobs recorded."</td></tr>}.into_any()
                        } else {
                            items.into_iter().map(|t| {
                                let id = t.id;
                                let tm = format_printing_timestamp(&t.timestamp);
                                let mat = if let Some(ref sz) = t.material_size { format!("{}m {}", sz, t.material_type.as_deref().unwrap_or("")) } else { "N/A".into() };
                                let name = t.service_name.clone();
                                let pm_label = t.payment_method.clone();
                                let cust = t.customer_name.clone();
                                let txn_clone = t.clone();
                                view!{
                                    <tr class="hover:bg-gray-50 transition-colors">
                                        <td class="px-5 py-4 text-sm text-gray-500 whitespace-nowrap">{tm}</td>
                                        <td class="px-5 py-4"><p class="text-sm font-medium text-gray-900">{name}</p></td>
                                        <td class="px-5 py-4 text-sm text-gray-600">{format!("{:.1}m", t.stock_metres_used)}</td>
                                        <td class="px-5 py-4 text-sm text-gray-600">{mat}</td>
                                        <td class="px-5 py-4 text-sm font-medium text-gray-900">{format!("KSh {:.2}", t.amount)}</td>
                                        <td class="px-5 py-4"><span class="status-badge status-badge--success capitalize">{pm_label}</span></td>
                                        <td class="px-5 py-4 text-sm text-gray-600">{cust}</td>
                                        <td class="px-5 py-4"><div class="flex items-center gap-2">
                                            <button on:click=move |_| { set_conv_cust.set(txn_clone.customer_name.clone()); set_conv_phone.set(String::new()); set_conv_paid.set("0".into()); set_conv_due.set(String::new()); set_show_convert.set(Some(txn_clone.clone())); } class="text-gray-400 hover:text-blue-600 transition-colors" title="Convert to debt">
                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"/></svg>
                                            </button>
                                            <button on:click=move |_| delete_job(id) class="text-gray-400 hover:text-red-600 transition-colors" title="Delete">
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
                    <div class="text-sm text-gray-600">"Showing "<span class="font-medium">{si}</span>" to "<span class="font-medium">{ei}</span>" of "<span class="font-medium">{n}</span>" jobs"</div>
                    <div class="flex gap-2">
                        <button on:click=move |_| { if cp>1 {set_current_page.set(cp-1)} } class=format!("px-3 py-1 text-sm font-medium rounded-md {}", if cp==1 {"bg-gray-200 text-gray-400 cursor-not-allowed"} else {"bg-black text-white hover:bg-gray-800"}) disabled=move || cp==1>"Previous"</button>
                        <span class="px-3 py-1 text-sm font-medium text-gray-700">{format!("Page {} of {}", cp, tp)}</span>
                        <button on:click=move |_| { if cp<tp {set_current_page.set(cp+1)} } class=format!("px-3 py-1 text-sm font-medium rounded-md {}", if cp>=tp {"bg-gray-200 text-gray-400 cursor-not-allowed"} else {"bg-black text-white hover:bg-gray-800"}) disabled=move || tp <= cp>"Next"</button>
                    </div>
                </div>}.into_any()
            }}}
        </div>

        // Record Job Modal
        {move || if show_record.get() { view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 900px;"><div class="modal-header"><h3 class="modal-title">"Record Printing Job"</h3><button class="modal-close-btn" on:click=move |_| set_show_record.set(false)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body"><div class="space-y-4">
            <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Select Material *"</label><div>
                <CustomDropdown items=material_items placeholder="Select material".to_string() on_select=Callback::new(move |v: String| { if let Ok(id) = v.parse() { set_mat_id.set(Some(id)); } })/>
            </div></div>
            <div class="grid grid-cols-2 gap-4">
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Metres Printed *"</label><input type="number" min="0.1" step="0.1" class="w-full" placeholder="Metres" prop:value=move || metres_printed.get() on:input=move |e| set_metres_printed.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Total Price (KSh) *"</label><input type="number" min="0" step="0.01" class="w-full" placeholder="Enter total price" prop:value=move || total_price.get() on:input=move |e| set_total_price.set(event_target_value(&e))/></div>
            </div>
            <div class="grid grid-cols-2 gap-4">
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Payment Method *"</label><div>
                    <CustomDropdown items=payment_items placeholder="Cash".to_string() on_select=Callback::new(move |v: String| set_payment.set(v))/>
                </div></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Name"</label><input type="text" class="w-full" placeholder="Walk-in" prop:value=move || customer.get() on:input=move |e| set_customer.set(event_target_value(&e))/></div>
            </div>
            <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Total Amount"</label><div class="px-3 py-2 bg-brand-500 text-white text-lg font-bold">{move || { let p: f64 = total_price.get().parse().unwrap_or(0.0); format!("KSh {:.2}", p) }}</div></div>
            <div class="modal-footer mt-4 pt-4 border-t border-gray-100">
                <button type="button" class="btn-secondary px-4 py-2 text-sm" on:click=move |_| set_show_record.set(false)>"Cancel"</button>
                <button type="button" class="btn-primary px-4 py-2 text-sm" on:click=submit_job>"Record Job"</button>
            </div>
        </div></div></div></div>}.into_any() } else { ().into_any() }}

        // Add Material Modal
        {move || if can_manage_materials && show_add_mat.get() { view!{<div class="modal-overlay open"><div class="modal-container"><div class="modal-header"><h3 class="modal-title">"Add Printing Material"</h3><button class="modal-close-btn" on:click=move |_| set_show_add_mat.set(false)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body"><div class="space-y-4">
            <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Material Name *"</label><input type="text" class="w-full" placeholder="e.g., White Banner Vinyl, Blue Satin Fabric" prop:value=move || mat_name.get() on:input=move |e| set_mat_name.set(event_target_value(&e))/></div>
            <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Width (metres) *"</label><input type="number" step="0.1" min="0.1" class="w-full" placeholder="Enter width in metres" prop:value=move || mat_width.get() on:input=move |e| set_mat_width.set(event_target_value(&e))/></div>
            <div class="grid grid-cols-2 gap-4">
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Rolls *"</label><input type="number" min="1" class="w-full" prop:value=move || mat_rolls.get() on:input=move |e| set_mat_rolls.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Metres per Roll *"</label><input type="number" step="0.1" min="1" class="w-full" prop:value=move || mat_mpr.get() on:input=move |e| set_mat_mpr.set(event_target_value(&e))/></div>
            </div>
            <div class="bg-gray-50 border border-gray-200 p-4"><div class="text-sm"><span class="text-gray-600">"Total metres will be calculated automatically"</span></div></div>
            <div class="modal-footer mt-4 pt-4 border-t border-gray-100">
                <button type="button" class="btn-secondary px-4 py-2 text-sm" on:click=move |_| set_show_add_mat.set(false)>"Cancel"</button>
                <button type="button" class="btn-primary px-4 py-2 text-sm" on:click=submit_add_mat>"Add Material"</button>
            </div>
        </div></div></div></div>}.into_any() } else { ().into_any() }}

        // Add Rolls Modal
        {move || show_add_rolls.get().map(|m| {
            let mpr = m.metres_per_roll;
            view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 500px;"><div class="modal-header"><h3 class="modal-title">"Add Rolls to Printing Material"</h3><button class="modal-close-btn" on:click=move |_| set_show_add_rolls.set(None)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body">
                <div class="bg-gray-50 p-4 mb-4"><p class="text-xs text-gray-500 uppercase tracking-wide">"Printing Material"</p><p class="font-semibold text-gray-900">{m.name.clone()}</p>
                    <p class="text-sm text-gray-600 mt-1"><span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800">{m.material_type.clone()}</span>
                    <span class="ml-2">{format!("Width: {}m", m.width)}</span></p>
                    <p class="text-sm text-gray-600 mt-1">{format!("Current: {:.1}m remaining ({:.1} rolls)", remaining(&m), rolls_remaining(&m))}</p>
                </div>
                <div class="space-y-4">
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Number of Rolls to Add *"</label><input type="number" min="1" step="1" class="w-full" placeholder="Enter rolls to add" prop:value=move || add_rolls_val.get() on:input=move |e| set_add_rolls_val.set(event_target_value(&e))/>
                        <p class="text-xs text-gray-500 mt-1">{format!("Each roll = {}m", mpr as u64)}</p></div>
                    <div class="bg-blue-50 border border-blue-200 p-3">
                        <p class="text-sm text-gray-700"><span class="font-medium">"New Total:"</span> {move || { let a: i64 = add_rolls_val.get().parse().unwrap_or(0); format!("{} rolls ({}m)", m.rolls + a, (m.total_metres + a as f64 * mpr) as u64) }}</p>
                    </div>
                </div>
            </div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" on:click=move |_| set_show_add_rolls.set(None)>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" on:click=submit_add_rolls>"Add Rolls"</button></div></div></div>}.into_any()
        }).unwrap_or_else(|| ().into_any())}

        // Convert to Debt Modal
        {move || show_convert.get().map(|t| {
            let amount = t.amount;
            let remaining = move || { let paid: f64 = conv_paid.get().parse().unwrap_or(0.0); (amount - paid).max(0.0) };
            view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 500px;"><div class="modal-header"><h3 class="modal-title">"Convert Printing Job to Debt"</h3><button class="modal-close-btn" on:click=move |_| set_show_convert.set(None)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body">
                <div class="bg-gray-50 p-4 mb-4"><p class="text-xs text-gray-500 uppercase tracking-wide">"Printing Job Details"</p><p class="font-semibold text-gray-900">{t.service_name.clone()}</p><p class="text-sm text-gray-600">{format!("{:.1}m - KSh {:.0}", t.stock_metres_used, amount)}</p></div>
                <div class="space-y-4">
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Name *"</label><input type="text" class="w-full" placeholder="Enter customer name" prop:value=move || conv_cust.get() on:input=move |e| set_conv_cust.set(event_target_value(&e))/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Phone"</label><input type="tel" class="w-full" placeholder="Optional" prop:value=move || conv_phone.get() on:input=move |e| set_conv_phone.set(event_target_value(&e))/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Amount Paid *"</label><input type="number" min="0" step="0.01" class="w-full" placeholder="0.00" prop:value=move || conv_paid.get() on:input=move |e| set_conv_paid.set(event_target_value(&e))/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Remaining Debt"</label><div class="px-3 py-2 bg-red-50 border border-red-200 text-lg font-bold text-red-600">{move || format!("KSh {:.0}", remaining())}</div></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Due Date"</label><input type="date" class="w-full" prop:value=move || conv_due.get() on:input=move |e| set_conv_due.set(event_target_value(&e))/></div>
                </div>
            </div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" on:click=move |_| set_show_convert.set(None)>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" on:click=submit_convert>"Create Debt"</button></div></div></div>}.into_any()
        }).unwrap_or_else(|| ().into_any())}
    </div> }
}
