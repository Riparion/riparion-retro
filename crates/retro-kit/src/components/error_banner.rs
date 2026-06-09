//! The inline validation-error line shared by the games' input screens. Renders
//! nothing when there's no error; a danger-tinted panel with the message when
//! there is. Pair it with [`super::number_entry::NumberEntry`] (or any input)
//! and feed it the engine's last rejection reason.

use dioxus::prelude::*;

use crate::theme::PANEL;

#[component]
pub fn ErrorBanner(message: Option<String>) -> Element {
    rsx! {
        if let Some(msg) = message {
            p { class: "{PANEL} chip-danger mx-3 p-3 text-sm text-center", "{msg}" }
        }
    }
}
