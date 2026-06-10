//! The scrolling teletype transcript. Engine output is lightly modernized to
//! mixed case; the player's own commands echo verbatim as `> ...`. Auto-scrolls
//! to the newest line whenever the log grows.

use dioxus::prelude::*;

use crate::engine::{modernize, Game, Line};

#[component]
pub fn Transcript() -> Element {
    let game = use_context::<Signal<Game>>();

    // Subscribe to transcript growth and pin the view to the bottom.
    use_effect(move || {
        let _len = game.read().transcript.len();
        let _ = document::eval(
            "requestAnimationFrame(() => { const el = document.getElementById('transcript'); \
             if (el) el.scrollTop = el.scrollHeight; });",
        );
    });

    let g = game.read();
    rsx! {
        div { id: "transcript", class: "transcript flex-1 min-h-0 overflow-y-auto px-3 py-2",
            for (i, line) in g.transcript.iter().enumerate() {
                match line {
                    Line::Echo(s) => rsx! { p { key: "{i}", class: "t-echo", "> {s}" } },
                    Line::Out(s) => rsx! { p { key: "{i}", class: "t-out", "{modernize(s.trim_end())}" } },
                }
            }
        }
    }
}
