use leptos::prelude::*;
use crate::api::Sale;

const BUSINESS_NAME: &str = "MULTIPRINTS";
const BUSINESS_PHONE: &str = "";
const BUSINESS_ADDRESS: &str = "";
const CURRENCY: &str = "KSh";

fn format_date(ts: &Option<String>) -> String {
    ts.as_ref()
        .and_then(|t| chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.3f").ok())
        .map(|d| d.format("%d/%m/%Y %I:%M %p").to_string())
        .unwrap_or_else(|| chrono::Local::now().format("%d/%m/%Y %I:%M %p").to_string())
}

fn receipt_num(id: i64) -> String { format!("RCP-{:04}", id) }

fn fmt_money(n: f64) -> String { format!("{:.0}", n) }

fn payment_label(method: &str) -> &str {
    match method {
        "cash" => "Cash",
        "mpesa" => "M-Pesa",
        "till" => "Till Number",
        "credit" => "Credit",
        _ => method,
    }
}

fn build_single_sale_receipt(sale: &Sale) -> String {
    let num = receipt_num(sale.id);
    let date = format_date(&sale.timestamp);
    let cust = sale.customer_name.as_str();
    let pname = sale.product_name.as_deref().unwrap_or("—");
    let qty = sale.quantity.as_deref().unwrap_or("1");
    let pay = payment_label(&sale.payment_method);

    format!(r#"<div class="receipt-container">
        <div class="receipt-header">
            <div class="receipt-business-name">{business}</div>
            {addr}{phone}
        </div>
        <div class="receipt-divider">================================</div>
        <div class="receipt-meta">
            <div class="receipt-row"><span>Receipt #:</span><span>{num}</span></div>
            <div class="receipt-row"><span>Date:</span><span>{date}</span></div>
            <div class="receipt-row"><span>Customer:</span><span>{cust}</span></div>
        </div>
        <div class="receipt-divider">--------------------------------</div>
        <div class="receipt-items">
            <div class="receipt-items-header">Items:</div>
            <div class="receipt-item">
                <div class="receipt-item-name">{name}</div>
                <div class="receipt-item-details">
                    <span>{qty}</span>
                    <span class="receipt-item-price">{cur} {amt}</span>
                </div>
            </div>
        </div>
        <div class="receipt-divider">--------------------------------</div>
        <div class="receipt-totals">
            <div class="receipt-row"><span>Payment:</span><span class="receipt-payment">{pay}</span></div>
        </div>
        <div class="receipt-divider">================================</div>
        <div class="receipt-total-row"><span>TOTAL:</span><span>{cur} {amt}</span></div>
        <div class="receipt-divider">================================</div>
        <div class="receipt-footer"><div>Thank you for your business!</div></div>
    </div>"#,
        business = BUSINESS_NAME,
        addr = if BUSINESS_ADDRESS.is_empty() { String::new() } else { format!("<div class=\"receipt-info\">{}</div>", BUSINESS_ADDRESS) },
        phone = if BUSINESS_PHONE.is_empty() { String::new() } else { format!("<div class=\"receipt-info\">Tel: {}</div>", BUSINESS_PHONE) },
        num = num, date = date, cust = cust, name = pname, qty = qty,
        cur = CURRENCY, amt = fmt_money(sale.amount), pay = pay,
    )
}

fn build_multi_sale_receipt(sales: &[Sale]) -> String {
    let total: f64 = sales.iter().map(|s| s.amount).sum();
    let nums: Vec<String> = sales.iter().map(|s| receipt_num(s.id)).collect();
    let num_display = nums.join(", ");
    let date = format_date(&None);
    let customers: Vec<&str> = sales.iter().map(|s| s.customer_name.as_str()).collect();
    let cust_display = if customers.iter().all(|c| *c == customers[0]) {
        customers[0].to_string()
    } else {
        format!("Multiple ({})", customers.len())
    };
    let payments: Vec<&str> = sales.iter().map(|s| payment_label(&s.payment_method)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let pay_display = payments.join(", ");

    let items: String = sales.iter().map(|s| {
        let name = s.product_name.as_deref().unwrap_or("—");
        let qty = s.quantity.as_deref().unwrap_or("1");
        format!(r#"<div class="receipt-item">
            <div class="receipt-item-name">{n}</div>
            <div class="receipt-item-details">
                <span>{q}</span>
                <span class="receipt-item-price">{c} {a}</span>
            </div>
        </div>"#, n = name, q = qty, c = CURRENCY, a = fmt_money(s.amount))
    }).collect();

    format!(r#"<div class="receipt-container">
        <div class="receipt-header">
            <div class="receipt-business-name">{business}</div>
            {addr}{phone}
        </div>
        <div class="receipt-divider">================================</div>
        <div class="receipt-meta">
            <div class="receipt-row"><span>Receipt #:</span><span style="font-size:9px">{num}</span></div>
            <div class="receipt-row"><span>Date:</span><span>{date}</span></div>
            <div class="receipt-row"><span>Customer:</span><span>{cust}</span></div>
            <div class="receipt-row"><span>Items:</span><span>{count}</span></div>
        </div>
        <div class="receipt-divider">--------------------------------</div>
        <div class="receipt-items">
            <div class="receipt-items-header">Items:</div>
            {items}
        </div>
        <div class="receipt-divider">--------------------------------</div>
        <div class="receipt-totals">
            <div class="receipt-row"><span>Payment:</span><span class="receipt-payment">{pay}</span></div>
        </div>
        <div class="receipt-divider">================================</div>
        <div class="receipt-total-row"><span>TOTAL:</span><span>{c} {total}</span></div>
        <div class="receipt-divider">================================</div>
        <div class="receipt-footer"><div>Thank you for your business!</div></div>
    </div>"#,
        business = BUSINESS_NAME,
        addr = if BUSINESS_ADDRESS.is_empty() { String::new() } else { format!("<div class=\"receipt-info\">{}</div>", BUSINESS_ADDRESS) },
        phone = if BUSINESS_PHONE.is_empty() { String::new() } else { format!("<div class=\"receipt-info\">Tel: {}</div>", BUSINESS_PHONE) },
        num = num_display, date = date, cust = cust_display, count = sales.len(),
        items = items, pay = pay_display, c = CURRENCY, total = fmt_money(total),
    )
}

#[component]
pub fn ReceiptModal(
    show: Signal<bool>,
    set_show: WriteSignal<bool>,
    receipt_html: Signal<String>,
    title: Signal<String>,
) -> impl IntoView {
    let print = move |_| {
        // Trigger browser print
        if let Some(window) = web_sys::window() {
            let _ = window.print();
        }
    };

    view! {
        {move || if show.get() {
            let html = receipt_html.get();
            view!{<div id="receipt-preview-modal" class="modal-overlay open">
                <div class="modal-container receipt-preview-modal">
                    <div class="modal-header">
                        <h3 class="modal-title">{title.get()}</h3>
                        <button class="modal-close-btn" on:click=move |_| set_show.set(false)>
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                            </svg>
                        </button>
                    </div>
                    <div class="modal-body receipt-preview-body">
                        <div class="receipt-preview-wrapper" inner_html=html></div>
                    </div>
                    <div class="modal-footer">
                        <button type="button" class="btn-secondary px-4 py-2" on:click=move |_| set_show.set(false)>"Close"</button>
                        <button type="button" class="btn-primary px-4 py-2 flex items-center gap-2" on:click=print>
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"></path>
                            </svg>
                            "Print"
                        </button>
                    </div>
                </div>
            </div>}.into_any()
        } else { ().into_any() }}
    }
}

/// Convenience: open a receipt modal for a single sale
pub fn open_sale_receipt(
    sale: &Sale,
    show: WriteSignal<bool>,
    html: WriteSignal<String>,
    title: WriteSignal<String>,
) {
    title.set(format!("Print Sale Receipt"));
    html.set(build_single_sale_receipt(sale));
    show.set(true);
}

/// Convenience: open a receipt modal for multiple sales
pub fn open_multi_sale_receipt(
    sales: &[Sale],
    show: WriteSignal<bool>,
    html: WriteSignal<String>,
    title: WriteSignal<String>,
) {
    title.set(if sales.len() == 1 {
        "Print Sale Receipt".into()
    } else {
        format!("Print {} Sales Receipt", sales.len())
    });
    html.set(build_multi_sale_receipt(sales));
    show.set(true);
}
