use super::stock_page::{get_hex as stock_color_hex, reflective_swatch_style};
use crate::api::{self, NewDebt, NewSale, Product, Sale, SalesPageQuery, StockItem};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;
use log::error;

#[path = "../components/dropdown.rs"]
mod dropdown_comp;
use dropdown_comp::{CustomDropdown, DropdownItem};
#[path = "../components/calendar.rs"]
mod calendar_comp;
use calendar_comp::MiniCalendar;
#[path = "../components/receipt.rs"]
mod receipt_comp;
use receipt_comp::{open_multi_sale_receipt, open_sale_receipt, ReceiptModal};
#[path = "../components/loading.rs"]
mod loading_comp;
use loading_comp::PageLoading;

#[derive(Clone, Copy, PartialEq)]
enum SaleTab {
    Stock,
    Product,
    Service,
}

fn format_sale_timestamp(ts: &Option<String>) -> String {
    ts.as_ref()
        .and_then(|t| {
            chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.3f")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S"))
                .ok()
        })
        .map(|dt| {
            let today = chrono::Local::now().date_naive();
            let sale_day = dt.date();
            let time = dt.format("%I:%M %p").to_string();

            if sale_day == today {
                format!("Today {}", time)
            } else if sale_day == today.pred_opt().unwrap_or(today) {
                format!("Yesterday {}", time)
            } else {
                dt.format("%d/%m/%Y %I:%M %p").to_string()
            }
        })
        .or_else(|| ts.clone())
        .unwrap_or_else(|| "-".to_string())
}

fn product_type_label(product_type: Option<&str>) -> &'static str {
    match product_type.unwrap_or("") {
        "life_saver" => "Lifesaver",
        "chevron" => "Chevron",
        "stripes" => "Stripes",
        _ => "Product",
    }
}

fn product_color_label(color: &str) -> String {
    match color {
        "white_red" => "White / Red".to_string(),
        "yellow_red" => "Yellow / Red".to_string(),
        "white" => "White".to_string(),
        "yellow" => "Yellow".to_string(),
        _ => color.to_string(),
    }
}

fn render_sale_item_cell(
    sale: &Sale,
    product: Option<&Product>,
    stock_item: Option<&StockItem>,
) -> AnyView {
    match sale.r#type.as_str() {
        "product" => {
            let name = sale
                .product_name
                .clone()
                .unwrap_or_else(|| "Product".to_string());
            let resolved_type = product
                .map(|p| p.product_type.as_str())
                .or(sale.product_type.as_deref());
            let resolved_color = product.and_then(|p| p.color.as_deref());
            let subtitle = resolved_color
                .map(|color| {
                    format!(
                        "{} • {}",
                        product_type_label(resolved_type),
                        product_color_label(color)
                    )
                })
                .unwrap_or_else(|| product_type_label(resolved_type).to_string());

            let preview = match resolved_type.unwrap_or("") {
                "life_saver" => view! {
                    <svg viewBox="0 0 24 24" class="w-8 h-8 flex-shrink-0">
                        <polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/>
                        <text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">!</text>
                    </svg>
                }
                .into_any(),
                "chevron" => {
                    let style = match resolved_color {
                        Some("white_red") => "background:linear-gradient(135deg,#ffffff 50%,#ef4444 50%)",
                        Some("yellow_red") => "background:linear-gradient(135deg,#eab308 50%,#ef4444 50%)",
                        _ => "background:linear-gradient(135deg,#ffffff 50%,#ef4444 50%)",
                    };
                    view! { <div class="w-8 h-8 rounded-sm shadow-sm flex-shrink-0" style=style></div> }
                        .into_any()
                }
                "stripes" => {
                    let (style, border) = match resolved_color {
                        Some("white") => ("background:#ffffff", "border border-gray-200"),
                        Some("yellow") => ("background:#eab308", ""),
                        _ => ("background:#ffffff", "border border-gray-200"),
                    };
                    let class = format!("w-8 h-8 rounded-sm shadow-sm flex-shrink-0 {}", border);
                    view! { <div class=class style=style></div> }.into_any()
                }
                _ => view! {
                    <div class="w-8 h-8 rounded-sm bg-gray-200 border border-gray-300 flex-shrink-0"></div>
                }
                .into_any(),
            };

            view! {
                <div class="flex items-center gap-3">
                    {preview}
                    <div class="min-w-0">
                        <p class="text-sm font-medium text-gray-900 truncate">{name}</p>
                        <p class="text-xs text-gray-500 truncate">{subtitle}</p>
                    </div>
                </div>
            }
            .into_any()
        }
        "stock" => {
            let is_reflective = stock_item
                .map(|item| item.sticker_type == "reflective")
                .or_else(|| sale.sticker_type.as_deref().map(|t| t == "reflective"))
                .unwrap_or(false);

            let title = stock_item
                .map(|item| {
                    format!(
                        "{} {}",
                        item.color,
                        if item.sticker_type == "reflective" {
                            "Reflective"
                        } else {
                            "Colored"
                        }
                    )
                })
                .or_else(|| sale.product_name.clone())
                .unwrap_or_else(|| "Stock Item".to_string());

            let subtitle = stock_item
                .map(|item| {
                    format!(
                        "{}in • {}",
                        item.size,
                        if item.sticker_type == "reflective" {
                            "Reflective film"
                        } else {
                            "Colored film"
                        }
                    )
                })
                .unwrap_or_else(|| {
                    if is_reflective {
                        "Reflective film".to_string()
                    } else {
                        "Colored film".to_string()
                    }
                });

            let color_name = stock_item
                .map(|item| item.color.clone())
                .or_else(|| title.split_whitespace().next().map(|part| part.to_string()))
                .unwrap_or_else(|| "gray".to_string());
            let color_hex = stock_color_hex(&color_name);
            let swatch_style = if is_reflective {
                reflective_swatch_style(color_hex)
            } else {
                format!("background-color: {};", color_hex)
            };

            view! {
                <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-lg border shadow-sm flex-shrink-0" style=swatch_style></div>
                    <div class="min-w-0">
                        <p class="text-sm font-medium text-gray-900 truncate">{title}</p>
                        <p class="text-xs text-gray-500 truncate">{subtitle}</p>
                    </div>
                </div>
            }
            .into_any()
        }
        "service" => {
            let name = sale
                .product_name
                .clone()
                .unwrap_or_else(|| "Service".to_string());
            view! {
                <div class="flex items-center gap-2">
                    <span class="status-badge bg-neutral-800 text-white">"Service"</span>
                    <span class="text-sm font-medium text-gray-900">{name}</span>
                </div>
            }
            .into_any()
        }
        _ => {
            let name = sale
                .product_name
                .clone()
                .unwrap_or_else(|| sale.r#type.clone());
            view! { <span class="text-sm font-medium text-gray-900">{name}</span> }.into_any()
        }
    }
}

