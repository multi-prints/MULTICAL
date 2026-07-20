use leptos::prelude::*;

/// Shared confirm dialog with built-in busy/idempotent confirm handling.
/// While `pending` is true: confirm is disabled, cancel/close/overlay are locked,
/// and the confirm label switches to a busy string.
#[component]
pub fn ConfirmModal(
    show: ReadSignal<bool>,
    title: String,
    message: String,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
    #[prop(optional, into)] pending: Option<Signal<bool>>,
    #[prop(optional)] confirm_label: Option<&'static str>,
    #[prop(optional)] pending_label: Option<&'static str>,
) -> impl IntoView {
    let pending = pending.unwrap_or_else(|| Signal::derive(|| false));
    let confirm_label = confirm_label.unwrap_or("Confirm");
    let pending_label = pending_label.unwrap_or("Working…");

    view! {
        <Show when=move || show.get()>
            <div class="modal-overlay open" on:click=move |e| {
                if e.target() == e.current_target() && !pending.get() {
                    on_cancel.call(());
                }
            }>
                <div class="modal-container modal-sm">
                    <div class="modal-header">
                        <h3 class="modal-title">{title.clone()}</h3>
                        <button
                            type="button"
                            class="modal-close-btn"
                            prop:disabled=move || pending.get()
                            on:click=move |_| {
                                if !pending.get() {
                                    on_cancel.call(());
                                }
                            }
                        >
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                            </svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <p class="modal-msg">{message.clone()}</p>
                    </div>
                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn-secondary"
                            prop:disabled=move || pending.get()
                            on:click=move |_| {
                                if !pending.get() {
                                    on_cancel.call(());
                                }
                            }
                        >
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            class="btn-danger"
                            prop:disabled=move || pending.get()
                            on:click=move |_| {
                                if !pending.get() {
                                    on_confirm.call(());
                                }
                            }
                        >
                            {move || if pending.get() { pending_label } else { confirm_label }}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
