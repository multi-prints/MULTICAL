use leptos::prelude::*;

/// Full-page centered spinner shown while a page's initial data loads.
#[component]
pub fn PageLoading(
    /// Optional status text under the spinner.
    #[prop(optional, into)]
    message: Option<String>,
) -> impl IntoView {
    let msg = message.unwrap_or_else(|| "Loading...".to_string());
    view! {
        <div class="page-loading" role="status" aria-live="polite" aria-busy="true">
            <div class="page-loading-spinner" aria-hidden="true"></div>
            <p>{msg}</p>
        </div>
    }
}
