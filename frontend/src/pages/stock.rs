use crate::api::{
    self, NewPrintingMaterial, NewStockItem, PrintingMaterial, StockItem, StockPageQuery,
};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;
use log::error;

#[path = "../components/loading.rs"]
mod loading_comp;
use loading_comp::PageLoading;

#[derive(Clone, Debug, PartialEq)]
struct SizeRow {
    id: u32,
    size: String,
    rolls: String,
    metres_per_roll: String,
}

struct ColorEntry {
    name: &'static str,
    hex: &'static str,
    category: &'static str,
    oracal_code: Option<&'static str>,
}

const BASE_COLORS: &[ColorEntry] = &[
    ColorEntry {
        name: "White",
        hex: "#FFFFFF",
        category: "neutral",
        oracal_code: Some("010"),
    },
    ColorEntry {
        name: "Soft White",
        hex: "#F5F5F0",
        category: "neutral",
        oracal_code: Some("011"),
    },
    ColorEntry {
        name: "Creme",
        hex: "#F5F5DC",
        category: "neutral",
        oracal_code: Some("012"),
    },
    ColorEntry {
        name: "Ivory",
        hex: "#FFFFF0",
        category: "neutral",
        oracal_code: Some("013"),
    },
    ColorEntry {
        name: "Beige",
        hex: "#F5DEB3",
        category: "neutral",
        oracal_code: Some("014"),
    },
    ColorEntry {
        name: "Cream",
        hex: "#FFFDD0",
        category: "neutral",
        oracal_code: None,
    },
    ColorEntry {
        name: "Black",
        hex: "#1A1A1A",
        category: "neutral",
        oracal_code: Some("070"),
    },
    ColorEntry {
        name: "Grey",
        hex: "#808080",
        category: "neutral",
        oracal_code: Some("071"),
    },
    ColorEntry {
        name: "Light Grey",
        hex: "#D3D3D3",
        category: "neutral",
        oracal_code: Some("072"),
    },
    ColorEntry {
        name: "Dark Grey",
        hex: "#404040",
        category: "neutral",
        oracal_code: Some("073"),
    },
    ColorEntry {
        name: "Charcoal",
        hex: "#36454F",
        category: "neutral",
        oracal_code: Some("074"),
    },
    ColorEntry {
        name: "Slate",
        hex: "#708090",
        category: "neutral",
        oracal_code: Some("075"),
    },
    ColorEntry {
        name: "Silver",
        hex: "#C0C0C0",
        category: "neutral",
        oracal_code: Some("076"),
    },
    ColorEntry {
        name: "Anthracite",
        hex: "#303030",
        category: "neutral",
        oracal_code: Some("077"),
    },
    ColorEntry {
        name: "Yellow",
        hex: "#FFD700",
        category: "yellow",
        oracal_code: Some("020"),
    },
    ColorEntry {
        name: "Lemon Yellow",
        hex: "#FFF44F",
        category: "yellow",
        oracal_code: Some("021"),
    },
    ColorEntry {
        name: "Bright Yellow",
        hex: "#FFEA00",
        category: "yellow",
        oracal_code: Some("022"),
    },
    ColorEntry {
        name: "Canary Yellow",
        hex: "#FFEF00",
        category: "yellow",
        oracal_code: Some("023"),
    },
    ColorEntry {
        name: "Golden Yellow",
        hex: "#FFDF00",
        category: "yellow",
        oracal_code: Some("024"),
    },
    ColorEntry {
        name: "Brimstone Yellow",
        hex: "#E6B800",
        category: "yellow",
        oracal_code: Some("025"),
    },
    ColorEntry {
        name: "Signal Yellow",
        hex: "#F7BA0B",
        category: "yellow",
        oracal_code: Some("019"),
    },
    ColorEntry {
        name: "Saffron",
        hex: "#F4C430",
        category: "yellow",
        oracal_code: None,
    },
    ColorEntry {
        name: "Amber",
        hex: "#FFBF00",
        category: "yellow",
        oracal_code: None,
    },
    ColorEntry {
        name: "Gold",
        hex: "#FFD700",
        category: "yellow",
        oracal_code: Some("024"),
    },
    ColorEntry {
        name: "Dark Yellow",
        hex: "#DAA520",
        category: "yellow",
        oracal_code: None,
    },
    ColorEntry {
        name: "Orange",
        hex: "#FF6600",
        category: "orange",
        oracal_code: Some("034"),
    },
    ColorEntry {
        name: "Bright Orange",
        hex: "#FF6B00",
        category: "orange",
        oracal_code: Some("034"),
    },
    ColorEntry {
        name: "Tangerine",
        hex: "#FF9966",
        category: "orange",
        oracal_code: Some("034"),
    },
    ColorEntry {
        name: "Burnt Orange",
        hex: "#CC5500",
        category: "orange",
        oracal_code: Some("035"),
    },
    ColorEntry {
        name: "Pastel Orange",
        hex: "#FFB366",
        category: "orange",
        oracal_code: Some("035"),
    },
    ColorEntry {
        name: "Red Orange",
        hex: "#FF5349",
        category: "orange",
        oracal_code: Some("036"),
    },
    ColorEntry {
        name: "Deep Orange",
        hex: "#E65C00",
        category: "orange",
        oracal_code: None,
    },
    ColorEntry {
        name: "Peach",
        hex: "#FFCBA4",
        category: "orange",
        oracal_code: None,
    },
    ColorEntry {
        name: "Apricot",
        hex: "#FBCEB1",
        category: "orange",
        oracal_code: None,
    },
    ColorEntry {
        name: "Red",
        hex: "#E60012",
        category: "red",
        oracal_code: Some("030"),
    },
    ColorEntry {
        name: "Bright Red",
        hex: "#FF0000",
        category: "red",
        oracal_code: Some("030"),
    },
    ColorEntry {
        name: "Dark Red",
        hex: "#8B0000",
        category: "red",
        oracal_code: Some("031"),
    },
    ColorEntry {
        name: "Ruby Red",
        hex: "#9B111E",
        category: "red",
        oracal_code: Some("032"),
    },
    ColorEntry {
        name: "Cherry Red",
        hex: "#DE3163",
        category: "red",
        oracal_code: Some("033"),
    },
    ColorEntry {
        name: "Burgundy",
        hex: "#800020",
        category: "red",
        oracal_code: None,
    },
    ColorEntry {
        name: "Maroon",
        hex: "#800000",
        category: "red",
        oracal_code: None,
    },
    ColorEntry {
        name: "Wine",
        hex: "#722F37",
        category: "red",
        oracal_code: None,
    },
    ColorEntry {
        name: "Crimson",
        hex: "#DC143C",
        category: "red",
        oracal_code: None,
    },
    ColorEntry {
        name: "Scarlet",
        hex: "#FF2400",
        category: "red",
        oracal_code: None,
    },
    ColorEntry {
        name: "Brick Red",
        hex: "#B22222",
        category: "red",
        oracal_code: None,
    },
    ColorEntry {
        name: "Signal Red",
        hex: "#CE1126",
        category: "red",
        oracal_code: Some("030"),
    },
    ColorEntry {
        name: "Fire Red",
        hex: "#FF4500",
        category: "red",
        oracal_code: None,
    },
    ColorEntry {
        name: "Oxblood",
        hex: "#660000",
        category: "red",
        oracal_code: None,
    },
    ColorEntry {
        name: "Pink",
        hex: "#FF69B4",
        category: "pink",
        oracal_code: Some("041"),
    },
    ColorEntry {
        name: "Light Pink",
        hex: "#FFB6C1",
        category: "pink",
        oracal_code: Some("041"),
    },
    ColorEntry {
        name: "Hot Pink",
        hex: "#FF69B4",
        category: "pink",
        oracal_code: Some("041"),
    },
    ColorEntry {
        name: "Rose",
        hex: "#FF007F",
        category: "pink",
        oracal_code: Some("042"),
    },
    ColorEntry {
        name: "Salmon",
        hex: "#FA8072",
        category: "pink",
        oracal_code: Some("043"),
    },
    ColorEntry {
        name: "Coral",
        hex: "#FF7F50",
        category: "pink",
        oracal_code: Some("044"),
    },
    ColorEntry {
        name: "Magenta",
        hex: "#FF00FF",
        category: "pink",
        oracal_code: Some("045"),
    },
    ColorEntry {
        name: "Fuchsia",
        hex: "#FF00FF",
        category: "pink",
        oracal_code: Some("045"),
    },
    ColorEntry {
        name: "Raspberry",
        hex: "#E30B5C",
        category: "pink",
        oracal_code: None,
    },
    ColorEntry {
        name: "Mauve",
        hex: "#E0B0FF",
        category: "pink",
        oracal_code: None,
    },
    ColorEntry {
        name: "Blush",
        hex: "#DE5D83",
        category: "pink",
        oracal_code: None,
    },
    ColorEntry {
        name: "Purple",
        hex: "#800080",
        category: "purple",
        oracal_code: Some("050"),
    },
    ColorEntry {
        name: "Violet",
        hex: "#8B5CF6",
        category: "purple",
        oracal_code: Some("051"),
    },
    ColorEntry {
        name: "Lavender",
        hex: "#E6E6FA",
        category: "purple",
        oracal_code: Some("043"),
    },
    ColorEntry {
        name: "Lilac",
        hex: "#C8A2C8",
        category: "purple",
        oracal_code: Some("052"),
    },
    ColorEntry {
        name: "Plum",
        hex: "#DDA0DD",
        category: "purple",
        oracal_code: Some("053"),
    },
    ColorEntry {
        name: "Dark Purple",
        hex: "#301934",
        category: "purple",
        oracal_code: Some("054"),
    },
    ColorEntry {
        name: "Light Purple",
        hex: "#B19CD9",
        category: "purple",
        oracal_code: Some("055"),
    },
    ColorEntry {
        name: "Indigo",
        hex: "#4B0082",
        category: "purple",
        oracal_code: Some("056"),
    },
    ColorEntry {
        name: "Amethyst",
        hex: "#9966CC",
        category: "purple",
        oracal_code: None,
    },
    ColorEntry {
        name: "Grape",
        hex: "#6F2DA8",
        category: "purple",
        oracal_code: None,
    },
    ColorEntry {
        name: "Royal Purple",
        hex: "#7851A9",
        category: "purple",
        oracal_code: None,
    },
    ColorEntry {
        name: "Purple Red",
        hex: "#960018",
        category: "purple",
        oracal_code: Some("026"),
    },
    ColorEntry {
        name: "Gentian Blue",
        hex: "#4B0082",
        category: "purple",
        oracal_code: Some("051"),
    },
    ColorEntry {
        name: "Blue",
        hex: "#0066CC",
        category: "blue",
        oracal_code: Some("060"),
    },
    ColorEntry {
        name: "Light Blue",
        hex: "#ADD8E6",
        category: "blue",
        oracal_code: Some("061"),
    },
    ColorEntry {
        name: "Dark Blue",
        hex: "#00008B",
        category: "blue",
        oracal_code: Some("062"),
    },
    ColorEntry {
        name: "Navy",
        hex: "#000080",
        category: "blue",
        oracal_code: Some("063"),
    },
    ColorEntry {
        name: "Navy Blue",
        hex: "#000080",
        category: "blue",
        oracal_code: Some("063"),
    },
    ColorEntry {
        name: "Royal Blue",
        hex: "#4169E1",
        category: "blue",
        oracal_code: Some("064"),
    },
    ColorEntry {
        name: "Sky Blue",
        hex: "#87CEEB",
        category: "blue",
        oracal_code: Some("065"),
    },
    ColorEntry {
        name: "Ice Blue",
        hex: "#D6EAF8",
        category: "blue",
        oracal_code: Some("056"),
    },
    ColorEntry {
        name: "Baby Blue",
        hex: "#89CFF0",
        category: "blue",
        oracal_code: Some("061"),
    },
    ColorEntry {
        name: "Azure",
        hex: "#007FFF",
        category: "blue",
        oracal_code: Some("052"),
    },
    ColorEntry {
        name: "Cobalt",
        hex: "#0047AB",
        category: "blue",
        oracal_code: Some("066"),
    },
    ColorEntry {
        name: "Sapphire",
        hex: "#0F52BA",
        category: "blue",
        oracal_code: None,
    },
    ColorEntry {
        name: "Teal",
        hex: "#008080",
        category: "blue",
        oracal_code: Some("067"),
    },
    ColorEntry {
        name: "Turquoise",
        hex: "#40E0D0",
        category: "blue",
        oracal_code: Some("054"),
    },
    ColorEntry {
        name: "Cyan",
        hex: "#00FFFF",
        category: "blue",
        oracal_code: Some("055"),
    },
    ColorEntry {
        name: "Aqua",
        hex: "#00FFFF",
        category: "blue",
        oracal_code: Some("055"),
    },
    ColorEntry {
        name: "King Blue",
        hex: "#0047AB",
        category: "blue",
        oracal_code: Some("049"),
    },
    ColorEntry {
        name: "Gentian",
        hex: "#1A237E",
        category: "blue",
        oracal_code: Some("051"),
    },
    ColorEntry {
        name: "Midnight Blue",
        hex: "#191970",
        category: "blue",
        oracal_code: None,
    },
    ColorEntry {
        name: "Steel Blue",
        hex: "#4682B4",
        category: "blue",
        oracal_code: None,
    },
    ColorEntry {
        name: "Powder Blue",
        hex: "#B0E0E6",
        category: "blue",
        oracal_code: None,
    },
    ColorEntry {
        name: "Green",
        hex: "#00B050",
        category: "green",
        oracal_code: Some("080"),
    },
    ColorEntry {
        name: "Light Green",
        hex: "#90EE90",
        category: "green",
        oracal_code: Some("081"),
    },
    ColorEntry {
        name: "Dark Green",
        hex: "#006400",
        category: "green",
        oracal_code: Some("082"),
    },
    ColorEntry {
        name: "Forest Green",
        hex: "#228B22",
        category: "green",
        oracal_code: Some("083"),
    },
    ColorEntry {
        name: "Lime",
        hex: "#32CD32",
        category: "green",
        oracal_code: Some("084"),
    },
    ColorEntry {
        name: "Lime Green",
        hex: "#32CD32",
        category: "green",
        oracal_code: Some("084"),
    },
    ColorEntry {
        name: "Emerald",
        hex: "#50C878",
        category: "green",
        oracal_code: Some("085"),
    },
    ColorEntry {
        name: "Emerald Green",
        hex: "#50C878",
        category: "green",
        oracal_code: Some("085"),
    },
    ColorEntry {
        name: "Olive",
        hex: "#808000",
        category: "green",
        oracal_code: Some("086"),
    },
    ColorEntry {
        name: "Olive Green",
        hex: "#808000",
        category: "green",
        oracal_code: Some("086"),
    },
    ColorEntry {
        name: "Mint",
        hex: "#98FF98",
        category: "green",
        oracal_code: Some("055"),
    },
    ColorEntry {
        name: "Mint Green",
        hex: "#98FF98",
        category: "green",
        oracal_code: Some("055"),
    },
    ColorEntry {
        name: "Sea Green",
        hex: "#2E8B57",
        category: "green",
        oracal_code: None,
    },
    ColorEntry {
        name: "Hunter Green",
        hex: "#355E3B",
        category: "green",
        oracal_code: None,
    },
    ColorEntry {
        name: "Sage",
        hex: "#9DC183",
        category: "green",
        oracal_code: None,
    },
    ColorEntry {
        name: "Grass Green",
        hex: "#7CFC00",
        category: "green",
        oracal_code: None,
    },
    ColorEntry {
        name: "Kelly Green",
        hex: "#4CBB17",
        category: "green",
        oracal_code: None,
    },
    ColorEntry {
        name: "Moss Green",
        hex: "#8A9A5B",
        category: "green",
        oracal_code: None,
    },
    ColorEntry {
        name: "Jade",
        hex: "#00A86B",
        category: "green",
        oracal_code: None,
    },
    ColorEntry {
        name: "Pine Green",
        hex: "#01796F",
        category: "green",
        oracal_code: None,
    },
    ColorEntry {
        name: "Signal Green",
        hex: "#00B050",
        category: "green",
        oracal_code: Some("080"),
    },
    ColorEntry {
        name: "Brown",
        hex: "#8B4513",
        category: "brown",
        oracal_code: Some("083"),
    },
    ColorEntry {
        name: "Light Brown",
        hex: "#CD853F",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Dark Brown",
        hex: "#3D2314",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Tan",
        hex: "#D2B48C",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Chocolate",
        hex: "#7B3F00",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Coffee",
        hex: "#6F4E37",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Caramel",
        hex: "#FFD59A",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Camel",
        hex: "#C19A6B",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Cocoa",
        hex: "#D2691E",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Copper",
        hex: "#B87333",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Bronze",
        hex: "#CD7F32",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Rust",
        hex: "#B7410E",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Mahogany",
        hex: "#C04000",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Nut Brown",
        hex: "#6B4423",
        category: "brown",
        oracal_code: Some("083"),
    },
    ColorEntry {
        name: "Sand",
        hex: "#C2B280",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Khaki",
        hex: "#F0E68C",
        category: "brown",
        oracal_code: None,
    },
    ColorEntry {
        name: "Metallic Gold",
        hex: "#D4AF37",
        category: "metallic",
        oracal_code: Some("024"),
    },
    ColorEntry {
        name: "Metallic Silver",
        hex: "#C0C0C0",
        category: "metallic",
        oracal_code: Some("076"),
    },
    ColorEntry {
        name: "Metallic Bronze",
        hex: "#CD7F32",
        category: "metallic",
        oracal_code: None,
    },
    ColorEntry {
        name: "Metallic Copper",
        hex: "#B87333",
        category: "metallic",
        oracal_code: None,
    },
    ColorEntry {
        name: "Chrome",
        hex: "#E8E8E8",
        category: "metallic",
        oracal_code: None,
    },
    ColorEntry {
        name: "Mirror Gold",
        hex: "#FFD700",
        category: "metallic",
        oracal_code: None,
    },
    ColorEntry {
        name: "Mirror Silver",
        hex: "#C0C0C0",
        category: "metallic",
        oracal_code: None,
    },
    ColorEntry {
        name: "Fluorescent Yellow",
        hex: "#CCFF00",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Neon Yellow",
        hex: "#CCFF00",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Fluorescent Orange",
        hex: "#FF6600",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Neon Orange",
        hex: "#FF6600",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Fluorescent Red",
        hex: "#FF3131",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Neon Red",
        hex: "#FF3131",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Fluorescent Green",
        hex: "#39FF14",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Neon Green",
        hex: "#39FF14",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Fluorescent Pink",
        hex: "#FF6EFF",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Neon Pink",
        hex: "#FF6EFF",
        category: "fluorescent",
        oracal_code: None,
    },
    ColorEntry {
        name: "Dayglo Orange",
        hex: "#FF5F00",
        category: "fluorescent",
        oracal_code: None,
    },
];

