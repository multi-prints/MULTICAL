#![allow(dead_code)]

use leptos::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast};

#[derive(Clone, Debug, PartialEq)]
pub enum DropdownPreview {
    /// Solid color chip (CSS hex, e.g. `#ef4444`).
    Color(String),
    /// Sticker stock: solid colored film, or reflective film with stripe overlay.
    Stock { color_hex: String, reflective: bool },
    Product {
        product_type: String,
        color: Option<String>,
    },
}

/// A single dropdown item.
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub preview: Option<DropdownPreview>,
    pub color: Option<String>,
    pub color_hex: Option<String>,
    pub badge: Option<String>,
}

impl DropdownItem {
    pub fn new(value: &str, label: &str) -> Self {
        DropdownItem {
            value: value.into(),
            label: label.into(),
            description: None,
            preview: None,
            color: None,
            color_hex: None,
            badge: None,
        }
    }
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }
    pub fn with_badge(mut self, badge: &str) -> Self {
        self.badge = Some(badge.into());
        self
    }
    pub fn with_color(mut self, color: &str, hex: Option<&str>) -> Self {
        self.preview = Some(DropdownPreview::Color(
            hex.map(str::to_string)
                .unwrap_or_else(|| get_color_hex(color).to_string()),
        ));
        self.color = Some(color.into());
        self.color_hex = hex.map(|h| h.into());
        self
    }
    /// Stock roll preview — uses the same reflective / colored look as the stock list.
    pub fn with_stock_preview(
        mut self,
        color: &str,
        hex: Option<&str>,
        sticker_type: &str,
    ) -> Self {
        let color_hex = hex
            .map(str::to_string)
            .unwrap_or_else(|| get_color_hex(color).to_string());
        let reflective = sticker_type.eq_ignore_ascii_case("reflective");
        self.preview = Some(DropdownPreview::Stock {
            color_hex: color_hex.clone(),
            reflective,
        });
        self.color = Some(color.into());
        self.color_hex = Some(color_hex);
        self
    }
    pub fn with_product_preview(mut self, product_type: &str, color: Option<&str>) -> Self {
        self.preview = Some(DropdownPreview::Product {
            product_type: product_type.into(),
            color: color.map(|c| c.into()),
        });
        self
    }
}

fn hex_luminance(hex: &str) -> f32 {
    let h = hex.trim().trim_start_matches('#');
    if h.len() < 6 {
        return 0.5;
    }
    let parse = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(128) as f32 / 255.0;
    let r = parse(0);
    let g = parse(2);
    let b = parse(4);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Match stock page reflective swatches (stripe overlay; visible on light bases).
fn reflective_swatch_css(hex: &str) -> String {
    if hex_luminance(hex) > 0.72 {
        format!(
            "background: \
repeating-linear-gradient(135deg, \
rgba(71,85,105,0.48) 0 2px, \
rgba(255,255,255,0.7) 2px 5px, \
rgba(148,163,184,0.42) 5px 7px, \
rgba(255,255,255,0.2) 7px 10px), \
linear-gradient(145deg, #ffffff 0%, {hex} 48%, #dbe3ee 100%); \
border: 1px solid #a78bfa;"
        )
    } else {
        format!(
            "background: \
repeating-linear-gradient(135deg, rgba(255,255,255,0.85) 0 4px, rgba(255,255,255,0.25) 4px 8px), {hex}; \
border: 1px solid #c4b5fd;"
        )
    }
}

fn colored_swatch_css(hex: &str) -> String {
    let border = if hex_luminance(hex) > 0.85 {
        "border: 1px solid rgba(15,23,42,0.18);"
    } else {
        "border: 1px solid rgba(0,0,0,0.08);"
    };
    format!("background-color: {hex}; {border}")
}

fn get_color_hex(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "red" => "#ef4444",
        "blue" => "#171717",
        "green" => "#22c55e",
        "yellow" => "#eab308",
        "orange" => "#f97316",
        "purple" => "#a855f7",
        "pink" => "#ec4899",
        "black" => "#1f2937",
        "white" | "gloss white" | "white gloss" => "#ffffff",
        "gold" | "gold metallic" => "#fbbf24",
        "silver" | "silver metallic" => "#9ca3af",
        "brown" => "#92400e",
        "grey" | "gray" => "#6b7280",
        "dark blue" | "navy" => "#1e3a8a",
        "dark red" | "maroon" => "#991b1b",
        "dark green" => "#166534",
        "dark purple" => "#581c87",
        "dark gray" => "#374151",
        "light blue" => "#93c5fd",
        "light green" => "#86efac",
        "light pink" => "#fbcfe8",
        "cyan" => "#06b6d4",
        "teal" => "#14b8a6",
        "lime" => "#84cc16",
        "coral" => "#ff7f50",
        "magenta" => "#ff00ff",
        "violet" => "#8b5cf6",
        "indigo" => "#6366f1",
        "turquoise" => "#40e0d0",
        "lavender" => "#e9d5ff",
        "mint" => "#a7f3d0",
        "mustard" => "#ca8a04",
        _ => "#9ca3af",
    }
}