#[component]
pub fn SalesPage(show_revenue_stats: bool) -> impl IntoView {
    let (sales, set_sales) = signal(Vec::<Sale>::new());
    let (products, set_products) = signal(Vec::<Product>::new());
    let (stock, set_stock) = signal(Vec::<StockItem>::new());
    let (today_total, set_today_total) = signal(0.0f64);
    let (all_revenue, set_all_revenue) = signal(0.0f64);
    let (product_sales_count, set_product_sales_count) = signal(0i64);
    let (total_count, set_total_count) = signal(0u32);
    let (show_modal, set_show_modal) = signal(false);
    let (active_tab, set_active_tab) = signal(SaleTab::Stock);
    let (current_page, set_current_page) = signal(1u32);
    let (selected, set_selected) = signal(0u32);
    let (selected_sales, set_selected_sales) = signal(Vec::<i64>::new());
    let (search, set_search) = signal(String::new());
    let (sort_by, set_sort_by) = signal("newest".to_string());

    // Convert-to-debt modal state
    let (show_convert, set_show_convert) = signal(None::<Sale>);
    let (convert_customer, set_convert_customer) = signal(String::new());
    let (convert_phone, set_convert_phone) = signal(String::new());
    let (convert_paid, set_convert_paid) = signal(String::new());
    let (convert_due_date, set_convert_due_date) = signal(String::new());
    let (_convert_due_label, set_convert_due_label) = signal(String::new());

    // Receipt / Print state
    let (show_receipt, set_show_receipt) = signal(false);
    let (receipt_html, set_receipt_html) = signal(String::new());
    let (receipt_title, set_receipt_title) = signal(String::new());

    // Delete confirmation
    let (del_id, set_del_id) = signal(None::<i64>);
    let (del_label, set_del_label) = signal(String::new());
    let (deleting_sale, set_deleting_sale) = signal(false);

    // Stock tab state
    let (stock_id, set_stock_id) = signal(None::<i64>);
    let (sale_unit, set_sale_unit) = signal("metres".to_string());
    let (stock_qty, set_stock_qty) = signal(String::new());
    let (stock_price, set_stock_price) = signal(String::new());
    let (stock_payment, set_stock_payment) = signal("cash".to_string());
    let (stock_cust, set_stock_cust) = signal("Walk-in".to_string());
    let (stock_submit_error, set_stock_submit_error) = signal(None::<String>);
    let (stock_submitting, set_stock_submitting) = signal(false);

    // Product tab state
    let (product_id, set_product_id) = signal(None::<i64>);
    let (product_qty, set_product_qty) = signal("1".to_string());
    let (product_price, set_product_price) = signal(0.0f64);
    let (product_payment, set_product_payment) = signal("cash".to_string());
    let (product_cust, set_product_cust) = signal("Walk-in".to_string());
    let (product_submit_error, set_product_submit_error) = signal(None::<String>);
    let (product_submitting, set_product_submitting) = signal(false);
    let (loading, set_loading) = signal(true);

    // Service tab state
    let (svc_name, set_svc_name) = signal(String::new());
    let (svc_price, set_svc_price) = signal(String::new());
    let (svc_payment, set_svc_payment) = signal("cash".to_string());
    let (svc_cust, set_svc_cust) = signal("Walk-in".to_string());
    let (svc_submitting, set_svc_submitting) = signal(false);
    let (convert_submitting, set_convert_submitting) = signal(false);

    let items_per_page = 10u32;

    // Full reload: sales list + product/stock catalogs (for sale form dropdowns)
    let reload = {
        let ss = set_sales;
        let sp = set_products;
        let sk = set_stock;
        let tt = set_today_total;
        let ar = set_all_revenue;
        let psc = set_product_sales_count;
        let tc = set_total_count;
        let sl = set_loading;
        let search_r = search;
        let sort_r = sort_by;
        let page_r = current_page;
        move || {
            leptos::task::spawn_local({
                let ss = ss;
                let sp = sp;
                let sk = sk;
                let tt = tt;
                let ar = ar;
                let psc = psc;
                let tc = tc;
                let sl = sl;
                let query = SalesPageQuery {
                    search: Some(search_r.get()),
                    sort_by: Some(sort_r.get()),
                    page: Some(page_r.get()),
                    per_page: Some(items_per_page),
                };
                async move {
                    if let Ok(page) = api::get_sales_page(&query).await {
                        tt.set(page.today_total);
                        ar.set(page.all_revenue);
                        psc.set(page.product_sales_count);
                        tc.set(page.total_count as u32);
                        ss.set(page.items);
                    }
                    if let Ok(p) = api::get_all_products().await {
                        sp.set(p);
                    }
                    if let Ok(sk_) = api::get_all_stock().await {
                        sk.set(sk_);
                    }
                    sl.set(false);
                }
            })
        }
    };

    // Lightweight poll: sales list only (avoids re-fetching catalogs every tick)
    let reload_list = {
        let ss = set_sales;
        let tt = set_today_total;
        let ar = set_all_revenue;
        let psc = set_product_sales_count;
        let tc = set_total_count;
        let search_r = search;
        let sort_r = sort_by;
        let page_r = current_page;
        move || {
            leptos::task::spawn_local({
                let ss = ss;
                let tt = tt;
                let ar = ar;
                let psc = psc;
                let tc = tc;
                let query = SalesPageQuery {
                    search: Some(search_r.get()),
                    sort_by: Some(sort_r.get()),
                    page: Some(page_r.get()),
                    per_page: Some(items_per_page),
                };
                async move {
                    if let Ok(page) = api::get_sales_page(&query).await {
                        tt.set(page.today_total);
                        ar.set(page.all_revenue);
                        psc.set(page.product_sales_count);
                        tc.set(page.total_count as u32);
                        ss.set(page.items);
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

    // Dropdown item signals
    let stock_dropdown_items = Signal::derive(move || {
        stock
            .get()
            .into_iter()
            .map(|s| {
                let remaining = s.total_metres
                    - if s.metres_used.is_nan() {
                        0.0
                    } else {
                        s.metres_used
                    };
                let color_hex = stock_color_hex(&s.color);
                DropdownItem::new(
                    &s.id.to_string(),
                    &format!(
                        "{} - {}in ({}) - {}m",
                        s.color,
                        s.size,
                        if s.sticker_type == "reflective" {
                            "Reflective"
                        } else {
                            "Colored"
                        },
                        remaining as u64
                    ),
                )
                .with_stock_preview(&s.color, Some(color_hex), &s.sticker_type)
                .with_badge(&format!("{} rolls", (remaining / 50.0).floor() as u64))
            })
            .collect::<Vec<_>>()
    });

    let sale_type_items = Signal::derive(move || {
        vec![
            DropdownItem::new("metres", "Metres").with_badge("per metre"),
            DropdownItem::new("rolls", "Whole Rolls").with_badge("50m each"),
        ]
    });
    let payment_method_items = Signal::derive(move || {
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
    let product_dropdown_items = Signal::derive(move || {
        products
            .get()
            .into_iter()
            .filter(|p| p.stock > 0)
            .map(|p| {
                let color = p.color.clone().unwrap_or_default();
                DropdownItem::new(&p.id.to_string(), &p.name)
                    .with_badge(&format!("{} in stock", p.stock))
                    .with_product_preview(
                        &p.product_type,
                        if color.trim().is_empty() {
                            None
                        } else {
                            Some(color.as_str())
                        },
                    )
            })
            .collect::<Vec<_>>()
    });

    let reset_stock_tab = move || {
        set_stock_id.set(None);
        set_sale_unit.set("metres".into());
        set_stock_qty.set(String::new());
        set_stock_price.set(String::new());
        set_stock_payment.set("cash".into());
        set_stock_cust.set("Walk-in".into());
        set_stock_submit_error.set(None);
        set_stock_submitting.set(false);
    };
    let reset_product_tab = move || {
        set_product_id.set(None);
        set_product_qty.set("1".into());
        set_product_price.set(0.0);
        set_product_payment.set("cash".into());
        set_product_cust.set("Walk-in".into());
        set_product_submit_error.set(None);
        set_product_submitting.set(false);
    };
    let reset_service_tab = move || {
        set_svc_name.set(String::new());
        set_svc_price.set(String::new());
        set_svc_payment.set("cash".into());
        set_svc_cust.set("Walk-in".into());
        set_svc_submitting.set(false);
    };

    let sale_busy =
        move || stock_submitting.get() || product_submitting.get() || svc_submitting.get();

    let close_modal = move || {
        if sale_busy() {
            return;
        }
        set_show_modal.set(false);
        reset_stock_tab();
        reset_product_tab();
        reset_service_tab();
    };

    // Stock sale submit
    let submit_stock = {
        let l = reload;
        move |_| {
            if stock_submitting.get() {
                return;
            }

            let sid = stock_id.get();
            let qty_val: f64 = stock_qty.get().parse().unwrap_or(0.0);
            let price_val: f64 = stock_price.get().parse().unwrap_or(0.0);
            if sid.is_none() {
                set_stock_submit_error.set(Some("Choose a stock item first.".into()));
                return;
            }
            if qty_val <= 0.0 {
                set_stock_submit_error.set(Some("Enter a valid quantity to sell.".into()));
                return;
            }
            if price_val <= 0.0 {
                set_stock_submit_error.set(Some("Enter a valid total price.".into()));
                return;
            }

            let selected_stock = match stock.get().iter().find(|s| s.id == sid.unwrap()).cloned() {
                Some(item) => item,
                None => {
                    set_stock_submit_error
                        .set(Some("The selected stock item could not be found.".into()));
                    return;
                }
            };

            let unit = sale_unit.get();
            let metres = if unit == "rolls" {
                qty_val * selected_stock.metres_per_roll
            } else {
                qty_val
            };
            let used = if selected_stock.metres_used.is_nan() {
                0.0
            } else {
                selected_stock.metres_used
            };
            let remaining = (selected_stock.total_metres - used).max(0.0);
            if metres > remaining {
                set_stock_submit_error.set(Some(format!(
                    "Only {:.1}m remains for this stock item.",
                    remaining
                )));
                return;
            }

            let sid = sid.unwrap();
            let payment_method = stock_payment.get();
            let customer_name = stock_cust.get();
            let label = format!(
                "{} {}",
                selected_stock.color,
                if selected_stock.sticker_type == "reflective" {
                    "Reflective"
                } else {
                    "Colored"
                }
            );
            let quantity_label = if unit == "rolls" {
                format!("{} rolls ({}m)", qty_val, metres)
            } else {
                format!("{}m", metres)
            };

            set_stock_submit_error.set(None);
            set_stock_submitting.set(true);
            leptos::task::spawn_local(async move {
                match api::add_sale(&NewSale {
                    r#type: "stock".into(),
                    product_id: None,
                    stock_id: Some(sid),
                    product_name: Some(format!("{} Sticker", label)),
                    product_type: None,
                    sticker_type: Some(selected_stock.sticker_type.clone()),
                    quantity: Some(quantity_label),
                    amount: price_val,
                    payment_method,
                    customer_name,
                    is_debt: 0,
                    product_quantity: None,
                    stock_metres_used: Some(metres),
                })
                .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        error!("add_sale failed: {}", e);
                        set_stock_submit_error
                            .set(Some(format!("Could not record the sale: {}", e)));
                        set_stock_submitting.set(false);
                        return;
                    }
                }

                set_show_modal.set(false);
                set_stock_id.set(None);
                set_sale_unit.set("metres".into());
                set_stock_qty.set(String::new());
                set_stock_price.set(String::new());
                set_stock_payment.set("cash".into());
                set_stock_cust.set("Walk-in".into());
                set_stock_submit_error.set(None);
                set_stock_submitting.set(false);
                l();
            });
        }
    };

    // Product sale submit
    let submit_product = {
        let l = reload;
        move |_| {
            if product_submitting.get() {
                return;
            }

            let pid = product_id.get();
            let pqty: i64 = product_qty.get().parse().unwrap_or(0);
            let price = product_price.get();
            if pid.is_none() {
                set_product_submit_error.set(Some("Choose a product first.".into()));
                return;
            }
            if pqty <= 0 {
                set_product_submit_error.set(Some("Enter a valid quantity to sell.".into()));
                return;
            }
            if price <= 0.0 {
                set_product_submit_error.set(Some("Enter a valid sale amount.".into()));
                return;
            }
            let p = products
                .get()
                .iter()
                .find(|p| p.id == pid.unwrap())
                .cloned();
            let name = p.as_ref().map(|p| p.name.clone()).unwrap_or_default();
            set_product_submit_error.set(None);
            set_product_submitting.set(true);
            leptos::task::spawn_local(async move {
                match api::add_sale(&NewSale {
                    r#type: "product".into(),
                    product_id: pid,
                    stock_id: None,
                    product_name: Some(name),
                    product_type: p.as_ref().map(|p| p.product_type.clone()),
                    sticker_type: None,
                    quantity: Some(pqty.to_string()),
                    amount: price,
                    payment_method: product_payment.get(),
                    customer_name: product_cust.get(),
                    is_debt: 0,
                    product_quantity: Some(pqty),
                    stock_metres_used: None,
                })
                .await
                {
                    Ok(_) => {
                        set_show_modal.set(false);
                        reset_product_tab();
                        set_product_submitting.set(false);
                    }
                    Err(e) => {
                        error!("add product sale failed: {}", e);
                        set_product_submit_error
                            .set(Some(format!("Could not record the sale: {}", e)));
                        set_product_submitting.set(false);
                        return;
                    }
                }
                l();
            });
        }
    };

    // Service sale submit
    let submit_service = {
        let l = reload;
        move |_| {
            if svc_submitting.get() {
                return;
            }
            let name = svc_name.get();
            let price: f64 = svc_price.get().parse().unwrap_or(0.0);
            if name.is_empty() || price <= 0.0 {
                return;
            }
            set_svc_submitting.set(true);
            leptos::task::spawn_local(async move {
                match api::add_sale(&NewSale {
                    r#type: "service".into(),
                    product_id: None,
                    stock_id: None,
                    product_name: Some(name),
                    product_type: None,
                    sticker_type: None,
                    quantity: Some("-".into()),
                    amount: price,
                    payment_method: svc_payment.get(),
                    customer_name: svc_cust.get(),
                    is_debt: 0,
                    product_quantity: None,
                    stock_metres_used: None,
                })
                .await
                {
                    Ok(_) => {
                        set_show_modal.set(false);
                        reset_service_tab();
                    }
                    Err(e) => {
                        error!("add service sale failed: {}", e);
                    }
                }
                set_svc_submitting.set(false);
                l();
            });
        }
    };

    let toggle_select = move |id: i64| {
        set_selected_sales.update(|v| {
            if let Some(p) = v.iter().position(|x| *x == id) {
                v.remove(p);
            } else {
                v.push(id);
            }
        });
        set_selected.update(|c| {
            let s = selected_sales.get();
            *c = s.len() as u32;
        });
    };
    let select_all = move |checked: bool| {
        if checked {
            let ids: Vec<i64> = sales.get().iter().map(|s| s.id).collect();
            set_selected_sales.set(ids);
            set_selected.set(sales.get().len() as u32);
        } else {
            set_selected_sales.set(Vec::new());
            set_selected.set(0);
        }
    };

    // Print selected sales receipt
    let print_selected = {
        let set_show = set_show_receipt;
        let set_html = set_receipt_html;
        let set_title = set_receipt_title;
        move |_| {
            let sel_ids = selected_sales.get();
            if sel_ids.is_empty() {
                return;
            }
            let all = sales.get();
            let selected: Vec<Sale> = all
                .into_iter()
                .filter(|s| sel_ids.contains(&s.id))
                .collect();
            open_multi_sale_receipt(&selected, set_show, set_html, set_title);
        }
    };

    let confirm_delete_sale = {
        let l = reload;
        move |_| {
            let Some(id) = del_id.get() else {
                return;
            };
            if deleting_sale.get() {
                return;
            }
            set_deleting_sale.set(true);
            leptos::task::spawn_local(async move {
                let _ = api::delete_sale(id).await;
                set_del_id.set(None);
                set_del_label.set(String::new());
                set_deleting_sale.set(false);
                // Drop from multi-select if it was checked
                set_selected_sales.update(|ids| {
                    ids.retain(|x| *x != id);
                    set_selected.set(ids.len() as u32);
                });
                l();
            });
        }
    };

    // Convert-to-debt submit
    let submit_convert = {
        let l = reload;
        move |_| {
            if convert_submitting.get() {
                return;
            }
            let sale = show_convert.get();
            if let Some(s) = sale {
                let name = convert_customer.get();
                if name.is_empty() {
                    return;
                }
                let paid: f64 = convert_paid.get().parse().unwrap_or(0.0);
                let remaining = s.amount - paid;
                if remaining <= 0.0 {
                    set_show_convert.set(None);
                    return;
                }
                let s_id = s.id;
                let phone = convert_phone.get();
                let due = convert_due_date.get();
                set_convert_submitting.set(true);
                leptos::task::spawn_local(async move {
                    let desc = {
                        let name = s
                            .product_name
                            .as_deref()
                            .filter(|n| !n.trim().is_empty())
                            .unwrap_or(match s.r#type.as_str() {
                                "stock" => "Sticker sale",
                                "product" => "Product sale",
                                "service" => "Service sale",
                                _ => "Sale",
                            });
                        let qty = s.quantity.as_deref().unwrap_or("-");
                        format!("Sale · {} × {}", name, qty)
                    };
                    let ok = api::add_debt(&NewDebt {
                        customer_name: name,
                        phone: Some(phone).filter(|p| !p.is_empty()),
                        amount: s.amount,
                        paid_amount: Some(paid),
                        remaining_amount: Some(remaining),
                        due_date: Some(due).filter(|d| !d.is_empty()),
                        description: Some(desc),
                        sale_id: Some(s_id),
                        service_transaction_id: None,
                    })
                    .await
                    .is_ok();
                    if ok {
                        let _ = api::update_sale(s_id, &serde_json::json!({"is_debt": 1})).await;
                        set_show_convert.set(None);
                    }
                    set_convert_submitting.set(false);
                    l();
                });
            }
        }
    };
    let open_convert = move |sale: Option<&Sale>| {
        if let Some(s) = sale {
            set_convert_customer.set(s.customer_name.clone());
            set_convert_phone.set(String::new());
            set_convert_paid.set("0".into());
            set_convert_due_date.set(String::new());
            set_convert_due_label.set(String::new());
            set_show_convert.set(Some(s.clone()));
        }
    };

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
    let paginated = move || sales.get();

    view! {
        <Show when=move || !loading.get() fallback=|| view! {
            <div id="page-sales" class="dash">
                <PageLoading message="Loading sales..."/>
            </div>
        }>
        <div id="page-sales" class="dash">
            <div class="dash-table-head">
                <div>
                    <h2 class="dash-section-title">"Transactions"</h2>
                    <p class="prod-sub">"Product, sticker, and service sales"</p>
                </div>
                <div class="dash-table-actions">
                    {move || if selected.get() > 0 {
                        view! {
                            <button
                                type="button"
                                id="btn-print-selected-sales"
                                class="sales-btn-secondary"
                                on:click=print_selected
                            >
                                {move || format!("Print ({})", selected.get())}
                            </button>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    <button
                        type="button"
                        id="btn-record-sale"
                        class="dash-btn-primary"
                        on:click=move |_| set_show_modal.set(true)
                    >
                        <span aria-hidden="true">"+"</span>
                        " Record Sale"
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
                    <p class="dash-metric-label">"Today's sales"</p>
                    <p class="dash-metric-value">{move || format!("KSh {:.0}", today_total.get())}</p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Transactions"</p>
                    <p class="dash-metric-value">{move || total_items()}</p>
                </div>
                <div class="prod-metric">
                    <p class="dash-metric-label">"Product sales"</p>
                    <p class="dash-metric-value">{move || product_sales_count.get()}</p>
                </div>
                {move || if show_revenue_stats {
                    view! {
                        <div class="prod-metric">
                            <p class="dash-metric-label">"All-time revenue"</p>
                            <p class="dash-metric-value dash-metric-value--sm">
                                {move || format!("KSh {:.0}", all_revenue.get())}
                            </p>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            </div>

            <div class="sales-toolbar">
                <label class="dash-search sales-search">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-4.35-4.35M11 18a7 7 0 100-14 7 7 0 000 14z"/>
                    </svg>
                    <input
                        type="search"
                        placeholder="Search item, customer, payment..."
                        prop:value=move || search.get()
                        on:input=move |e| set_search.set(event_target_value(&e))
                        aria-label="Search sales"
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
                <table class="dash-table sales-table">
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
                            <th>"Item"</th>
                            <th>"Qty"</th>
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
                                        <td colspan="8" class="dash-table-empty">"No sales recorded."</td>
                                    </tr>
                                }.into_any();
                            }
                            items.into_iter().map(|sale| {
                                let id = sale.id;
                                let tm = format_sale_timestamp(&sale.timestamp);
                                let is_sel = move || selected_sales.get().contains(&id);
                                let qty_display = sale.quantity.clone().unwrap_or_else(|| "—".into());
                                let pm_display = sale.payment_method.clone();
                                let cust_display = sale.customer_name.clone();
                                let product_match = if sale.r#type == "product" {
                                    sale.product_id.and_then(|pid| products.get().into_iter().find(|p| p.id == pid))
                                } else {
                                    None
                                };
                                let stock_match = if sale.r#type == "stock" {
                                    sale.stock_id.and_then(|sid| stock.get().into_iter().find(|s| s.id == sid))
                                } else {
                                    None
                                };
                                let item_cell = render_sale_item_cell(&sale, product_match.as_ref(), stock_match.as_ref());
                                let sale_for_debt = sale.clone();
                                let sale_for_receipt = sale.clone();
                                let amount = sale.amount;
                                let is_debt = sale.is_debt;
                                let paid_display = if is_debt == 0 {
                                    amount
                                } else if sale.amount_paid > 0.0 {
                                    sale.amount_paid
                                } else {
                                    0.0
                                };
                                let del_label_text = {
                                    let name = sale
                                        .product_name
                                        .clone()
                                        .filter(|n| !n.trim().is_empty())
                                        .unwrap_or_else(|| {
                                            if sale.r#type == "stock" {
                                                "Sticker sale".into()
                                            } else if sale.r#type == "service" {
                                                "Service sale".into()
                                            } else {
                                                "Sale".into()
                                            }
                                        });
                                    let cust = sale.customer_name.trim();
                                    if cust.is_empty() || cust.eq_ignore_ascii_case("walk-in") {
                                        format!("{} · KSh {:.0}", name, amount)
                                    } else {
                                        format!("{} · {} · KSh {:.0}", name, cust, amount)
                                    }
                                };
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
                                        <td>{item_cell}</td>
                                        <td class="dash-td-muted tnum">{qty_display}</td>
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
                                                view! { <span class="dash-status is-ok capitalize">{pm_display}</span> }.into_any()
                                            }}
                                        </td>
                                        <td class="dash-td-muted">{cust_display}</td>
                                        <td>
                                            <div class="prod-actions">
                                                <button
                                                    type="button"
                                                    class="prod-btn-icon"
                                                    title="Print receipt"
                                                    aria-label="Print receipt"
                                                    on:click=move |_| open_sale_receipt(
                                                        &sale_for_receipt,
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
                                                            on:click=move |_| open_convert(Some(&sale_for_debt))
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
                                                    aria-label="Delete sale"
                                                    on:click=move |_| {
                                                        set_del_label.set(del_label_text.clone());
                                                        set_del_id.set(Some(id));
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

        // Record Sale Modal
        {move || if show_modal.get() { view!{<div id="modal-record-sale" class="modal-overlay open">
            <div class="modal-container" style="max-width: 640px;">
                <div class="modal-header">
                    <h3 class="modal-title">"Record New Sale"</h3>
                    <button
                        type="button"
                        class="modal-close-btn"
                        prop:disabled=sale_busy
                        on:click=move |_| close_modal()
                    >
                        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                    </button>
                </div>
                <div class="modal-body">
                    <div class="modal-tabs">
                        <button type="button" on:click=move |_| set_active_tab.set(SaleTab::Stock) class=move || if active_tab.get()==SaleTab::Stock {"is-active"} else {""}>"Stock Sale"</button>
                        <button type="button" on:click=move |_| set_active_tab.set(SaleTab::Product) class=move || if active_tab.get()==SaleTab::Product {"is-active"} else {""}>"Products"</button>
                        <button type="button" on:click=move |_| set_active_tab.set(SaleTab::Service) class=move || if active_tab.get()==SaleTab::Service {"is-active"} else {""}>"Service"</button>
                    </div>

                    // Stock Sale Tab
                    {move || if active_tab.get() == SaleTab::Stock { view!{<div>
                        <div class="space-y-4">
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label>"Stock Item"</label>
                                    <CustomDropdown items=stock_dropdown_items placeholder="Choose sticker".to_string()
                                        on_select=Callback::new(move |v: String| { if let Ok(id) = v.parse() { set_stock_id.set(Some(id)); } })/>
                                </div>
                                <div>
                                    <label>"Sale Type"</label>
                                    <CustomDropdown items=sale_type_items placeholder="Metres".to_string()
                                        on_select=Callback::new(move |v: String| set_sale_unit.set(v))/>
                                </div>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label>{move || if sale_unit.get()=="rolls" {"Rolls Sold"} else {"Metres"}}</label>
                                    <input type="number" step="0.1" min="0.1" class="w-full" placeholder="0" prop:value=move || stock_qty.get() on:input=move |e| set_stock_qty.set(event_target_value(&e))/>
                                </div>
                                <div>
                                    <label>"Payment Method"</label>
                                    <CustomDropdown items=payment_method_items placeholder="Cash".to_string()
                                        on_select=Callback::new(move |v: String| set_stock_payment.set(v))/>
                                </div>
                            </div>
                            <div>
                                <label>"Total Price (KSh)"</label>
                                <input type="number" step="1" min="1" class="w-full" placeholder="0" prop:value=move || stock_price.get() on:input=move |e| set_stock_price.set(event_target_value(&e))/>
                            </div>
                            <div class="modal-total">
                                <span class="modal-total-label">"Due now"</span>
                                <span>{move || {let p:f64=stock_price.get().parse().unwrap_or(0.0); format!("KSh {:.2}", p)}}</span>
                            </div>
                            <div>
                                <label>"Customer Name (Optional)"</label>
                                <input type="text" class="w-full" placeholder="Walk-in Customer" prop:value=move || stock_cust.get() on:input=move |e| set_stock_cust.set(event_target_value(&e))/>
                            </div>
                            {move || stock_submit_error.get().map(|msg| view! {
                                <div class="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
                                    {msg}
                                </div>
                            })}
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn-secondary" prop:disabled=sale_busy on:click=move |_| close_modal()>"Cancel"</button>
                            <button type="button" class="btn-primary" on:click=submit_stock prop:disabled=move || stock_submitting.get()>
                                {move || if stock_submitting.get() { "Recording..." } else { "Record Stock Sale" }}
                            </button>
                        </div>
                    </div>}.into_any() } else { ().into_any() }}

                    // Product Sale Tab
                    {move || if active_tab.get() == SaleTab::Product { view!{<div>
                        <div class="space-y-4">
                            <div>
                                <label>"Product"</label>
                                <CustomDropdown items=product_dropdown_items placeholder="Choose product".to_string()
                                    on_select=Callback::new(move |v: String| { if let Ok(id) = v.parse() { set_product_id.set(Some(id)); } })/>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label>"Quantity"</label>
                                    <input type="number" min="1" class="w-full" placeholder="1" prop:value=move || product_qty.get() on:input=move |e| set_product_qty.set(event_target_value(&e))/>
                                </div>
                                <div>
                                    <label>"Payment Method"</label>
                                    <CustomDropdown items=payment_method_items placeholder="Cash".to_string()
                                        on_select=Callback::new(move |v: String| set_product_payment.set(v))/>
                                </div>
                            </div>
                            <div>
                                <label>"Sale Amount (KSh)"</label>
                                <input type="number" step="1" min="1" class="w-full" placeholder="0" prop:value=move || product_price.get().to_string() on:input=move |e| set_product_price.set(event_target_value(&e).parse().unwrap_or(0.0))/>
                            </div>
                            <div class="modal-total">
                                <span class="modal-total-label">"Due now"</span>
                                <span>{move || { let p = product_price.get(); format!("KSh {:.2}", p) }}</span>
                            </div>
                            <div>
                                <label>"Customer Name (Optional)"</label>
                                <input type="text" class="w-full" placeholder="Walk-in Customer" prop:value=move || product_cust.get() on:input=move |e| set_product_cust.set(event_target_value(&e))/>
                            </div>
                            {move || product_submit_error.get().map(|msg| view! {
                                <div class="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
                                    {msg}
                                </div>
                            })}
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn-secondary" prop:disabled=sale_busy on:click=move |_| close_modal()>"Cancel"</button>
                            <button type="button" class="btn-primary" on:click=submit_product prop:disabled=move || product_submitting.get()>
                                {move || if product_submitting.get() { "Recording..." } else { "Record Product Sale" }}
                            </button>
                        </div>
                    </div>}.into_any() } else { ().into_any() }}

                    // Service Sale Tab
                    {move || if active_tab.get() == SaleTab::Service { view!{<div>
                        <div class="space-y-4">
                            <div>
                                <label>"Service Name *"</label>
                                <input type="text" class="w-full" placeholder="e.g., Design, Lamination" prop:value=move || svc_name.get() on:input=move |e| set_svc_name.set(event_target_value(&e))/>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label>"Price (KSh) *"</label>
                                    <input type="number" step="1" min="1" class="w-full" placeholder="0" prop:value=move || svc_price.get() on:input=move |e| set_svc_price.set(event_target_value(&e))/>
                                </div>
                                <div>
                                    <label>"Payment Method"</label>
                                    <CustomDropdown items=payment_method_items placeholder="Cash".to_string()
                                        on_select=Callback::new(move |v: String| set_svc_payment.set(v))/>
                                </div>
                            </div>
                            <div class="modal-total">
                                <span class="modal-total-label">"Due now"</span>
                                <span>{move || {let p:f64=svc_price.get().parse().unwrap_or(0.0); format!("KSh {:.2}", p)}}</span>
                            </div>
                            <div>
                                <label>"Customer Name (Optional)"</label>
                                <input type="text" class="w-full" placeholder="Walk-in Customer" prop:value=move || svc_cust.get() on:input=move |e| set_svc_cust.set(event_target_value(&e))/>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn-secondary" prop:disabled=sale_busy on:click=move |_| close_modal()>"Cancel"</button>
                            <button type="button" class="btn-primary" on:click=submit_service prop:disabled=move || svc_submitting.get()>
                                {move || if svc_submitting.get() { "Recording..." } else { "Record Service Sale" }}
                            </button>
                        </div>
                    </div>}.into_any() } else { ().into_any() }}
                </div>
            </div>
        </div>}.into_any() } else { ().into_any() }}

        // Convert-to-Debt Modal
        {move || show_convert.get().map(|sale| {
            let amount = sale.amount;
            let pname = sale.product_name.clone().unwrap_or_default();
            let pqty = sale.quantity.clone().unwrap_or_else(|| "-".into());
            let remaining = move || { let paid:f64 = convert_paid.get().parse().unwrap_or(0.0); (amount - paid).max(0.0) };
            view!{<div id="modal-convert-debt" class="modal-overlay open"><div class="modal-container" style="max-width: 500px;">
                <div class="modal-header"><h3 class="modal-title">"Convert Sale to Debt"</h3>
                    <button
                        class="modal-close-btn"
                        prop:disabled=move || convert_submitting.get()
                        on:click=move |_| {
                            if !convert_submitting.get() {
                                set_show_convert.set(None);
                            }
                        }
                    ><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button>
                </div>
                <div class="modal-body">
                    <div class="bg-gray-50 p-4 mb-4"><p class="text-xs text-gray-500 uppercase tracking-wide">"Sale Details"</p>
                        <p class="font-semibold text-gray-900">{pname}</p>
                        <p class="text-sm text-gray-600">{format!("{} - KSh {:.0}", pqty, amount)}</p>
                    </div>
                    <div class="space-y-4">
                        <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Name *"</label><input type="text" class="w-full" placeholder="Enter customer name" prop:value=move || convert_customer.get() prop:disabled=move || convert_submitting.get() on:input=move |e| set_convert_customer.set(event_target_value(&e))/></div>
                        <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Customer Phone"</label><input type="tel" class="w-full" placeholder="Optional" prop:value=move || convert_phone.get() prop:disabled=move || convert_submitting.get() on:input=move |e| set_convert_phone.set(event_target_value(&e))/></div>
                        <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Amount Paid *"</label><input type="number" min="0" step="0.01" class="w-full" placeholder="0.00" prop:value=move || convert_paid.get() prop:disabled=move || convert_submitting.get() on:input=move |e| set_convert_paid.set(event_target_value(&e))/></div>
                        <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Remaining Debt"</label><div class="px-3 py-2 bg-red-50 border border-red-200 text-lg font-bold text-red-600">{move || format!("KSh {:.0}", remaining())}</div></div>
                        <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Due Date"</label><MiniCalendar date_r=convert_due_date date_w=set_convert_due_date label=set_convert_due_label/></div>
                    </div>
                </div>
                <div class="modal-footer">
                    <button
                        type="button"
                        class="btn-secondary"
                        prop:disabled=move || convert_submitting.get()
                        on:click=move |_| {
                            if !convert_submitting.get() {
                                set_show_convert.set(None);
                            }
                        }
                    >"Cancel"</button>
                    <button
                        type="button"
                        class="btn-primary"
                        prop:disabled=move || convert_submitting.get()
                        on:click=submit_convert
                    >{move || if convert_submitting.get() { "Creating..." } else { "Create Debt" }}</button>
                </div>
            </div></div>}
        }).map(|v| v.into_any()).unwrap_or_else(|| ().into_any())}

        <ReceiptModal show=Signal::derive(move || show_receipt.get()) set_show=set_show_receipt receipt_html=Signal::derive(move || receipt_html.get()) title=Signal::derive(move || receipt_title.get())/>

        // Delete sale confirmation
        <Show when=move || del_id.get().is_some()>
            <div
                class="modal-overlay open"
                on:click=move |e| {
                    if e.target() == e.current_target() && !deleting_sale.get() {
                        set_del_id.set(None);
                        set_del_label.set(String::new());
                    }
                }
            >
                <div class="modal-container modal-sm">
                    <div class="modal-header">
                        <h3 class="modal-title">"Delete Sale?"</h3>
                        <button
                            type="button"
                            class="modal-close-btn"
                            prop:disabled=move || deleting_sale.get()
                            on:click=move |_| {
                                if !deleting_sale.get() {
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
                            "Are you sure you want to delete "
                            <span class="modal-entity">{move || del_label.get()}</span>
                            "? This action cannot be undone."
                        </p>
                    </div>
                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn-secondary"
                            prop:disabled=move || deleting_sale.get()
                            on:click=move |_| {
                                if !deleting_sale.get() {
                                    set_del_id.set(None);
                                    set_del_label.set(String::new());
                                }
                            }
                        >"Cancel"</button>
                        <button
                            type="button"
                            class="btn-danger"
                            prop:disabled=move || deleting_sale.get()
                            on:click=confirm_delete_sale
                        >{move || if deleting_sale.get() { "Deleting..." } else { "Delete" }}</button>
                    </div>
                </div>
            </div>
        </Show>
    </div>
        </Show>
    }
}
