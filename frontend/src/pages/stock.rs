use leptos::prelude::*;
use crate::api::{self, NewStockItem, StockItem};
use log::error;

#[derive(Clone, Debug, PartialEq)]
struct SizeRow { id: u32, size: String, rolls: String, metres_per_roll: String }

struct ColorEntry {
    name: &'static str,
    hex: &'static str,
    category: &'static str,
    oracal_code: Option<&'static str>,
}

const BASE_COLORS: &[ColorEntry] = &[
    ColorEntry { name: "White", hex: "#FFFFFF", category: "neutral", oracal_code: Some("010") },
    ColorEntry { name: "Soft White", hex: "#F5F5F0", category: "neutral", oracal_code: Some("011") },
    ColorEntry { name: "Creme", hex: "#F5F5DC", category: "neutral", oracal_code: Some("012") },
    ColorEntry { name: "Ivory", hex: "#FFFFF0", category: "neutral", oracal_code: Some("013") },
    ColorEntry { name: "Beige", hex: "#F5DEB3", category: "neutral", oracal_code: Some("014") },
    ColorEntry { name: "Cream", hex: "#FFFDD0", category: "neutral", oracal_code: None },
    ColorEntry { name: "Black", hex: "#1A1A1A", category: "neutral", oracal_code: Some("070") },
    ColorEntry { name: "Grey", hex: "#808080", category: "neutral", oracal_code: Some("071") },
    ColorEntry { name: "Light Grey", hex: "#D3D3D3", category: "neutral", oracal_code: Some("072") },
    ColorEntry { name: "Dark Grey", hex: "#404040", category: "neutral", oracal_code: Some("073") },
    ColorEntry { name: "Charcoal", hex: "#36454F", category: "neutral", oracal_code: Some("074") },
    ColorEntry { name: "Slate", hex: "#708090", category: "neutral", oracal_code: Some("075") },
    ColorEntry { name: "Silver", hex: "#C0C0C0", category: "neutral", oracal_code: Some("076") },
    ColorEntry { name: "Anthracite", hex: "#303030", category: "neutral", oracal_code: Some("077") },
    ColorEntry { name: "Yellow", hex: "#FFD700", category: "yellow", oracal_code: Some("020") },
    ColorEntry { name: "Lemon Yellow", hex: "#FFF44F", category: "yellow", oracal_code: Some("021") },
    ColorEntry { name: "Bright Yellow", hex: "#FFEA00", category: "yellow", oracal_code: Some("022") },
    ColorEntry { name: "Canary Yellow", hex: "#FFEF00", category: "yellow", oracal_code: Some("023") },
    ColorEntry { name: "Golden Yellow", hex: "#FFDF00", category: "yellow", oracal_code: Some("024") },
    ColorEntry { name: "Brimstone Yellow", hex: "#E6B800", category: "yellow", oracal_code: Some("025") },
    ColorEntry { name: "Signal Yellow", hex: "#F7BA0B", category: "yellow", oracal_code: Some("019") },
    ColorEntry { name: "Saffron", hex: "#F4C430", category: "yellow", oracal_code: None },
    ColorEntry { name: "Amber", hex: "#FFBF00", category: "yellow", oracal_code: None },
    ColorEntry { name: "Gold", hex: "#FFD700", category: "yellow", oracal_code: Some("024") },
    ColorEntry { name: "Dark Yellow", hex: "#DAA520", category: "yellow", oracal_code: None },
    ColorEntry { name: "Orange", hex: "#FF6600", category: "orange", oracal_code: Some("034") },
    ColorEntry { name: "Bright Orange", hex: "#FF6B00", category: "orange", oracal_code: Some("034") },
    ColorEntry { name: "Tangerine", hex: "#FF9966", category: "orange", oracal_code: Some("034") },
    ColorEntry { name: "Burnt Orange", hex: "#CC5500", category: "orange", oracal_code: Some("035") },
    ColorEntry { name: "Pastel Orange", hex: "#FFB366", category: "orange", oracal_code: Some("035") },
    ColorEntry { name: "Red Orange", hex: "#FF5349", category: "orange", oracal_code: Some("036") },
    ColorEntry { name: "Deep Orange", hex: "#E65C00", category: "orange", oracal_code: None },
    ColorEntry { name: "Peach", hex: "#FFCBA4", category: "orange", oracal_code: None },
    ColorEntry { name: "Apricot", hex: "#FBCEB1", category: "orange", oracal_code: None },
    ColorEntry { name: "Red", hex: "#E60012", category: "red", oracal_code: Some("030") },
    ColorEntry { name: "Bright Red", hex: "#FF0000", category: "red", oracal_code: Some("030") },
    ColorEntry { name: "Dark Red", hex: "#8B0000", category: "red", oracal_code: Some("031") },
    ColorEntry { name: "Ruby Red", hex: "#9B111E", category: "red", oracal_code: Some("032") },
    ColorEntry { name: "Cherry Red", hex: "#DE3163", category: "red", oracal_code: Some("033") },
    ColorEntry { name: "Burgundy", hex: "#800020", category: "red", oracal_code: None },
    ColorEntry { name: "Maroon", hex: "#800000", category: "red", oracal_code: None },
    ColorEntry { name: "Wine", hex: "#722F37", category: "red", oracal_code: None },
    ColorEntry { name: "Crimson", hex: "#DC143C", category: "red", oracal_code: None },
    ColorEntry { name: "Scarlet", hex: "#FF2400", category: "red", oracal_code: None },
    ColorEntry { name: "Brick Red", hex: "#B22222", category: "red", oracal_code: None },
    ColorEntry { name: "Signal Red", hex: "#CE1126", category: "red", oracal_code: Some("030") },
    ColorEntry { name: "Fire Red", hex: "#FF4500", category: "red", oracal_code: None },
    ColorEntry { name: "Oxblood", hex: "#660000", category: "red", oracal_code: None },
    ColorEntry { name: "Pink", hex: "#FF69B4", category: "pink", oracal_code: Some("041") },
    ColorEntry { name: "Light Pink", hex: "#FFB6C1", category: "pink", oracal_code: Some("041") },
    ColorEntry { name: "Hot Pink", hex: "#FF69B4", category: "pink", oracal_code: Some("041") },
    ColorEntry { name: "Rose", hex: "#FF007F", category: "pink", oracal_code: Some("042") },
    ColorEntry { name: "Salmon", hex: "#FA8072", category: "pink", oracal_code: Some("043") },
    ColorEntry { name: "Coral", hex: "#FF7F50", category: "pink", oracal_code: Some("044") },
    ColorEntry { name: "Magenta", hex: "#FF00FF", category: "pink", oracal_code: Some("045") },
    ColorEntry { name: "Fuchsia", hex: "#FF00FF", category: "pink", oracal_code: Some("045") },
    ColorEntry { name: "Raspberry", hex: "#E30B5C", category: "pink", oracal_code: None },
    ColorEntry { name: "Mauve", hex: "#E0B0FF", category: "pink", oracal_code: None },
    ColorEntry { name: "Blush", hex: "#DE5D83", category: "pink", oracal_code: None },
    ColorEntry { name: "Purple", hex: "#800080", category: "purple", oracal_code: Some("050") },
    ColorEntry { name: "Violet", hex: "#8B5CF6", category: "purple", oracal_code: Some("051") },
    ColorEntry { name: "Lavender", hex: "#E6E6FA", category: "purple", oracal_code: Some("043") },
    ColorEntry { name: "Lilac", hex: "#C8A2C8", category: "purple", oracal_code: Some("052") },
    ColorEntry { name: "Plum", hex: "#DDA0DD", category: "purple", oracal_code: Some("053") },
    ColorEntry { name: "Dark Purple", hex: "#301934", category: "purple", oracal_code: Some("054") },
    ColorEntry { name: "Light Purple", hex: "#B19CD9", category: "purple", oracal_code: Some("055") },
    ColorEntry { name: "Indigo", hex: "#4B0082", category: "purple", oracal_code: Some("056") },
    ColorEntry { name: "Amethyst", hex: "#9966CC", category: "purple", oracal_code: None },
    ColorEntry { name: "Grape", hex: "#6F2DA8", category: "purple", oracal_code: None },
    ColorEntry { name: "Royal Purple", hex: "#7851A9", category: "purple", oracal_code: None },
    ColorEntry { name: "Purple Red", hex: "#960018", category: "purple", oracal_code: Some("026") },
    ColorEntry { name: "Gentian Blue", hex: "#4B0082", category: "purple", oracal_code: Some("051") },
    ColorEntry { name: "Blue", hex: "#0066CC", category: "blue", oracal_code: Some("060") },
    ColorEntry { name: "Light Blue", hex: "#ADD8E6", category: "blue", oracal_code: Some("061") },
    ColorEntry { name: "Dark Blue", hex: "#00008B", category: "blue", oracal_code: Some("062") },
    ColorEntry { name: "Navy", hex: "#000080", category: "blue", oracal_code: Some("063") },
    ColorEntry { name: "Navy Blue", hex: "#000080", category: "blue", oracal_code: Some("063") },
    ColorEntry { name: "Royal Blue", hex: "#4169E1", category: "blue", oracal_code: Some("064") },
    ColorEntry { name: "Sky Blue", hex: "#87CEEB", category: "blue", oracal_code: Some("065") },
    ColorEntry { name: "Ice Blue", hex: "#D6EAF8", category: "blue", oracal_code: Some("056") },
    ColorEntry { name: "Baby Blue", hex: "#89CFF0", category: "blue", oracal_code: Some("061") },
    ColorEntry { name: "Azure", hex: "#007FFF", category: "blue", oracal_code: Some("052") },
    ColorEntry { name: "Cobalt", hex: "#0047AB", category: "blue", oracal_code: Some("066") },
    ColorEntry { name: "Sapphire", hex: "#0F52BA", category: "blue", oracal_code: None },
    ColorEntry { name: "Teal", hex: "#008080", category: "blue", oracal_code: Some("067") },
    ColorEntry { name: "Turquoise", hex: "#40E0D0", category: "blue", oracal_code: Some("054") },
    ColorEntry { name: "Cyan", hex: "#00FFFF", category: "blue", oracal_code: Some("055") },
    ColorEntry { name: "Aqua", hex: "#00FFFF", category: "blue", oracal_code: Some("055") },
    ColorEntry { name: "King Blue", hex: "#0047AB", category: "blue", oracal_code: Some("049") },
    ColorEntry { name: "Gentian", hex: "#1A237E", category: "blue", oracal_code: Some("051") },
    ColorEntry { name: "Midnight Blue", hex: "#191970", category: "blue", oracal_code: None },
    ColorEntry { name: "Steel Blue", hex: "#4682B4", category: "blue", oracal_code: None },
    ColorEntry { name: "Powder Blue", hex: "#B0E0E6", category: "blue", oracal_code: None },
    ColorEntry { name: "Green", hex: "#00B050", category: "green", oracal_code: Some("080") },
    ColorEntry { name: "Light Green", hex: "#90EE90", category: "green", oracal_code: Some("081") },
    ColorEntry { name: "Dark Green", hex: "#006400", category: "green", oracal_code: Some("082") },
    ColorEntry { name: "Forest Green", hex: "#228B22", category: "green", oracal_code: Some("083") },
    ColorEntry { name: "Lime", hex: "#32CD32", category: "green", oracal_code: Some("084") },
    ColorEntry { name: "Lime Green", hex: "#32CD32", category: "green", oracal_code: Some("084") },
    ColorEntry { name: "Emerald", hex: "#50C878", category: "green", oracal_code: Some("085") },
    ColorEntry { name: "Emerald Green", hex: "#50C878", category: "green", oracal_code: Some("085") },
    ColorEntry { name: "Olive", hex: "#808000", category: "green", oracal_code: Some("086") },
    ColorEntry { name: "Olive Green", hex: "#808000", category: "green", oracal_code: Some("086") },
    ColorEntry { name: "Mint", hex: "#98FF98", category: "green", oracal_code: Some("055") },
    ColorEntry { name: "Mint Green", hex: "#98FF98", category: "green", oracal_code: Some("055") },
    ColorEntry { name: "Sea Green", hex: "#2E8B57", category: "green", oracal_code: None },
    ColorEntry { name: "Hunter Green", hex: "#355E3B", category: "green", oracal_code: None },
    ColorEntry { name: "Sage", hex: "#9DC183", category: "green", oracal_code: None },
    ColorEntry { name: "Grass Green", hex: "#7CFC00", category: "green", oracal_code: None },
    ColorEntry { name: "Kelly Green", hex: "#4CBB17", category: "green", oracal_code: None },
    ColorEntry { name: "Moss Green", hex: "#8A9A5B", category: "green", oracal_code: None },
    ColorEntry { name: "Jade", hex: "#00A86B", category: "green", oracal_code: None },
    ColorEntry { name: "Pine Green", hex: "#01796F", category: "green", oracal_code: None },
    ColorEntry { name: "Signal Green", hex: "#00B050", category: "green", oracal_code: Some("080") },
    ColorEntry { name: "Brown", hex: "#8B4513", category: "brown", oracal_code: Some("083") },
    ColorEntry { name: "Light Brown", hex: "#CD853F", category: "brown", oracal_code: None },
    ColorEntry { name: "Dark Brown", hex: "#3D2314", category: "brown", oracal_code: None },
    ColorEntry { name: "Tan", hex: "#D2B48C", category: "brown", oracal_code: None },
    ColorEntry { name: "Chocolate", hex: "#7B3F00", category: "brown", oracal_code: None },
    ColorEntry { name: "Coffee", hex: "#6F4E37", category: "brown", oracal_code: None },
    ColorEntry { name: "Caramel", hex: "#FFD59A", category: "brown", oracal_code: None },
    ColorEntry { name: "Camel", hex: "#C19A6B", category: "brown", oracal_code: None },
    ColorEntry { name: "Cocoa", hex: "#D2691E", category: "brown", oracal_code: None },
    ColorEntry { name: "Copper", hex: "#B87333", category: "brown", oracal_code: None },
    ColorEntry { name: "Bronze", hex: "#CD7F32", category: "brown", oracal_code: None },
    ColorEntry { name: "Rust", hex: "#B7410E", category: "brown", oracal_code: None },
    ColorEntry { name: "Mahogany", hex: "#C04000", category: "brown", oracal_code: None },
    ColorEntry { name: "Nut Brown", hex: "#6B4423", category: "brown", oracal_code: Some("083") },
    ColorEntry { name: "Sand", hex: "#C2B280", category: "brown", oracal_code: None },
    ColorEntry { name: "Khaki", hex: "#F0E68C", category: "brown", oracal_code: None },
    ColorEntry { name: "Metallic Gold", hex: "#D4AF37", category: "metallic", oracal_code: Some("024") },
    ColorEntry { name: "Metallic Silver", hex: "#C0C0C0", category: "metallic", oracal_code: Some("076") },
    ColorEntry { name: "Metallic Bronze", hex: "#CD7F32", category: "metallic", oracal_code: None },
    ColorEntry { name: "Metallic Copper", hex: "#B87333", category: "metallic", oracal_code: None },
    ColorEntry { name: "Chrome", hex: "#E8E8E8", category: "metallic", oracal_code: None },
    ColorEntry { name: "Mirror Gold", hex: "#FFD700", category: "metallic", oracal_code: None },
    ColorEntry { name: "Mirror Silver", hex: "#C0C0C0", category: "metallic", oracal_code: None },
    ColorEntry { name: "Fluorescent Yellow", hex: "#CCFF00", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Neon Yellow", hex: "#CCFF00", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Fluorescent Orange", hex: "#FF6600", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Neon Orange", hex: "#FF6600", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Fluorescent Red", hex: "#FF3131", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Neon Red", hex: "#FF3131", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Fluorescent Green", hex: "#39FF14", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Neon Green", hex: "#39FF14", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Fluorescent Pink", hex: "#FF6EFF", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Neon Pink", hex: "#FF6EFF", category: "fluorescent", oracal_code: None },
    ColorEntry { name: "Dayglo Orange", hex: "#FF5F00", category: "fluorescent", oracal_code: None },
];

const REFLECTIVE_COLORS: &[ColorEntry] = &[
    ColorEntry { name: "White", hex: "#FFFFFF", category: "reflective", oracal_code: None },
    ColorEntry { name: "Yellow", hex: "#FFD700", category: "reflective", oracal_code: None },
    ColorEntry { name: "Red", hex: "#E60012", category: "reflective", oracal_code: None },
    ColorEntry { name: "Orange", hex: "#FF6600", category: "reflective", oracal_code: None },
    ColorEntry { name: "Green", hex: "#00B050", category: "reflective", oracal_code: None },
    ColorEntry { name: "Blue", hex: "#0066CC", category: "reflective", oracal_code: None },
    ColorEntry { name: "Fluorescent Yellow", hex: "#CCFF00", category: "reflective", oracal_code: None },
    ColorEntry { name: "Fluorescent Orange", hex: "#FF6600", category: "reflective", oracal_code: None },
    ColorEntry { name: "Fluorescent Lime", hex: "#39FF14", category: "reflective", oracal_code: None },
];

const SYNONYMS: &[(&str, &str)] = &[
    ("pure white", "white"), ("bright white", "white"), ("snow white", "white"), ("paper white", "white"),
    ("jet black", "black"), ("matte black", "black"), ("gloss black", "black"),
    ("light grey", "light gray"), ("dark grey", "dark gray"), ("pewter", "slate"), ("graphite", "charcoal"),
    ("baby blue", "light blue"), ("powder blue", "light blue"), ("midnight", "navy"), ("cornflower", "azure"), ("electric blue", "bright blue"),
    ("fire engine red", "bright red"), ("blood red", "dark red"), ("cherry", "cherry red"),
    ("grass", "grass green"), ("lime", "lime green"), ("pine", "forest green"),
    ("canary", "canary yellow"), ("buttercup", "bright yellow"), ("dandelion", "golden yellow"),
    ("royal purple", "purple"), ("deep purple", "dark purple"), ("ultra violet", "violet"), ("electric purple", "purple"),
    ("bubblegum", "hot pink"), ("rose pink", "rose"), ("blush pink", "blush"),
    ("pumpkin", "orange"), ("carrot", "orange"),
    ("espresso", "dark brown"), ("latte", "light brown"), ("mocha", "coffee"), ("chocolate brown", "chocolate"), ("sienna", "rust"), ("terracotta", "rust"),
    ("gold metallic", "metallic gold"), ("silver metallic", "metallic silver"), ("copper metallic", "metallic copper"), ("bronze metallic", "metallic bronze"),
    ("grey", "gray"), ("grey dark", "dark grey"), ("grey light", "light grey"),
    ("collared", "colored"), ("matt black", "matte black"), ("mate black", "matte black"), ("mate", "matte"), ("glos", "gloss"), ("metalic", "metallic"),
    ("florescent", "fluorescent"), ("flourescent", "fluorescent"), ("reflectve", "reflective"), ("refelctive", "reflective"),
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

    let reload = move || leptos::task::spawn_local(async move {
        match api::get_all_stock().await {
            Ok(d) => { error!("get_all_stock returned {} items", d.len()); set_stock.set(d); }
            Err(e) => error!("get_all_stock failed: {}", e),
        }
    });
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

    // ---- Actions ----
    let add_stock_action = {
        let set_stock = set_stock;
        move |color: String, sticker_type: String, rows: Vec<SizeRow>| {
            leptos::task::spawn_local(async move {
                for r in &rows {
                    let rolls = r.rolls.parse::<i64>().unwrap_or(0);
                    if !color.is_empty() && !r.size.is_empty() && rolls > 0 {
                        let mpr = if sticker_type == "reflective" { r.metres_per_roll.parse::<f64>().unwrap_or(50.0) } else { 50.0 };
                        if let Err(e) = api::add_stock(&NewStockItem {
                            color: color.clone(), size: r.size.clone(), sticker_type: sticker_type.clone(),
                            rolls, metres_per_roll: Some(mpr), total_metres: None, metres_used: 0.0,
                            custom_metres_per_roll: if sticker_type == "reflective" { Some(mpr) } else { None },
                        }).await { error!("add_stock failed: {}", e); }
                    }
                }
                match api::get_all_stock().await {
                    Ok(d) => { set_stock.set(d); }
                    Err(e) => error!("get_all_stock after add failed: {}", e),
                }
                set_show_add.set(false);
            });
        }
    };
    let delete_stock_action = {
        let set_stock = set_stock;
        move |id: i64| {
            leptos::task::spawn_local(async move {
                let _ = api::delete_stock(id).await;
                if let Ok(d) = api::get_all_stock().await { set_stock.set(d); }
            });
        }
    };
    let add_rolls_action = {
        let set_stock = set_stock;
        move |id: i64, rolls: i64| {
            leptos::task::spawn_local(async move {
                if let Ok(all) = api::get_all_stock().await {
                    if let Some(s) = all.into_iter().find(|x| x.id == id) {
                        let _ = api::update_stock(id, &serde_json::json!({
                            "rolls": s.rolls + rolls,
                            "total_metres": s.total_metres + rolls as f64 * s.metres_per_roll
                        })).await;
                    }
                }
                if let Ok(d) = api::get_all_stock().await { set_stock.set(d); }
            });
        }
    };

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
            {move || { let items = paginated(); if items.is_empty() { view!{<tr class="text-center"><td colspan="10" class="px-5 py-8 text-gray-500"><div class="flex flex-col items-center justify-center"><svg class="w-12 h-12 text-gray-300 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path></svg><p>"No stock added yet"</p><button on:click=move |_| { reset_add_modal(); set_show_add.set(true); } class="text-black font-semibold hover:underline text-sm mt-2">"Add your first stock"</button></div></td></tr>}.into_any() } else { items.into_iter().map(|item| { let id=item.id; let rem=remaining(&item); let left=rolls_left(&item); let (sl,sc)=status(&item); let typ=item.sticker_type.clone(); view!{<tr class="hover:bg-gray-50 transition-colors"><td class="px-6 py-4"><span class=format!("inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium {}", if typ=="reflective" {"bg-purple-50 text-purple-700"} else {"bg-blue-50 text-blue-700"})>{if typ=="reflective" {"Reflective"} else {"Colored"}}</span></td><td class="px-6 py-4"><div class="flex items-center gap-3"><div class="w-8 h-8 rounded-lg border border-gray-200 shadow-sm" style=format!("background-color: {}", get_hex(&item.color))></div><span class="text-sm font-medium text-gray-900">{item.color.clone()}</span></div></td><td class="px-6 py-4 text-sm text-gray-600">{item.size.clone()}"in"</td><td class="px-6 py-4 text-sm text-gray-600">{item.rolls}</td><td class="px-6 py-4 text-sm text-gray-600">{format!("{}m", item.total_metres as u64)}</td><td class="px-6 py-4 text-sm text-gray-600">{format!("{}m", item.metres_used as u64)}</td><td class="px-6 py-4 text-sm font-medium text-gray-900">{format!("{}m", rem as u64)}</td><td class="px-6 py-4 text-sm text-gray-900">{left}</td><td class="px-6 py-4"><span class=format!("status-badge {}", sc)>{sl}</span></td><td class="px-6 py-4"><div class="flex gap-2"><button on:click=move |_| { set_add_rolls_value.set(String::new()); set_add_rolls_item.set(Some(item.clone())); } class="px-3 py-1 text-xs font-medium bg-black text-white rounded-md hover:bg-gray-800 transition-colors">"Add Rolls"</button><button on:click=move |_| { delete_stock_action(id); } class="text-gray-400 hover:text-red-600 transition-colors"><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg></button></div></td></tr>}}).collect::<Vec<_>>().into_any() } }}
        </tbody></table>{move || { let n=total_items(); if n==0 {().into_any()} else { let cp=current_page.get(); let tp=total_pages(); let si=(cp-1)*items_per_page+1; let ei=(cp*items_per_page).min(n); view!{<div id="stock-pagination" class="flex items-center justify-between px-5 py-3 bg-gray-50 border-t border-gray-200"><div class="text-sm text-gray-600">"Showing "<span class="font-medium">{si}</span>" to "<span class="font-medium">{ei}</span>" of "<span class="font-medium">{n}</span>" stock items"</div><div class="flex gap-2"><button on:click=move |_| { if cp>1 {set_current_page.set(cp-1)} } class=format!("px-3 py-1 text-sm font-medium rounded-md {}", if cp==1 {"bg-gray-200 text-gray-400 cursor-not-allowed"} else {"bg-black text-white hover:bg-gray-800"}) disabled=move || cp==1>"Previous"</button><span class="px-3 py-1 text-sm font-medium text-gray-700">{format!("Page {} of {}", cp, tp)}</span><button on:click=move |_| { if cp<tp {set_current_page.set(cp+1)} } class=format!("px-3 py-1 text-sm font-medium rounded-md {}", if cp>=tp {"bg-gray-200 text-gray-400 cursor-not-allowed"} else {"bg-black text-white hover:bg-gray-800"}) disabled=move || tp <= cp>"Next"</button></div></div>}.into_any() } }}</div>
        {move || if show_add.get() { view!{<div id="modal-add-stock" class="modal-overlay open"><div class="modal-container"><div class="modal-header"><h3 class="modal-title">"Add New Stock"</h3><button class="modal-close-btn" on:click=move |_| set_show_add.set(false)><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg></button></div><form action="javascript:void(0);"><div class="modal-body"><div class="space-y-5"><div><label>"Sticker Type"</label><div class="flex gap-2 mt-1"><button type="button" on:click=move |_| set_sticker_type.set("colored".into()) class=move || format!("sticker-type-btn flex-1 px-4 py-2 {} font-medium text-sm transition-all", if sticker_type.get()=="colored" {"border border-brand-500 bg-brand-50 text-brand-600"} else {"border border-gray-200 bg-white text-gray-500 hover:border-gray-300"})>"Colored"</button><button type="button" on:click=move |_| set_sticker_type.set("reflective".into()) class=move || format!("sticker-type-btn flex-1 px-4 py-2 {} font-medium text-sm transition-all", if sticker_type.get()=="reflective" {"border border-brand-500 bg-brand-50 text-brand-600"} else {"border border-gray-200 bg-white text-gray-500 hover:border-gray-300"})>"Reflective"</button></div></div><div><label>"Color"</label><div class="relative mt-1"><input type="text" class="w-full pr-12" placeholder=move || if sticker_type.get()=="reflective" {"e.g. Red, White, Yellow"} else {"e.g. Red Dark, Black Matte"} autocomplete="off" prop:value=move || color.get() on:focus=move |_| set_show_suggestions.set(true) on:input=move |e| { set_color.set(event_target_value(&e)); set_show_suggestions.set(true); }/><div class="absolute right-3 top-1/2 -translate-y-1/2 w-5 h-5 border border-gray-200" style=move || format!("background-color: {};", get_hex(&color.get()))></div>{move || if show_suggestions.get() { let f=color.get().to_lowercase(); let typ=sticker_type.get(); let suggestions=get_suggestions(&f, &typ, 20); if suggestions.is_empty() { ().into_any() } else { let original_input = color.get(); view!{<div class="absolute z-50 w-full mt-1 bg-white border border-gray-200 shadow-lg max-h-60 overflow-y-auto">{suggestions.into_iter().map(|(n,h,cat,oracal)| { let name=n.to_string(); let orig = original_input.clone(); view!{<div class="color-suggestion-item px-3 py-2 hover:bg-gray-100 cursor-pointer flex items-center gap-3 transition-colors" on:click=move |_| { let final_color = preserve_modifiers(&orig, &name); set_color.set(final_color); set_show_suggestions.set(false); }><div class="w-6 h-6 rounded border border-gray-200 shadow-sm flex-shrink-0" style=format!("background: {};", h)></div><div class="flex-1 min-w-0"><div class="text-sm font-medium text-gray-900 leading-5">{n}</div><div class="text-[11px] uppercase tracking-wide text-gray-400 leading-4">{category_label(cat)}</div></div>{oracal.map(|code| view!{<div class="text-[10px] text-gray-400 whitespace-nowrap">"ORACAL " {code}</div>})}</div>}}).collect::<Vec<_>>()}</div>}.into_any() } } else {().into_any()} }</div><p class="text-xs text-gray-400 mt-1">{move || if sticker_type.get()=="reflective" {"Enter reflective color"} else {"Enter color with variant (dark, light, matte, gloss)"}}</p></div><div><label>"Size Variants"</label><div class="space-y-3 mt-2"><For each=move || rows.get() key=|r| r.id children=move |r| { let id=r.id; view!{<div class="grid grid-cols-2 gap-3" data-row-id=id><div><label>{if id==0 {"Width (inches) *"} else {"Width (inches)"}}</label><input type="number" class="w-full size-input" step="1" min="1" placeholder="e.g. 24" prop:value=r.size on:input=move |e| update_row(id,"size",event_target_value(&e))/></div><div><label>{if id==0 {"Rolls *"} else {"Rolls"}}</label><input type="number" class="w-full rolls-input" min="1" placeholder="e.g. 5" prop:value=r.rolls on:input=move |e| update_row(id,"rolls",event_target_value(&e))/></div>{move || if sticker_type.get()=="reflective" { view!{<div><label>{if id==0 {"Metres per Roll *"} else {"Metres per Roll"}}</label><input type="number" class="w-full metres-per-roll-input" step="0.1" min="1" prop:value=r.metres_per_roll.clone() on:input=move |e| update_row(id,"mpr",event_target_value(&e))/></div>}.into_any() } else {().into_any()} }<div><label>"Total Metres"</label><div class="px-3 py-2 bg-gray-50 border border-gray-200 text-gray-600 text-sm"><span class="metres-display font-medium">{move || rows.get().iter().find(|x| x.id==id).map(row_total).unwrap_or(0.0) as u64}</span>"m"</div></div>{if id!=0 {view!{<div class="col-span-2 flex justify-end"><button type="button" class="remove-row-btn text-xs text-gray-400 hover:text-red-500 font-medium" on:click=move |_| set_rows.update(|rs| rs.retain(|x| x.id!=id))>"Remove"</button></div>}.into_any()} else {().into_any()}}</div>}}/></div><button type="button" class="mt-3 text-sm text-brand-500 hover:text-brand-600 font-medium flex items-center gap-1" on:click=move |_| add_row()><svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>"Add size"</button></div><div class="bg-gray-50 border border-gray-100 p-4"><div class="flex justify-between items-center text-sm"><span class="font-medium text-gray-600">"Total"</span><div class="flex gap-6"><span class="text-gray-500"><span class="font-semibold text-gray-900">{move || total_rolls_modal()}</span>" rolls"</span><span class="text-gray-500"><span class="font-semibold text-gray-900">{move || total_metres_modal() as u64}</span>"m"</span></div></div></div></div></div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2 text-sm" on:click=move |_| set_show_add.set(false)>"Cancel"</button><button type="button" class="btn-primary px-4 py-2 text-sm" on:click=move |_| { add_stock_action(color.get(), sticker_type.get(), rows.get()); } >"Add Stock"</button></div></form></div></div>}.into_any()} else {().into_any()} }
        {move || add_rolls_item.get().map(|item| { let added=move || add_rolls_value.get().parse::<i64>().unwrap_or(0); let new_rolls=move || item.rolls + added(); let new_metres=move || item.total_metres + added() as f64 * item.metres_per_roll; view!{<div id="modal-add-rolls" class="modal-overlay open"><div class="modal-container" style="max-width: 500px;"><div class="modal-header"><h3 class="modal-title">"Add Rolls to Stock"</h3><button class="modal-close-btn" on:click=move |_| set_add_rolls_item.set(None)>"×"</button></div><div class="modal-body"><div class="bg-gray-50 p-4 mb-4"><p class="text-xs text-gray-500 uppercase tracking-wide">"Stock Item"</p><p class="font-semibold text-gray-900">{format!("{} - {}\" {}", item.color, item.size, if item.sticker_type=="reflective" {"Reflective"} else {"Colored"})}</p><p class="text-sm text-gray-600 mt-1">{format!("Current: {}m remaining ({} rolls)", remaining(&item) as u64, rolls_left(&item))}</p></div><div class="space-y-4"><div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Number of Rolls to Add *"</label><input type="number" min="1" step="1" class="w-full" placeholder="Enter rolls to add" prop:value=move || add_rolls_value.get() on:input=move |e| set_add_rolls_value.set(event_target_value(&e))/><p class="text-xs text-gray-500 mt-1">{format!("Each roll = {}m", item.metres_per_roll as u64)}</p></div><div class="bg-blue-50 border border-blue-200 p-3"><p class="text-sm text-gray-700"><span class="font-medium">"New Total:"</span> {move || format!("{} rolls ({}m)", new_rolls(), new_metres() as u64)}</p></div></div></div><div class="modal-footer"><button type="button" class="btn-secondary px-4 py-2" on:click=move |_| set_add_rolls_item.set(None)>"Cancel"</button><button type="button" class="btn-primary px-4 py-2" on:click=move |_| { let id=item.id; let rolls=added(); if rolls>0 { add_rolls_action(id, rolls); } }>"Add Rolls"</button></div></div></div>}.into_any() }).unwrap_or_else(|| ().into_any())}
    </div> }
}

fn category_label(category: &str) -> &'static str {
    match category {
        "neutral" => "Neutral",
        "red" => "Red",
        "yellow" => "Yellow",
        "orange" => "Orange",
        "pink" => "Pink",
        "purple" => "Purple",
        "blue" => "Blue",
        "green" => "Green",
        "brown" => "Brown",
        "metallic" => "Metallic",
        "fluorescent" => "Fluorescent",
        "reflective" => "Reflective",
        _ => "Color",
    }
}

fn resolve_synonym(input: &str) -> &str {
    let lower = input.to_lowercase();
    for (from, to) in SYNONYMS {
        if *from == lower.as_str() {
            return to;
        }
    }
    input
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a.as_bytes()[i - 1] == b.as_bytes()[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i - 1][j] + 1,
                    matrix[i][j - 1] + 1,
                ),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }
    matrix[a_len][b_len]
}

fn adjust_brightness(hex: &str, percent: i32) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return format!("#{}", hex);
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as i32;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as i32;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as i32;

    let (r, g, b) = if percent > 0 {
        (
            std::cmp::min(255, r + ((255 - r) * percent / 100)),
            std::cmp::min(255, g + ((255 - g) * percent / 100)),
            std::cmp::min(255, b + ((255 - b) * percent / 100)),
        )
    } else {
        (
            std::cmp::max(0, r + (r * percent / 100)),
            std::cmp::max(0, g + (g * percent / 100)),
            std::cmp::max(0, b + (b * percent / 100)),
        )
    };

    format!("#{:02X}{:02X}{:02X}", r as u8, g as u8, b as u8)
}

fn parse_color_with_modifiers(input: &str) -> &'static str {
    let normalized = input
        .to_lowercase()
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if normalized.is_empty() {
        return "#9ca3af";
    }

    let resolved = resolve_synonym(&normalized);

    // Try exact match first
    for entry in BASE_COLORS.iter().chain(REFLECTIVE_COLORS.iter()) {
        if entry.name.to_lowercase() == resolved {
            return entry.hex;
        }
    }

    // Parse modifiers
    let modifiers = ["dark", "light", "bright", "pale", "deep", "pastel", "medium", "vivid"];
    let finish_modifiers = ["gloss", "glossy", "matte", "matt", "satin", "metallic", "chrome", "mirror", "pearl", "fluorescent", "neon"];

    let words: Vec<&str> = resolved.split_whitespace().collect();

    // Check for modifier + color pattern (e.g., "dark red", "light blue")
    for (i, word) in words.iter().enumerate() {
        if modifiers.contains(word) {
            // Find the color part
            let color_part: String = words.iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, w)| *w)
                .collect::<Vec<_>>()
                .join(" ");

            // Try to find the color
            for entry in BASE_COLORS.iter().chain(REFLECTIVE_COLORS.iter()) {
                if entry.name.to_lowercase() == color_part {
                    let adjustment = match *word {
                        "dark" | "deep" => -30,
                        "light" | "pale" | "pastel" => 30,
                        "bright" | "vivid" => 10,
                        "medium" => 0,
                        _ => 0,
                    };
                    let adjusted = adjust_brightness(entry.hex, adjustment);
                    return Box::leak(adjusted.into_boxed_str());
                }
            }
        }
    }

    // Check for color + modifier pattern (e.g., "red dark", "blue light")
    for (i, word) in words.iter().enumerate() {
        if modifiers.contains(word) || finish_modifiers.contains(word) {
            let color_part: String = words.iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, w)| *w)
                .collect::<Vec<_>>()
                .join(" ");

            for entry in BASE_COLORS.iter().chain(REFLECTIVE_COLORS.iter()) {
                if entry.name.to_lowercase() == color_part {
                    if modifiers.contains(word) {
                        let adjustment = match *word {
                            "dark" | "deep" => -30,
                            "light" | "pale" | "pastel" => 30,
                            "bright" | "vivid" => 10,
                            "medium" => 0,
                            _ => 0,
                        };
                        let adjusted = adjust_brightness(entry.hex, adjustment);
                        return Box::leak(adjusted.into_boxed_str());
                    }
                    return entry.hex;
                }
            }
        }
    }

    // Word match fallback
    for entry in BASE_COLORS.iter().chain(REFLECTIVE_COLORS.iter()) {
        let name_l = entry.name.to_lowercase();
        if words.iter().any(|w| *w == name_l) || resolved.contains(&name_l) {
            return entry.hex;
        }
    }

    // Fuzzy match fallback
    let mut best_match = None;
    let mut best_distance = usize::MAX;
    for entry in BASE_COLORS.iter().chain(REFLECTIVE_COLORS.iter()) {
        let distance = levenshtein_distance(&resolved, &entry.name.to_lowercase());
        if distance < best_distance && distance <= 2 {
            best_distance = distance;
            best_match = Some(entry.hex);
        }
    }

    best_match.unwrap_or("#9ca3af")
}

fn get_hex(name: &str) -> &'static str {
    parse_color_with_modifiers(name)
}

/// When the user clicks a suggestion, preserve any modifier/finish words
/// from their original input. E.g. typing "Black Matte" + clicking "Blue" → "Blue Matte"
fn preserve_modifiers(original_input: &str, selected_name: &str) -> String {
    let skip_words: &[&str] = &["matte", "matt", "gloss", "glossy", "satin",
        "metallic", "chrome", "mirror", "pearl", "fluorescent", "neon",
        "dark", "light", "bright", "pale", "deep", "pastel", "medium", "vivid"];
    let modifiers: Vec<&str> = original_input
        .split_whitespace()
        .filter(|w| skip_words.contains(&w.to_lowercase().as_str()))
        .collect();
    if modifiers.is_empty() {
        selected_name.to_string()
    } else {
        format!("{} {}", selected_name, modifiers.join(" "))
    }
}

fn get_suggestions(filter: &str, sticker_type: &str, limit: usize) -> Vec<(&'static str, &'static str, &'static str, Option<&'static str>)> {
    let f = filter.to_lowercase();
    let source: &[ColorEntry] = if sticker_type == "reflective" { REFLECTIVE_COLORS } else { BASE_COLORS };
    // Words that describe finish/modifier rather than color
    let skip_words: &[&str] = &["matte", "matt", "gloss", "glossy", "satin",
        "metallic", "chrome", "mirror", "pearl", "fluorescent", "neon",
        "dark", "light", "bright", "pale", "deep", "pastel", "medium", "vivid"];
    let all_words: Vec<&str> = f.split_whitespace().collect();
    // Extract the core color words (strip modifiers)
    let color_words: Vec<&str> = all_words.iter()
        .filter(|w| !skip_words.contains(w))
        .copied()
        .collect();
    // If everything was stripped (user typed only modifiers like "matte"), use all words
    let search_words = if color_words.is_empty() { &all_words } else { &color_words };
    let mut results: Vec<_> = source.iter()
        .filter(|c| {
            if f.is_empty() { return true; }
            let name_l = c.name.to_lowercase();
            // Exact match on the full filter
            if name_l == f { return true; }
            // Name contains the full filter
            if name_l.contains(&f) { return true; }
            // Filter contains the name (e.g. "black matte" contains "black")
            if f.contains(&name_l) { return true; }
            // Any core color word matches the name
            if search_words.iter().any(|w| name_l.contains(w)) { return true; }
            false
        })
        .map(|c| (c.name, c.hex, c.category, c.oracal_code))
        .collect();
    results.sort_by(|a, b| {
        let a_exact = a.0.to_lowercase() == f;
        let b_exact = b.0.to_lowercase() == f;
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.cmp(b.0),
        }
    });
    results.truncate(limit);
    results
}

