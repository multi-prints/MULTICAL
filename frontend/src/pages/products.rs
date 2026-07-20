use crate::api::{self, ProductsPageQuery, SuccessResponse};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;

#[path = "../components/loading.rs"]
mod loading_comp;
use loading_comp::PageLoading;

#[component]
pub fn ProductsPage() -> impl IntoView {
    let (products, set_products) = signal(Vec::<crate::api::Product>::new());
    let (total_count, set_total_count) = signal(0u32);
    let (total_stock_units, set_total_stock_units) = signal(0i64);
    let (life_saver_stock, set_life_saver_stock) = signal(0i64);
    let (chevron_stock, set_chevron_stock) = signal(0i64);
    let (stripes_stock, set_stripes_stock) = signal(0i64);
    let (stock_value, set_stock_value) = signal(0.0f64);
    let (page, set_page) = signal(1u32);
    let (loading, set_loading) = signal(true);
    let (show_add, set_show_add) = signal(false);
    let (show_stock, set_show_stock) = signal(false);
    let (del_id, set_del_id) = signal(None::<i64>);
    let (stock_pid, set_stock_pid) = signal(None::<i64>);
    let (stock_pname, set_stock_pname) = signal(String::new());
    let (stock_pstock, set_stock_pstock) = signal(0i64);
    let (stock_qty, set_stock_qty) = signal(0i64);
    let (add_qty, set_add_qty) = signal(0i64);
    let (_add_error, set_add_error) = signal(None::<String>);
    let (sel_type, set_sel_type) = signal("life_saver".to_string());
    let (sel_color, set_sel_color) = signal("white_red".to_string());
    let (sel_size, set_sel_size) = signal("1x1".to_string());
    let per_page = 10u32;

    let refetch = {
        let sp = set_products;
        let stc = set_total_count;
        let stsu = set_total_stock_units;
        let slss = set_life_saver_stock;
        let scs = set_chevron_stock;
        let sss = set_stripes_stock;
        let ssv = set_stock_value;
        let sl = set_loading;
        move || {
            leptos::task::spawn_local(async move {
                if let Ok(p) = api::get_products_page(&ProductsPageQuery {
                    page: Some(page.get()),
                    per_page: Some(per_page),
                })
                .await
                {
                    stc.set(p.total_count as u32);
                    stsu.set(p.total_stock_units);
                    slss.set(p.life_saver_stock);
                    scs.set(p.chevron_stock);
                    sss.set(p.stripes_stock);
                    ssv.set(p.stock_value);
                    sp.set(p.items);
                }
                sl.set(false);
            });
        }
    };

    let (live_tick, set_live_tick) = signal(0u64);
    create_effect(move |_| {
        let _ = page.get();
        let _ = live_tick.get();
        refetch();
    });
    use_auto_refresh(LIVE_REFRESH_MS, move || {
        set_live_tick.update(|t| *t = t.wrapping_add(1));
    });

    #[allow(clippy::type_complexity)]
    let add_action: Action<
        (String, Option<String>, Option<String>, i64),
        Result<api::Product, String>,
        SyncStorage,
    > = Action::new_unsync(
        move |input: &(String, Option<String>, Option<String>, i64)| {
            let (pt, color_opt, size_opt, qty) = input.clone();
            let pname = if pt == "life_saver" {
                "Life Saver".into()
            } else if pt == "stripes" {
                format!(
                    "{} Stripes",
                    if color_opt.as_deref() == Some("white") {
                        "White"
                    } else {
                        "Yellow"
                    }
                )
            } else {
                let cn = if color_opt.as_deref() == Some("white_red") {
                    "White / Red"
                } else {
                    "Yellow / Red"
                };
                format!("{} Chevron ({})", cn, size_opt.as_deref().unwrap_or("1x1"))
            };
            async move {
                api::add_product(&crate::api::NewProduct {
                    name: pname,
                    product_type: pt,
                    color: color_opt,
                    size: size_opt,
                    selling_price: 0.0,
                    stock: qty,
                })
                .await
            }
        },
    );
    let delete_action: Action<i64, Result<SuccessResponse, String>, SyncStorage> =
        Action::new_unsync(move |id: &i64| {
            let id = *id;
            async move { api::delete_product(id).await }
        });
    let stock_action: Action<(i64, i64, i64), Result<SuccessResponse, String>, SyncStorage> =
        Action::new_unsync(move |(pid, add_qty, _current): &(i64, i64, i64)| {
            let (pid, add_qty) = (*pid, *add_qty);
            async move { api::adjust_product_stock(pid, add_qty).await }
        });

    let add_ver = add_action.version();
    create_effect(move |_| {
        let _ = add_ver.get();
        if let Some(result) = add_action.value().get() {
            match result {
                Ok(_) => {
                    set_show_add.set(false);
                    set_add_qty.set(0);
                    set_add_error.set(None);
                    refetch();
                }
                Err(e) => {
                    set_add_error.set(Some(e));
                }
            }
        }
    });
    let del_ver = delete_action.version();
    create_effect(move |_| {
        let _ = del_ver.get();
        if let Some(Ok(_)) = delete_action.value().get() {
            set_del_id.set(None);
            refetch();
        }
    });
    let stock_ver = stock_action.version();
    create_effect(move |_| {
        let _ = stock_ver.get();
        if let Some(Ok(_)) = stock_action.value().get() {
            set_show_stock.set(false);
            set_stock_qty.set(0);
            refetch();
        }
    });

    let (add_trigger, set_add_trigger) = signal(false);
    let add_payload = store_value((String::new(), String::new(), String::new(), 0i64));
    create_effect(move |_| {
        if add_trigger.get() {
            let (pt, col, sz, qty) = add_payload.get_value();
            let color_opt = if pt == "life_saver" {
                None
            } else {
                Some(col.clone())
            };
            let size_opt = if pt == "chevron" {
                Some(sz.clone())
            } else {
                None
            };
            add_action.dispatch((pt, color_opt, size_opt, qty));
            set_add_trigger.set(false);
        }
    });

    let (del_trigger, set_del_trigger) = signal(None::<i64>);
    create_effect(move |_| {
        if let Some(id) = del_trigger.get() {
            delete_action.dispatch(id);
            set_del_trigger.set(None);
        }
    });

    let (stock_trigger, set_stock_trigger) = signal(false);
    let stock_payload = store_value((0i64, 0i64, 0i64));
    create_effect(move |_| {
        if stock_trigger.get() {
            stock_action.dispatch(stock_payload.get_value());
            set_stock_trigger.set(false);
        }
    });

    let (query, set_query) = signal(String::new());

    let filtered = move || {
        let q = query.get().trim().to_lowercase();
        let items = products.get();
        if q.is_empty() {
            return items;
        }
        items
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.product_type.to_lowercase().contains(&q)
                    || p.color.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || p.size.as_deref().unwrap_or("").to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    };

    let cls = move |base: &str, active: bool| {
        if active {
            format!("{} border-2 border-gray-900 bg-gray-50 rounded", base)
        } else {
            format!(
                "{} border border-gray-200 bg-white hover:border-gray-300 rounded",
                base
            )
        }
    };
    let tx = move |active: bool| {
        if active {
            "font-medium text-gray-900"
        } else {
            "font-medium text-gray-500"
        }
    };
    let sx = move |active: bool| {
        if active {
            "text-xs text-gray-500"
        } else {
            "text-xs text-gray-400"
        }
    };

    let add_pending = add_action.pending();
    let del_pending = delete_action.pending();
    let stock_pending = stock_action.pending();

    view! {
        <Show when=move || !loading.get() fallback=|| view! {
            <div class="dash">
                <PageLoading message="Loading products..."/>
            </div>
        }>
        <div class="dash">
            <div class="dash-table-head">
                <div>
                    <h2 class="dash-section-title">"Inventory"</h2>
                    <p class="prod-sub">"Life Savers, Chevrons, and Stripes"</p>
                </div>
                <div class="dash-table-actions">
                    <label class="dash-search">
                        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-4.35-4.35M11 18a7 7 0 100-14 7 7 0 000 14z"/>
                        </svg>
                        <input
                            type="search"
                            placeholder="Search..."
                            prop:value=move || query.get()
                            on:input=move |ev| set_query.set(event_target_value(&ev))
                            aria-label="Search products"
                        />
                    </label>
                    <button type="button" class="dash-btn-primary" on:click=move |_| set_show_add.set(true)>
                        <span aria-hidden="true">"+"</span>
                        " Add Product"
                    </button>
                </div>
            </div>

            <div class="prod-metrics dash-card">
                <div class="prod-metric">
                    <p class="dash-metric-label">"Total units"</p>
                    <p class="dash-metric-value">{move || total_stock_units.get()}</p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Life Savers"</p>
                    <p class="dash-metric-value">{move || life_saver_stock.get()}</p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Chevrons"</p>
                    <p class="dash-metric-value">{move || chevron_stock.get()}</p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Stripes"</p>
                    <p class="dash-metric-value">{move || stripes_stock.get()}</p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Stock value"</p>
                    <p class="dash-metric-value dash-metric-value--sm">
                        {move || format!("KSh {:.0}", stock_value.get())}
                    </p>
                </div>
            </div>

            <div class="dash-card dash-table-card">
                <table class="dash-table">
                    <thead>
                        <tr>
                            <th>"Type"</th>
                            <th>"Color"</th>
                            <th>"Size"</th>
                            <th>"Stock"</th>
                            <th>"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let all = filtered();
                            if all.is_empty() {
                                return view! {
                                    <tr>
                                        <td colspan="5" class="dash-table-empty">
                                            {if query.get().trim().is_empty() {
                                                "No products yet — add your first product."
                                            } else {
                                                "No products match your search."
                                            }}
                                        </td>
                                    </tr>
                                }.into_any();
                            }
                            all.into_iter().map(|p| {
                                let is_ls = p.product_type == "life_saver";
                                let is_ch = p.product_type == "chevron";
                                let type_cls = if is_ls {
                                    "dash-status is-ok"
                                } else if is_ch {
                                    "dash-status is-warn"
                                } else {
                                    "dash-status is-info"
                                };
                                let type_label = if is_ls {
                                    "Lifesaver"
                                } else if is_ch {
                                    "Chevron"
                                } else {
                                    "Stripes"
                                };
                                let color_cell = if let Some(ref col) = p.color {
                                    let swatch = if is_ch {
                                        let (c1, c2) = if col == "white_red" {
                                            ("#ffffff", "#ef4444")
                                        } else {
                                            ("#eab308", "#ef4444")
                                        };
                                        format!("background:linear-gradient(135deg,{} 50%,{} 50%)", c1, c2)
                                    } else {
                                        let b = if col == "white" { ";border:1px solid #d1d5db" } else { "" };
                                        format!(
                                            "background-color:{}{}",
                                            if col == "white" { "#ffffff" } else { "#eab308" },
                                            b
                                        )
                                    };
                                    let label: String = match col.as_str() {
                                        "white_red" => "White / Red",
                                        "yellow_red" => "Yellow / Red",
                                        "white" => "White",
                                        "yellow" => "Yellow",
                                        _ => col.as_str(),
                                    }
                                    .into();
                                    view! {
                                        <div class="prod-color-cell">
                                            <span class="prod-swatch" style=swatch></span>
                                            <span class="dash-td-muted">{label}</span>
                                        </div>
                                    }.into_any()
                                } else if is_ls {
                                    view! {
                                        <div class="prod-color-cell">
                                            <svg viewBox="0 0 24 24" class="prod-ls-icon" aria-hidden="true">
                                                <polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/>
                                                <text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">"!"</text>
                                            </svg>
                                            <span class="dash-td-muted">"Lifesaver"</span>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span class="dash-td-muted">"—"</span> }.into_any()
                                };
                                let size = p.size.clone().unwrap_or_else(|| "—".into());
                                let pname = p.name.clone();
                                let pid = p.id;
                                let pstock = p.stock;
                                view! {
                                    <tr>
                                        <td><span class=type_cls>{type_label}</span></td>
                                        <td>{color_cell}</td>
                                        <td class="dash-td-muted">{size}</td>
                                        <td class="dash-td-strong tnum">{format!("{} units", pstock)}</td>
                                        <td>
                                            <div class="prod-actions">
                                                <button
                                                    type="button"
                                                    class="prod-btn-add"
                                                    on:click=move |_| {
                                                        set_stock_pid.set(Some(pid));
                                                        set_stock_pname.set(pname.clone());
                                                        set_stock_pstock.set(pstock);
                                                        set_stock_qty.set(0);
                                                        set_show_stock.set(true);
                                                    }
                                                >"+ Add"</button>
                                                <button
                                                    type="button"
                                                    class="prod-btn-icon is-danger"
                                                    aria-label="Delete product"
                                                    on:click=move |_| set_del_id.set(Some(pid))
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
                    let total = total_count.get() as usize;
                    if total == 0 && query.get().trim().is_empty() {
                        return ().into_any();
                    }
                    let total_pages = (total as u32).div_ceil(per_page).max(1);
                    let cur = page.get().min(total_pages.max(1));
                    let start = ((cur - 1) * per_page) as usize;
                    let end = (start + per_page as usize).min(total);
                    let showing = filtered().len();
                    let count_label = if query.get().trim().is_empty() {
                        if total == 0 {
                            "No products".to_string()
                        } else {
                            format!("Showing {}–{} of {}", start + 1, end, total)
                        }
                    } else {
                        format!("{} match{}", showing, if showing == 1 { "" } else { "es" })
                    };
                    let page_label = format!("Page {} of {}", cur, total_pages);
                    // Avoid `>=` / `>` inside view! attrs — the macro treats `>` as end of tag
                    let prev_disabled = cur <= 1;
                    let next_disabled = cur >= total_pages;
                    let go_prev = move |_| {
                        set_page.update(|p| *p = p.saturating_sub(1).max(1));
                    };
                    let go_next = move |_| {
                        set_page.update(move |p| {
                            let next = *p + 1;
                            *p = if next > total_pages { total_pages } else { next };
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
                                >
                                    "Previous"
                                </button>
                                <span class="prod-pager-meta">{page_label}</span>
                                <button
                                    type="button"
                                    class="prod-pager-btn"
                                    prop:disabled=next_disabled
                                    on:click=go_next
                                >
                                    "Next"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>
        </div>

        <Show when=move || show_add.get()>
            <div class="modal-overlay open" on:click=move |e| {
                if e.target() == e.current_target() && !add_pending.get() {
                    set_show_add.set(false);
                }
            }>
                <div class="modal-container" style="max-width:520px">
                    <div class="modal-header">
                        <h3 class="modal-title">"Add New Product"</h3>
                        <button
                            type="button"
                            class="modal-close-btn"
                            prop:disabled=move || add_pending.get()
                            on:click=move |_| {
                                if !add_pending.get() {
                                    set_show_add.set(false);
                                }
                            }
                        >
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <div class="space-y-5">
                            <div>
                                <label>"Product Type"</label>
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
                                    <label>"Color"</label>
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
                                    <label>"Size"</label>
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
                                <label>"Stock Quantity"</label>
                                <input type="number" min="1" required placeholder="0" on:input=move |e| set_add_qty.set(event_target_value(&e).parse().unwrap_or(0)) />
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn-secondary"
                            prop:disabled=move || add_pending.get()
                            on:click=move |_| {
                                if !add_pending.get() {
                                    set_show_add.set(false);
                                    set_add_qty.set(0);
                                    set_sel_type.set("life_saver".into());
                                    set_sel_color.set("white_red".into());
                                    set_sel_size.set("1x1".into());
                                }
                            }
                        >"Cancel"</button>
                        <button
                            type="button"
                            class="btn-primary"
                            on:click=move |_| {
                                if add_pending.get() {
                                    return;
                                }
                                add_payload.set_value((sel_type.get(), sel_color.get(), sel_size.get(), add_qty.get()));
                                set_add_trigger.set(true);
                            }
                            prop:disabled=move || add_pending.get()
                        >{move || if add_pending.get() { "Saving..." } else { "Save" }}</button>
                    </div>
                </div>
            </div>
        </Show>

        <Show when=move || show_stock.get()>
            <div class="modal-overlay open" on:click=move |e| {
                if e.target() == e.current_target() && !stock_pending.get() {
                    set_show_stock.set(false);
                }
            }>
                <div class="modal-container" style="max-width:500px">
                    <div class="modal-header">
                        <h3 class="modal-title">"Add Product Stock"</h3>
                        <button
                            type="button"
                            class="modal-close-btn"
                            prop:disabled=move || stock_pending.get()
                            on:click=move |_| {
                                if !stock_pending.get() {
                                    set_show_stock.set(false);
                                }
                            }
                        >
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <div class="bg-gray-50 p-4 mb-4">
                            <p class="dash-metric-label">"Product"</p>
                            <p class="dash-td-strong">{move || stock_pname.get()}</p>
                            <p class="prod-sub mt-1">"Current stock: " {move || stock_pstock.get()} " units"</p>
                        </div>
                        <div>
                            <label>"Quantity to Add *"</label>
                            <input type="number" min="1" placeholder="Enter units to add" autofocus on:input=move |e| set_stock_qty.set(event_target_value(&e).parse().unwrap_or(0)) />
                        </div>
                        <div class="bg-neutral-50 p-3 mt-4">
                            <p class="text-sm text-gray-700"><span class="font-medium">"New Total Stock: "</span><span>{move || stock_pstock.get() + stock_qty.get()}</span> " units"</p>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn-secondary"
                            prop:disabled=move || stock_pending.get()
                            on:click=move |_| {
                                if !stock_pending.get() {
                                    set_show_stock.set(false);
                                }
                            }
                        >"Cancel"</button>
                        <button
                            type="button"
                            class="btn-primary"
                            on:click=move |_| {
                                if stock_pending.get() {
                                    return;
                                }
                                if let (Some(pid), qty, cur) = (stock_pid.get(), stock_qty.get(), stock_pstock.get()) {
                                    if qty > 0 {
                                        stock_payload.set_value((pid, qty, cur));
                                        set_stock_trigger.set(true);
                                    }
                                }
                            }
                            prop:disabled=move || stock_pending.get()
                        >{move || if stock_pending.get() { "Adding..." } else { "Add Stock" }}</button>
                    </div>
                </div>
            </div>
        </Show>

        <Show when=move || del_id.get().is_some()>
            <div class="modal-overlay open" on:click=move |e| {
                if e.target() == e.current_target() && !del_pending.get() {
                    set_del_id.set(None);
                }
            }>
                <div class="modal-container modal-sm">
                    <div class="modal-header">
                        <h3 class="modal-title">"Delete Product?"</h3>
                        <button
                            type="button"
                            class="modal-close-btn"
                            prop:disabled=move || del_pending.get()
                            on:click=move |_| {
                                if !del_pending.get() {
                                    set_del_id.set(None);
                                }
                            }
                        >
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <p class="modal-msg">"Are you sure you want to delete this product? This action cannot be undone."</p>
                    </div>
                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn-secondary"
                            prop:disabled=move || del_pending.get()
                            on:click=move |_| {
                                if !del_pending.get() {
                                    set_del_id.set(None);
                                }
                            }
                        >"Cancel"</button>
                        <button
                            type="button"
                            class="btn-danger"
                            on:click=move |_| {
                                if del_pending.get() {
                                    return;
                                }
                                if let Some(id) = del_id.get() {
                                    set_del_trigger.set(Some(id));
                                }
                            }
                            prop:disabled=move || del_pending.get()
                        >{move || if del_pending.get() { "Deleting..." } else { "Delete" }}</button>
                    </div>
                </div>
            </div>
        </Show>
        </Show>
    }
}
