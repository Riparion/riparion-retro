//! A one-button message panel: paced narration with a single acknowledge action.
//! The single-button companion to [`super::decision::Decision`], sharing the same
//! centered prompt-panel shell.

use dioxus::prelude::*;

use crate::theme::{BTN_PRIMARY, PANEL};

#[component]
pub fn Notice(
    message: String,
    /// Acknowledge-button label (defaults to the classic tap-through chevrons).
    #[props(default = "▸ ▸ ▸".to_string())]
    button: String,
    on_ack: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col justify-center p-4 max-w-md w-full mx-auto",
            div { class: "{PANEL} p-4 text-center text-lg leading-snug", "{message}" }
            button {
                class: "{BTN_PRIMARY} mt-4 py-3",
                onclick: move |_| on_ack.call(()),
                "{button}"
            }
        }
    }
}
