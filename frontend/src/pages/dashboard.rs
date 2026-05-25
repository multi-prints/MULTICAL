use leptos::prelude::*;
use crate::api::{self, Sale, Debt};

#[component]
pub fn DashboardPage() -> impl IntoView {
    let (sales, set_sales) = signal(Vec::<Sale>::new());
    let (debts, set_debts) = signal(Vec::<Debt>::new());
    let (total_revenue, set_total_revenue) = signal(0.0);
    let (total_outstanding, set_total_outstanding) = signal(0.0);
    let (sales_count, set_sales_count) = signal(0i64);
    let (loading, set_loading) = signal(true);

    leptos::task::spawn_local(async move {
        if let Ok(s) = api::get_all_sales().await {
            let count = s.len() as i64;
            let rev: f64 = s.iter().map(|s| s.amount).sum();
            set_sales.set(s.into_iter().take(10).collect());
            set_sales_count.set(count);
            set_total_revenue.set(rev);
        }
        if let Ok(d) = api::get_all_debts().await {
            let pending: Vec<Debt> = d.into_iter().filter(|d| d.status == "pending").collect();
            let outstanding: f64 = pending.iter().map(|d| d.remaining_amount).sum();
            set_total_outstanding.set(outstanding);
            set_debts.set(pending.into_iter().take(10).collect());
        }
        set_loading.set(false);
    });

    let fmt = |amount: f64| format!("KSh {}", amount);

    view! {
        {move || if loading.get() {
            view! { <p class="text-gray-500">"Loading..."</p> }.into_any()
        } else {
            view! {
                <div>
                    <h1 class="text-2xl font-bold mb-6">"Dashboard"</h1>
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
                        <div class="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
                            <p class="text-sm text-gray-500 mb-1">"Total Revenue"</p>
                            <p class="text-2xl font-bold text-gray-900">{move || fmt(total_revenue.get())}</p>
                        </div>
                        <div class="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
                            <p class="text-sm text-gray-500 mb-1">"Total Sales"</p>
                            <p class="text-2xl font-bold text-gray-900">{move || sales_count.get()}</p>
                        </div>
                        <div class="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
                            <p class="text-sm text-gray-500 mb-1">"Outstanding Debts"</p>
                            <p class="text-2xl font-bold text-red-600">{move || fmt(total_outstanding.get())}</p>
                        </div>
                    </div>

                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <div class="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
                            <h2 class="font-semibold mb-4">"Recent Sales"</h2>
                            <div class="space-y-3">
                                {move || {
                                    let items: Vec<_> = sales.get();
                                    if items.is_empty() {
                                        view! { <p class="text-gray-400 text-sm">"No sales recorded yet"</p> }.into_any()
                                    } else {
                                        items.into_iter().map(|sale| view! {
                                            <div class="flex justify-between items-center py-2 border-b border-gray-50 text-sm">
                                                <div>
                                                    <span class="font-medium">{sale.product_name.unwrap_or(sale.r#type)}</span>
                                                    <span class="text-gray-500 ml-2">{sale.customer_name}</span>
                                                </div>
                                                <span class="font-medium">{fmt(sale.amount)}</span>
                                            </div>
                                        }).collect::<Vec<_>>().into_any()
                                    }
                                }}
                            </div>
                        </div>

                        <div class="bg-white rounded-xl p-5 shadow-sm border border-gray-100">
                            <h2 class="font-semibold mb-4">"Pending Debts"</h2>
                            <div class="space-y-3">
                                {move || {
                                    let items: Vec<_> = debts.get();
                                    if items.is_empty() {
                                        view! { <p class="text-gray-400 text-sm">"No pending debts"</p> }.into_any()
                                    } else {
                                        items.into_iter().map(|debt| view! {
                                            <div class="flex justify-between items-center py-2 border-b border-gray-50 text-sm">
                                                <div>
                                                    <span class="font-medium">{debt.customer_name}</span>
                                                    <span class="text-gray-500 ml-2">"due " {debt.due_date.unwrap_or_default()}</span>
                                                </div>
                                                <span class="font-medium text-red-600">{fmt(debt.remaining_amount)}</span>
                                            </div>
                                        }).collect::<Vec<_>>().into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                </div>
            }.into_any()
        }}
    }
}
