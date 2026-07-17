use leptos::prelude::*;

mod api;
mod app;
mod auto_refresh;

fn main() {
    console_log::init_with_level(log::Level::Debug).ok();
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <app::App /> });
}
