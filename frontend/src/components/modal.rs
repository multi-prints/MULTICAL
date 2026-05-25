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
            <div class="fixed inset-0 z-50 flex items-center justify-center">
                <div class="absolute inset-0 bg-black/40" on:click=move |_| on_cancel.call(())></div>
                <div class="relative bg-white rounded-xl shadow-xl p-6 w-full max-w-md mx-4">
                    <h3 class="text-lg font-semibold mb-2">{title.clone()}</h3>
                    <p class="text-gray-600 text-sm mb-6">{message.clone()}</p>
                    <div class="flex justify-end gap-3">
                        <button
                            on:click=move |_| on_cancel.call(())
                            class="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg"
                        >
                            "Cancel"
                        </button>
                        <button
                            on:click=move |_| on_confirm.call(())
                            class="px-4 py-2 text-sm bg-red-600 text-white hover:bg-red-700 rounded-lg"
                        >
                            "Confirm"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
