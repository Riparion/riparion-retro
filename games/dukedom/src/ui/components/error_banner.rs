//! The inline validation-error line shared by the input screens (feed, land,
//! plant). Renders nothing when there's no error.

use dioxus::prelude::*;

use retro_kit::theme::PANEL;

#[component]
pub fn ErrorBanner(message: Option<String>) -> Element {
    rsx! {
        if let Some(msg) = message {
            p { class: "{PANEL} chip-danger mx-3 p-3 text-sm text-center", "{msg}" }
        }
    }
}