const REFLECTIVE_COLORS: &[ColorEntry] = &[
    ColorEntry {
        name: "White",
        hex: "#FFFFFF",
        category: "reflective",
        oracal_code: None,
    },
    ColorEntry {
        name: "Yellow",
        hex: "#FFD700",
        category: "reflective",
        oracal_code: None,
    },
    ColorEntry {
        name: "Red",
        hex: "#E60012",
        category: "reflective",
        oracal_code: None,
    },
    ColorEntry {
        name: "Orange",
        hex: "#FF6600",
        category: "reflective",
        oracal_code: None,
    },
    ColorEntry {
        name: "Green",
        hex: "#00B050",
        category: "reflective",
        oracal_code: None,
    },
    ColorEntry {
        name: "Blue",
        hex: "#0066CC",
        category: "reflective",
        oracal_code: None,
    },
    ColorEntry {
        name: "Fluorescent Yellow",
        hex: "#CCFF00",
        category: "reflective",
        oracal_code: None,
    },
    ColorEntry {
        name: "Fluorescent Orange",
        hex: "#FF6600",
        category: "reflective",
        oracal_code: None,
    },
    ColorEntry {
        name: "Fluorescent Lime",
        hex: "#39FF14",
        category: "reflective",
        oracal_code: None,
    },
];

