#![allow(dead_code)]

use leptos::prelude::*;

/// A single dropdown item.
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownItem {
    pub value: String,
    pub label: String,
    pub color: Option<String>,
    pub color_hex: Option<String>,
    pub badge: Option<String>,
}

impl DropdownItem {
    pub fn new(value: &str, label: &str) -> Self {
        DropdownItem { value: value.into(), label: label.into(), color: None, color_hex: None, badge: None }
    }
    pub fn with_badge(mut self, badge: &str) -> Self { self.badge = Some(badge.into()); self }
    pub fn with_color(mut self, color: &str, hex: Option<&str>) -> Self {
        self.color = Some(color.into());
        self.color_hex = hex.map(|h| h.into());
        self
    }
}

fn get_color_hex(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "red" => "#ef4444", "blue" => "#3b82f6", "green" => "#22c55e", "yellow" => "#eab308",
        "orange" => "#f97316", "purple" => "#a855f7", "pink" => "#ec4899", "black" => "#1f2937",
        "white" | "gloss white" | "white gloss" => "#ffffff",
        "gold" | "gold metallic" => "#fbbf24", "silver" | "silver metallic" => "#9ca3af",
        "brown" => "#92400e", "grey" | "gray" => "#6b7280",
        "dark blue" | "navy" => "#1e3a8a", "dark red" | "maroon" => "#991b1b",
        "dark green" => "#166534", "dark purple" => "#581c87", "dark gray" => "#374151",
        "light blue" => "#93c5fd", "light green" => "#86efac", "light pink" => "#fbcfe8",
        "cyan" => "#06b6d4", "teal" => "#14b8a6", "lime" => "#84cc16",
        "coral" => "#ff7f50", "magenta" => "#ff00ff", "violet" => "#8b5cf6",
        "indigo" => "#6366f1", "turquoise" => "#40e0d0", "lavender" => "#e9d5ff",
        "mint" => "#a7f3d0", "mustard" => "#ca8a04",
        _ => "#9ca3af",
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
    let (sel_color, set_sel_color) = signal(None::<String>);

    let toggle = move |e: leptos::ev::MouseEvent| { e.prevent_default(); e.stop_propagation(); set_open.update(|o| *o = !*o); };
    let select_item = move |value: String, label: String, hex: Option<String>| {
        set_sel_label.set(label);
        set_sel_color.set(hex);
        set_open.set(false);
        on_select.run(value);
    };

    view! {
        <div class="custom-dropdown" class:open=move || is_open.get()>
            <button type="button" class="dropdown-trigger" aria-haspopup="listbox"
                aria-expanded=move || is_open.get().to_string()
                on:click=toggle>
                <span class="dropdown-selected">
                    {move || sel_color.get().map(|c| view!{<span class="dropdown-color-swatch" style=format!("background-color: {}", c)></span>})}
                    <span class="dropdown-selected-label">{move || sel_label.get()}</span>
                </span>
                <svg class="dropdown-arrow" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
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
                            let hex = item.color_hex.clone()
                                .or_else(|| item.color.as_ref().map(|c| get_color_hex(c).to_string()));
                            let sel = select_item.clone();
                            view!{<li class="dropdown-item" role="option"
                                on:click=move |_| sel(val.clone(), lab.clone(), hex.clone())>
                                {hex.clone().map(|h| view!{<span class="dropdown-color-swatch" style=format!("background-color: {}", h)></span>})}
                                <span class="dropdown-item-label">{item.label.clone()}</span>
                                {item.badge.map(|b| view!{<span class="dropdown-item-badge">{b}</span>})}
                            </li>}
                        }).collect::<Vec<_>>().into_any()
                    }
                }}
            </ul>
        </div>
    }
}
