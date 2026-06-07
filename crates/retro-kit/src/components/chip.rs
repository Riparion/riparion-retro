//! One labeled value in a status-bar chip grid.

use dioxus::prelude::*;

#[component]
pub fn Chip(
    label: String,
    value: String,
    /// Highlight with the danger tint (low fuel, debt, damage...).
    #[props(default)] danger: bool,
) -> Element {
    let danger_class = if danger { "chip-danger" } else { "" };
    rsx! {
        div { class: "chip {danger_class}",
            div { class: "chip-label", "{label}" }
            div { "{value}" }
        }
    }
}
