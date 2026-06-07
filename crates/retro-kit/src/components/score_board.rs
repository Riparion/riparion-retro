//! The local hall-of-fame panel on every game's title screen: a bordered
//! list of formatted (entry, score) rows under a `── TITLE ──` heading.
//! Renders nothing while the board is empty.

use dioxus::prelude::*;

use crate::theme::PANEL;

#[component]
pub fn ScoreBoard(title: String, rows: Vec<(String, String)>) -> Element {
    if rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "{PANEL} p-3 w-full max-w-xs text-left",
            h2 { class: "chip-label mb-2 text-center", "── {title} ──" }
            for (i, (left, right)) in rows.iter().enumerate() {
                div { key: "{i}", class: "flex justify-between text-sm gap-2",
                    span { class: "truncate", "{left}" }
                    span { class: "opacity-80 shrink-0", "{right}" }
                }
            }
        }
    }
}
