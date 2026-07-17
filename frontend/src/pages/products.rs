use crate::api::{self, ProductsPageQuery, SuccessResponse};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;

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

    let total = move || total_stock_units.get();
    let ls_s = move || life_saver_stock.get();
    let ch_s = move || chevron_stock.get();
    let st_s = move || stripes_stock.get();
    let sv = move || stock_value.get();

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
                            </td></tr> }.into_any();
                        }
                        all.into_iter().map(|p| {
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
                                let label: String = match col.as_str() {
                                    "white_red" => "White / Red",
                                    "yellow_red" => "Yellow / Red",
                                    "white" => "White",
                                    "yellow" => "Yellow",
                                    _ => col.as_str(),
                                }.into();
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
                                        <span class="text-sm text-[#525252]">"Lifesaver"</span>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span class="text-sm text-gray-400">"-"</span> }.into_any()
                            };
                            view! {
                                <tr class="border-b border-[#F0F0F0] hover:bg-[#F5F5F5] transition-all duration-100">
                                    <td class="px-4 py-[14px] text-sm">
                                        <span class=move || format!("inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium {}", badged)>
                                            {if p.product_type == "life_saver" { "Lifesaver" } else if p.product_type == "chevron" { "Chevron" } else { "Stripes" }}
                                        </span>
                                    </td>
                                    <td class="px-4 py-[14px] text-sm">{color_cell}</td>
                                    <td class="px-4 py-[14px] text-sm text-[#525252] font-medium">{p.size.unwrap_or_else(|| "-".into())}</td>
                                    <td class="px-6 py-4">
                                        <span class="text-sm font-medium text-[#0A0A0A]">{p.stock} " units"</span>
                                    </td>
                                    <td class="px-4 py-4">
                                        <div class="flex items-center gap-2">
                                            <button on:click=move |_| { set_stock_pid.set(Some(p.id)); set_stock_pname.set(p.name.clone()); set_stock_pstock.set(p.stock); set_stock_qty.set(0); set_show_stock.set(true); } class="px-3 py-1 text-xs font-medium bg-black text-white rounded-md hover:bg-gray-800 transition-colors">"+ Add"</button>
                                            <button on:click=move |_| set_del_id.set(Some(p.id)) class="text-gray-400 hover:text-red-600 transition-colors">
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
                    if all.is_empty() {
                        return ().into_any();
                    }
                    let total = total_count.get() as usize;
                    let tp = (total as u32).div_ceil(per_page).max(1);
                    let cur = page.get().min(tp.max(1));
                    let start = ((cur - 1) * per_page) as usize;
                    let end = (start + per_page as usize).min(total);
                    view! {
                        <div class="flex items-center justify-between px-5 py-3 bg-gray-50 border-t border-gray-200 text-sm">
                            <div class="text-gray-600">"Showing " <span class="font-medium">{start + 1}</span> " to " <span class="font-medium">{end}</span> " of " <span class="font-medium">{total}</span> " products"</div>
                            <div class="flex gap-2 items-center">
                                <button on:click=move |_| set_page.set(((page.get() as i32 - 1).max(1)) as u32) class={if cur <= 1 { "px-3 py-1 text-sm font-medium rounded bg-gray-200 text-gray-400 cursor-not-allowed" } else { "px-3 py-1 text-sm font-medium rounded bg-black text-white hover:bg-gray-800" }} disabled={move || cur <= 1}>"Previous"</button>
                                <span class="px-3 py-1 text-sm font-medium text-gray-700">"Page " {cur} " of " {tp}</span>
                                <button on:click=move |_| set_page.set((page.get() + 1).min(tp)) class={if cur >= tp { "px-3 py-1 text-sm font-medium rounded bg-gray-200 text-gray-400 cursor-not-allowed" } else { "px-3 py-1 text-sm font-medium rounded bg-black text-white hover:bg-gray-800" }} disabled={move || cur >= tp}>"Next"</button>
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
                            <p class="text-sm text-gray-700"><span class="font-medium">"New Total Stock: "</span><span>{move || stock_pstock.get() + stock_qty.get()}</span> " units"</p>
                        </div>
                    </div>
                    <div class="flex justify-end gap-2.5 px-6 py-4 bg-[#F5F5F5] border-t border-[#F0F0F0]">
                        <button on:click=move |_| set_show_stock.set(false) class="px-4 py-2 text-sm font-medium bg-white text-[#0A0A0A] border border-[#E5E5E5] hover:bg-[#F5F5F5]">"Cancel"</button>
                        <button
                            on:click=move |_| {
                                if let (Some(pid), qty, cur) = (stock_pid.get(), stock_qty.get(), stock_pstock.get()) {
                                    if qty > 0 {
                                        stock_payload.set_value((pid, qty, cur));
                                        set_stock_trigger.set(true);
                                    }
                                }
                            }
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
                            on:click=move |_| {
                                if let Some(id) = del_id.get() {
                                    set_del_trigger.set(Some(id));
                                }
                            }
                            disabled=move || del_pending.get()
                            class="px-4 py-2 text-sm font-medium bg-red-600 text-white border-none hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed"
                        >{move || if del_pending.get() { "Deleting..." } else { "Delete" }}</button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
