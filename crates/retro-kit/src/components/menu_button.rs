//! A full-width menu action button: a title line with an optional dimmed hint
//! line beneath it — the two-line "what + why" button the games' choice screens
//! share (press on / go hunting / eat well / circle the wagons …). Pass
//! `primary` for the emphasized default action; omit `hint` for a single line.

use dioxus::prelude::*;

use crate::theme::{BTN_PRIMARY, BTN_WIDE};

#[component]
pub fn MenuButton(
    title: String,
    /// Secondary explanatory line (empty hides it).
    #[props(default)] hint: String,
    /// Render as the emphasized primary action.
    #[props(default)] primary: bool,
    #[props(default)] disabled: bool,
    onclick: EventHandler<()>,
) -> Element {
    let base = if primary { BTN_PRIMARY } else { BTN_WIDE };
    rsx! {
        button {
            class: "{base} py-4",
            disabled,
            onclick: move |_| onclick.call(()),
            div { "{title}" }
            if !hint.is_empty() {
                div { class: "text-xs opacity-60", "{hint}" }
            }
        }
    }
}