fn render_dropdown_preview(preview: &DropdownPreview, compact: bool) -> AnyView {
    match preview {
        DropdownPreview::Color(hex) => {
            let class = "dropdown-color-swatch";
            view! { <span class=class style=format!("background-color: {}", hex)></span> }.into_any()
        }
        DropdownPreview::Stock {
            color_hex,
            reflective,
        } => {
            let style = if *reflective {
                reflective_swatch_css(color_hex)
            } else {
                colored_swatch_css(color_hex)
            };
            let cls = if compact {
                "dropdown-color-swatch"
            } else {
                "dropdown-color-swatch dropdown-color-swatch--lg"
            };
            view! { <span class=cls style=style></span> }.into_any()
        }
        DropdownPreview::Product { product_type, color } => {
            match product_type.as_str() {
                "life_saver" => {
                    let cls = if compact { "w-5 h-5 flex-shrink-0" } else { "w-6 h-6 flex-shrink-0" };
                    view! {
                        <svg viewBox="0 0 24 24" class=cls>
                            <polygon points="12,2 22,20 2,20" fill="#ffffff" stroke="#ef4444" stroke-width="2"/>
                            <text x="12" y="15" text-anchor="middle" font-size="9" font-weight="bold" fill="#1a1a1a">!</text>
                        </svg>
                    }.into_any()
                }
                "chevron" => {
                    let style = match color.as_deref() {
                        Some("white_red") => "background:linear-gradient(135deg,#ffffff 50%,#ef4444 50%)",
                        Some("yellow_red") => "background:linear-gradient(135deg,#eab308 50%,#ef4444 50%)",
                        _ => "background:linear-gradient(135deg,#ffffff 50%,#ef4444 50%)",
                    };
                    let cls = if compact {
                        "w-5 h-5 rounded-sm shadow-sm flex-shrink-0"
                    } else {
                        "w-6 h-6 rounded-sm shadow-sm flex-shrink-0"
                    };
                    view! { <div class=cls style=style></div> }.into_any()
                }
                "stripes" => {
                    let (style, border) = match color.as_deref() {
                        Some("white") => ("background:#ffffff", "border border-gray-200"),
                        Some("yellow") => ("background:#eab308", ""),
                        _ => ("background:#ffffff", "border border-gray-200"),
                    };
                    let cls = if compact {
                        format!("w-5 h-5 rounded-sm shadow-sm flex-shrink-0 {}", border)
                    } else {
                        format!("w-6 h-6 rounded-sm shadow-sm flex-shrink-0 {}", border)
                    };
                    view! { <div class=cls style=style></div> }.into_any()
                }
                _ => view! { <span class="dropdown-color-swatch" style="background-color: #9ca3af"></span> }.into_any(),
            }
        }
    }
}

#[component]
pub fn CustomDropdown(
    items: Signal<Vec<DropdownItem>>,
    placeholder: String,
    on_select: Callback<String>,
) -> impl IntoView {
    let (is_open, set_open) = signal(false);
    let (sel_label, set_sel_label) = signal(placeholder.clone());
    let (sel_preview, set_sel_preview) = signal(None::<DropdownPreview>);

    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            let set_open = set_open;
            let listener = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                set_open.set(false);
            }));
            let _ =
                window.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref());
            listener.forget();
        }
    });

    let toggle = move |e: leptos::ev::MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        set_open.update(|o| *o = !*o);
    };
    let select_item = move |value: String, label: String, preview: Option<DropdownPreview>| {
        set_sel_label.set(label);
        set_sel_preview.set(preview);
        set_open.set(false);
        on_select.run(value);
    };

    view! {
        <div class="custom-dropdown" class:open=move || is_open.get() on:click=move |e| e.stop_propagation()>
            <button type="button" class="dropdown-trigger" aria-haspopup="listbox"
                aria-expanded=move || is_open.get().to_string()
                on:click=toggle>
                <span class="dropdown-selected">
                    {move || sel_preview.get().map(|p| render_dropdown_preview(&p, true))}
                    <span class="dropdown-selected-label">{move || sel_label.get()}</span>
                </span>
                <svg class="dropdown-arrow" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <polyline points="6 9 12 15 18 9"></polyline>
                </svg>
            </button>
            <ul class="dropdown-menu" role="listbox" style:display=move || if is_open.get() { "block" } else { "none" }>
                {move || {
                    let list = items.get();
                    if list.is_empty() {
                        view!{<li class="dropdown-item dropdown-item-empty">"No options available"</li>}.into_any()
                    } else {
                        list.into_iter().map(|item| {
                            let val = item.value.clone();
                            let lab = item.label.clone();
                            let desc = item.description.clone();
                            let preview = item.preview.clone().or_else(|| {
                                item.color_hex.clone()
                                    .or_else(|| item.color.as_ref().map(|c| get_color_hex(c).to_string()))
                                    .map(DropdownPreview::Color)
                            });
                            let sel = select_item;
                            view!{<li class="dropdown-item" role="option"
                                on:click=move |_| sel(val.clone(), lab.clone(), preview.clone())>
                                {preview.clone().map(|p| render_dropdown_preview(&p, false))}
                                <div class="flex-1 min-w-0">
                                    <div class="dropdown-item-label">{item.label.clone()}</div>
                                    {desc.map(|d| view! { <div class="dropdown-item-desc">{d}</div> })}
                                </div>
                                {item.badge.map(|b| view!{<span class="dropdown-item-badge">{b}</span>})}
                            </li>}
                        }).collect::<Vec<_>>().into_any()
                    }
                }}
            </ul>
        </div>
    }
}
