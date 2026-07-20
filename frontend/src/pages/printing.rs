use crate::api::{
    self, NewDebt, NewPrintingMaterial, NewServiceTransaction, PrintingMaterial, PrintingPageQuery,
    ServiceTransaction,
};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;

#[path = "../components/dropdown.rs"]
mod dropdown_comp;
use dropdown_comp::{CustomDropdown, DropdownItem};
#[path = "../components/loading.rs"]
mod loading_comp;
use loading_comp::PageLoading;
#[path = "../components/calendar.rs"]
mod calendar_comp;
use calendar_comp::MiniCalendar;
#[path = "../components/receipt.rs"]
mod receipt_comp;
use receipt_comp::{open_multi_printing_receipt, open_printing_receipt, ReceiptModal};

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
    let (_conv_due_label, set_conv_due_label) = signal(String::new());
    // Receipt preview
    let (show_receipt, set_show_receipt) = signal(false);
    let (receipt_html, set_receipt_html) = signal(String::new());
    let (receipt_title, set_receipt_title) = signal(String::new());
    // Multi-select for bulk receipt print (same pattern as sales)
    let (selected, set_selected) = signal(0u32);
    let (selected_jobs, set_selected_jobs) = signal(Vec::<i64>::new());
    let (loading, set_loading) = signal(true);
    let (job_submitting, set_job_submitting) = signal(false);
    let (mat_submitting, set_mat_submitting) = signal(false);
    let (rolls_submitting, set_rolls_submitting) = signal(false);
    let (convert_submitting, set_convert_submitting) = signal(false);

    let reload = {
        let sj = set_jobs;
        let sm = set_materials;
        let ste = set_today_earnings;
        let stjc = set_total_jobs_count;
        let smu = set_material_used;
        let strv = set_total_revenue;
        let stc = set_total_count;
        let sl = set_loading;
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
                let sl = sl;
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
                    sl.set(false);
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
            if job_submitting.get() {
                return;
            }
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
            let payment_method = payment.get();
            let customer_name = customer.get();
            set_job_submitting.set(true);
            leptos::task::spawn_local(async move {
                let ok = api::add_service_transaction(&NewServiceTransaction {
                    service_id: None,
                    service_name: name,
                    quantity: 1.0,
                    price: Some(price),
                    amount: Some(price),
                    payment_method,
                    customer_name,
                    notes: Some(format!("Printing - {}m", metres as u64)),
                    stock_id: None,
                    stock_metres_used: metres,
                    material_size: m.as_ref().map(|m| m.width.to_string()),
                    material_type: m.as_ref().map(|m| m.material_type.clone()),
                    printing_material_id: mid,
                    is_debt: 0,
                })
                .await
                .is_ok();
                if ok {
                    set_show_record.set(false);
                }
                set_job_submitting.set(false);
                l();
            });
        }
    };

    // Submit add material
    let submit_add_mat = {
        let l = reload;
        move |_| {
            if mat_submitting.get() {
                return;
            }
            let name = mat_name.get();
            let width: f64 = mat_width.get().parse().unwrap_or(0.0);
            let rolls: i64 = mat_rolls.get().parse().unwrap_or(0);
            let mpr: f64 = mat_mpr.get().parse().unwrap_or(50.0);
            if name.is_empty() || width <= 0.0 || rolls <= 0 {
                return;
            }
            set_mat_submitting.set(true);
            leptos::task::spawn_local(async move {
                let ok = api::add_printing_material(&NewPrintingMaterial {
                    name,
                    material_type: "Custom".into(),
                    width,
                    rolls,
                    metres_per_roll: mpr,
                    total_metres: Some(rolls as f64 * mpr),
                    metres_used: 0.0,
                    color: None,
                })
                .await
                .is_ok();
                if ok {
                    set_show_add_mat.set(false);
                }
                set_mat_submitting.set(false);
                l();
            });
        }
    };

    // Submit add rolls
    let submit_add_rolls = {
        let l = reload;
        move |_| {
            if rolls_submitting.get() {
                return;
            }
            let mat = show_add_rolls.get();
            let added: i64 = add_rolls_val.get().parse().unwrap_or(0);
            if mat.is_none() || added <= 0 {
                return;
            }
            let m = mat.unwrap();
            set_rolls_submitting.set(true);
            leptos::task::spawn_local(async move {
                let ok = api::add_printing_material_rolls(m.id, added).await.is_ok();
                if ok {
                    set_show_add_rolls.set(None);
                }
                set_rolls_submitting.set(false);
                l();
            });
        }
    };

    // Submit convert to debt
    let submit_convert = {
        let l = reload;
        move |_| {
            if convert_submitting.get() {
                return;
            }
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
                let phone = conv_phone.get();
                let due = conv_due.get();
                set_convert_submitting.set(true);
                leptos::task::spawn_local(async move {
                    let desc = {
                        let metres = t.stock_metres_used;
                        let mat = t.material_type.as_deref().unwrap_or("").trim();
                        if mat.is_empty() {
                            format!("Printing · {} · {:.1}m", t.service_name, metres)
                        } else {
                            format!("Printing · {} · {:.1}m · {}", t.service_name, metres, mat)
                        }
                    };
                    let ok = api::add_debt(&NewDebt {
                        customer_name: name,
                        phone: Some(phone).filter(|p| !p.is_empty()),
                        amount: t.amount,
                        paid_amount: Some(paid),
                        remaining_amount: Some(remaining),
                        due_date: Some(due).filter(|d| !d.is_empty()),
                        description: Some(desc),
                        sale_id: None,
                        service_transaction_id: Some(t_id),
                    })
                    .await
                    .is_ok();
                    if ok {
                        let _ = api::update_service_transaction(
                            t_id,
                            &serde_json::json!({"is_debt": 1}),
                        )
                        .await;
                        set_show_convert.set(None);
                    }
                    set_convert_submitting.set(false);
                    l();
                });
            }
        }
    };

    let toggle_select = move |id: i64| {
        set_selected_jobs.update(|v| {
            if let Some(p) = v.iter().position(|x| *x == id) {
                v.remove(p);
            } else {
                v.push(id);
            }
        });
        set_selected.update(|c| {
            *c = selected_jobs.get().len() as u32;
        });
    };
    let select_all = move |checked: bool| {
        if checked {
            let ids: Vec<i64> = jobs.get().iter().map(|j| j.id).collect();
            let n = ids.len() as u32;
            set_selected_jobs.set(ids);
            set_selected.set(n);
        } else {
            set_selected_jobs.set(Vec::new());
            set_selected.set(0);
        }
    };
    let print_selected = {
        let set_show = set_show_receipt;
        let set_html = set_receipt_html;
        let set_title = set_receipt_title;
        move |_| {
            let sel_ids = selected_jobs.get();
            if sel_ids.is_empty() {
                return;
            }
            let selected: Vec<ServiceTransaction> = jobs
                .get()
                .into_iter()
                .filter(|j| sel_ids.contains(&j.id))
                .collect();
            open_multi_printing_receipt(&selected, set_show, set_html, set_title);
        }
    };

    let delete_job = move |id: i64| {
        let l = reload;
        leptos::task::spawn_local(async move {
            let _ = api::delete_service_transaction(id).await;
            set_selected_jobs.update(|ids| {
                ids.retain(|x| *x != id);
                set_selected.set(ids.len() as u32);
            });
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

    view! {
        <Show when=move || !loading.get() fallback=|| view! {
            <div id="page-printing" class="dash">
                <PageLoading message="Loading printing..."/>
            </div>
        }>
        <div id="page-printing" class="dash">
            <div class="dash-table-head">
                <div>
                    <h2 class="dash-section-title">"Printing jobs"</h2>
                    <p class="prod-sub">"Materials, metres, and service income"</p>
                </div>
                <div class="dash-table-actions">
                    {move || if selected.get() > 0 {
                        view! {
                            <button
                                type="button"
                                id="btn-print-selected-jobs"
                                class="sales-btn-secondary"
                                on:click=print_selected
                            >
                                {move || format!("Print ({})", selected.get())}
                            </button>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    {move || if can_manage_materials {
                        view! {
                            <button
                                type="button"
                                class="sales-btn-secondary"
                                on:click=move |_| {
                                    set_mat_name.set(String::new());
                                    set_mat_width.set(String::new());
                                    set_mat_rolls.set("1".into());
                                    set_mat_mpr.set("50".into());
                                    set_show_add_mat.set(true);
                                }
                            >"+ Material"</button>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    <button
                        type="button"
                        class="dash-btn-primary"
                        on:click=move |_| {
                            set_mat_id.set(None);
                            set_metres_printed.set("1".into());
                            set_total_price.set(String::new());
                            set_payment.set("cash".into());
                            set_customer.set("Walk-in".into());
                            set_show_record.set(true);
                        }
                    >
                        <span aria-hidden="true">"+"</span>
                        " Record Job"
                    </button>
                </div>
            </div>

            <div class=move || {
                if show_revenue_stats {
                    "prod-metrics dash-card sales-metrics sales-metrics--4"
                } else {
                    "prod-metrics dash-card sales-metrics sales-metrics--3"
                }
            }>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Today's earnings"</p>
                    <p class="dash-metric-value">{move || format!("KSh {:.0}", today_earnings.get())}</p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Total jobs"</p>
                    <p class="dash-metric-value">{move || total_jobs_count.get()}</p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Material used"</p>
                    <p class="dash-metric-value">{move || format!("{}m", material_used.get() as u64)}</p>
                </div>
                {move || if show_revenue_stats {
                    view! {
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Total revenue"</p>
                            <p class="dash-metric-value dash-metric-value--sm">
                                {move || format!("KSh {:.0}", total_revenue.get())}
                            </p>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            </div>

            // Materials
            <div class="dash-card print-materials-card">
                <div class="print-section-head">
                    <div>
                        <h3 class="dash-chart-title">"Materials inventory"</h3>
                        <p class="prod-sub">"Banner, satin, canvas, and more"</p>
                    </div>
                </div>
                <div class="print-materials-list">
                    {move || {
                        let mats = materials.get();
                        if mats.is_empty() {
                            return view! {
                                <div class="dash-table-empty print-materials-empty">"No materials yet."</div>
                            }.into_any();
                        }
                        mats.into_iter().map(|m| {
                            let rem = remaining(&m);
                            let rr = rolls_remaining(&m);
                            let pct = if m.total_metres > 0.0 {
                                rem / m.total_metres * 100.0
                            } else {
                                0.0
                            };
                            let (status_label, status_cls, bar_cls) = if pct > 20.0 {
                                ("Healthy", "dash-status is-ok", "print-progress-fill is-ok")
                            } else if pct > 10.0 {
                                ("Low", "dash-status is-warn", "print-progress-fill is-warn")
                            } else {
                                ("Critical", "dash-status is-danger", "print-progress-fill is-danger")
                            };
                            let mid = m.id;
                            let m_clone = m.clone();
                            view! {
                                <div class="print-mat-row">
                                    <div class="print-mat-main">
                                        <div class="print-mat-title-row">
                                            <span class="dash-td-strong">{m.name.clone()}</span>
                                            <span class="dash-status is-info">{m.material_type.clone()}</span>
                                            <span class=status_cls>{status_label}</span>
                                        </div>
                                        <div class="print-mat-stats">
                                            <div class="print-mat-stat">
                                                <span class="dash-metric-label">"Width"</span>
                                                <span class="dash-td-strong tnum">{format!("{}m", m.width)}</span>
                                            </div>
                                            <div class="print-mat-stat">
                                                <span class="dash-metric-label">"Rolls"</span>
                                                <span class="dash-td-strong tnum">{m.rolls}</span>
                                            </div>
                                            <div class="print-mat-stat">
                                                <span class="dash-metric-label">"Remaining"</span>
                                                <span class="dash-td-strong tnum">{format!("{:.1} rolls", rr)}</span>
                                                <span class="prod-sub tnum">{format!("{:.1}m left", rem)}</span>
                                            </div>
                                        </div>
                                        <div class="print-progress">
                                            <div class="print-progress-meta">
                                                <span class="dash-metric-label">"Stock level"</span>
                                                <span class="prod-sub tnum">{format!("{:.0}%", pct.max(0.0))}</span>
                                            </div>
                                            <div class="print-progress-track">
                                                <div
                                                    class=bar_cls
                                                    style=format!("width:{:.1}%", pct.clamp(0.0, 100.0))
                                                ></div>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="prod-actions">
                                        <button
                                            type="button"
                                            class="prod-btn-add"
                                            on:click=move |_| {
                                                set_add_rolls_val.set(String::new());
                                                set_show_add_rolls.set(Some(m_clone.clone()));
                                            }
                                        >"Add Rolls"</button>
                                        <button
                                            type="button"
                                            class="prod-btn-icon is-danger"
                                            aria-label="Delete material"
                                            on:click=move |_| delete_material(mid)
                                        >
                                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>().into_any()
                    }}
                </div>
            </div>

            // Jobs table — toolbar separate from table card
            <div class="sales-toolbar">
                <label class="dash-search sales-search">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-4.35-4.35M11 18a7 7 0 100-14 7 7 0 000 14z"/>
                    </svg>
                    <input
                        type="search"
                        placeholder="Search job, customer, material..."
                        prop:value=move || search.get()
                        on:input=move |e| set_search.set(event_target_value(&e))
                        aria-label="Search printing jobs"
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
            <div class="dash-card dash-table-card">
                <table class="dash-table print-jobs-table">
                    <thead>
                        <tr>
                            <th class="sales-col-check">
                                <label class="custom-checkbox">
                                    <input
                                        type="checkbox"
                                        prop:checked=move || {
                                            let s = selected.get();
                                            let t = total_items();
                                            s > 0 && s == t
                                        }
                                        on:change=move |e| select_all(event_target_checked(&e))
                                    />
                                    <span class="checkmark"></span>
                                </label>
                            </th>
                            <th>"Date"</th>
                            <th>"Job"</th>
                            <th>"Metres"</th>
                            <th>"Material"</th>
                            <th>"Amount"</th>
                            <th>"Payment"</th>
                            <th>"Customer"</th>
                            <th>"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let items = paginated();
                            if items.is_empty() {
                                return view! {
                                    <tr>
                                        <td colspan="9" class="dash-table-empty">"No printing jobs recorded."</td>
                                    </tr>
                                }.into_any();
                            }
                            items.into_iter().map(|t| {
                                let id = t.id;
                                let tm = format_printing_timestamp(&t.timestamp);
                                let mat = if let Some(ref sz) = t.material_size {
                                    format!("{}m {}", sz, t.material_type.as_deref().unwrap_or(""))
                                } else {
                                    "N/A".into()
                                };
                                let name = t.service_name.clone();
                                let pm_label = t.payment_method.clone();
                                let cust = t.customer_name.clone();
                                let txn_clone = t.clone();
                                let txn_for_receipt = t.clone();
                                let amount = t.amount;
                                let is_debt = t.is_debt;
                                let paid_display = if is_debt == 0 {
                                    amount
                                } else if t.amount_paid > 0.0 {
                                    t.amount_paid
                                } else {
                                    0.0
                                };
                                let metres = t.stock_metres_used;
                                let is_sel = move || selected_jobs.get().contains(&id);
                                view! {
                                    <tr class=move || {
                                        let mut cls = if is_sel() { "sales-row is-selected" } else { "sales-row" }.to_string();
                                        if is_debt > 0 {
                                            cls.push_str(" is-debt-row");
                                        }
                                        cls
                                    }>
                                        <td class="sales-col-check">
                                            <label class="custom-checkbox">
                                                <input type="checkbox" prop:checked=is_sel on:change=move |_| toggle_select(id)/>
                                                <span class="checkmark"></span>
                                            </label>
                                        </td>
                                        <td class="dash-td-muted tnum">{tm}</td>
                                        <td class="dash-td-strong">{name}</td>
                                        <td class="dash-td-muted tnum">{format!("{:.1}m", metres)}</td>
                                        <td class="dash-td-muted">{mat}</td>
                                        <td class="dash-td-strong tnum">
                                            <div class="sale-amount-cell">
                                                <span>{format!("KSh {:.0}", paid_display)}</span>
                                                {if is_debt > 0 && paid_display + 0.009 < amount {
                                                    view! {
                                                        <span class="sale-amount-of tnum">{format!("of {:.0}", amount)}</span>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                            </div>
                                        </td>
                                        <td>
                                            {if is_debt == 1 {
                                                view! { <span class="dash-status is-debt">"Debt"</span> }.into_any()
                                            } else if is_debt == 2 {
                                                view! { <span class="dash-status is-debt-paid">"Debt paid"</span> }.into_any()
                                            } else {
                                                view! { <span class="dash-status is-ok capitalize">{pm_label}</span> }.into_any()
                                            }}
                                        </td>
                                        <td class="dash-td-muted">{cust}</td>
                                        <td>
                                            <div class="prod-actions">
                                                <button
                                                    type="button"
                                                    class="prod-btn-icon"
                                                    title="Print receipt"
                                                    aria-label="Print receipt"
                                                    on:click=move |_| open_printing_receipt(
                                                        &txn_for_receipt,
                                                        set_show_receipt,
                                                        set_receipt_html,
                                                        set_receipt_title,
                                                    )
                                                >
                                                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"/>
                                                    </svg>
                                                </button>
                                                {if is_debt == 0 {
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="prod-btn-icon"
                                                            title="Convert to debt"
                                                            aria-label="Convert to debt"
                                                            on:click=move |_| {
                                                                set_conv_cust.set(txn_clone.customer_name.clone());
                                                                set_conv_phone.set(String::new());
                                                                set_conv_paid.set("0".into());
                                                                set_conv_due.set(String::new());
                                                                set_conv_due_label.set(String::new());
                                                                set_show_convert.set(Some(txn_clone.clone()));
                                                            }
                                                        >
                                                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"/>
                                                            </svg>
                                                        </button>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                                <button
                                                    type="button"
                                                    class="prod-btn-icon is-danger"
                                                    title="Delete"
                                                    aria-label="Delete job"
                                                    on:click=move |_| delete_job(id)
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

        // Record Job Modal
        {move || if show_record.get() { view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 900px;"><div class="modal-header"><h3 class="modal-title">"Record Printing Job"</h3><button class="modal-close-btn" prop:disabled=move || job_submitting.get() on:click=move |_| { if !job_submitting.get() { set_show_record.set(false); } }><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body"><div class="space-y-4">
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
                <button type="button" class="btn-secondary px-4 py-2 text-sm" prop:disabled=move || job_submitting.get() on:click=move |_| { if !job_submitting.get() { set_show_record.set(false); } }>"Cancel"</button>
                <button type="button" class="btn-primary px-4 py-2 text-sm" prop:disabled=move || job_submitting.get() on:click=submit_job>{move || if job_submitting.get() { "Recording..." } else { "Record Job" }}</button>
            </div>
        </div></div></div></div>}.into_any() } else { ().into_any() }}

        // Add Material Modal
        {move || if can_manage_materials && show_add_mat.get() { view!{<div class="modal-overlay open"><div class="modal-container"><div class="modal-header"><h3 class="modal-title">"Add Printing Material"</h3><button class="modal-close-btn" prop:disabled=move || mat_submitting.get() on:click=move |_| { if !mat_submitting.get() { set_show_add_mat.set(false); } }><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body"><div class="space-y-4">
            <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Material Name *"</label><input type="text" class="w-full" placeholder="e.g., White Banner Vinyl, Blue Satin Fabric" prop:value=move || mat_name.get() on:input=move |e| set_mat_name.set(event_target_value(&e))/></div>
            <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Width (metres) *"</label><input type="number" step="0.1" min="0.1" class="w-full" placeholder="Enter width in metres" prop:value=move || mat_width.get() on:input=move |e| set_mat_width.set(event_target_value(&e))/></div>
            <div class="grid grid-cols-2 gap-4">
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Rolls *"</label><input type="number" min="1" class="w-full" prop:value=move || mat_rolls.get() on:input=move |e| set_mat_rolls.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Metres per Roll *"</label><input type="number" step="0.1" min="1" class="w-full" prop:value=move || mat_mpr.get() on:input=move |e| set_mat_mpr.set(event_target_value(&e))/></div>
            </div>
            <div class="bg-gray-50 border border-gray-200 p-4"><div class="text-sm"><span class="text-gray-600">"Total metres will be calculated automatically"</span></div></div>
            <div class="modal-footer mt-4 pt-4 border-t border-gray-100">
                <button type="button" class="btn-secondary px-4 py-2 text-sm" prop:disabled=move || mat_submitting.get() on:click=move |_| { if !mat_submitting.get() { set_show_add_mat.set(false); } }>"Cancel"</button>
                <button type="button" class="btn-primary px-4 py-2 text-sm" prop:disabled=move || mat_submitting.get() on:click=submit_add_mat>{move || if mat_submitting.get() { "Adding..." } else { "Add Material" }}</button>
            </div>
        </div></div></div></div>}.into_any() } else { ().into_any() }}

        // Add Rolls Modal
        {move || show_add_rolls.get().map(|m| {
            let mpr = m.metres_per_roll;
            view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 500px;"><div class="modal-header"><h3 class="modal-title">"Add Rolls to Printing Material"</h3><button class="modal-close-btn" prop:disabled=move || rolls_submitting.get() on:click=move |_| { if !rolls_submitting.get() { set_show_add_rolls.set(None); } }><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body">
                <div class="bg-gray-50 p-4 mb-4"><p class="text-xs text-gray-500 uppercase tracking-wide">"Printing Material"</p><p class="font-semibold text-gray-900">{m.name.clone()}</p>
                    <p class="text-sm text-gray-600 mt-1"><span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-neutral-100 text-neutral-800">{m.material_type.clone()}</span>
                    <span class="ml-2">{format!("Width: {}m", m.width)}</span></p>
                    <p class="text-sm text-gray-600 mt-1">{format!("Current: {:.1}m remaining ({:.1} rolls)", remaining(&m), rolls_remaining(&m))}</p>
                </div>
                <div class="space-y-4">
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Number of Rolls to Add *"</label><input type="number" min="1" step="1" class="w-full" placeholder="Enter rolls to add" prop:value=move || add_rolls_val.get() on:input=move |e| set_add_rolls_val.set(event_target_value(&e))/>
                        <p class="text-xs text-gray-500 mt-1">{format!("Each roll = {}m", mpr as u64)}</p></div>
                    <div class="bg-neutral-50 border border-neutral-200 p-3">
                        <p class="text-sm text-gray-700"><span class="font-medium">"New Total:"</span> {move || { let a: i64 = add_rolls_val.get().parse().unwrap_or(0); format!("{} rolls ({}m)", m.rolls + a, (m.total_metres + a as f64 * mpr) as u64) }}</p>
                    </div>
                </div>
            </div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" prop:disabled=move || rolls_submitting.get() on:click=move |_| { if !rolls_submitting.get() { set_show_add_rolls.set(None); } }>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" prop:disabled=move || rolls_submitting.get() on:click=submit_add_rolls>{move || if rolls_submitting.get() { "Adding..." } else { "Add Rolls" }}</button></div></div></div>}.into_any()
        }).unwrap_or_else(|| ().into_any())}

        // Convert to Debt Modal
        {move || show_convert.get().map(|t| {
            let amount = t.amount;
            let remaining = move || { let paid: f64 = conv_paid.get().parse().unwrap_or(0.0); (amount - paid).max(0.0) };
            view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 500px;"><div class="modal-header"><h3 class="modal-title">"Convert Printing Job to Debt"</h3><button class="modal-close-btn" prop:disabled=move || convert_submitting.get() on:click=move |_| { if !convert_submitting.get() { set_show_convert.set(None); } }><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><div class="modal-body">
                <div class="bg-gray-50 p-4 mb-4"><p class="text-xs text-gray-500 uppercase tracking-wide">"Printing Job Details"</p><p class="font-semibold text-gray-900">{t.service_name.clone()}</p><p class="text-sm text-gray-600">{format!("{:.1}m - KSh {:.0}", t.stock_metres_used, amount)}</p></div>
                <div class="space-y-4">
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Name *"</label><input type="text" class="w-full" placeholder="Enter customer name" prop:value=move || conv_cust.get() on:input=move |e| set_conv_cust.set(event_target_value(&e))/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Phone"</label><input type="tel" class="w-full" placeholder="Optional" prop:value=move || conv_phone.get() on:input=move |e| set_conv_phone.set(event_target_value(&e))/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Amount Paid *"</label><input type="number" min="0" step="0.01" class="w-full" placeholder="0.00" prop:value=move || conv_paid.get() on:input=move |e| set_conv_paid.set(event_target_value(&e))/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Remaining Debt"</label><div class="px-3 py-2 bg-red-50 border border-red-200 text-lg font-bold text-red-600">{move || format!("KSh {:.0}", remaining())}</div></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Due Date"</label><MiniCalendar date_r=conv_due date_w=set_conv_due label=set_conv_due_label/></div>
                </div>
            </div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" prop:disabled=move || convert_submitting.get() on:click=move |_| { if !convert_submitting.get() { set_show_convert.set(None); } }>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" prop:disabled=move || convert_submitting.get() on:click=submit_convert>{move || if convert_submitting.get() { "Creating..." } else { "Create Debt" }}</button></div></div></div>}.into_any()
        }).unwrap_or_else(|| ().into_any())}

        <ReceiptModal
            show=Signal::derive(move || show_receipt.get())
            set_show=set_show_receipt
            receipt_html=Signal::derive(move || receipt_html.get())
            title=Signal::derive(move || receipt_title.get())
        />
    </div>
        </Show>
    }
}
