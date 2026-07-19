use leptos::prelude::*;

#[component]
pub fn ConfirmModal(
    show: ReadSignal<bool>,
    title: String,
    message: String,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || show.get()>
            <div class="modal-overlay open" on:click=move |e| {
                if e.target() == e.current_target() {
                    on_cancel.call(());
                }
            }>
                <div class="modal-container modal-sm">
                    <div class="modal-header">
                        <h3 class="modal-title">{title.clone()}</h3>
                        <button type="button" class="modal-close-btn" on:click=move |_| on_cancel.call(())>
                            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                            </svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <p class="modal-msg">{message.clone()}</p>
                    </div>
                    <div class="modal-footer">
                        <button type="button" class="btn-secondary" on:click=move |_| on_cancel.call(())>
                            "Cancel"
                        </button>
                        <button type="button" class="btn-danger" on:click=move |_| on_confirm.call(())>
                            "Confirm"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
