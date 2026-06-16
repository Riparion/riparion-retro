//! One labeled value in a status-bar chip grid.

use dioxus::prelude::*;

#[component]
pub fn Chip(
    label: String,
    value: String,
    /// Highlight with the danger tint (low fuel, debt, damage...).
    #[props(default)] danger: bool,
    /// Make the chip a tappable shortcut. When set, the chip takes a pointer
    /// cursor and routes clicks here; left unset it's a plain read-out.
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    let danger_class = if danger { "chip-danger" } else { "" };
    // A shortcut chip gets a pointer cursor and a real click listener; a plain
    // read-out gets neither — no stray no-op handler on every status chip.
    match onclick {
        Some(handler) => rsx! {
            div {
                class: "chip {danger_class} chip-clickable",
                onclick: move |evt| handler.call(evt),
                div { class: "chip-label", "{label}" }
                div { "{value}" }
            }
        },
        None => rsx! {
            div { class: "chip {danger_class}",
                div { class: "chip-label", "{label}" }
                div { "{value}" }
            }
        },
    }
}
