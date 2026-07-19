use crate::api::{self, User, UserInfo};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;

#[path = "../components/dropdown.rs"]
mod dropdown_comp;
use dropdown_comp::{CustomDropdown, DropdownItem};
#[path = "../components/loading.rs"]
mod loading_comp;
use loading_comp::PageLoading;

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
    let (app_version, set_app_version) = signal("1.1.4".to_string());
    let (platform, set_platform) = signal("Tauri (Desktop)".to_string());
    let (update_status, set_update_status) = signal(None::<String>);
    let (checking_update, set_checking_update) = signal(false);
    let (show_uninstall, set_show_uninstall) = signal(false);
    let (uninstalling, set_uninstalling) = signal(false);
    let (loading, set_loading) = signal(true);

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
            set_loading.set(false);
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
        // Non-admin pages may never call load_users successfully for listing;
        // still clear spinner after version fetch so settings is usable.
        set_loading.set(false);
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

    view! {
        <Show when=move || !loading.get() fallback=|| view! {
            <div id="page-settings" class="dash settings-page">
                <PageLoading message="Loading settings..."/>
            </div>
        }>
        <div id="page-settings" class="dash settings-page">
            <div class="dash-table-head">
                <div>
                    <h2 class="dash-section-title">"Preferences"</h2>
                    <p class="prod-sub">"Account, data, and system details"</p>
                </div>
                <div class="dash-period settings-tabs" role="tablist" aria-label="Settings sections">
                    <button
                        type="button"
                        class=move || if active_tab.get() == SettingsTab::Account { "dash-period-btn is-active" } else { "dash-period-btn" }
                        on:click=move |_| set_active_tab.set(SettingsTab::Account)
                    >"Account"</button>
                    {move || if is_admin() {
                        view! {
                            <button
                                type="button"
                                class=move || if active_tab.get() == SettingsTab::Backup { "dash-period-btn is-active" } else { "dash-period-btn" }
                                on:click=move |_| set_active_tab.set(SettingsTab::Backup)
                            >"Backup & Data"</button>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                    <button
                        type="button"
                        class=move || if active_tab.get() == SettingsTab::About { "dash-period-btn is-active" } else { "dash-period-btn" }
                        on:click=move |_| set_active_tab.set(SettingsTab::About)
                    >"About"</button>
                </div>
            </div>

            {move || msg.get().map(|(ok, m)| {
                let cls = if ok { "settings-alert is-ok" } else { "settings-alert is-err" };
                view! { <div class=cls>{m}</div> }
            })}

            // Account
            {move || if active_tab.get() == SettingsTab::Account {
                view! {
                    <div class="settings-stack">
                        <div class="dash-card settings-card">
                            <h3 class="settings-card-title">"Change username"</h3>
                            <div class="settings-form">
                                <div class="settings-field">
                                    <label class="settings-label">"Current username"</label>
                                    <input type="text" class="settings-input is-readonly" readonly prop:value=cur_user/>
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"New username"</label>
                                    <input
                                        type="text"
                                        class="settings-input"
                                        placeholder="Enter new username"
                                        prop:value=move || new_username.get()
                                        on:input=move |e| set_new_username.set(event_target_value(&e))
                                    />
                                </div>
                                <div class="settings-actions">
                                    <button type="button" class="dash-btn-primary" on:click=change_username>"Update username"</button>
                                </div>
                            </div>
                        </div>

                        <div class="dash-card settings-card">
                            <h3 class="settings-card-title">"Change password"</h3>
                            <div class="settings-form">
                                <div class="settings-field">
                                    <label class="settings-label">"Current password"</label>
                                    <input
                                        type="password"
                                        class="settings-input"
                                        placeholder="Enter current password"
                                        prop:value=move || old_pw.get()
                                        on:input=move |e| set_old_pw.set(event_target_value(&e))
                                    />
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"New password"</label>
                                    <input
                                        type="password"
                                        class="settings-input"
                                        placeholder="Enter new password"
                                        prop:value=move || new_pw.get()
                                        on:input=move |e| set_new_pw.set(event_target_value(&e))
                                    />
                                </div>
                                <div class="settings-actions">
                                    <button type="button" class="dash-btn-primary" on:click=change_pw>"Update password"</button>
                                </div>
                            </div>
                        </div>

                        {move || if is_admin() {
                            view! {
                                <div class="dash-card settings-card settings-users">
                                    <h3 class="settings-card-title">"User management"</h3>
                                    <div class="settings-user-form">
                                        <div class="settings-field">
                                            <label class="settings-label">"Username"</label>
                                            <input
                                                type="text"
                                                class="settings-input"
                                                placeholder="Username"
                                                prop:value=move || new_user.get()
                                                on:input=move |e| set_new_user.set(event_target_value(&e))
                                            />
                                        </div>
                                        <div class="settings-field">
                                            <label class="settings-label">"Password"</label>
                                            <input
                                                type="password"
                                                class="settings-input"
                                                placeholder="Password"
                                                prop:value=move || new_upass.get()
                                                on:input=move |e| set_new_upass.set(event_target_value(&e))
                                            />
                                        </div>
                                        <div class="settings-field">
                                            <label class="settings-label">"Role"</label>
                                            <CustomDropdown
                                                items=role_items
                                                placeholder="Employee".to_string()
                                                on_select=Callback::new(move |v: String| set_new_role.set(v))
                                            />
                                        </div>
                                        <div class="settings-field settings-field--btn">
                                            <label class="settings-label" style="opacity:0">"Add"</label>
                                            <button type="button" class="dash-btn-primary" on:click=add_user>"Add user"</button>
                                        </div>
                                    </div>
                                    <div class="dash-table-card settings-users-table-wrap">
                                        <table class="dash-table settings-users-table">
                                            <thead>
                                                <tr>
                                                    <th>"Username"</th>
                                                    <th>"Role"</th>
                                                    <th>"Actions"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {move || {
                                                    users.get().into_iter().map(|u| {
                                                        let uname = u.username.clone();
                                                        let is_self = uname == cur_user();
                                                        let role = u.role.clone();
                                                        view! {
                                                            <tr class="sales-row">
                                                                <td class="dash-td-strong">
                                                                    {u.username.clone()}
                                                                    {if is_self {
                                                                        view! { <span class="prod-sub">" (you)"</span> }.into_any()
                                                                    } else {
                                                                        ().into_any()
                                                                    }}
                                                                </td>
                                                                <td class="dash-td-muted capitalize">{role}</td>
                                                                <td>
                                                                    {if u.username != "admin" && !is_self {
                                                                        let un = uname.clone();
                                                                        let del = delete_user;
                                                                        view! {
                                                                            <button
                                                                                type="button"
                                                                                class="prod-btn-icon is-danger"
                                                                                aria-label="Remove user"
                                                                                on:click=move |_| del(un.clone())
                                                                            >
                                                                                <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                                                                </svg>
                                                                            </button>
                                                                        }.into_any()
                                                                    } else {
                                                                        view! { <span class="prod-sub">"—"</span> }.into_any()
                                                                    }}
                                                                </td>
                                                            </tr>
                                                        }
                                                    }).collect::<Vec<_>>()
                                                }}
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            // Backup
            {move || if active_tab.get() == SettingsTab::Backup && is_admin() {
                view! {
                    <div class="settings-stack">
                        <div class="dash-card settings-card">
                            <h3 class="settings-card-title">"Database backup"</h3>
                            <p class="prod-sub settings-card-desc">"Export or import your business data"</p>
                            <div class="settings-action-list">
                                <button type="button" class="settings-action-row">
                                    <div class="settings-action-left">
                                        <span class="settings-action-icon is-blue" aria-hidden="true">
                                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg>
                                        </span>
                                        <span>
                                            <span class="settings-action-title">"Export database"</span>
                                            <span class="prod-sub">"Download all your data as JSON"</span>
                                        </span>
                                    </div>
                                    <svg class="settings-action-chev" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5l7 7-7 7"/></svg>
                                </button>
                                <button type="button" class="settings-action-row">
                                    <div class="settings-action-left">
                                        <span class="settings-action-icon is-green" aria-hidden="true">
                                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/></svg>
                                        </span>
                                        <span>
                                            <span class="settings-action-title">"Import database"</span>
                                            <span class="prod-sub">"Restore data from a backup file"</span>
                                        </span>
                                    </div>
                                    <svg class="settings-action-chev" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5l7 7-7 7"/></svg>
                                </button>
                            </div>
                        </div>

                        <div class="dash-card settings-card settings-danger">
                            <h3 class="settings-card-title is-danger">"Danger zone"</h3>
                            <p class="prod-sub settings-card-desc is-danger">"Irreversible actions — proceed with caution"</p>
                            <div class="settings-danger-actions">
                                <button
                                    type="button"
                                    class="settings-btn-danger"
                                    on:click=move |_| {
                                        leptos::task::spawn_local(async move {
                                            let _ = api::clear_all_data().await;
                                            set_msg.set(Some((true, "All data cleared".into())));
                                        });
                                    }
                                >"Clear all data"</button>
                                <button
                                    type="button"
                                    class="settings-btn-danger is-solid"
                                    on:click=move |_| set_show_uninstall.set(true)
                                >"Uninstall MULTIPRINTS"</button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            // Uninstall modal
            {move || if show_uninstall.get() {
                view! {
                    <div class="modal-overlay open">
                        <div class="modal-container" style="max-width:420px">
                            <div class="modal-header">
                                <h3 class="modal-title">"Uninstall MULTIPRINTS"</h3>
                                <button type="button" class="modal-close-btn" on:click=move |_| set_show_uninstall.set(false)>
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                                </button>
                            </div>
                            <div class="modal-body">
                                <p class="prod-sub">"This will remove the application and all its data from your system."</p>
                                <p class="settings-danger-note">"This action cannot be undone."</p>
                            </div>
                            <div class="modal-footer">
                                <button type="button" class="btn-secondary" on:click=move |_| set_show_uninstall.set(false)>"Cancel"</button>
                                <button
                                    type="button"
                                    class="settings-btn-danger is-solid"
                                    prop:disabled=move || uninstalling.get()
                                    on:click=do_uninstall
                                >{move || if uninstalling.get() { "Uninstalling…" } else { "Uninstall" }}</button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            // About
            {move || if active_tab.get() == SettingsTab::About {
                view! {
                    <div class="settings-stack">
                        <div class="dash-card settings-card settings-about">
                            <div class="settings-about-mark" aria-hidden="true">"M"</div>
                            <h3 class="settings-about-name">"MULTIPRINTS"</h3>
                            <p class="prod-sub">"Inventory & sales management"</p>
                            <div class="settings-version-pill">
                                <span class="dash-metric-label">"Version"</span>
                                <span class="dash-td-strong">{app_version.get()}</span>
                            </div>
                            <button
                                type="button"
                                class="dash-btn-primary"
                                prop:disabled=move || checking_update.get()
                                on:click=install_update
                            >{move || if checking_update.get() { "Updating…" } else { "Check & install update" }}</button>
                            {move || update_status.get().map(|text| view! {
                                <p class="prod-sub">{text}</p>
                            })}
                        </div>

                        <div class="dash-card settings-card">
                            <h3 class="settings-card-title">"System information"</h3>
                            <div class="settings-kv-list">
                                <div class="settings-kv"><span class="dash-metric-label">"Platform"</span><span class="dash-td-strong">{platform.get()}</span></div>
                                <div class="settings-kv"><span class="dash-metric-label">"Database"</span><span class="dash-td-strong">"SQLite"</span></div>
                                <div class="settings-kv"><span class="dash-metric-label">"Framework"</span><span class="dash-td-strong">"Rust + Leptos"</span></div>
                            </div>
                        </div>

                        <div class="dash-card settings-card">
                            <h3 class="settings-card-title">"Developer"</h3>
                            <div class="settings-kv-list">
                                <div class="settings-kv"><span class="dash-metric-label">"Author"</span><span class="dash-td-strong">"Godwin Mayodi"</span></div>
                                <div class="settings-kv"><span class="dash-metric-label">"Email"</span><span class="dash-td-strong">"codegoddy@gmail.com"</span></div>
                                <div class="settings-kv">
                                    <span class="dash-metric-label">"Repository"</span>
                                    <a href="https://github.com/multi-prints/MULTICAL" target="_blank" class="settings-link">"github.com/multi-prints/MULTICAL"</a>
                                </div>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

        </div>
        </Show>
    }
}
