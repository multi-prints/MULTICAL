#![allow(dead_code)]

use crate::api::Debt;
use chrono::Datelike;
use leptos::prelude::*;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn dim(month: i32, year: i32) -> u32 {
    if month == 1 {
        if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
            29
        } else {
            28
        }
    } else if [3, 5, 8, 10].contains(&month) {
        30
    } else {
        31
    }
}

#[component]
pub fn CalendarModal(
    show: Signal<bool>,
    set_show: WriteSignal<bool>,
    debts: Signal<Vec<Debt>>,
) -> impl IntoView {
    let (month, set_month) = signal(chrono::Local::now().month() as i32 - 1);
    let (year, set_year) = signal(chrono::Local::now().year());
    let (sel_date, set_sel_date) = signal(None::<(i32, u32)>);

    let prev = move |_| {
        if month.get() == 0 {
            set_month.set(11);
            set_year.update(|y| *y -= 1);
        } else {
            set_month.update(|m| *m -= 1);
        }
    };
    let next = move |_| {
        if month.get() == 11 {
            set_month.set(0);
            set_year.update(|y| *y += 1);
        } else {
            set_month.update(|m| *m += 1);
        }
    };
    let label = move || format!("{} {}", MONTHS[month.get() as usize], year.get());

    // Debts grouped by day
    let by_day = move || {
        let mut map: std::collections::BTreeMap<u32, Vec<Debt>> = std::collections::BTreeMap::new();
        for d in debts.get() {
            if let Some(ref dd) = d.due_date {
                if let Ok(p) = chrono::NaiveDate::parse_from_str(dd, "%Y-%m-%d") {
                    if p.month() as i32 - 1 == month.get() && p.year() == year.get() {
                        map.entry(p.day()).or_default().push(d);
                    }
                }
            }
        }
        map
    };

    let sel_debts = move || {
        let (sm, sd) = sel_date.get()?;
        if sm == month.get() {
            by_day().get(&sd).cloned()
        } else {
            None
        }
    };

    view! {
        {move || if show.get() { view!{<div class="modal-overlay open"><div class="modal-container" style="max-width: 900px;">
            <div class="modal-header"><h3 class="modal-title">"Debts Calendar"</h3>
                <button class="modal-close-btn" on:click=move |_| set_show.set(false)>
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                </button>
            </div>
            <div class="modal-body">
                <div class="flex items-center justify-between mb-6">
                    <button on:click=prev class="p-2 hover:bg-gray-100 transition-colors"><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/></svg></button>
                    <h4 class="text-lg font-semibold text-gray-900">{label}</h4>
                    <button on:click=next class="p-2 hover:bg-gray-100 transition-colors"><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/></svg></button>
                </div>
                <div class="calendar-grid">
                    {["Sun","Mon","Tue","Wed","Thu","Fri","Sat"].into_iter().map(|h| view!{<div class="calendar-day-header">{h}</div>}).collect::<Vec<_>>()}
                    {move || {
                        let m = month.get(); let y = year.get();
                        let first = chrono::NaiveDate::from_ymd_opt(y, (m+1) as u32, 1).unwrap();
                        let so = first.weekday().num_days_from_sunday() as usize;
                        let today = chrono::Local::now().date_naive();
                        let bd = by_day();
                        let sel = sel_date.get();
                        let mut out: Vec<leptos::prelude::AnyView> = Vec::new();
                        for _ in 0..so { out.push(view!{<div class="calendar-day other-month"></div>}.into_any()); }
                        for d in 1..=dim(m, y) {
                            let dd = chrono::NaiveDate::from_ymd_opt(y, (m+1) as u32, d).unwrap();
                            let is_t = dd == today;
                            let is_s = sel.is_some_and(|(sm,sd)| sm==m && sd==d);
                            let day_debts = bd.get(&d).cloned().unwrap_or_default();
                            let all_p = !day_debts.is_empty() && day_debts.iter().all(|dd| dd.status=="paid");
                            let days_u = (dd - today).num_days();
                            let sc = if day_debts.is_empty() { "" }
                                else if all_p { "paid" } else if days_u < 0 { "overdue" }
                                else if days_u <= 3 { "due-soon" } else { "upcoming" };
                            let cls = format!("calendar-day{}{}{}",
                                if is_t { " today" } else { "" },
                                if is_s { " selected" } else { "" },
                                if !day_debts.is_empty() { format!(" has-debts {}-day", sc) } else { String::new() }
                            );
                            let count_text = if day_debts.is_empty() { String::new() }
                                else if all_p { format!("{} paid", day_debts.len()) }
                                else { let pc=day_debts.iter().filter(|dd|dd.status=="paid").count(); let pn=day_debts.len()-pc;
                                    if pc>0 { format!("{} pending, {} paid", pn, pc) } else { format!("{} debt{}", pn, if pn>1{"s"}else{""}) }
                                };
                            out.push(view!{<div class=cls>
                                <div class="calendar-day-number">{d}</div>
                                {if !day_debts.is_empty() { view!{<>
                                    <div class=format!("calendar-debt-indicator {}", sc)></div>
                                    <div class="calendar-debt-count">{count_text}</div>
                                </>}.into_any() } else { ().into_any() }}
                                <div class="absolute inset-0 cursor-pointer" on:click=move |_| set_sel_date.set(Some((m, d)))></div>
                            </div>}.into_any());
                        }
                        out.into_any()
                    }}
                </div>
                <div class="calendar-legend mt-4">
                    <div class="calendar-legend-item"><div class="calendar-legend-dot overdue"></div><span>"Overdue"</span></div>
                    <div class="calendar-legend-item"><div class="calendar-legend-dot due-soon"></div><span>"Due Soon"</span></div>
                    <div class="calendar-legend-item"><div class="calendar-legend-dot upcoming"></div><span>"Upcoming"</span></div>
                    <div class="calendar-legend-item"><div class="calendar-legend-dot paid"></div><span>"Paid"</span></div>
                </div>
                {move || sel_debts().map(|debts| {
                    let (sm, sd) = sel_date.get().unwrap();
                    let title = format!("Debts for {} {}, {}", MONTHS[sm as usize], sd, year.get());
                    view!{<div class="mt-6"><h5 class="font-medium text-gray-900 mb-3">{title}</h5><div class="space-y-2">
                        {if debts.is_empty() { view!{<p class="text-sm text-gray-500 italic">"No debts due on this day"</p>}.into_any() }
                        else { debts.into_iter().map(|debt| {
                            let paid = debt.status=="paid";
                            let dd = debt.due_date.clone().unwrap_or_default();
                            let diff = if dd.is_empty() { 0 } else {
                                chrono::NaiveDate::parse_from_str(&dd,"%Y-%m-%d").map(|p| (p-chrono::Local::now().date_naive()).num_days()).unwrap_or(0)
                            };
                            let (border, badge_cls, badge_text) = if paid {
                                ("border-green-200 bg-green-50 opacity-75", "bg-green-100 text-green-700", "Paid".to_string())
                            } else if diff < 0 {
                                ("border-red-200 bg-red-50", "bg-red-100 text-red-700", format!("Overdue by {} day{}", diff.abs(), if diff.abs()>1{"s"}else{""}))
                            } else if diff <= 3 {
                                ("border-amber-200 bg-amber-50", "bg-amber-100 text-amber-700",
                                    if diff==0 {"Due Today".into()} else if diff==1 {"Due Tomorrow".into()} else {format!("Due in {} days", diff)})
                            } else {
                                ("border-neutral-200 bg-neutral-50", "bg-neutral-100 text-neutral-800", format!("Due in {} days", diff))
                            };
                            view!{<div class=format!("p-4 rounded-lg border {}", border)>
                                <div class="flex items-start justify-between"><div class="flex-1">
                                    <div class="flex items-center gap-2"><h6 class="font-medium text-gray-900">{debt.customer_name.clone()}</h6>
                                        <span class=format!("px-2 py-0.5 rounded-full text-xs font-medium {}", badge_cls)>{badge_text}</span></div>
                                    <p class=format!("text-lg font-semibold text-gray-900 mt-1{}", if paid{" line-through"}else{""})>{format!("KSh {:.2}", debt.remaining_amount)}</p>
                                    {debt.description.as_ref().map(|desc| view!{<p class="text-xs text-gray-500 mt-1">{desc.clone()}</p>})}
                                </div></div>
                            </div>}
                        }).collect::<Vec<_>>().into_any() }}
                    </div></div>}
                })}
            </div>
        </div></div>}.into_any()}else{().into_any()}}
    }
}

