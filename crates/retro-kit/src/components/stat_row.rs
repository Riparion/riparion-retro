//! One label/value line of a reckoning panel (game-over stats and the like).

use dioxus::prelude::*;

#[component]
pub fn StatRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex justify-between gap-3",
            span { class: "chip-label", "{label}" }
            span { class: "text-right", "{value}" }
        }
    }
}
