use leptos::prelude::*;
use leptos_router::components::*;

#[component]
pub fn Sidebar(user_role: String, on_logout: Callback<leptos::ev::MouseEvent>) -> impl IntoView {
    let is_admin = user_role == "admin";
    let (active_page, _set_active) = signal("dashboard");

    let nav_link = |path: &str, label: &str, icon: &str, visible: bool| {
        if !visible { return None; }
        let p = path.to_string();
        let l = label.to_string();
        let i = icon.to_string();
        let is_dash = p == "/";
        let href = if is_dash { "/".into() } else { format!("/{}", p) };
        let cls = if active_page.get() == p || (is_dash && active_page.get() == "dashboard") {
            "flex items-center gap-3 px-4 py-2.5 mx-2 rounded-lg text-sm bg-brand-50 text-brand-700 font-medium"
        } else {
            "flex items-center gap-3 px-4 py-2.5 mx-2 rounded-lg text-sm text-gray-600 hover:bg-gray-50"
        };
        Some(view! {
            <A href=href class=cls>
                <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d=&i/>
                </svg>
                <span>{l}</span>
            </A>
        })
    };

    view! {
        <aside class="w-64 bg-white border-r border-gray-200 flex flex-col shrink-0">
            <div class="p-4 border-b border-gray-100">
                <div class="flex items-center gap-2">
                    <svg class="w-8 h-8 text-brand-600" viewBox="0 0 32 32" fill="none">
                        <rect x="2" y="2" width="28" height="28" rx="4" stroke="currentColor" stroke-width="1.5"/>
                        <path d="M9 11h14M9 16h10M9 21h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                    </svg>
                    <span class="font-semibold text-lg">"MULTIPRINTS"</span>
                </div>
            </div>

            <nav class="flex-1 py-2 overflow-y-auto">
                {nav_link("dashboard", "Dashboard", "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z", true)}
                {nav_link("products", "Products", "M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4", true)}
                {nav_link("stock", "Stock", "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10", true)}
                {nav_link("sales", "Sales", "M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 11-4 0 2 2 0 014 0z", true)}
                {nav_link("printing", "Printing", "M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z", true)}
                {nav_link("debts", "Debts", "M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z", true)}
            </nav>

            <div class="border-t border-gray-100 p-2">
                <A href="/settings" class="flex items-center gap-3 px-4 py-2.5 mx-2 rounded-lg text-sm text-gray-600 hover:bg-gray-50">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    <span>"Settings"</span>
                </A>
                <button on:click=on_logout class="flex items-center gap-3 px-4 py-2.5 mx-2 w-[calc(100%-1rem)] rounded-lg text-sm text-red-600 hover:bg-red-50">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"/>
                    </svg>
                    <span>"Log out"</span>
                </button>
            </div>
        </aside>
    }
}
