use crate::api::{Sale, ServiceTransaction};
use leptos::prelude::*;

const BUSINESS_NAME: &str = "MULTIPRINTS";
const BUSINESS_PHONE: &str = "";
const BUSINESS_ADDRESS: &str = "";
const CURRENCY: &str = "KSh";

fn parse_timestamp(ts: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.3f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S"))
        .ok()
}

fn format_date(ts: &Option<String>) -> String {
    ts.as_ref()
        .and_then(|t| parse_timestamp(t))
        .map(|d| d.format("%d/%m/%Y %I:%M %p").to_string())
        .unwrap_or_else(|| chrono::Local::now().format("%d/%m/%Y %I:%M %p").to_string())
}

fn format_multi_receipt_date(sales: &[Sale]) -> String {
    sales
        .iter()
        .filter_map(|s| s.timestamp.as_deref().and_then(parse_timestamp))
        .max()
        .map(|d| d.format("%d/%m/%Y %I:%M %p").to_string())
        .unwrap_or_else(|| chrono::Local::now().format("%d/%m/%Y %I:%M %p").to_string())
}

fn format_multi_printing_receipt_date(jobs: &[ServiceTransaction]) -> String {
    jobs.iter()
        .filter_map(|j| j.timestamp.as_deref().and_then(parse_timestamp))
        .max()
        .map(|d| d.format("%d/%m/%Y %I:%M %p").to_string())
        .unwrap_or_else(|| chrono::Local::now().format("%d/%m/%Y %I:%M %p").to_string())
}

fn receipt_num(id: i64) -> String {
    format!("RCP-{:04}", id)
}

fn printing_receipt_num(id: i64) -> String {
    format!("PRT-{:04}", id)
}

fn fmt_money(n: f64) -> String {
    format!("{:.2}", n)
}

