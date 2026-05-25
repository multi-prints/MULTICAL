use leptos::prelude::*;
use crate::api::{self, NewStockItem, StockItem};

#[derive(Clone, Debug, PartialEq)]
struct SizeRow { id: u32, size: String, rolls: String, metres_per_roll: String }

const COLORS: &[(&str, &str, &str)] = &[
    ("Red", "#ef4444", "basic"), ("Blue", "#3b82f6", "basic"), ("Green", "#22c55e", "basic"),
    ("Yellow", "#eab308", "basic"), ("Orange", "#f97316", "basic"), ("Purple", "#a855f7", "basic"),
    ("Pink", "#ec4899", "basic"), ("Black", "#1f2937", "basic"), ("White", "#ffffff", "basic"),
    ("Dark Red", "#991b1b", "variant"), ("Dark Blue", "#1e3a8a", "variant"), ("Dark Green", "#166534", "variant"),
    ("Light Blue", "#93c5fd", "variant"), ("Light Green", "#86efac", "variant"), ("Black Matte", "#111827", "variant"),
    ("White Gloss", "#ffffff", "variant"), ("Gold Metallic", "#fbbf24", "metallic"), ("Silver Metallic", "#9ca3af", "metallic"),
];

#[component]
pub fn StockPage() -> impl IntoView {
    let (stock, set_stock) = signal(Vec::<StockItem>::new());
    let (show_add, set_show_add) = signal(false);
    let (show_suggestions, set_show_suggestions) = signal(false);
    let (color, set_color) = signal(String::new());
    let (sticker_type, set_sticker_type) = signal("colored".to_string());
    let (rows, set_rows) = signal(vec![SizeRow { id: 0, size: String::new(), rolls: String::new(), metres_per_roll: "50".into() }]);
    let (next_row_id, set_next_row_id) = signal(1u32);
    let (current_page, set_current_page) = signal(1u32);
    let (add_rolls_item, set_add_rolls_item) = signal(None::<StockItem>);
    let (add_rolls_value, set_add_rolls_value) = signal(String::new());
    let items_per_page = 10u32;

    let reload = move || leptos::task::spawn_local(async move { if let Ok(d) = api::get_all_stock().await { set_stock.set(d); } });
    reload();

    let reset_add_modal = move || {
        set_color.set(String::new());
        set_sticker_type.set("colored".into());
        set_rows.set(vec![SizeRow { id: 0, size: String::new(), rolls: String::new(), metres_per_roll: "50".into() }]);
        set_next_row_id.set(1);
        set_show_suggestions.set(false);
    };
    let add_row = move || {
        let id = next_row_id.get();
        set_next_row_id.set(id + 1);
        set_rows.update(|r| r.push(SizeRow { id, size: String::new(), rolls: String::new(), metres_per_roll: "50".into() }));
    };
    let update_row = move |id: u32, field: &'static str, value: String| set_rows.update(|rs| {
        if let Some(r) = rs.iter_mut().find(|r| r.id == id) {
            match field { "size" => r.size = value, "rolls" => r.rolls = value, "mpr" => r.metres_per_roll = value, _ => {} }
        }
    });

    let total_items = move || stock.get().len() as u32;
    let total_pages = move || { let n = total_items(); if n == 0 { 1 } else { (n + items_per_page - 1) / items_per_page } };
    let paginated = move || { let items = stock.get(); let start = ((current_page.get() - 1) * items_per_page) as usize; items.into_iter().skip(start).take(items_per_page as usize).collect::<Vec<_>>() };
    let remaining = |i: &StockItem| i.total_metres - if i.metres_used.is_nan() { 0.0 } else { i.metres_used };
    let rolls_left = move |i: &StockItem| (remaining(i) / if i.metres_per_roll > 0.0 { i.metres_per_roll } else { 50.0 }).floor() as i64;
    let status = move |i: &StockItem| { let pct = if i.total_metres <= 0.0 { 0.0 } else { remaining(i) / i.total_metres * 100.0 }; if pct <= 0.0 { ("Out of Stock", "status-badge--error") } else if pct <= 20.0 { ("Low Stock", "status-badge--warning") } else { ("In Stock", "status-badge--success") } };
    let row_total = move |r: &SizeRow| r.rolls.parse::<f64>().unwrap_or(0.0) * if sticker_type.get() == "reflective" { r.metres_per_roll.parse::<f64>().unwrap_or(0.0) } else { 50.0 };
    let total_rolls_modal = move || rows.get().iter().map(|r| r.rolls.parse::<i64>().unwrap_or(0)).sum::<i64>();
    let total_metres_modal = move || rows.get().iter().map(row_total).sum::<f64>();

    view! { <div id="page-stock" class="page-content">
        <div class="flex items-center justify-between mb-6"><div><h1 class="page-title">"Stock"</h1><p class="page-subtitle">"Manage your sticker inventory"</p></div>
            <button id="btn-add-stock" class="flex items-center gap-2 btn-primary px-4 py-2 text-sm" on:click=move |_| { reset_add_modal(); set_show_add.set(true); }><svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>"Add Stock"</button></div>
        <div class="grid grid-cols-4 gap-4 mb-6">
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium mb-1">"Total Colors"</p><p class="text-xl font-semibold">{move || stock.get().len()}</p></div>
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium mb-1">"Total Rolls"</p><p class="text-xl font-semibold">{move || stock.get().iter().map(|s| s.rolls).sum::<i64>()}</p></div>
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium mb-1">"Total Metres"</p><p class="text-xl font-semibold">{move || format!("{}m", stock.get().iter().map(|s| s.total_metres).sum::<f64>() as u64)}</p></div>
            <div class="stat-card-modern"><p class="text-xs text-gray-500 font-medium mb-1">"Remaining"</p><p class="text-xl font-semibold">{move || format!("{}m", stock.get().iter().map(|s| remaining(s)).sum::<f64>() as u64)}</p></div>
        </div>
        <div class="dashboard-panel overflow-hidden"><table class="w-full data-table"><thead><tr><th>"Type"</th><th>"Color"</th><th>"Width"</th><th>"Rolls"</th><th>"Total m"</th><th>"Used"</th><th>"Remaining"</th><th>"Left"</th><th>"Status"</th><th>"Actions"</th></tr></thead><tbody>
            {move || { let items = paginated(); if items.is_empty() { view!{<tr class="text-center"><td colspan="10" class="px-5 py-8 text-gray-500"><div class="flex flex-col items-center justify-center"><svg class="w-12 h-12 text-gray-300 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path></svg><p>"No stock added yet"</p><button on:click=move |_| { reset_add_modal(); set_show_add.set(true); } class="text-black font-semibold hover:underline text-sm mt-2">"Add your first stock"</button></div></td></tr>}.into_any() } else { items.into_iter().map(|item| { let id=item.id; let rem=remaining(&item); let left=rolls_left(&item); let (sl,sc)=status(&item); let typ=item.sticker_type.clone(); view!{<tr class="hover:bg-gray-50 transition-colors"><td class="px-6 py-4"><span class=format!("inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium {}", if typ=="reflective" {"bg-purple-50 text-purple-700"} else {"bg-blue-50 text-blue-700"})>{if typ=="reflective" {"Reflective"} else {"Colored"}}</span></td><td class="px-6 py-4"><div class="flex items-center gap-3"><div class="w-8 h-8 rounded-lg border border-gray-200 shadow-sm" style=format!("background-color: {}", get_hex(&item.color))></div><span class="text-sm font-medium text-gray-900">{item.color.clone()}</span></div></td><td class="px-6 py-4 text-sm text-gray-600">{item.size.clone()}"in"</td><td class="px-6 py-4 text-sm text-gray-600">{item.rolls}</td><td class="px-6 py-4 text-sm text-gray-600">{format!("{}m", item.total_metres as u64)}</td><td class="px-6 py-4 text-sm text-gray-600">{format!("{}m", item.metres_used as u64)}</td><td class="px-6 py-4 text-sm font-medium text-gray-900">{format!("{}m", rem as u64)}</td><td class="px-6 py-4 text-sm text-gray-900">{left}</td><td class="px-6 py-4"><span class=format!("status-badge {}", sc)>{sl}</span></td><td class="px-6 py-4"><div class="flex gap-2"><button on:click=move |_| { set_add_rolls_value.set(String::new()); set_add_rolls_item.set(Some(item.clone())); } class="px-3 py-1 text-xs font-medium bg-black text-white rounded-md hover:bg-gray-800 transition-colors">"Add Rolls"</button><button on:click=move |_| leptos::task::spawn_local(async move { let _=api::delete_stock(id).await; reload(); }) class="text-gray-400 hover:text-red-600 transition-colors"><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg></button></div></td></tr>}}).collect::<Vec<_>>().into_any() } }}
        </tbody></table>{move || { let n=total_items(); if n==0 {().into_any()} else { let cp=current_page.get(); let tp=total_pages(); let si=(cp-1)*items_per_page+1; let ei=(cp*items_per_page).min(n); view!{<div id="stock-pagination" class="flex items-center justify-between px-5 py-3 bg-gray-50 border-t border-gray-200"><div class="text-sm text-gray-600">"Showing "<span class="font-medium">{si}</span>" to "<span class="font-medium">{ei}</span>" of "<span class="font-medium">{n}</span>" stock items"</div><div class="flex gap-2"><button on:click=move |_| { if cp>1 {set_current_page.set(cp-1)} } class=format!("px-3 py-1 text-sm font-medium rounded-md {}", if cp==1 {"bg-gray-200 text-gray-400 cursor-not-allowed"} else {"bg-black text-white hover:bg-gray-800"}) disabled=move || cp==1>"Previous"</button><span class="px-3 py-1 text-sm font-medium text-gray-700">{format!("Page {} of {}", cp, tp)}</span><button on:click=move |_| { if cp<tp {set_current_page.set(cp+1)} } class=format!("px-3 py-1 text-sm font-medium rounded-md {}", if cp>=tp {"bg-gray-200 text-gray-400 cursor-not-allowed"} else {"bg-black text-white hover:bg-gray-800"}) disabled=move || cp>=tp>"Next"</button></div></div>}.into_any() } }}</div>
        {move || if show_add.get() { view!{<div id="modal-add-stock" class="modal-overlay open"><div class="modal-container"><div class="modal-header"><h3 class="modal-title">"Add New Stock"</h3><button class="modal-close-btn" on:click=move |_| set_show_add.set(false)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><form action="javascript:void(0);"><div class="modal-body"><div class="space-y-5"><div><label>"Sticker Type"</label><div class="flex gap-2 mt-1"><button type="button" on:click=move |_| set_sticker_type.set("colored".into()) class=move || format!("sticker-type-btn flex-1 px-4 py-2 {} font-medium text-sm transition-all", if sticker_type.get()=="colored" {"border border-brand-500 bg-brand-50 text-brand-600"} else {"border border-gray-200 bg-white text-gray-500 hover:border-gray-300"})>"Colored"</button><button type="button" on:click=move |_| set_sticker_type.set("reflective".into()) class=move || format!("sticker-type-btn flex-1 px-4 py-2 {} font-medium text-sm transition-all", if sticker_type.get()=="reflective" {"border border-brand-500 bg-brand-50 text-brand-600"} else {"border border-gray-200 bg-white text-gray-500 hover:border-gray-300"})>"Reflective"</button></div></div><div><label>"Color"</label><div class="relative mt-1"><input type="text" class="w-full pr-12" placeholder=move || if sticker_type.get()=="reflective" {"e.g. Red, White, Yellow"} else {"e.g. Red Dark, Black Matte"} autocomplete="off" prop:value=move || color.get() on:focus=move |_| set_show_suggestions.set(true) on:input=move |e| { set_color.set(event_target_value(&e)); set_show_suggestions.set(true); }/><div class="absolute right-3 top-1/2 -translate-y-1/2 w-5 h-5 border border-gray-200" style=move || format!("background-color: {};", get_hex(&color.get()))></div>{move || if show_suggestions.get() { let f=color.get().to_lowercase(); view!{<div class="absolute z-50 w-full mt-1 bg-white border border-gray-200 shadow-lg max-h-60 overflow-y-auto">{COLORS.iter().filter(|c| f.is_empty() || c.0.to_lowercase().contains(&f)).map(|(n,h,cat)| { let name=n.to_string(); view!{<div class="color-suggestion-item px-3 py-2 hover:bg-gray-100 cursor-pointer flex items-center gap-3 transition-colors" on:click=move |_| { set_color.set(name.clone()); set_show_suggestions.set(false); }><div class="w-6 h-6 rounded border border-gray-200 shadow-sm flex-shrink-0" style=format!("background: {};", h)></div><div class="flex-1 min-w-0"><div class="text-sm font-medium text-gray-900 leading-5">{*n}</div><div class="text-[11px] uppercase tracking-wide text-gray-400 leading-4">{category_label(cat)}</div></div></div>}}).collect::<Vec<_>>()}</div>}.into_any() } else {().into_any()} }</div><p class="text-xs text-gray-400 mt-1">{move || if sticker_type.get()=="reflective" {"Enter reflective color"} else {"Enter color with variant (dark, light, matte, gloss)"}}</p></div><div><label>"Size Variants"</label><div class="space-y-3 mt-2"><For each=move || rows.get() key=|r| r.id children=move |r| { let id=r.id; view!{<div class="grid grid-cols-2 gap-3" data-row-id=id><div><label>{if id==0 {"Width (inches) *"} else {"Width (inches)"}}</label><input type="number" class="w-full size-input" step="1" min="1" placeholder="e.g. 24" prop:value=r.size on:input=move |e| update_row(id,"size",event_target_value(&e))/></div><div><label>{if id==0 {"Rolls *"} else {"Rolls"}}</label><input type="number" class="w-full rolls-input" min="1" placeholder="e.g. 5" prop:value=r.rolls on:input=move |e| update_row(id,"rolls",event_target_value(&e))/></div>{move || if sticker_type.get()=="reflective" { view!{<div><label>{if id==0 {"Metres per Roll *"} else {"Metres per Roll"}}</label><input type="number" class="w-full metres-per-roll-input" step="0.1" min="1" prop:value=r.metres_per_roll.clone() on:input=move |e| update_row(id,"mpr",event_target_value(&e))/></div>}.into_any() } else {().into_any()} }<div><label>"Total Metres"</label><div class="px-3 py-2 bg-gray-50 border border-gray-200 text-gray-600 text-sm"><span class="metres-display font-medium">{move || rows.get().iter().find(|x| x.id==id).map(row_total).unwrap_or(0.0) as u64}</span>"m"</div></div>{if id!=0 {view!{<div class="col-span-2 flex justify-end"><button type="button" class="remove-row-btn text-xs text-gray-400 hover:text-red-500 font-medium" on:click=move |_| set_rows.update(|rs| rs.retain(|x| x.id!=id))>"Remove"</button></div>}.into_any()} else {().into_any()}}</div>}}/></div><button type="button" class="mt-3 text-sm text-brand-500 hover:text-brand-600 font-medium flex items-center gap-1" on:click=move |_| add_row()><svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>"Add size"</button></div><div class="bg-gray-50 border border-gray-100 p-4"><div class="flex justify-between items-center text-sm"><span class="font-medium text-gray-600">"Total"</span><div class="flex gap-6"><span class="text-gray-500"><span class="font-semibold text-gray-900">{move || total_rolls_modal()}</span>" rolls"</span><span class="text-gray-500"><span class="font-semibold text-gray-900">{move || total_metres_modal() as u64}</span>"m"</span></div></div></div></div></div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2 text-sm" on:click=move |_| set_show_add.set(false)>"Cancel"</button><button type="button" class="btn-primary px-4 py-2 text-sm" on:click=move |_| { let c=color.get(); let typ=sticker_type.get(); let data=rows.get(); set_show_add.set(false); leptos::task::spawn_local(async move { for r in data { let rolls=r.rolls.parse().unwrap_or(0); if !c.is_empty() && !r.size.is_empty() && rolls>0 { let mpr=if typ=="reflective" {r.metres_per_roll.parse().unwrap_or(50.0)} else {50.0}; let _=api::add_stock(&NewStockItem{color:c.clone(), size:r.size, sticker_type:typ.clone(), rolls, metres_per_roll:Some(mpr), total_metres:None, metres_used:0.0, custom_metres_per_roll: if typ=="reflective" {Some(mpr)} else {None}}).await; }} reload(); });}>"Add Stock"</button></div></form></div></div>}.into_any()} else {().into_any()} }
        {move || add_rolls_item.get().map(|item| { let added=move || add_rolls_value.get().parse::<i64>().unwrap_or(0); let new_rolls=move || item.rolls + added(); let new_metres=move || item.total_metres + added() as f64 * item.metres_per_roll; view!{<div id="modal-add-rolls" class="modal-overlay open"><div class="modal-container" style="max-width: 500px;"><div class="modal-header"><h3 class="modal-title">"Add Rolls to Stock"</h3><button class="modal-close-btn" on:click=move |_| set_add_rolls_item.set(None)>"×"</button></div><div class="modal-body"><div class="bg-gray-50 p-4 mb-4"><p class="text-xs text-gray-500 uppercase tracking-wide">"Stock Item"</p><p class="font-semibold text-gray-900">{format!("{} - {}\" {}", item.color, item.size, if item.sticker_type=="reflective" {"Reflective"} else {"Colored"})}</p><p class="text-sm text-gray-600 mt-1">{format!("Current: {}m remaining ({} rolls)", remaining(&item) as u64, rolls_left(&item))}</p></div><div class="space-y-4"><div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Number of Rolls to Add *"</label><input type="number" min="1" step="1" class="w-full" placeholder="Enter rolls to add" prop:value=move || add_rolls_value.get() on:input=move |e| set_add_rolls_value.set(event_target_value(&e))/><p class="text-xs text-gray-500 mt-1">{format!("Each roll = {}m", item.metres_per_roll as u64)}</p></div><div class="bg-blue-50 border border-blue-200 p-3"><p class="text-sm text-gray-700"><span class="font-medium">"New Total:"</span> {move || format!("{} rolls ({}m)", new_rolls(), new_metres() as u64)}</p></div></div></div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" on:click=move |_| set_add_rolls_item.set(None)>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" on:click=move |_| { let id=item.id; let rolls=added(); if rolls>0 { set_add_rolls_item.set(None); leptos::task::spawn_local(async move { let cur=api::get_all_stock().await.ok().and_then(|v| v.into_iter().find(|x| x.id==id)); if let Some(s)=cur { let _=api::update_stock(id, &serde_json::json!({"rolls": s.rolls + rolls, "total_metres": s.total_metres + rolls as f64 * s.metres_per_roll})).await; } reload(); }); }}>"Add Rolls"</button></div></div></div>}.into_any() }).unwrap_or_else(|| ().into_any())}
    </div> }
}

fn category_label(category: &str) -> &'static str {
    match category {
        "basic" => "Basic",
        "variant" => "Variant",
        "metallic" => "Metallic",
        _ => "Color",
    }
}

fn get_hex(name: &str) -> &'static str {
    let normalized = name
        .to_lowercase()
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if normalized.is_empty() {
        return "#9ca3af";
    }

    // Exact catalogue match first.
    if let Some((_, hex, _)) = COLORS.iter().find(|(n, _, _)| n.to_lowercase() == normalized) {
        return hex;
    }

    // Variant-aware detection, so entries like "red dark", "dark red",
    // "black matte", and "gloss white" all resolve to a realistic swatch.
    match normalized.as_str() {
        "dark red" | "red dark" | "maroon" | "burgundy" => "#991b1b",
        "light red" | "red light" => "#fca5a5",
        "dark blue" | "blue dark" | "navy" | "navy blue" => "#1e3a8a",
        "light blue" | "blue light" | "sky blue" => "#93c5fd",
        "dark green" | "green dark" | "forest green" => "#166534",
        "light green" | "green light" | "lime green" => "#86efac",
        "dark yellow" | "yellow dark" | "mustard" => "#ca8a04",
        "light yellow" | "yellow light" => "#fde68a",
        "dark orange" | "orange dark" => "#c2410c",
        "light orange" | "orange light" => "#fdba74",
        "dark purple" | "purple dark" => "#6b21a8",
        "light purple" | "purple light" | "lavender" => "#c4b5fd",
        "dark pink" | "pink dark" => "#be185d",
        "light pink" | "pink light" => "#f9a8d4",
        "matte black" | "black matte" | "gloss black" | "black gloss" => "#111827",
        "matte white" | "white matte" | "gloss white" | "white gloss" => "#ffffff",
        "gold" | "gold metallic" | "metallic gold" => "#fbbf24",
        "silver" | "silver metallic" | "metallic silver" | "grey" | "gray" => "#9ca3af",
        _ => {
            let words: Vec<&str> = normalized.split_whitespace().collect();
            for (color, hex, _) in COLORS {
                let color_l = color.to_lowercase();
                if words.iter().any(|w| *w == color_l) || normalized.contains(&color_l) {
                    return hex;
                }
            }
            "#9ca3af"
        }
    }
}
