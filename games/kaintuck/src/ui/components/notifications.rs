//! The trader's name in the status bar, doubled as the notification-center
//! button. Market reports (wharf-factor lines, rumor payoffs, next-leg price
//! tips, trader gossip) are logged to `state.reports` rather than popped up; the
//! badge shows the unread count, and tapping drops a sheet down from the top with
//! the full log. Opening marks everything read, clearing the badge.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::sheet::Sheet;
use retro_kit::theme::PANEL;

#[component]
pub fn NotificationButton() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut open = use_signal(|| false);

    let g = game.read();
    let trader = g.state.trader.clone();
    let unread = g.state.reports.iter().filter(|r| !r.read).count();
    // Only materialize the list while the sheet is open — this component
    // re-renders on every game mutation, so cloning the log each time it's closed
    // is pure waste. Newest first: the most recent dock talk sits at the top.
    // Each report is one or more lines: an assertion, plus any later affirm/disprove
    // verdict appended onto it (joined with a newline). Split so each reads on its
    // own line without leaning on a `whitespace-pre-line` utility (not in our CSS).
    let reports: Vec<Vec<String>> = if open() {
        g.state
            .reports
            .iter()
            .rev()
            .map(|r| r.text.lines().map(str::to_string).collect())
            .collect()
    } else {
        Vec::new()
    };
    drop(g);

    rsx! {
        button {
            class: "notif-name-btn font-bold tracking-widest truncate",
            onclick: move |_| {
                open.set(true);
                // Reading the log clears the unread badge — but only dirty the
                // signal when something is actually unread, to avoid a needless
                // mutation + persistence re-check on every open.
                let has_unread = game.read().state.reports.iter().any(|r| !r.read);
                if has_unread {
                    let mut w = game.write();
                    for r in w.state.reports.iter_mut() {
                        r.read = true;
                    }
                }
            },
            span { class: "truncate", "{trader}" }
            if unread > 0 {
                span { class: "notif-badge", "{unread}" }
            }
        }
        Sheet {
            open: open(),
            from_top: true,
            title: "MARKET REPORTS".to_string(),
            on_close: move |_| open.set(false),
            if reports.is_empty() {
                div { class: "{PANEL} p-3 text-center text-sm opacity-70",
                    "No market reports yet."
                }
            } else {
                div { class: "flex flex-col gap-2",
                    for entry in reports {
                        div { class: "{PANEL} p-2 text-sm leading-snug flex flex-col gap-1",
                            for line in entry {
                                div { "{line}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
