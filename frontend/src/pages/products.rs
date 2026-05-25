use leptos::prelude::*;
use crate::api::{self, Product, NewProduct, ProductUpdate};

#[component]
pub fn ProductsPage() -> impl IntoView {
    let (products, set_products) = signal(Vec::<Product>::new());
    let (show_form, set_show_form) = signal(false);
    let (editing_id, set_editing_id) = signal(None::<i64>);
    let (name, set_name) = signal(String::new());
    let (product_type, set_product_type) = signal("life_saver".to_string());
    let (color, set_color) = signal(String::new());
    let (size, set_size) = signal(String::new());
    let (price, set_price) = signal(0.0);
    let (stock_qty, set_stock_qty) = signal(0i64);

    let load = {
        let set_products = set_products;
        move || { leptos::task::spawn_local(async move {
            if let Ok(p) = api::get_all_products().await { set_products.set(p); }
        });}
    };
    load();

    let reset = move || {
        set_name.set(String::new()); set_color.set(String::new()); set_size.set(String::new());
        set_price.set(0.0); set_stock_qty.set(0i64); set_editing_id.set(None); set_show_form.set(false);
    };

    let save = {
        let load = load.clone();
        move |_| {
            let n = name.get(); if n.is_empty() { return; }
            let pt = product_type.get(); let p = price.get(); let s = stock_qty.get();
            let col = { let c = color.get(); if c.is_empty() { None } else { Some(c) } };
            let sz = { let z = size.get(); if z.is_empty() { None } else { Some(z) } };
            leptos::task::spawn_local(async move {
                if let Some(id) = editing_id.get() {
                    let _ = api::update_product(id, &ProductUpdate {
                        name: Some(n), product_type: Some(pt), selling_price: Some(p),
                        stock: Some(s), color: col, size: sz,
                    }).await;
                } else {
                    let _ = api::add_product(&NewProduct {
                        name: n, product_type: pt, color: col, size: sz,
                        selling_price: p, stock: s,
                    }).await;
                }
                reset(); load();
            });
        }
    };

    view! {
        <div>
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold">"Products"</h1>
                <button on:click=move |_| { reset(); set_show_form.set(true); }
                    class="px-4 py-2 bg-brand-600 text-white rounded-lg text-sm hover:bg-brand-700">"+ Add Product"</button>
            </div>

            {move || if show_form.get() {
                view! {
                    <div class="bg-white rounded-xl p-5 shadow-sm border border-gray-100 mb-6">
                        <h2 class="font-semibold mb-4">{if editing_id.get().is_some() { "Edit Product" } else { "New Product" }}</h2>
                        <div class="grid grid-cols-2 gap-4">
                            <div><label class="block text-sm font-medium text-gray-700 mb-1">"Name"</label>
                                <input type="text" class="w-full px-3 py-2 border rounded-lg text-sm" prop:value=move || name.get() on:input=move |e| set_name.set(event_target_value(&e)) /></div>
                            <div><label class="block text-sm font-medium text-gray-700 mb-1">"Type"</label>
                                <select class="w-full px-3 py-2 border rounded-lg text-sm" on:change=move |e| set_product_type.set(event_target_value(&e))>
                                    <option value="life_saver">"Life Saver"</option>
                                    <option value="chevron">"Chevron"</option>
                                    <option value="stripes">"Stripes"</option>
                                </select></div>
                            <div><label class="block text-sm font-medium text-gray-700 mb-1">"Color"</label>
                                <input type="text" class="w-full px-3 py-2 border rounded-lg text-sm" prop:value=move || color.get() on:input=move |e| set_color.set(event_target_value(&e)) /></div>
                            <div><label class="block text-sm font-medium text-gray-700 mb-1">"Size"</label>
                                <input type="text" class="w-full px-3 py-2 border rounded-lg text-sm" prop:value=move || size.get() on:input=move |e| set_size.set(event_target_value(&e)) /></div>
                            <div><label class="block text-sm font-medium text-gray-700 mb-1">"Price (KSh)"</label>
                                <input type="number" step="0.01" class="w-full px-3 py-2 border rounded-lg text-sm" prop:value=move || price.get() on:input=move |e| set_price.set(event_target_value(&e).parse().unwrap_or(0.0)) /></div>
                            <div><label class="block text-sm font-medium text-gray-700 mb-1">"Stock"</label>
                                <input type="number" class="w-full px-3 py-2 border rounded-lg text-sm" prop:value=move || stock_qty.get() on:input=move |e| set_stock_qty.set(event_target_value(&e).parse().unwrap_or(0)) /></div>
                        </div>
                        <div class="flex justify-end gap-3 mt-4">
                            <button on:click=move |_| reset() class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg">"Cancel"</button>
                            <button on:click=save class="px-4 py-2 text-sm bg-brand-600 text-white hover:bg-brand-700 rounded-lg">"Save"</button>
                        </div>
                    </div>
                }.into_any()
            } else { ().into_any() }}

            <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
                <table class="w-full text-sm"><thead class="bg-gray-50 border-b">
                    <tr><th class="text-left px-4 py-3 font-medium text-gray-600">"Name"</th><th class="text-left px-4 py-3 font-medium text-gray-600">"Type"</th><th class="text-left px-4 py-3 font-medium text-gray-600">"Color"</th><th class="text-left px-4 py-3 font-medium text-gray-600">"Size"</th><th class="text-right px-4 py-3 font-medium text-gray-600">"Price"</th><th class="text-right px-4 py-3 font-medium text-gray-600">"Stock"</th><th class="text-right px-4 py-3 font-medium text-gray-600">"Actions"</th></tr>
                </thead><tbody>
                    {move || {
                        let items = products.get();
                        if items.is_empty() {
                            view! { <tr><td colspan="7" class="text-center text-gray-400 py-8">"No products yet"</td></tr> }.into_any()
                        } else {
                            items.into_iter().map(|p| {
                                let load = load.clone();
                                view! {
                                    <tr class="border-b border-gray-50 hover:bg-gray-50">
                                        <td class="px-4 py-3 font-medium">{p.name.clone()}</td>
                                        <td class="px-4 py-3 text-gray-600">{p.product_type.clone()}</td>
                                        <td class="px-4 py-3 text-gray-600">{p.color.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 text-gray-600">{p.size.unwrap_or_default()}</td>
                                        <td class="px-4 py-3 text-right">"KSh " {p.selling_price}</td>
                                        <td class="px-4 py-3 text-right">{p.stock}</td>
                                        <td class="px-4 py-3 text-right space-x-2">
                                            <button on:click={
                                                let p2 = p.clone();
                                                move |_| { set_name.set(p2.name.clone()); set_product_type.set(p2.product_type.clone());
                                                    set_color.set(p2.color.clone().unwrap_or_default()); set_size.set(p2.size.clone().unwrap_or_default());
                                                    set_price.set(p2.selling_price); set_stock_qty.set(p2.stock);
                                                    set_editing_id.set(Some(p2.id)); set_show_form.set(true); }
                                            } class="text-brand-600 hover:underline text-xs">"Edit"</button>
                                            <button on:click={
                                                let pid = p.id; let load = load.clone();
                                                move |_| { leptos::task::spawn_local(async move { let _ = api::delete_product(pid).await; load(); }); }
                                            } class="text-red-600 hover:underline text-xs">"Del"</button>
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>().into_any()
                        }
                    }}
                </tbody></table>
            </div>
        </div>
    }
}
