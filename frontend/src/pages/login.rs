use leptos::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use crate::api::{self, UserInfo, LoginResponse};

#[component]
pub fn LoginPage(
    set_user: WriteSignal<Option<UserInfo>>,
    set_token: WriteSignal<Option<String>>,
) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let handle_login = {
        move |_| {
            let u = username.get();
            let p = password.get();
            if u.is_empty() || p.is_empty() {
                set_error.set(Some("Please enter username and password".into()));
                return;
            }
            set_loading.set(true);
            set_error.set(None);
            leptos::task::spawn_local(async move {
                match api::login(&u, &p).await {
                    Ok(LoginResponse { success: true, token: Some(tok), user: Some(user_info), .. }) => {
                        LocalStorage::set("sessionToken", &tok).ok();
                        LocalStorage::set("currentUser", &serde_json::to_string(&user_info).unwrap_or_default()).ok();
                        set_token.set(Some(tok));
                        set_user.set(Some(user_info));
                    }
                    Ok(LoginResponse { error: Some(msg), .. }) => { set_error.set(Some(msg)); }
                    _ => { set_error.set(Some("Login failed. Please try again.".into())); }
                }
                set_loading.set(false);
            });
        }
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50">
            <div class="w-full max-w-sm">
                <div class="bg-white rounded-2xl shadow-sm border border-gray-100 p-8">
                    <div class="text-center mb-8">
                        <svg class="w-12 h-12 text-brand-600 mx-auto mb-3" viewBox="0 0 32 32" fill="none">
                            <rect x="2" y="2" width="28" height="28" rx="4" stroke="currentColor" stroke-width="1.5"/>
                            <path d="M9 11h14M9 16h10M9 21h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                        </svg>
                        <h1 class="text-xl font-bold">"MULTIPRINTS"</h1>
                        <p class="text-sm text-gray-500 mt-1">"Sign in to continue"</p>
                    </div>

                    {move || error.get().map(|msg| view! {
                        <div class="bg-red-50 text-red-700 text-sm px-4 py-2.5 rounded-lg mb-4">{msg}</div>
                    })}

                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Username"</label>
                            <input type="text" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent"
                                placeholder="Enter username"
                                on:input=move |e| set_username.set(event_target_value(&e)) />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Password"</label>
                            <input type="password" class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent"
                                placeholder="Enter password"
                                on:input=move |e| set_password.set(event_target_value(&e)) />
                        </div>
                        <button class="w-full py-2.5 bg-brand-600 text-white rounded-lg text-sm font-medium hover:bg-brand-700 disabled:opacity-50"
                            disabled=move || loading.get()
                            on:click=handle_login>
                            {move || if loading.get() { "Signing in..." } else { "Sign In" }}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
