use leptos::prelude::*;

#[component]
pub fn Toast(
    message: ReadSignal<Option<(String, String)>>,
    set_message: WriteSignal<Option<(String, String)>>,
) -> impl IntoView {
    let visible = move || message.get().is_some();

    // Auto-hide after 3 seconds
    create_effect(move |_| {
        if let Some(_) = message.get() {
            let set = set_message;
            set_timeout(
                move || set.set(None),
                std::time::Duration::from_secs(3),
            );
        }
    });

    view! {
        <Show when=visible>
            <div class="fixed bottom-6 right-6 z-50 bg-gray-900 text-white px-5 py-3 rounded-lg shadow-lg max-w-sm animate-slide-up">
                <div class="font-medium text-sm">
                    {move || message.get().map(|(t, _)| t).unwrap_or_default()}
                </div>
                <div class="text-gray-300 text-xs mt-1">
                    {move || message.get().map(|(_, m)| m).unwrap_or_default()}
                </div>
            </div>
        </Show>
    }
}
