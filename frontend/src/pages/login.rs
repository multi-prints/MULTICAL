use crate::api::{self, LoginResponse, UserInfo};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;

/// Standalone login page (kept in sync with the active shell login in `app.rs`).
#[component]
pub fn LoginPage(
    set_user: WriteSignal<Option<UserInfo>>,
    set_token: WriteSignal<Option<String>>,
) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (show_pw, set_show_pw) = signal(false);

    let do_login = move || {
        if loading.get() {
            return;
        }
        let u = username.get().trim().to_string();
        let p = password.get();
        if u.is_empty() || p.is_empty() {
            set_error.set("Please enter username and password".into());
            return;
        }
        set_error.set(String::new());
        set_loading.set(true);
        leptos::task::spawn_local(async move {
            match api::login(&u, &p).await {
                Ok(LoginResponse {
                    success: true,
                    token: Some(tok),
                    user: Some(user_info),
                    ..
                }) => {
                    LocalStorage::set("sessionToken", &tok).ok();
                    LocalStorage::set(
                        "currentUser",
                        serde_json::to_string(&user_info).unwrap_or_default(),
                    )
                    .ok();
                    set_token.set(Some(tok));
                    set_user.set(Some(user_info));
                }
                Ok(LoginResponse {
                    error: Some(msg), ..
                }) => set_error.set(msg),
                Ok(_) => set_error.set("Invalid username or password".into()),
                Err(e) => set_error.set(e),
            }
            set_loading.set(false);
        });
    };

    let on_submit = {
        let do_login = do_login;
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            do_login();
        }
    };

    view! {
        <div class="login-page">
            <div class="login-shell">
                <div class="login-card">
                    <div class="login-brand">
                        <div class="login-brand-mark" aria-hidden="true">
                            <svg viewBox="0 0 32 32" fill="none">
                                <rect x="2.5" y="2.5" width="27" height="27" rx="5" stroke="currentColor" stroke-width="1.75"/>
                                <path d="M9 11.5h14M9 16h11M9 20.5h7" stroke="currentColor" stroke-width="1.75" stroke-linecap="round"/>
                            </svg>
                        </div>
                        <h1 class="login-title">"MULTIPRINTS"</h1>
                        <p class="login-subtitle">"Sign in to continue"</p>
                    </div>

                    <form class="login-form" on:submit=on_submit>
                        <Show when=move || !error.get().is_empty()>
                            <div class="login-error" role="alert">{move || error.get()}</div>
                        </Show>

                        <div class="login-field">
                            <label class="login-label" for="login-username">"Username"</label>
                            <input
                                id="login-username"
                                type="text"
                                class="login-input"
                                placeholder="Enter username"
                                autocomplete="username"
                                prop:value=move || username.get()
                                on:input=move |e| set_username.set(event_target_value(&e))
                            />
                        </div>

                        <div class="login-field">
                            <label class="login-label" for="login-password">"Password"</label>
                            <div class="login-password-wrap">
                                <input
                                    id="login-password"
                                    type=move || if show_pw.get() { "text" } else { "password" }
                                    class="login-input login-input--password"
                                    placeholder="Enter password"
                                    autocomplete="current-password"
                                    prop:value=move || password.get()
                                    on:input=move |e| set_password.set(event_target_value(&e))
                                />
                                <button
                                    type="button"
                                    class="login-pw-toggle"
                                    aria-label=move || if show_pw.get() { "Hide password" } else { "Show password" }
                                    on:click=move |_| set_show_pw.update(|v| *v = !*v)
                                >
                                    {move || if show_pw.get() {
                                        view! {
                                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                                    d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"/>
                                            </svg>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                                    d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                                                    d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                                            </svg>
                                        }.into_any()
                                    }}
                                </button>
                            </div>
                        </div>

                        <button
                            type="submit"
                            class="login-submit"
                            prop:disabled=move || loading.get()
                        >
                            {move || if loading.get() {
                                view! {
                                    <span class="login-submit-inner">
                                        <span class="login-spinner" aria-hidden="true"></span>
                                        "Signing in..."
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span>"Sign in"</span> }.into_any()
                            }}
                        </button>
                    </form>

                    <p class="login-footer">"© 2026 MULTIPRINTS"</p>
                </div>
            </div>
        </div>
    }
}