fn payment_label(method: &str) -> &str {
    match method {
        "cash" => "Cash",
        "mpesa" => "M-Pesa",
        "till" => "Till Number",
        "credit" => "Credit",
        _ => method,
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn business_info_html() -> String {
    let mut lines = Vec::new();
    if !BUSINESS_ADDRESS.is_empty() {
        lines.push(format!(
            "<div class=\"receipt-info\">{}</div>",
            escape_html(BUSINESS_ADDRESS)
        ));
    }
    if !BUSINESS_PHONE.is_empty() {
        lines.push(format!(
            "<div class=\"receipt-info\">Tel: {}</div>",
            escape_html(BUSINESS_PHONE)
        ));
    }
    lines.join("")
}

fn build_receipt_shell(
    document_type: &str,
    section_title: &str,
    meta_html: String,
    items_html: String,
    summary_html: String,
    total_html: String,
) -> String {
    format!(
        r#"<div class="receipt-container">
            <div class="receipt-paper-accent"></div>
            <div class="receipt-header">
                <div class="receipt-business-name">{business}</div>
                <div class="receipt-document-type">{doc_type}</div>
                {business_info}
            </div>

            <div class="receipt-rule"></div>
            <div class="receipt-meta">{meta}</div>

            <div class="receipt-rule receipt-rule--soft"></div>
            <div class="receipt-section-title">{section_title}</div>
            <div class="receipt-items">{items}</div>

            <div class="receipt-rule receipt-rule--soft"></div>
            <div class="receipt-summary">{summary}</div>

            <div class="receipt-rule"></div>
            <div class="receipt-total-row">{total}</div>

            <div class="receipt-rule"></div>
            <div class="receipt-footer">
                <div class="receipt-footer-title">Thank you for your business</div>
                <div class="receipt-footer-copy">Please keep this receipt for reference.</div>
            </div>
        </div>"#,
        business = escape_html(BUSINESS_NAME),
        doc_type = escape_html(document_type),
        section_title = escape_html(section_title),
        business_info = business_info_html(),
        meta = meta_html,
        items = items_html,
        summary = summary_html,
        total = total_html,
    )
}

fn build_single_sale_receipt(sale: &Sale) -> String {
    let num = receipt_num(sale.id);
    let date = format_date(&sale.timestamp);
    let cust = if sale.customer_name.trim().is_empty() {
        "Walk-in"
    } else {
        sale.customer_name.as_str()
    };
    let pname = sale.product_name.as_deref().unwrap_or("—");
    let qty = sale.quantity.as_deref().unwrap_or("1");
    let pay = payment_label(&sale.payment_method);
    let amount = fmt_money(sale.amount);

    let meta_html = format!(
        r#"<div class="receipt-meta-row"><span>Receipt No</span><span>{num}</span></div>
           <div class="receipt-meta-row"><span>Date</span><span>{date}</span></div>
           <div class="receipt-meta-row"><span>Customer</span><span>{cust}</span></div>"#,
        num = escape_html(&num),
        date = escape_html(&date),
        cust = escape_html(cust),
    );

    let items_html = format!(
        r#"<div class="receipt-item-row">
            <div class="receipt-item-copy">
                <div class="receipt-item-name">{name}</div>
                <div class="receipt-item-meta">Qty: {qty}</div>
            </div>
            <div class="receipt-item-amount">{currency} {amount}</div>
        </div>"#,
        name = escape_html(pname),
        qty = escape_html(qty),
        currency = CURRENCY,
        amount = amount,
    );

    let summary_html = format!(
        r#"<div class="receipt-row"><span>Items</span><span>1</span></div>
           <div class="receipt-row"><span>Payment</span><span class="receipt-payment">{pay}</span></div>
           <div class="receipt-row"><span>Subtotal</span><span>{currency} {amount}</span></div>"#,
        pay = escape_html(pay),
        currency = CURRENCY,
        amount = amount,
    );

    let total_html = format!(
        r#"<span>TOTAL</span><span>{currency} {amount}</span>"#,
        currency = CURRENCY,
        amount = amount,
    );

    build_receipt_shell(
        "Sales Receipt",
        "Items Purchased",
        meta_html,
        items_html,
        summary_html,
        total_html,
    )
}

fn build_multi_sale_receipt(sales: &[Sale]) -> String {
    let total: f64 = sales.iter().map(|s| s.amount).sum();
    let total_display = fmt_money(total);
    let nums: Vec<String> = sales.iter().map(|s| receipt_num(s.id)).collect();
    let num_display = nums.join(", ");
    let date = format_multi_receipt_date(sales);
    let customers: Vec<&str> = sales
        .iter()
        .map(|s| {
            if s.customer_name.trim().is_empty() {
                "Walk-in"
            } else {
                s.customer_name.as_str()
            }
        })
        .collect();
    let cust_display = if customers.is_empty() {
        "Walk-in".to_string()
    } else if customers.iter().all(|c| *c == customers[0]) {
        customers[0].to_string()
    } else {
        format!("Multiple ({})", customers.len())
    };
    let payments: Vec<&str> = sales
        .iter()
        .map(|s| payment_label(&s.payment_method))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let pay_display = payments.join(", ");

    let meta_html = format!(
        r#"<div class="receipt-meta-row"><span>Receipt Ref</span><span class="receipt-meta-compact">{num}</span></div>
           <div class="receipt-meta-row"><span>Date</span><span>{date}</span></div>
           <div class="receipt-meta-row"><span>Customer</span><span>{cust}</span></div>
           <div class="receipt-meta-row"><span>Transactions</span><span>{count}</span></div>"#,
        num = escape_html(&num_display),
        date = escape_html(&date),
        cust = escape_html(&cust_display),
        count = sales.len(),
    );

    let items_html: String = sales
        .iter()
        .map(|s| {
            let name = s.product_name.as_deref().unwrap_or("—");
            let qty = s.quantity.as_deref().unwrap_or("1");
            format!(
                r#"<div class="receipt-item-row">
                    <div class="receipt-item-copy">
                        <div class="receipt-item-name">{name}</div>
                        <div class="receipt-item-meta">Qty: {qty}</div>
                    </div>
                    <div class="receipt-item-amount">{currency} {amount}</div>
                </div>"#,
                name = escape_html(name),
                qty = escape_html(qty),
                currency = CURRENCY,
                amount = fmt_money(s.amount),
            )
        })
        .collect();

    let summary_html = format!(
        r#"<div class="receipt-row"><span>Transactions</span><span>{count}</span></div>
           <div class="receipt-row"><span>Payment</span><span class="receipt-payment">{pay}</span></div>
           <div class="receipt-row"><span>Subtotal</span><span>{currency} {amount}</span></div>"#,
        count = sales.len(),
        pay = escape_html(&pay_display),
        currency = CURRENCY,
        amount = total_display,
    );

    let total_html = format!(
        r#"<span>TOTAL</span><span>{currency} {amount}</span>"#,
        currency = CURRENCY,
        amount = total_display,
    );

    build_receipt_shell(
        "Sales Summary Receipt",
        "Items Purchased",
        meta_html,
        items_html,
        summary_html,
        total_html,
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

fn build_single_printing_receipt(job: &ServiceTransaction) -> String {
    let num = printing_receipt_num(job.id);
    let date = format_date(&job.timestamp);
    let cust = if job.customer_name.trim().is_empty() {
        "Walk-in"
    } else {
        job.customer_name.as_str()
    };
    let service = if job.service_name.trim().is_empty() {
        "Printing job"
    } else {
        job.service_name.as_str()
    };
    let pay = payment_label(&job.payment_method);
    let amount = fmt_money(job.amount);
    let metres = job.stock_metres_used;

    let material_bits: Vec<String> = [
        job.material_size
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| format!("{}m", s)),
        job.material_type
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    ]
    .into_iter()
    .flatten()
    .collect();
    let material_line = if material_bits.is_empty() {
        String::new()
    } else {
        format!(" · {}", material_bits.join(" "))
    };

    let mut item_meta = format!("{:.1}m printed{}", metres, material_line);
    if let Some(notes) = job
        .notes
        .as_ref()
        .map(|n| n.trim())
        .filter(|n| !n.is_empty())
    {
        item_meta.push_str(" · ");
        item_meta.push_str(notes);
    }

    let meta_html = format!(
        r#"<div class="receipt-meta-row"><span>Receipt No</span><span>{num}</span></div>
           <div class="receipt-meta-row"><span>Date</span><span>{date}</span></div>
           <div class="receipt-meta-row"><span>Customer</span><span>{cust}</span></div>"#,
        num = escape_html(&num),
        date = escape_html(&date),
        cust = escape_html(cust),
    );

    let items_html = format!(
        r#"<div class="receipt-item-row">
            <div class="receipt-item-copy">
                <div class="receipt-item-name">{name}</div>
                <div class="receipt-item-meta">{meta}</div>
            </div>
            <div class="receipt-item-amount">{currency} {amount}</div>
        </div>"#,
        name = escape_html(service),
        meta = escape_html(&item_meta),
        currency = CURRENCY,
        amount = amount,
    );

    let status_line = if job.is_debt > 0 {
        r#"<div class="receipt-row"><span>Status</span><span class="receipt-payment">Debt</span></div>"#
    } else {
        ""
    };

    let summary_html = format!(
        r#"<div class="receipt-row"><span>Jobs</span><span>1</span></div>
           <div class="receipt-row"><span>Metres</span><span>{metres:.1}m</span></div>
           <div class="receipt-row"><span>Payment</span><span class="receipt-payment">{pay}</span></div>
           {status}
           <div class="receipt-row"><span>Subtotal</span><span>{currency} {amount}</span></div>"#,
        metres = metres,
        pay = escape_html(pay),
        status = status_line,
        currency = CURRENCY,
        amount = amount,
    );

    let total_html = format!(
        r#"<span>TOTAL</span><span>{currency} {amount}</span>"#,
        currency = CURRENCY,
        amount = amount,
    );

    build_receipt_shell(
        "Printing Receipt",
        "Service Details",
        meta_html,
        items_html,
        summary_html,
        total_html,
    )
}

pub fn open_sale_receipt(
    sale: &Sale,
    show: WriteSignal<bool>,
    html: WriteSignal<String>,
    title: WriteSignal<String>,
) {
    title.set("Print Sale Receipt".into());
    html.set(build_single_sale_receipt(sale));
    show.set(true);
}

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

pub fn open_printing_receipt(
    job: &ServiceTransaction,
    show: WriteSignal<bool>,
    html: WriteSignal<String>,
    title: WriteSignal<String>,
) {
    title.set("Print Printing Receipt".into());
    html.set(build_single_printing_receipt(job));
    show.set(true);
}

fn build_multi_printing_receipt(jobs: &[ServiceTransaction]) -> String {
    let total: f64 = jobs.iter().map(|j| j.amount).sum();
    let total_metres: f64 = jobs.iter().map(|j| j.stock_metres_used).sum();
    let total_display = fmt_money(total);
    let nums: Vec<String> = jobs.iter().map(|j| printing_receipt_num(j.id)).collect();
    let num_display = nums.join(", ");
    let date = format_multi_printing_receipt_date(jobs);
    let customers: Vec<&str> = jobs
        .iter()
        .map(|j| {
            if j.customer_name.trim().is_empty() {
                "Walk-in"
            } else {
                j.customer_name.as_str()
            }
        })
        .collect();
    let cust_display = if customers.is_empty() {
        "Walk-in".to_string()
    } else if customers.iter().all(|c| *c == customers[0]) {
        customers[0].to_string()
    } else {
        format!("Multiple ({})", customers.len())
    };
    let payments: Vec<&str> = jobs
        .iter()
        .map(|j| payment_label(&j.payment_method))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let pay_display = payments.join(", ");
    let debt_jobs = jobs.iter().filter(|j| j.is_debt > 0).count();

    let meta_html = format!(
        r#"<div class="receipt-meta-row"><span>Receipt Ref</span><span class="receipt-meta-compact">{num}</span></div>
           <div class="receipt-meta-row"><span>Date</span><span>{date}</span></div>
           <div class="receipt-meta-row"><span>Customer</span><span>{cust}</span></div>
           <div class="receipt-meta-row"><span>Jobs</span><span>{count}</span></div>"#,
        num = escape_html(&num_display),
        date = escape_html(&date),
        cust = escape_html(&cust_display),
        count = jobs.len(),
    );

    let items_html: String = jobs
        .iter()
        .map(|j| {
            let service = if j.service_name.trim().is_empty() {
                "Printing job"
            } else {
                j.service_name.as_str()
            };
            let material_bits: Vec<String> = [
                j.material_size
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| format!("{}m", s)),
                j.material_type
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
            ]
            .into_iter()
            .flatten()
            .collect();
            let material_line = if material_bits.is_empty() {
                String::new()
            } else {
                format!(" · {}", material_bits.join(" "))
            };
            let mut item_meta = format!("{:.1}m printed{}", j.stock_metres_used, material_line);
            if j.is_debt > 0 {
                item_meta.push_str(" · Debt");
            }
            format!(
                r#"<div class="receipt-item-row">
                    <div class="receipt-item-copy">
                        <div class="receipt-item-name">{name}</div>
                        <div class="receipt-item-meta">{meta}</div>
                    </div>
                    <div class="receipt-item-amount">{currency} {amount}</div>
                </div>"#,
                name = escape_html(service),
                meta = escape_html(&item_meta),
                currency = CURRENCY,
                amount = fmt_money(j.amount),
            )
        })
        .collect();

    let debt_line = if debt_jobs > 0 {
        format!(
            r#"<div class="receipt-row"><span>On debt</span><span class="receipt-payment">{n} job(s)</span></div>"#,
            n = debt_jobs
        )
    } else {
        String::new()
    };

    let summary_html = format!(
        r#"<div class="receipt-row"><span>Jobs</span><span>{count}</span></div>
           <div class="receipt-row"><span>Metres</span><span>{metres:.1}m</span></div>
           <div class="receipt-row"><span>Payment</span><span class="receipt-payment">{pay}</span></div>
           {debt}
           <div class="receipt-row"><span>Subtotal</span><span>{currency} {amount}</span></div>"#,
        count = jobs.len(),
        metres = total_metres,
        pay = escape_html(&pay_display),
        debt = debt_line,
        currency = CURRENCY,
        amount = total_display,
    );

    let total_html = format!(
        r#"<span>TOTAL</span><span>{currency} {amount}</span>"#,
        currency = CURRENCY,
        amount = total_display,
    );

    build_receipt_shell(
        "Printing Summary Receipt",
        "Service Details",
        meta_html,
        items_html,
        summary_html,
        total_html,
    )
}

pub fn open_multi_printing_receipt(
    jobs: &[ServiceTransaction],
    show: WriteSignal<bool>,
    html: WriteSignal<String>,
    title: WriteSignal<String>,
) {
    if jobs.is_empty() {
        return;
    }
    if jobs.len() == 1 {
        open_printing_receipt(&jobs[0], show, html, title);
        return;
    }
    title.set(format!("Print {} Jobs Receipt", jobs.len()));
    html.set(build_multi_printing_receipt(jobs));
    show.set(true);
}