/// Compact inline date picker — opens below a trigger element.
/// `date_r` / `date_w` are the YYYY-MM-DD value, `label` is a display string (e.g. "15 May 2026").
#[component]
pub fn MiniCalendar(
    date_r: ReadSignal<String>,
    date_w: WriteSignal<String>,
    #[allow(unused)] label: WriteSignal<String>,
) -> impl IntoView {
    let (open, set_open) = signal(false);
    let (month, set_month) = signal(chrono::Local::now().month() as i32 - 1);
    let (year, set_year) = signal(chrono::Local::now().year());

    let prev = move |_| {
        if month.get() == 0 {
            set_month.set(11);
            set_year.update(|y| *y -= 1);
        } else {
            set_month.update(|m| *m -= 1);
        }
    };
    let next = move |_| {
        if month.get() == 11 {
            set_month.set(0);
            set_year.update(|y| *y += 1);
        } else {
            set_month.update(|m| *m += 1);
        }
    };
    let month_label = move || format!("{} {}", MONTHS[month.get() as usize], year.get());

    let select_date = move |d: u32| {
        let m = month.get() + 1;
        let y = year.get();
        let val = format!("{:04}-{:02}-{:02}", y, m, d);
        date_w.set(val.clone());
        label.set(format!("{} {} {}", d, MONTHS[(m - 1) as usize], y));
        set_open.set(false);
    };

    view! {
        <div class="inline-block">
            <button type="button"
                class="px-3 py-2 border border-gray-200 rounded-lg text-sm text-gray-700 bg-white hover:border-gray-300 transition-colors flex items-center gap-2"
                on:click=move |_| set_open.set(true)
            >
                <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                </svg>
                <span>{move || {
                    let d = date_r.get();
                    if d.is_empty() { "Pick date".to_string() }
                    else { format_date_display(&d) }
                }}</span>
            </button>
        </div>
        {move || if open.get() {
            view!{<div class="modal-overlay open" on:click=move |_| set_open.set(false)>
                <div class="modal-container" style="max-width:340px" on:click=move |e| e.stop_propagation()>
                    <div class="modal-header">
                        <h3 class="modal-title">"Pick a Date"</h3>
                        <button class="modal-close-btn" on:click=move |_| set_open.set(false)>
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                        </button>
                    </div>
                    <div class="modal-body">
                        <div class="flex items-center justify-between mb-4">
                            <button on:click=prev class="p-1.5 hover:bg-gray-100 rounded transition-colors">
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/></svg>
                            </button>
                            <span class="text-sm font-semibold text-gray-900">{month_label}</span>
                            <button on:click=next class="p-1.5 hover:bg-gray-100 rounded transition-colors">
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/></svg>
                            </button>
                        </div>
                        <div class="grid grid-cols-7 gap-1 text-center mb-1">
                            {["Su","Mo","Tu","We","Th","Fr","Sa"].into_iter().map(|h| view!{<div class="text-[10px] font-medium text-gray-400 py-1">{h}</div>}).collect::<Vec<_>>()}
                        </div>
                        <div class="grid grid-cols-7 gap-1 text-center">
                            {move || {
                                let m = month.get(); let y = year.get();
                                let first = chrono::NaiveDate::from_ymd_opt(y, (m+1) as u32, 1).unwrap();
                                let so = first.weekday().num_days_from_sunday() as usize;
                                let today = chrono::Local::now().date_naive();
                                let sel_val = date_r.get();
                                let mut cells: Vec<leptos::prelude::AnyView> = Vec::new();
                                for _ in 0..so { cells.push(view!{<div></div>}.into_any()); }
                                for d in 1..=dim(m, y) {
                                    let dd = chrono::NaiveDate::from_ymd_opt(y, (m+1) as u32, d).unwrap();
                                    let is_today = dd == today;
                                    let is_sel = sel_val == format!("{:04}-{:02}-{:02}", y, m+1, d);
                                    let cls = if is_sel { "bg-black text-white rounded-full font-medium" }
                                        else if is_today { "border border-black rounded-full font-medium" }
                                        else { "hover:bg-gray-100 rounded-full" };
                                    let d2 = d;
                                    cells.push(view!{<div class=format!("text-sm py-2 cursor-pointer transition-colors {}", cls)
                                        on:click=move |_| select_date(d2)
                                    >{d2}</div>}.into_any());
                                }
                                cells.into_any()
                            }}
                        </div>
                    </div>
                </div>
            </div>}.into_any()
        } else { ().into_any() }}
    }
}

fn format_date_display(date_str: &str) -> String {
    if let Ok(d) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        format!(
            "{} {} {}",
            d.day(),
            MONTHS[(d.month() - 1) as usize],
            d.year()
        )
    } else {
        date_str.to_string()
    }
}