const SYNONYMS: &[(&str, &str)] = &[
    ("pure white", "white"),
    ("bright white", "white"),
    ("snow white", "white"),
    ("paper white", "white"),
    ("jet black", "black"),
    ("matte black", "black"),
    ("gloss black", "black"),
    ("light grey", "light gray"),
    ("dark grey", "dark gray"),
    ("pewter", "slate"),
    ("graphite", "charcoal"),
    ("baby blue", "light blue"),
    ("powder blue", "light blue"),
    ("midnight", "navy"),
    ("cornflower", "azure"),
    ("electric blue", "bright blue"),
    ("fire engine red", "bright red"),
    ("blood red", "dark red"),
    ("cherry", "cherry red"),
    ("grass", "grass green"),
    ("lime", "lime green"),
    ("pine", "forest green"),
    ("canary", "canary yellow"),
    ("buttercup", "bright yellow"),
    ("dandelion", "golden yellow"),
    ("royal purple", "purple"),
    ("deep purple", "dark purple"),
    ("ultra violet", "violet"),
    ("electric purple", "purple"),
    ("bubblegum", "hot pink"),
    ("rose pink", "rose"),
    ("blush pink", "blush"),
    ("pumpkin", "orange"),
    ("carrot", "orange"),
    ("espresso", "dark brown"),
    ("latte", "light brown"),
    ("mocha", "coffee"),
    ("chocolate brown", "chocolate"),
    ("sienna", "rust"),
    ("terracotta", "rust"),
    ("gold metallic", "metallic gold"),
    ("silver metallic", "metallic silver"),
    ("copper metallic", "metallic copper"),
    ("bronze metallic", "metallic bronze"),
    ("grey", "gray"),
    ("grey dark", "dark grey"),
    ("grey light", "light grey"),
    ("collared", "colored"),
    ("matt black", "matte black"),
    ("mate black", "matte black"),
    ("mate", "matte"),
    ("glos", "gloss"),
    ("metalic", "metallic"),
    ("florescent", "fluorescent"),
    ("flourescent", "fluorescent"),
    ("reflectve", "reflective"),
    ("refelctive", "reflective"),
];

