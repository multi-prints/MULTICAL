use crate::api::{self, User, UserInfo};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;

#[path = "../components/dropdown.rs"]
mod dropdown_comp;
use dropdown_comp::{CustomDropdown, DropdownItem};

#[derive(Clone, Copy, PartialEq)]
enum SettingsTab {
    Account,
    Backup,
    About,
}

#[component]
pub fn SettingsPage(
    user: ReadSignal<Option<UserInfo>>,
    set_user: WriteSignal<Option<UserInfo>>,
) -> impl IntoView {
    let (active_tab, set_active_tab) = signal(SettingsTab::Account);
    let (msg, set_msg) = signal(None::<(bool, String)>);
    let (users, set_users) = signal(Vec::<User>::new());
    let (app_version, set_app_version) = signal("1.1.3".to_string());
    let (platform, set_platform) = signal("Tauri (Desktop)".to_string());
    let (update_status, set_update_status) = signal(None::<String>);
    let (checking_update, set_checking_update) = signal(false);
    let (show_uninstall, set_show_uninstall) = signal(false);
    let (uninstalling, set_uninstalling) = signal(false);

    // Username change
    let (new_username, set_new_username) = signal(String::new());
    // Password change
    let (old_pw, set_old_pw) = signal(String::new());
    let (new_pw, set_new_pw) = signal(String::new());
    // Add user (admin)
    let (new_user, set_new_user) = signal(String::new());
    let (new_upass, set_new_upass) = signal(String::new());
    let (new_role, set_new_role) = signal("employee".to_string());

    let cur_user = move || user.get().map(|u| u.username).unwrap_or_default();
    let is_admin = move || {
        user.get()
            .map(|u| u.role.as_str() == "admin")
            .unwrap_or(false)
    };
    let role_items = Signal::derive(move || {
        vec![
            DropdownItem::new("employee", "Employee"),
            DropdownItem::new("admin", "Admin"),
        ]
    });

    let load_users = move || {
        leptos::task::spawn_local(async move {
            if let Ok(u) = api::get_all_users().await {
                set_users.set(u);
            }
        });
    };
    load_users();

    leptos::task::spawn_local(async move {
        if let Ok(v) = api::get_app_version().await {
            set_app_version.set(v);
        }
        if let Ok(p) = api::get_platform().await {
            set_platform.set(format!("Tauri ({})", p));
        }
    });

    let install_update = move |_| {
        set_checking_update.set(true);
        set_update_status.set(Some("Checking for updates...".into()));
        leptos::task::spawn_local(async move {
            match api::check_and_install_update().await {
                Ok(result) => set_update_status.set(Some(result.message)),
                Err(e) => set_update_status.set(Some(e)),
            }
            set_checking_update.set(false);
        });
    };

    let do_uninstall = move |_| {
        set_uninstalling.set(true);
        leptos::task::spawn_local(async move {
            match api::uninstall_app().await {
                Ok(_) => set_msg.set(Some((true, "Uninstalling…".into()))),
                Err(e) => {
                    set_msg.set(Some((false, e)));
                    set_uninstalling.set(false);
                }
            }
        });
    };

    let change_username = move |_| {
        let old = cur_user();
        let new_name = new_username.get().trim().to_string();
        if old.is_empty() || new_name.is_empty() || old == new_name {
            return;
        }
        leptos::task::spawn_local(async move {
            match api::update_username(&old, &new_name).await {
                Ok(r) if r.success => {
                    set_user.update(|u| {
                        if let Some(info) = u {
                            info.username = new_name.clone();
                            LocalStorage::set(
                                "currentUser",
                                serde_json::to_string(info).unwrap_or_default(),
                            )
                            .ok();
                        }
                    });
                    set_new_username.set(String::new());
                    set_msg.set(Some((true, "Username updated successfully".into())));
                }
                Ok(r) => set_msg.set(Some((false, r.error.unwrap_or_else(|| "Failed".into())))),
                Err(e) => set_msg.set(Some((false, e))),
            }
        });
    };

    let change_pw = move |_| {
        let o = old_pw.get();
        let n = new_pw.get();
        if o.is_empty() || n.is_empty() {
            return;
        }
        let un = cur_user();
        leptos::task::spawn_local(async move {
            match api::update_password(&un, &o, &n).await {
                Ok(r) if r.success => {
                    set_msg.set(Some((true, "Password updated successfully".into())));
                    set_old_pw.set(String::new());
                    set_new_pw.set(String::new());
                }
                Ok(r) => set_msg.set(Some((false, r.error.unwrap_or_else(|| "Failed".into())))),
                Err(e) => set_msg.set(Some((false, e))),
            }
        });
    };

    let add_user = move |_| {
        let n = new_user.get();
        let p = new_upass.get();
        let r = new_role.get();
        if n.is_empty() || p.is_empty() {
            return;
        }
        leptos::task::spawn_local(async move {
            match api::add_user(&n, &p, &r).await {
                Ok(r) if r.success => {
                    set_msg.set(Some((true, "User added".into())));
                    set_new_user.set(String::new());
                    set_new_upass.set(String::new());
                    if let Ok(u) = api::get_all_users().await {
                        set_users.set(u);
                    }
                }
                Ok(r) => set_msg.set(Some((false, r.error.unwrap_or_default()))),
                Err(e) => set_msg.set(Some((false, e))),
            }
        });
    };

    let delete_user = move |username: String| {
        leptos::task::spawn_local(async move {
            let _ = api::delete_user(username).await;
            if let Ok(u) = api::get_all_users().await {
                set_users.set(u);
            }
        });
    };

    view! {<div id="page-settings" class="page-content">
        <div class="flex items-center justify-between mb-6">
            <div><h1 class="page-title">"Settings"</h1><p class="page-subtitle">"Manage your application preferences and account"</p></div>
        </div>

        // Tabs
        <div class="mb-6"><div class="border-b border-gray-200"><nav class="flex gap-8">
            <button class=move || format!("settings-tab pb-4 px-2 text-sm font-medium border-b-2 transition-colors {}", if active_tab.get()==SettingsTab::Account {"border-brand-500 text-gray-900"} else {"border-transparent text-gray-500 hover:text-gray-700"})
                on:click=move |_| set_active_tab.set(SettingsTab::Account)>"Account"</button>
            {move || if is_admin() { view!{<button class=move || format!("settings-tab pb-4 px-2 text-sm font-medium border-b-2 transition-colors {}", if active_tab.get()==SettingsTab::Backup {"border-brand-500 text-gray-900"} else {"border-transparent text-gray-500 hover:text-gray-700"})
                on:click=move |_| set_active_tab.set(SettingsTab::Backup)>"Backup & Data"</button>}.into_any() } else { ().into_any() }}
            <button class=move || format!("settings-tab pb-4 px-2 text-sm font-medium border-b-2 transition-colors {}", if active_tab.get()==SettingsTab::About {"border-brand-500 text-gray-900"} else {"border-transparent text-gray-500 hover:text-gray-700"})
                on:click=move |_| set_active_tab.set(SettingsTab::About)>"About"</button>
        </nav></div></div>

        // Message
        {move || msg.get().map(|(ok, m)| view!{<div class=format!("px-4 py-3 rounded-lg mb-4 text-sm {}", if ok {"bg-green-50 text-green-700"} else {"bg-red-50 text-red-700"})>{m}</div>})}

        // Account Panel
        {move || if active_tab.get() == SettingsTab::Account { view!{<div>
            <div class="dashboard-panel p-6 mb-6">
                <h3 class="text-base font-semibold text-gray-900 mb-4">"Change Username"</h3>
                <div class="space-y-4">
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Current Username"</label>
                        <input type="text" class="w-full bg-gray-50" readonly prop:value=cur_user/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"New Username"</label>
                        <input type="text" class="w-full" placeholder="Enter new username"
                            prop:value=move || new_username.get() on:input=move |e| set_new_username.set(event_target_value(&e))/></div>
                    <div class="flex justify-end pt-4 border-t border-gray-100">
                        <button type="button" class="btn-primary px-6 py-2" on:click=change_username>"Update Username"</button>
                    </div>
                </div>
            </div>

            <div class="dashboard-panel p-6">
                <h3 class="text-base font-semibold text-gray-900 mb-4">"Change Password"</h3>
                <div class="space-y-4">
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Current Password"</label>
                        <input type="password" class="w-full" placeholder="Enter current password"
                            prop:value=move || old_pw.get() on:input=move |e| set_old_pw.set(event_target_value(&e))/></div>
                    <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"New Password"</label>
                        <input type="password" class="w-full" placeholder="Enter new password"
                            prop:value=move || new_pw.get() on:input=move |e| set_new_pw.set(event_target_value(&e))/></div>
                    <div class="flex justify-end pt-4 border-t border-gray-100">
                        <button type="button" class="btn-primary px-6 py-2" on:click=change_pw>"Update Password"</button>
                    </div>
                </div>
            </div>
        </div>}.into_any() } else { ().into_any() }}

        // Backup & Data Panel (admin only)
        {move || if active_tab.get() == SettingsTab::Backup && is_admin() { view!{<div>
            <div class="dashboard-panel p-6 mb-6">
                <h3 class="text-base font-semibold text-gray-900 mb-2">"Database Backup"</h3>
                <p class="text-sm text-gray-500 mb-4">"Export or import your business data"</p>
                <div class="space-y-3">
                    <button class="w-full flex items-center justify-between p-4 border border-gray-200 hover:bg-gray-50 transition-colors">
                        <div class="flex items-center gap-3">
                            <div class="w-10 h-10 bg-blue-50 flex items-center justify-center">
                                <svg class="w-5 h-5 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg>
                            </div>
                            <div class="text-left"><p class="text-sm font-medium text-gray-900">"Export Database"</p><p class="text-xs text-gray-500">"Download all your data as JSON file"</p></div>
                        </div>
                        <svg class="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/></svg>
                    </button>
                    <button class="w-full flex items-center justify-between p-4 border border-gray-200 hover:bg-gray-50 transition-colors">
                        <div class="flex items-center gap-3">
                            <div class="w-10 h-10 bg-green-50 flex items-center justify-center">
                                <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/></svg>
                            </div>
                            <div class="text-left"><p class="text-sm font-medium text-gray-900">"Import Database"</p><p class="text-xs text-gray-500">"Restore data from backup file"</p></div>
                        </div>
                        <svg class="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/></svg>
                    </button>
                </div>
            </div>
            <div class="dashboard-panel p-6 bg-red-50 border-red-200">
                <h3 class="text-base font-semibold text-red-900 mb-2">"Danger Zone"</h3>
                <p class="text-sm text-red-700 mb-4">"Irreversible actions - proceed with caution"</p>
                <div class="space-y-3">
                    <button class="w-full bg-red-600 hover:bg-red-700 text-white font-medium px-4 py-2 transition-colors"
                        on:click=move |_| { leptos::task::spawn_local(async move { let _ = api::clear_all_data().await; set_msg.set(Some((true, "All data cleared".into()))); }); }>"Clear All Data"</button>
                    <button class="w-full bg-red-600 hover:bg-red-700 text-white font-medium px-4 py-2 transition-colors"
                        on:click=move |_| set_show_uninstall.set(true)>"Uninstall MULTIPRINTS"</button>
                </div>
            </div>
        </div>}.into_any() } else { ().into_any() }}

        // Uninstall confirmation modal
        {move || if show_uninstall.get() { view!{
            <div class="fixed inset-0 z-50 flex items-center justify-center">
                <div class="absolute inset-0 bg-black/40" on:click=move |_| set_show_uninstall.set(false)></div>
                <div class="relative bg-white rounded-xl shadow-xl p-6 w-full max-w-md mx-4">
                    <h3 class="text-lg font-semibold mb-2">"Uninstall MULTIPRINTS"</h3>
                    <p class="text-gray-600 text-sm mb-2">"This will remove the application and all its data from your system."</p>
                    <p class="text-red-600 text-sm mb-6">"This action cannot be undone."</p>
                    <div class="flex justify-end gap-3">
                        <button on:click=move |_| set_show_uninstall.set(false)
                            class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg">"Cancel"</button>
                        <button on:click=do_uninstall prop:disabled=move || uninstalling.get()
                            class="px-4 py-2 text-sm bg-red-600 text-white hover:bg-red-700 rounded-lg disabled:opacity-50">
                            {move || if uninstalling.get() { "Uninstalling…" } else { "Uninstall" }}
                        </button>
                    </div>
                </div>
            </div>
        }.into_any() } else { ().into_any() }}

        // About Panel
        {move || if active_tab.get() == SettingsTab::About { view!{<div>
            <div class="dashboard-panel p-6 mb-6">
                <div class="text-center py-8">
                    <div class="w-16 h-16 bg-brand-500 flex items-center justify-center mx-auto mb-4"><span class="text-2xl font-bold text-white">"M"</span></div>
                    <h3 class="text-xl font-bold text-gray-900 mb-2">"Multiprints"</h3>
                    <p class="text-sm text-gray-500 mb-4">"Inventory & Sales Management System"</p>
                    <div class="inline-flex items-center gap-2 bg-gray-100 px-4 py-2">
                        <span class="text-xs font-medium text-gray-500">"Version"</span>
                        <span class="text-xs font-bold text-gray-900">{app_version.get()}</span>
                    </div>
                    <div class="mt-6">
                        <button class="btn-primary px-5 py-2 disabled:opacity-50" prop:disabled=move || checking_update.get() on:click=install_update>
                            {move || if checking_update.get() { "Updating..." } else { "Check & Install Update" }}
                        </button>
                    </div>
                    {move || update_status.get().map(|text| view!{<p class="mt-3 text-xs text-gray-500">{text}</p>})}
                </div>
            </div>
            <div class="dashboard-panel p-6 mb-6">
                <h3 class="text-base font-semibold text-gray-900 mb-4">"System Information"</h3>
                <div class="space-y-3">
                    <div class="flex justify-between py-2 border-b border-gray-100"><span class="text-sm text-gray-500">"Platform"</span><span class="text-sm font-medium text-gray-900">{platform.get()}</span></div>
                    <div class="flex justify-between py-2 border-b border-gray-100"><span class="text-sm text-gray-500">"Database"</span><span class="text-sm font-medium text-gray-900">"SQLite"</span></div>
                    <div class="flex justify-between py-2 border-b border-gray-100"><span class="text-sm text-gray-500">"Framework"</span><span class="text-sm font-medium text-gray-900">"Rust + Leptos"</span></div>
                </div>
            </div>
            <div class="dashboard-panel p-6">
                <h3 class="text-base font-semibold text-gray-900 mb-4">"Developer"</h3>
                <div class="space-y-3">
                    <div class="flex justify-between py-2 border-b border-gray-100"><span class="text-sm text-gray-500">"Author"</span><span class="text-sm font-medium text-gray-900">"Godwin Mayodi"</span></div>
                    <div class="flex justify-between py-2 border-b border-gray-100"><span class="text-sm text-gray-500">"Email"</span><span class="text-sm font-medium text-gray-900">"codegoddy@gmail.com"</span></div>
                    <div class="flex justify-between py-2 border-b border-gray-100"><span class="text-sm text-gray-500">"Repository"</span><a href="https://github.com/multi-prints/MULTICAL" target="_blank" class="text-sm font-medium text-brand-500 hover:underline">"github.com/multi-prints/MULTICAL"</a></div>
                </div>
            </div>
        </div>}.into_any() } else { ().into_any() }}

        // Admin: User Management
        {move || if is_admin() { view!{<div class="dashboard-panel p-6 mt-6">
            <h3 class="text-base font-semibold text-gray-900 mb-4">"User Management"</h3>
            <div class="grid grid-cols-3 gap-4 mb-6">
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Username"</label><input type="text" class="w-full border border-[#E5E5E5] rounded px-3 py-2 text-sm font-sans text-[#0A0A0A] bg-white outline-none" placeholder="Username" prop:value=move || new_user.get() on:input=move |e| set_new_user.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Password"</label><input type="password" class="w-full border border-[#E5E5E5] rounded px-3 py-2 text-sm font-sans text-[#0A0A0A] bg-white outline-none" placeholder="Password" prop:value=move || new_upass.get() on:input=move |e| set_new_upass.set(event_target_value(&e))/></div>
                <div><label class="block text-xs font-medium text-gray-500 uppercase tracking-wide mb-1">"Role"</label>
                    <div class="w-full">
                        <CustomDropdown items=role_items placeholder="Employee".to_string() on_select=Callback::new(move |v: String| set_new_role.set(v))/>
                    </div></div>
            </div>
            <button class="btn-primary px-6 py-2" on:click=add_user>"Add User"</button>
            <div class="mt-6"><table class="w-full data-table">
                <thead><tr><th>"Username"</th><th>"Role"</th><th>"Actions"</th></tr></thead>
                <tbody>
                    {move || {
                        let items = users.get();
                        items.into_iter().map(|u| {
                            let uname = u.username.clone();
                            let is_self = uname == cur_user();
                            view!{<tr class="border-b border-gray-50">
                                <td class="px-4 py-2 font-medium">{u.username.clone()}{if is_self {view!{<span class="text-xs text-gray-400 ml-2">"(you)"</span>}.into_any()} else {().into_any()}}</td>
                                <td class="px-4 py-2 text-gray-600 capitalize">{u.role.clone()}</td>
                                <td class="px-4 py-2 text-right">
                                    {if u.username != "admin" && !is_self {
                                        let un = uname.clone();
                                        let del = delete_user;
                                        view!{<button on:click=move |_| del(un.clone()) class="text-red-600 hover:underline text-xs">"Remove"</button>}.into_any()
                                    } else { ().into_any() }}
                                </td>
                            </tr>}
                        }).collect::<Vec<_>>()
                    }}
                </tbody>
            </table></div>
        </div>}.into_any() } else { ().into_any() }}
    </div>}
}
