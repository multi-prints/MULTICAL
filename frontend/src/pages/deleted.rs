//! Admin audit log of deleted sales and printing jobs.

use crate::api::{self, DeletedRecord, DeletedRecordsQuery};
use crate::auto_refresh::{use_auto_refresh, LIVE_REFRESH_MS};
use leptos::prelude::*;

#[path = "../components/loading.rs"]
mod loading_comp;
use loading_comp::PageLoading;

#[derive(Clone, Copy, PartialEq)]
enum KindFilter {
    All,
    Sale,
    Printing,
}

fn format_ts(ts: &Option<String>) -> String {
    ts.as_ref()
        .and_then(|t| {
            chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.3f")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S"))
                .or_else(|_| chrono::DateTime::parse_from_rfc3339(t).map(|d| d.naive_local()))
                .ok()
        })
        .map(|dt| dt.format("%d/%m/%Y %I:%M %p").to_string())
        .or_else(|| ts.clone())
        .unwrap_or_else(|| "—".into())
}

fn kind_label(kind: &str) -> String {
    match kind {
        "sale" => "Sale".into(),
        "printing" => "Printing".into(),
        other => other.to_string(),
    }
}

#[component]
pub fn DeletedRecordsPage() -> impl IntoView {
    let (loading, set_loading) = signal(true);
    let (items, set_items) = signal(Vec::<DeletedRecord>::new());
    let (total, set_total) = signal(0i64);
    let (search, set_search) = signal(String::new());
    let (kind, set_kind) = signal(KindFilter::All);
    let (page, set_page) = signal(1u32);
    let (error, set_error) = signal(None::<String>);
    let per_page = 40u32;

    let load = {
        move || {
            let q_search = search.get();
            let k = kind.get();
            let p = page.get();
            set_loading.set(true);
            set_error.set(None);
            leptos::task::spawn_local(async move {
                let source_kind = match k {
                    KindFilter::All => None,
                    KindFilter::Sale => Some("sale".into()),
                    KindFilter::Printing => Some("printing".into()),
                };
                let query = DeletedRecordsQuery {
                    source_kind,
                    search: if q_search.trim().is_empty() {
                        None
                    } else {
                        Some(q_search.trim().to_string())
                    },
                    page: Some(p),
                    per_page: Some(per_page),
                };
                match api::get_deleted_records(&query).await {
                    Ok(data) => {
                        set_items.set(data.items);
                        set_total.set(data.total_count);
                    }
                    Err(e) => set_error.set(Some(e)),
                }
                set_loading.set(false);
            });
        }
    };

    // Initial + filter reload
    Effect::new(move |_| {
        let _ = search.get();
        let _ = kind.get();
        let _ = page.get();
        load();
    });

    use_auto_refresh(LIVE_REFRESH_MS, load);

    let total_pages = move || {
        let t = total.get().max(0) as u32;
        t.div_ceil(per_page).max(1)
    };

    view! {
        <div id="page-deleted" class="dash">
            <div class="dash-table-head">
                <div>
                    <h2 class="dash-section-title">"Deleted records"</h2>
                    <p class="prod-sub">
                        "Audit log of sales and printing jobs removed by staff. Use this when following up on mistakes."
                    </p>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="settings-alert is-err">{e}</div>
            })}

            <div class="dash-toolbar deleted-toolbar">
                <div class="dash-period" role="group" aria-label="Record type">
                    <button
                        type="button"
                        class=move || if kind.get() == KindFilter::All { "dash-period-btn is-active" } else { "dash-period-btn" }
                        on:click=move |_| { set_page.set(1); set_kind.set(KindFilter::All); }
                    >"All"</button>
                    <button
                        type="button"
                        class=move || if kind.get() == KindFilter::Sale { "dash-period-btn is-active" } else { "dash-period-btn" }
                        on:click=move |_| { set_page.set(1); set_kind.set(KindFilter::Sale); }
                    >"Sales"</button>
                    <button
                        type="button"
                        class=move || if kind.get() == KindFilter::Printing { "dash-period-btn is-active" } else { "dash-period-btn" }
                        on:click=move |_| { set_page.set(1); set_kind.set(KindFilter::Printing); }
                    >"Printing"</button>
                </div>
                <input
                    type="search"
                    class="settings-input deleted-search"
                    placeholder="Search summary, customer, staff…"
                    prop:value=move || search.get()
                    on:input=move |ev| {
                        set_page.set(1);
                        set_search.set(event_target_value(&ev));
                    }
                />
            </div>

            <Show when=move || !loading.get() fallback=|| view! { <PageLoading message="Loading deleted records…"/> }>
                <div class="dash-table-card">
                    <table class="dash-table">
                        <thead>
                            <tr>
                                <th>"Deleted"</th>
                                <th>"Type"</th>
                                <th>"Item"</th>
                                <th>"Amount"</th>
                                <th>"Customer"</th>
                                <th>"Recorded by"</th>
                                <th>"Deleted by"</th>
                                <th>"Original date"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                let rows = items.get();
                                if rows.is_empty() {
                                    return view! {
                                        <tr>
                                            <td colspan="8" class="dash-table-empty">
                                                "No deleted records yet."
                                            </td>
                                        </tr>
                                    }.into_any();
                                }
                                rows.into_iter().map(|r| {
                                    let del_at = format_ts(&r.deleted_at);
                                    let orig = format_ts(&r.original_timestamp);
                                    let kind = kind_label(&r.source_kind);
                                    let summary = r.summary.clone();
                                    let amount = format!("KSh {:.0}", r.amount);
                                    let cust = r.customer_name.clone().unwrap_or_else(|| "—".into());
                                    let created = r.created_by.clone().unwrap_or_else(|| "—".into());
                                    let deleted_by = r.deleted_by.clone();
                                    view! {
                                        <tr class="sales-row">
                                            <td class="dash-td-muted tnum">{del_at}</td>
                                            <td>
                                                <span class=move || {
                                                    if kind == "Sale" {
                                                        "deleted-kind is-sale"
                                                    } else {
                                                        "deleted-kind is-printing"
                                                    }
                                                }>{kind.clone()}</span>
                                            </td>
                                            <td class="dash-td-strong">{summary}</td>
                                            <td class="dash-td-strong tnum">{amount}</td>
                                            <td class="dash-td-muted">{cust}</td>
                                            <td class="dash-td-muted">{created}</td>
                                            <td class="dash-td-muted">{deleted_by}</td>
                                            <td class="dash-td-muted tnum">{orig}</td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }}
                        </tbody>
                    </table>
                </div>

                {move || {
                    let n = total.get();
                    if n == 0 {
                        return ().into_any();
                    }
                    let cp = page.get();
                    let tp = total_pages();
                    let si = (cp - 1) * per_page + 1;
                    let ei = ((cp * per_page) as i64).min(n);
                    view! {
                        <div class="dash-pagination">
                            <span class="prod-sub">{format!("Showing {}–{} of {}", si, ei, n)}</span>
                            <div class="dash-pagination-btns">
                                <button
                                    type="button"
                                    class="btn-secondary"
                                    prop:disabled=cp <= 1
                                    on:click=move |_| set_page.update(|p| *p = p.saturating_sub(1).max(1))
                                >"Previous"</button>
                                <span class="prod-sub">{format!("Page {} of {}", cp, tp)}</span>
                                <button
                                    type="button"
                                    class="btn-secondary"
                                    prop:disabled=cp >= tp
                                    on:click=move |_| set_page.update(|p| *p = (*p + 1).min(tp))
                                >"Next"</button>
                            </div>
                        </div>
                    }.into_any()
                }}
            </Show>
        </div>
    }
}