#[component]
pub fn StockPage(can_manage_materials: bool) -> impl IntoView {
    // Page tab: stickers | materials
    let (inventory_tab, set_inventory_tab) = signal("stickers".to_string());
    // Modal tab when adding (can differ briefly; synced on open)
    let (modal_tab, set_modal_tab) = signal("stickers".to_string());

    let (stock, set_stock) = signal(Vec::<StockItem>::new());
    let (materials, set_materials) = signal(Vec::<PrintingMaterial>::new());
    let (total_rolls, set_total_rolls) = signal(0i64);
    let (total_metres, set_total_metres) = signal(0.0f64);
    let (remaining_metres, set_remaining_metres) = signal(0.0f64);
    let (total_count, set_total_count) = signal(0u32);
    let (show_add, set_show_add) = signal(false);
    let (color, set_color) = signal(String::new());
    let (sticker_type, set_sticker_type) = signal("colored".to_string());
    let (rows, set_rows) = signal(vec![SizeRow {
        id: 0,
        size: String::new(),
        rolls: String::new(),
        metres_per_roll: "50".into(),
    }]);
    let (next_row_id, set_next_row_id) = signal(1u32);
    let (current_page, set_current_page) = signal(1u32);
    let (add_rolls_item, set_add_rolls_item) = signal(None::<StockItem>);
    let (add_rolls_value, set_add_rolls_value) = signal(String::new());
    let (add_mat_rolls_item, set_add_mat_rolls_item) = signal(None::<PrintingMaterial>);
    let (add_mat_rolls_value, set_add_mat_rolls_value) = signal(String::new());
    let (del_id, set_del_id) = signal(None::<i64>);
    let (del_label, set_del_label) = signal(String::new());
    let (del_kind, set_del_kind) = signal("stock".to_string()); // "stock" | "material"
    let (loading, set_loading) = signal(true);
    let items_per_page = 10u32;

    // Print material form fields
    let (mat_name, set_mat_name) = signal(String::new());
    let (mat_width, set_mat_width) = signal(String::new());
    let (mat_rolls, set_mat_rolls) = signal("1".to_string());
    let (mat_mpr, set_mat_mpr) = signal("50".to_string());

    let reload = move || {
        leptos::task::spawn_local(async move {
            match api::get_stock_page(&StockPageQuery {
                page: Some(current_page.get()),
                per_page: Some(items_per_page),
            })
            .await
            {
                Ok(page) => {
                    set_total_rolls.set(page.total_rolls);
                    set_total_metres.set(page.total_metres);
                    set_remaining_metres.set(page.remaining_metres);
                    set_total_count.set(page.total_count as u32);
                    set_stock.set(page.items);
                }
                Err(e) => error!("get_stock_page failed: {}", e),
            }
            match api::get_all_printing_materials().await {
                Ok(m) => set_materials.set(m),
                Err(e) => error!("get_all_printing_materials failed: {}", e),
            }
            set_loading.set(false);
        })
    };

    let (live_tick, set_live_tick) = signal(0u64);
    create_effect(move |_| {
        let _ = current_page.get();
        let _ = live_tick.get();
        reload();
    });
    use_auto_refresh(LIVE_REFRESH_MS, move || {
        set_live_tick.update(|t| *t = t.wrapping_add(1));
    });

    let reset_sticker_form = move || {
        set_color.set(String::new());
        set_sticker_type.set("colored".into());
        set_rows.set(vec![SizeRow {
            id: 0,
            size: String::new(),
            rolls: String::new(),
            metres_per_roll: "50".into(),
        }]);
        set_next_row_id.set(1);
    };
    let reset_material_form = move || {
        set_mat_name.set(String::new());
        set_mat_width.set(String::new());
        set_mat_rolls.set("1".into());
        set_mat_mpr.set("50".into());
    };
    let reset_add_modal = move || {
        reset_sticker_form();
        reset_material_form();
    };
    let add_row = move || {
        let id = next_row_id.get();
        set_next_row_id.set(id + 1);
        set_rows.update(|r| {
            r.push(SizeRow {
                id,
                size: String::new(),
                rolls: String::new(),
                metres_per_roll: "50".into(),
            })
        });
    };
    let update_row = move |id: u32, field: &'static str, value: String| {
        set_rows.update(|rs| {
            if let Some(r) = rs.iter_mut().find(|r| r.id == id) {
                match field {
                    "size" => r.size = value,
                    "rolls" => r.rolls = value,
                    "mpr" => r.metres_per_roll = value,
                    _ => {}
                }
            }
        })
    };

    let total_items = move || total_count.get();
    let total_pages = move || {
        let n = total_items();
        if n == 0 {
            1
        } else {
            n.div_ceil(items_per_page)
        }
    };
    let remaining = |i: &StockItem| {
        i.total_metres
            - if i.metres_used.is_nan() {
                0.0
            } else {
                i.metres_used
            }
    };
    let rolls_left = move |i: &StockItem| {
        (remaining(i)
            / if i.metres_per_roll > 0.0 {
                i.metres_per_roll
            } else {
                50.0
            })
        .floor() as i64
    };
    let status = move |i: &StockItem| {
        let pct = if i.total_metres <= 0.0 {
            0.0
        } else {
            remaining(i) / i.total_metres * 100.0
        };
        if pct <= 0.0 {
            ("Out of Stock", "status-badge--error")
        } else if pct <= 20.0 {
            ("Low Stock", "status-badge--warning")
        } else {
            ("In Stock", "status-badge--success")
        }
    };
    let mat_remaining = |m: &PrintingMaterial| {
        m.total_metres
            - if m.metres_used.is_nan() {
                0.0
            } else {
                m.metres_used
            }
    };
    let mat_rolls_left = move |m: &PrintingMaterial| {
        let rem = mat_remaining(m);
        if m.metres_per_roll > 0.0 {
            (rem / m.metres_per_roll).floor() as i64
        } else {
            (rem / 50.0).floor() as i64
        }
    };
    let mat_status = move |m: &PrintingMaterial| {
        let pct = if m.total_metres <= 0.0 {
            0.0
        } else {
            mat_remaining(m) / m.total_metres * 100.0
        };
        if pct <= 0.0 {
            ("Out of Stock", "status-badge--error")
        } else if pct <= 20.0 {
            ("Low Stock", "status-badge--warning")
        } else {
            ("In Stock", "status-badge--success")
        }
    };

    // Materials metrics (client-side from full list)
    let mat_metrics = move || {
        let mats = materials.get();
        let count = mats.len() as u32;
        let rolls: i64 = mats.iter().map(|m| m.rolls).sum();
        let total_m: f64 = mats.iter().map(|m| m.total_metres).sum();
        let rem_m: f64 = mats.iter().map(mat_remaining).sum();
        (count, rolls, total_m, rem_m)
    };

    let row_total = move |r: &SizeRow| {
        r.rolls.parse::<f64>().unwrap_or(0.0)
            * if sticker_type.get() == "reflective" {
                r.metres_per_roll.parse::<f64>().unwrap_or(0.0)
            } else {
                50.0
            }
    };
    let total_rolls_modal = move || {
        rows.get()
            .iter()
            .map(|r| r.rolls.parse::<i64>().unwrap_or(0))
            .sum::<i64>()
    };
    let total_metres_modal = move || rows.get().iter().map(row_total).sum::<f64>();
    let mat_total_metres_preview = move || {
        let rolls: f64 = mat_rolls.get().parse().unwrap_or(0.0);
        let mpr: f64 = mat_mpr.get().parse().unwrap_or(0.0);
        rolls * mpr
    };

    let (adding_stock, set_adding_stock) = signal(false);
    let (adding_material, set_adding_material) = signal(false);
    let (adding_rolls, set_adding_rolls) = signal(false);
    let (adding_mat_rolls, set_adding_mat_rolls) = signal(false);
    let (deleting, set_deleting) = signal(false);

    let add_stock_action = {
        move |color: String, sticker_type: String, rows: Vec<SizeRow>| {
            if adding_stock.get() {
                return;
            }
            set_adding_stock.set(true);
            leptos::task::spawn_local(async move {
                for r in &rows {
                    let rolls = r.rolls.parse::<i64>().unwrap_or(0);
                    if !color.is_empty() && !r.size.is_empty() && rolls > 0 {
                        let mpr = if sticker_type == "reflective" {
                            r.metres_per_roll.parse::<f64>().unwrap_or(50.0)
                        } else {
                            50.0
                        };
                        if let Err(e) = api::add_stock(&NewStockItem {
                            color: color.clone(),
                            size: r.size.clone(),
                            sticker_type: sticker_type.clone(),
                            rolls,
                            metres_per_roll: Some(mpr),
                            total_metres: None,
                            metres_used: 0.0,
                            custom_metres_per_roll: if sticker_type == "reflective" {
                                Some(mpr)
                            } else {
                                None
                            },
                        })
                        .await
                        {
                            error!("add_stock failed: {}", e);
                        }
                    }
                }
                set_show_add.set(false);
                set_adding_stock.set(false);
                reload();
            });
        }
    };

    let add_material_action = {
        move |_| {
            if adding_material.get() {
                return;
            }
            let name = mat_name.get().trim().to_string();
            let width: f64 = mat_width.get().parse().unwrap_or(0.0);
            let rolls: i64 = mat_rolls.get().parse().unwrap_or(0);
            let mpr: f64 = mat_mpr.get().parse().unwrap_or(0.0);
            if name.is_empty() || width <= 0.0 || rolls <= 0 || mpr <= 0.0 {
                return;
            }
            set_adding_material.set(true);
            leptos::task::spawn_local(async move {
                match api::add_printing_material(&NewPrintingMaterial {
                    // DB still requires material_type; the name is what staff actually use.
                    name: name.clone(),
                    material_type: name,
                    width,
                    rolls,
                    metres_per_roll: mpr,
                    total_metres: Some(rolls as f64 * mpr),
                    metres_used: 0.0,
                    color: None,
                })
                .await
                {
                    Ok(_) => {
                        set_show_add.set(false);
                        set_inventory_tab.set("materials".into());
                    }
                    Err(e) => error!("add_printing_material failed: {}", e),
                }
                set_adding_material.set(false);
                reload();
            });
        }
    };

    let delete_action = {
        move |id: i64, kind: String| {
            if deleting.get() {
                return;
            }
            set_deleting.set(true);
            leptos::task::spawn_local(async move {
                if kind == "material" {
                    let _ = api::delete_printing_material(id).await;
                } else {
                    let _ = api::delete_stock(id).await;
                }
                set_del_id.set(None);
                set_del_label.set(String::new());
                set_deleting.set(false);
                reload();
            });
        }
    };

    let add_rolls_action = {
        move |id: i64, rolls: i64| {
            if adding_rolls.get() {
                return;
            }
            set_adding_rolls.set(true);
            leptos::task::spawn_local(async move {
                if let Err(e) = api::add_stock_rolls(id, rolls).await {
                    error!("add_stock_rolls failed: {}", e);
                } else {
                    set_add_rolls_item.set(None);
                    set_add_rolls_value.set(String::new());
                }
                set_adding_rolls.set(false);
                reload();
            });
        }
    };

    let add_mat_rolls_action = {
        move |id: i64, rolls: i64| {
            if adding_mat_rolls.get() {
                return;
            }
            set_adding_mat_rolls.set(true);
            leptos::task::spawn_local(async move {
                if let Err(e) = api::add_printing_material_rolls(id, rolls).await {
                    error!("add_printing_material_rolls failed: {}", e);
                } else {
                    set_add_mat_rolls_item.set(None);
                    set_add_mat_rolls_value.set(String::new());
                }
                set_adding_mat_rolls.set(false);
                reload();
            });
        }
    };

    let (query, set_query) = signal(String::new());
    let filtered_stock = move || {
        let q = query.get().trim().to_lowercase();
        let items = stock.get();
        if q.is_empty() {
            return items;
        }
        items
            .into_iter()
            .filter(|item| {
                item.color.to_lowercase().contains(&q)
                    || item.size.to_lowercase().contains(&q)
                    || item.sticker_type.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    };
    let filtered_materials = move || {
        let q = query.get().trim().to_lowercase();
        let items = materials.get();
        if q.is_empty() {
            return items;
        }
        items
            .into_iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&q)
                    || m.material_type.to_lowercase().contains(&q)
                    || m.width.to_string().contains(&q)
            })
            .collect::<Vec<_>>()
    };

    let modal_busy = move || adding_stock.get() || adding_material.get();

    view! {
        <Show when=move || !loading.get() fallback=|| view! {
            <div id="page-stock" class="dash">
                <PageLoading message="Loading stock..."/>
            </div>
        }>
        <div id="page-stock" class="dash">
            <div class="dash-table-head">
                <div>
                    <h2 class="dash-section-title">"Inventory"</h2>
                    <p class="prod-sub">"Sticker film and printing materials"</p>
                </div>
                <div class="dash-table-actions">
                    <label class="dash-search">
                        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-4.35-4.35M11 18a7 7 0 100-14 7 7 0 000 14z"/>
                        </svg>
                        <input
                            type="search"
                            placeholder="Search..."
                            prop:value=move || query.get()
                            on:input=move |ev| set_query.set(event_target_value(&ev))
                            aria-label="Search inventory"
                        />
                    </label>
                    {move || {
                        let is_mat = inventory_tab.get() == "materials";
                        let show_add_btn = !is_mat || can_manage_materials;
                        if !show_add_btn {
                            return ().into_any();
                        }
                        let label = if is_mat { " Add Material" } else { " Add Stock" };
                        view! {
                            <button
                                type="button"
                                id="btn-add-stock"
                                class="dash-btn-primary"
                                on:click=move |_| {
                                    reset_add_modal();
                                    set_modal_tab.set(if inventory_tab.get() == "materials" {
                                        "materials".into()
                                    } else {
                                        "stickers".into()
                                    });
                                    set_show_add.set(true);
                                }
                            >
                                <span aria-hidden="true">"+"</span>
                                {label}
                            </button>
                        }.into_any()
                    }}
                </div>
            </div>

            <div class="inventory-tabs" role="tablist" aria-label="Inventory type">
                <button
                    type="button"
                    role="tab"
                    class=move || if inventory_tab.get() == "stickers" { "is-active" } else { "" }
                    prop:aria-selected=move || inventory_tab.get() == "stickers"
                    on:click=move |_| {
                        set_inventory_tab.set("stickers".into());
                        set_query.set(String::new());
                    }
                >"Stickers"</button>
                <button
                    type="button"
                    role="tab"
                    class=move || if inventory_tab.get() == "materials" { "is-active" } else { "" }
                    prop:aria-selected=move || inventory_tab.get() == "materials"
                    on:click=move |_| {
                        set_inventory_tab.set("materials".into());
                        set_query.set(String::new());
                    }
                >"Print materials"</button>
            </div>

            // ---- Stickers tab ----
            {move || if inventory_tab.get() == "stickers" {
                view! {
                    <div class="prod-metrics dash-card stock-metrics">
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Colors / SKUs"</p>
                            <p class="dash-metric-value">{move || total_items()}</p>
                        </div>
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Total rolls"</p>
                            <p class="dash-metric-value">{move || total_rolls.get()}</p>
                        </div>
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Total metres"</p>
                            <p class="dash-metric-value">{move || format!("{}m", total_metres.get() as u64)}</p>
                        </div>
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Remaining"</p>
                            <p class="dash-metric-value">{move || format!("{}m", remaining_metres.get() as u64)}</p>
                        </div>
                    </div>

                    <div class="dash-card dash-table-card">
                        <table class="dash-table stock-table">
                            <thead>
                                <tr>
                                    <th>"Type"</th>
                                    <th>"Color"</th>
                                    <th>"Width"</th>
                                    <th>"Rolls"</th>
                                    <th>"Total"</th>
                                    <th>"Used"</th>
                                    <th>"Remaining"</th>
                                    <th>"Rolls left"</th>
                                    <th>"Status"</th>
                                    <th>"Actions"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    let items = filtered_stock();
                                    if items.is_empty() {
                                        return view! {
                                            <tr>
                                                <td colspan="10" class="dash-table-empty">
                                                    {if query.get().trim().is_empty() {
                                                        "No stock yet — add your first rolls."
                                                    } else {
                                                        "No stock matches your search."
                                                    }}
                                                </td>
                                            </tr>
                                        }.into_any();
                                    }
                                    items.into_iter().map(|item| {
                                        let id = item.id;
                                        let rem = remaining(&item);
                                        let left = rolls_left(&item);
                                        let (sl, sc) = status(&item);
                                        let typ = item.sticker_type.clone();
                                        let is_reflective = typ == "reflective";
                                        let type_cls = if is_reflective {
                                            "dash-status is-warn"
                                        } else {
                                            "dash-status is-info"
                                        };
                                        let type_label = if is_reflective { "Reflective" } else { "Colored" };
                                        let color_hex = get_hex(&item.color);
                                        let swatch_style = if is_reflective {
                                            reflective_swatch_style(color_hex)
                                        } else {
                                            let border = if hex_luminance(color_hex) > 0.85 {
                                                "border: 1px solid rgba(15,23,42,0.18);"
                                            } else {
                                                "border: 1px solid rgba(0,0,0,0.08);"
                                            };
                                            format!("background-color: {color_hex}; {border}")
                                        };
                                        let color_name = item.color.clone();
                                        let color_display = color_name.clone();
                                        let size = item.size.clone();
                                        let size_display = size.clone();
                                        let item_for_add = item.clone();
                                        let status_cls = match sc {
                                            "status-badge--error" => "dash-status is-danger",
                                            "status-badge--warning" => "dash-status is-warn",
                                            _ => "dash-status is-ok",
                                        };
                                        view! {
                                            <tr>
                                                <td><span class=type_cls>{type_label}</span></td>
                                                <td>
                                                    <div class="prod-color-cell">
                                                        <span class="prod-swatch" style=swatch_style></span>
                                                        <span class="dash-td-strong">{color_display}</span>
                                                    </div>
                                                </td>
                                                <td class="dash-td-muted tnum">{format!("{}\"", size_display)}</td>
                                                <td class="dash-td-muted tnum">{item.rolls}</td>
                                                <td class="dash-td-muted tnum">{format!("{}m", item.total_metres as u64)}</td>
                                                <td class="dash-td-muted tnum">{format!("{}m", item.metres_used as u64)}</td>
                                                <td class="dash-td-strong tnum">{format!("{}m", rem as u64)}</td>
                                                <td class="dash-td-muted tnum">{left}</td>
                                                <td><span class=status_cls>{sl}</span></td>
                                                <td>
                                                    <div class="prod-actions">
                                                        <button
                                                            type="button"
                                                            class="prod-btn-add"
                                                            on:click=move |_| {
                                                                set_add_rolls_value.set(String::new());
                                                                set_add_rolls_item.set(Some(item_for_add.clone()));
                                                            }
                                                        >"Add Rolls"</button>
                                                        <button
                                                            type="button"
                                                            class="prod-btn-icon is-danger"
                                                            aria-label="Delete stock"
                                                            on:click=move |_| {
                                                                set_del_kind.set("stock".into());
                                                                set_del_label.set(format!(
                                                                    "{} · {}\" {}",
                                                                    color_name,
                                                                    size,
                                                                    if is_reflective { "Reflective" } else { "Colored" }
                                                                ));
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
                            if n == 0 && query.get().trim().is_empty() {
                                return ().into_any();
                            }
                            let cp = current_page.get();
                            let tp = total_pages();
                            let si = if n == 0 { 0 } else { (cp - 1) * items_per_page + 1 };
                            let ei = (cp * items_per_page).min(n);
                            let showing = filtered_stock().len();
                            let count_label = if query.get().trim().is_empty() {
                                if n == 0 {
                                    "No stock".to_string()
                                } else {
                                    format!("Showing {}–{} of {}", si, ei, n)
                                }
                            } else {
                                format!("{} match{}", showing, if showing == 1 { "" } else { "es" })
                            };
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
                }.into_any()
            } else {
                // ---- Print materials tab ----
                let (m_count, m_rolls, m_total, m_rem) = mat_metrics();
                view! {
                    <div class="prod-metrics dash-card stock-metrics">
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Materials"</p>
                            <p class="dash-metric-value">{m_count}</p>
                        </div>
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Total rolls"</p>
                            <p class="dash-metric-value">{m_rolls}</p>
                        </div>
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Total metres"</p>
                            <p class="dash-metric-value">{format!("{}m", m_total as u64)}</p>
                        </div>
                        <div class="prod-metric">
                            <p class="dash-metric-label">"Remaining"</p>
                            <p class="dash-metric-value">{format!("{}m", m_rem as u64)}</p>
                        </div>
                    </div>

                    <div class="dash-card dash-table-card">
                        <table class="dash-table stock-table materials-table">
                            <thead>
                                <tr>
                                    <th>"Name"</th>
                                    <th>"Width"</th>
                                    <th>"m/roll"</th>
                                    <th>"Rolls"</th>
                                    <th>"Total"</th>
                                    <th>"Used"</th>
                                    <th>"Remaining"</th>
                                    <th>"Status"</th>
                                    <th>"Actions"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    let items = filtered_materials();
                                    if items.is_empty() {
                                        return view! {
                                            <tr>
                                                <td colspan="9" class="dash-table-empty">
                                                    {if query.get().trim().is_empty() {
                                                        "No print materials yet — add banner, satin, canvas, and more."
                                                    } else {
                                                        "No materials match your search."
                                                    }}
                                                </td>
                                            </tr>
                                        }.into_any();
                                    }
                                    items.into_iter().map(|m| {
                                        let id = m.id;
                                        let rem = mat_remaining(&m);
                                        let left = mat_rolls_left(&m);
                                        let (sl, sc) = mat_status(&m);
                                        let name = m.name.clone();
                                        let name_del = name.clone();
                                        let m_for_add = m.clone();
                                        let status_cls = match sc {
                                            "status-badge--error" => "dash-status is-danger",
                                            "status-badge--warning" => "dash-status is-warn",
                                            _ => "dash-status is-ok",
                                        };
                                        view! {
                                            <tr>
                                                <td><span class="dash-td-strong">{name}</span></td>
                                                <td class="dash-td-muted tnum">{format!("{}m", m.width)}</td>
                                                <td class="dash-td-muted tnum">{format!("{}m", m.metres_per_roll as u64)}</td>
                                                <td class="dash-td-muted tnum">{m.rolls}</td>
                                                <td class="dash-td-muted tnum">{format!("{}m", m.total_metres as u64)}</td>
                                                <td class="dash-td-muted tnum">{format!("{}m", m.metres_used as u64)}</td>
                                                <td class="dash-td-strong tnum">{format!("{:.1}m", rem)}
                                                    <span class="prod-sub tnum">{format!(" ({} rolls)", left)}</span>
                                                </td>
                                                <td><span class=status_cls>{sl}</span></td>
                                                <td>
                                                    <div class="prod-actions">
                                                        <button
                                                            type="button"
                                                            class="prod-btn-add"
                                                            on:click=move |_| {
                                                                set_add_mat_rolls_value.set(String::new());
                                                                set_add_mat_rolls_item.set(Some(m_for_add.clone()));
                                                            }
                                                        >"Add Rolls"</button>
                                                        <button
                                                            type="button"
                                                            class="prod-btn-icon is-danger"
                                                            aria-label="Delete material"
                                                            on:click=move |_| {
                                                                set_del_kind.set("material".into());
                                                                set_del_label.set(name_del.clone());
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
                            let showing = filtered_materials().len();
                            let total = materials.get().len();
                            if total == 0 && query.get().trim().is_empty() {
                                return ().into_any();
                            }
                            let count_label = if query.get().trim().is_empty() {
                                format!("{} material{}", total, if total == 1 { "" } else { "s" })
                            } else {
                                format!("{} match{}", showing, if showing == 1 { "" } else { "es" })
                            };
                            view! {
                                <div class="dash-table-foot">
                                    <span class="dash-table-count">{count_label}</span>
                                </div>
                            }.into_any()
                        }}
                    </div>
                }.into_any()
            }}

            // ---- Add inventory modal (tabs: Stickers | Print materials) ----
            {move || if show_add.get() {
                let stickers_active = modal_tab.get() == "stickers";
                let show_mat_tab = can_manage_materials;
                view! {
                    <div id="modal-add-stock" class="modal-overlay open">
                        <div class="modal-container">
                            <div class="modal-header">
                                <h3 class="modal-title">"Add inventory"</h3>
                                <button
                                    class="modal-close-btn"
                                    prop:disabled=modal_busy
                                    on:click=move |_| {
                                        if !modal_busy() {
                                            set_show_add.set(false);
                                        }
                                    }
                                >
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                    </svg>
                                </button>
                            </div>
                            <div class="modal-body">
                                <div class="modal-tabs">
                                    <button
                                        type="button"
                                        class=move || if modal_tab.get() == "stickers" { "is-active" } else { "" }
                                        prop:disabled=modal_busy
                                        on:click=move |_| set_modal_tab.set("stickers".into())
                                    >"Stickers"</button>
                                    {if show_mat_tab {
                                        view! {
                                            <button
                                                type="button"
                                                class=move || if modal_tab.get() == "materials" { "is-active" } else { "" }
                                                prop:disabled=modal_busy
                                                on:click=move |_| set_modal_tab.set("materials".into())
                                            >"Print materials"</button>
                                        }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                </div>

                                {if stickers_active {
                                    view! {
                                        <div class="space-y-5">
                                            <div>
                                                <label>"Sticker Type"</label>
                                                <div class="flex gap-2 mt-1">
                                                    <button
                                                        type="button"
                                                        on:click=move |_| set_sticker_type.set("colored".into())
                                                        class=move || format!(
                                                            "sticker-type-btn flex-1 px-4 py-2 {} font-medium text-sm transition-all",
                                                            if sticker_type.get()=="colored" {
                                                                "border border-brand-500 bg-brand-50 text-brand-600"
                                                            } else {
                                                                "border border-gray-200 bg-white text-gray-500 hover:border-gray-300"
                                                            }
                                                        )
                                                    >"Colored"</button>
                                                    <button
                                                        type="button"
                                                        on:click=move |_| set_sticker_type.set("reflective".into())
                                                        class=move || format!(
                                                            "sticker-type-btn flex-1 px-4 py-2 {} font-medium text-sm transition-all",
                                                            if sticker_type.get()=="reflective" {
                                                                "border border-brand-500 bg-brand-50 text-brand-600"
                                                            } else {
                                                                "border border-gray-200 bg-white text-gray-500 hover:border-gray-300"
                                                            }
                                                        )
                                                    >"Reflective"</button>
                                                </div>
                                            </div>
                                            <div>
                                                <label>"Color"</label>
                                                <div class="relative mt-1">
                                                    <input
                                                        type="text"
                                                        class="w-full pr-12"
                                                        placeholder=move || if sticker_type.get()=="reflective" {
                                                            "e.g. Red, White, Yellow"
                                                        } else {
                                                            "e.g. Red Dark, Black Matte"
                                                        }
                                                        autocomplete="off"
                                                        prop:value=move || color.get()
                                                        on:input=move |e| set_color.set(event_target_value(&e))
                                                    />
                                                    <div
                                                        class="absolute right-3 top-1/2 -translate-y-1/2 w-5 h-5 border border-gray-200"
                                                        style=move || format!("background-color: {};", get_hex(&color.get()))
                                                    ></div>
                                                </div>
                                                <p class="text-xs text-gray-400 mt-1">
                                                    {move || if sticker_type.get()=="reflective" {
                                                        "Enter reflective color"
                                                    } else {
                                                        "Enter color with variant (dark, light, matte, gloss)"
                                                    }}
                                                </p>
                                            </div>
                                            <div>
                                                <label>"Size Variants"</label>
                                                <div class="space-y-3 mt-2">
                                                    <For
                                                        each=move || rows.get()
                                                        key=|r| r.id
                                                        children=move |r| {
                                                            let id = r.id;
                                                            view! {
                                                                <div class="grid grid-cols-2 gap-3" data-row-id=id>
                                                                    <div>
                                                                        <label>{if id==0 {"Width (inches) *"} else {"Width (inches)"}}</label>
                                                                        <input
                                                                            type="number"
                                                                            class="w-full size-input"
                                                                            step="1"
                                                                            min="1"
                                                                            placeholder="e.g. 24"
                                                                            prop:value=r.size
                                                                            on:input=move |e| update_row(id,"size",event_target_value(&e))
                                                                        />
                                                                    </div>
                                                                    <div>
                                                                        <label>{if id==0 {"Rolls *"} else {"Rolls"}}</label>
                                                                        <input
                                                                            type="number"
                                                                            class="w-full rolls-input"
                                                                            min="1"
                                                                            placeholder="e.g. 5"
                                                                            prop:value=r.rolls
                                                                            on:input=move |e| update_row(id,"rolls",event_target_value(&e))
                                                                        />
                                                                    </div>
                                                                    {move || if sticker_type.get()=="reflective" {
                                                                        view! {
                                                                            <div>
                                                                                <label>{if id==0 {"Metres per Roll *"} else {"Metres per Roll"}}</label>
                                                                                <input
                                                                                    type="number"
                                                                                    class="w-full metres-per-roll-input"
                                                                                    step="0.1"
                                                                                    min="1"
                                                                                    prop:value=r.metres_per_roll.clone()
                                                                                    on:input=move |e| update_row(id,"mpr",event_target_value(&e))
                                                                                />
                                                                            </div>
                                                                        }.into_any()
                                                                    } else {
                                                                        ().into_any()
                                                                    }}
                                                                    <div>
                                                                        <label>"Total Metres"</label>
                                                                        <div class="px-3 py-2 bg-gray-50 border border-gray-200 text-gray-600 text-sm">
                                                                            <span class="metres-display font-medium">
                                                                                {move || rows.get().iter().find(|x| x.id==id).map(row_total).unwrap_or(0.0) as u64}
                                                                            </span>
                                                                            "m"
                                                                        </div>
                                                                    </div>
                                                                    {if id!=0 {
                                                                        view! {
                                                                            <div class="col-span-2 flex justify-end">
                                                                                <button
                                                                                    type="button"
                                                                                    class="remove-row-btn text-xs text-gray-400 hover:text-red-500 font-medium"
                                                                                    on:click=move |_| set_rows.update(|rs| rs.retain(|x| x.id!=id))
                                                                                >"Remove"</button>
                                                                            </div>
                                                                        }.into_any()
                                                                    } else {
                                                                        ().into_any()
                                                                    }}
                                                                </div>
                                                            }
                                                        }
                                                    />
                                                </div>
                                                <button
                                                    type="button"
                                                    class="mt-3 text-sm text-brand-500 hover:text-brand-600 font-medium flex items-center gap-1"
                                                    on:click=move |_| add_row()
                                                >
                                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                                                    </svg>
                                                    "Add size"
                                                </button>
                                            </div>
                                            <div class="bg-gray-50 border border-gray-100 p-4">
                                                <div class="flex justify-between items-center text-sm">
                                                    <span class="font-medium text-gray-600">"Total"</span>
                                                    <div class="flex gap-6">
                                                        <span class="text-gray-500">
                                                            <span class="font-semibold text-gray-900">{move || total_rolls_modal()}</span>
                                                            " rolls"
                                                        </span>
                                                        <span class="text-gray-500">
                                                            <span class="font-semibold text-gray-900">{move || total_metres_modal() as u64}</span>
                                                            "m"
                                                        </span>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    // Print materials form — metres per roll is always manual
                                    view! {
                                        <div class="space-y-4">
                                            <div>
                                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
                                                    "Material Name *"
                                                </label>
                                                <input
                                                    type="text"
                                                    class="w-full"
                                                    placeholder="e.g., White Banner Vinyl, Blue Satin Fabric"
                                                    prop:value=move || mat_name.get()
                                                    on:input=move |e| set_mat_name.set(event_target_value(&e))
                                                />
                                            </div>
                                            <div>
                                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
                                                    "Width (metres) *"
                                                </label>
                                                <input
                                                    type="number"
                                                    step="0.1"
                                                    min="0.1"
                                                    class="w-full"
                                                    placeholder="Enter width in metres"
                                                    prop:value=move || mat_width.get()
                                                    on:input=move |e| set_mat_width.set(event_target_value(&e))
                                                />
                                            </div>
                                            <div class="grid grid-cols-2 gap-4">
                                                <div>
                                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
                                                        "Rolls *"
                                                    </label>
                                                    <input
                                                        type="number"
                                                        min="1"
                                                        class="w-full"
                                                        prop:value=move || mat_rolls.get()
                                                        on:input=move |e| set_mat_rolls.set(event_target_value(&e))
                                                    />
                                                </div>
                                                <div>
                                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
                                                        "Metres per Roll *"
                                                    </label>
                                                    <input
                                                        type="number"
                                                        step="0.1"
                                                        min="1"
                                                        class="w-full"
                                                        prop:value=move || mat_mpr.get()
                                                        on:input=move |e| set_mat_mpr.set(event_target_value(&e))
                                                    />
                                                    <p class="text-xs text-gray-400 mt-1">
                                                        "Required — print media rolls vary (not always 50m)"
                                                    </p>
                                                </div>
                                            </div>
                                            <div class="bg-gray-50 border border-gray-200 p-4">
                                                <div class="flex justify-between items-center text-sm">
                                                    <span class="text-gray-600">"Total metres"</span>
                                                    <span class="font-semibold text-gray-900">
                                                        {move || format!("{}m", mat_total_metres_preview() as u64)}
                                                    </span>
                                                </div>
                                                <p class="text-xs text-gray-400 mt-1">
                                                    "Calculated as rolls × metres per roll"
                                                </p>
                                            </div>
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                            <div class="modal-footer">
                                <button
                                    type="button"
                                    class="btn-secondary"
                                    prop:disabled=modal_busy
                                    on:click=move |_| {
                                        if !modal_busy() {
                                            set_show_add.set(false);
                                        }
                                    }
                                >"Cancel"</button>
                                {if stickers_active {
                                    view! {
                                        <button
                                            type="button"
                                            class="btn-primary"
                                            prop:disabled=move || adding_stock.get()
                                            on:click=move |_| {
                                                add_stock_action(color.get(), sticker_type.get(), rows.get());
                                            }
                                        >
                                            {move || if adding_stock.get() { "Adding..." } else { "Add Stock" }}
                                        </button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <button
                                            type="button"
                                            class="btn-primary"
                                            prop:disabled=move || adding_material.get()
                                            on:click=add_material_action
                                        >
                                            {move || if adding_material.get() { "Adding..." } else { "Add Material" }}
                                        </button>
                                    }.into_any()
                                }}
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            // Add rolls — stickers
            {move || add_rolls_item.get().map(|item| {
                let added = move || add_rolls_value.get().parse::<i64>().unwrap_or(0);
                let new_rolls = move || item.rolls + added();
                let new_metres = move || item.total_metres + added() as f64 * item.metres_per_roll;
                view! {
                    <div id="modal-add-rolls" class="modal-overlay open">
                        <div class="modal-container" style="max-width: 500px;">
                            <div class="modal-header">
                                <h3 class="modal-title">"Add Rolls to Stock"</h3>
                                <button
                                    type="button"
                                    class="modal-close-btn"
                                    prop:disabled=move || adding_rolls.get()
                                    on:click=move |_| {
                                        if !adding_rolls.get() {
                                            set_add_rolls_item.set(None);
                                        }
                                    }
                                >
                                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                    </svg>
                                </button>
                            </div>
                            <div class="modal-body">
                                <div class="bg-gray-50 p-4 mb-4">
                                    <p class="text-xs text-gray-500 uppercase tracking-wide">"Stock Item"</p>
                                    <p class="font-semibold text-gray-900">
                                        {format!(
                                            "{} - {}\" {}",
                                            item.color,
                                            item.size,
                                            if item.sticker_type=="reflective" {"Reflective"} else {"Colored"}
                                        )}
                                    </p>
                                    <p class="text-sm text-gray-600 mt-1">
                                        {format!("Current: {}m remaining ({} rolls)", remaining(&item) as u64, rolls_left(&item))}
                                    </p>
                                </div>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
                                            "Number of Rolls to Add *"
                                        </label>
                                        <input
                                            type="number"
                                            min="1"
                                            step="1"
                                            class="w-full"
                                            placeholder="Enter rolls to add"
                                            prop:value=move || add_rolls_value.get()
                                            on:input=move |e| set_add_rolls_value.set(event_target_value(&e))
                                        />
                                        <p class="text-xs text-gray-500 mt-1">
                                            {format!("Each roll = {}m", item.metres_per_roll as u64)}
                                        </p>
                                    </div>
                                    <div class="bg-neutral-50 border border-neutral-200 p-3">
                                        <p class="text-sm text-gray-700">
                                            <span class="font-medium">"New Total:"</span>
                                            {move || format!(" {} rolls ({}m)", new_rolls(), new_metres() as u64)}
                                        </p>
                                    </div>
                                </div>
                            </div>
                            <div class="modal-footer">
                                <button
                                    type="button"
                                    class="btn-secondary"
                                    prop:disabled=move || adding_rolls.get()
                                    on:click=move |_| {
                                        if !adding_rolls.get() {
                                            set_add_rolls_item.set(None);
                                        }
                                    }
                                >"Cancel"</button>
                                <button
                                    type="button"
                                    class="btn-primary"
                                    prop:disabled=move || adding_rolls.get()
                                    on:click=move |_| {
                                        let id = item.id;
                                        let rolls = added();
                                        if rolls > 0 {
                                            add_rolls_action(id, rolls);
                                        }
                                    }
                                >
                                    {move || if adding_rolls.get() { "Adding..." } else { "Add Rolls" }}
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            }).unwrap_or_else(|| ().into_any())}

            // Add rolls — print materials (uses stored metres_per_roll)
            {move || add_mat_rolls_item.get().map(|m| {
                let mpr = m.metres_per_roll;
                let added = move || add_mat_rolls_value.get().parse::<i64>().unwrap_or(0);
                view! {
                    <div id="modal-add-mat-rolls" class="modal-overlay open">
                        <div class="modal-container" style="max-width: 500px;">
                            <div class="modal-header">
                                <h3 class="modal-title">"Add Rolls to Printing Material"</h3>
                                <button
                                    class="modal-close-btn"
                                    prop:disabled=move || adding_mat_rolls.get()
                                    on:click=move |_| {
                                        if !adding_mat_rolls.get() {
                                            set_add_mat_rolls_item.set(None);
                                        }
                                    }
                                >
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                    </svg>
                                </button>
                            </div>
                            <div class="modal-body">
                                <div class="bg-gray-50 p-4 mb-4">
                                    <p class="text-xs text-gray-500 uppercase tracking-wide">"Printing Material"</p>
                                    <p class="font-semibold text-gray-900">{m.name.clone()}</p>
                                    <p class="text-sm text-gray-600 mt-1">
                                        {format!("Width: {}m · {}m per roll", m.width, m.metres_per_roll as u64)}
                                    </p>
                                    <p class="text-sm text-gray-600 mt-1">
                                        {format!(
                                            "Current: {:.1}m remaining ({} rolls)",
                                            mat_remaining(&m),
                                            mat_rolls_left(&m)
                                        )}
                                    </p>
                                </div>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">
                                            "Number of Rolls to Add *"
                                        </label>
                                        <input
                                            type="number"
                                            min="1"
                                            step="1"
                                            class="w-full"
                                            placeholder="Enter rolls to add"
                                            prop:value=move || add_mat_rolls_value.get()
                                            on:input=move |e| set_add_mat_rolls_value.set(event_target_value(&e))
                                        />
                                        <p class="text-xs text-gray-500 mt-1">
                                            {format!("Each roll = {}m (from this material)", mpr as u64)}
                                        </p>
                                    </div>
                                    <div class="bg-neutral-50 border border-neutral-200 p-3">
                                        <p class="text-sm text-gray-700">
                                            <span class="font-medium">"New Total:"</span>
                                            {move || {
                                                let a = added();
                                                format!(
                                                    " {} rolls ({}m)",
                                                    m.rolls + a,
                                                    (m.total_metres + a as f64 * mpr) as u64
                                                )
                                            }}
                                        </p>
                                    </div>
                                </div>
                            </div>
                            <div class="modal-footer">
                                <button
                                    type="button"
                                    class="btn-secondary px-4 py-2"
                                    prop:disabled=move || adding_mat_rolls.get()
                                    on:click=move |_| {
                                        if !adding_mat_rolls.get() {
                                            set_add_mat_rolls_item.set(None);
                                        }
                                    }
                                >"Cancel"</button>
                                <button
                                    type="button"
                                    class="btn-primary px-4 py-2"
                                    prop:disabled=move || adding_mat_rolls.get()
                                    on:click=move |_| {
                                        let id = m.id;
                                        let rolls = added();
                                        if rolls > 0 {
                                            add_mat_rolls_action(id, rolls);
                                        }
                                    }
                                >
                                    {move || if adding_mat_rolls.get() { "Adding..." } else { "Add Rolls" }}
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            }).unwrap_or_else(|| ().into_any())}

            <Show when=move || del_id.get().is_some()>
                <div
                    class="modal-overlay open"
                    on:click=move |e| {
                        if e.target() == e.current_target() && !deleting.get() {
                            set_del_id.set(None);
                            set_del_label.set(String::new());
                        }
                    }
                >
                    <div class="modal-container modal-sm">
                        <div class="modal-header">
                            <h3 class="modal-title">
                                {move || if del_kind.get() == "material" {
                                    "Delete Material?"
                                } else {
                                    "Delete Stock?"
                                }}
                            </h3>
                            <button
                                type="button"
                                class="modal-close-btn"
                                prop:disabled=move || deleting.get()
                                on:click=move |_| {
                                    if !deleting.get() {
                                        set_del_id.set(None);
                                        set_del_label.set(String::new());
                                    }
                                }
                            >
                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
                                prop:disabled=move || deleting.get()
                                on:click=move |_| {
                                    if !deleting.get() {
                                        set_del_id.set(None);
                                        set_del_label.set(String::new());
                                    }
                                }
                            >"Cancel"</button>
                            <button
                                type="button"
                                class="btn-danger"
                                prop:disabled=move || deleting.get()
                                on:click=move |_| {
                                    if let Some(id) = del_id.get() {
                                        delete_action(id, del_kind.get());
                                    }
                                }
                            >{move || if deleting.get() { "Deleting..." } else { "Delete" }}</button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
        </Show>
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

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for (j, item) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
        *item = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a.as_bytes()[i - 1] == b.as_bytes()[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
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
        .replace(['-', '_'], " ")
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
    let modifiers = [
        "dark", "light", "bright", "pale", "deep", "pastel", "medium", "vivid",
    ];
    let finish_modifiers = [
        "gloss",
        "glossy",
        "matte",
        "matt",
        "satin",
        "metallic",
        "chrome",
        "mirror",
        "pearl",
        "fluorescent",
        "neon",
    ];

    let words: Vec<&str> = resolved.split_whitespace().collect();

    // Check for modifier + color pattern (e.g., "dark red", "light blue")
    for (i, word) in words.iter().enumerate() {
        if modifiers.contains(word) {
            // Find the color part
            let color_part: String = words
                .iter()
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
            let color_part: String = words
                .iter()
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
        let distance = levenshtein_distance(resolved, &entry.name.to_lowercase());
        if distance < best_distance && distance <= 2 {
            best_distance = distance;
            best_match = Some(entry.hex);
        }
    }

    best_match.unwrap_or("#9ca3af")
}

pub(crate) fn get_hex(name: &str) -> &'static str {
    parse_color_with_modifiers(name)
}

/// Relative luminance of a `#RRGGBB` color (0.0 dark → 1.0 light).
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

/// CSS for reflective stock swatches.
/// Light bases (white / pale yellow) need slate-silver stripes — pure white
/// stripes on white would be invisible and look like "no preview".
pub(crate) fn reflective_swatch_style(hex: &str) -> String {
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
