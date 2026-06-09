//! A two-button decision panel: a centered prompt over a secondary (left) and a
//! primary (right) action. The shared shape behind every yes/no-style prompt —
//! games wire the two callbacks to their own response handling, and choose the
//! labels (Pay/Refuse, Strike/Hold, No/Yes, …).

use dioxus::prelude::*;

use crate::theme::{BTN, BTN_PRIMARY, PANEL};

#[component]
pub fn Decision(
    prompt: String,
    /// Left button label (rendered as the secondary action).
    left: String,
    on_left: EventHandler<()>,
    /// Right button label (rendered as the emphasized primary action).
    right: String,
    on_right: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col justify-center p-4 max-w-md w-full mx-auto",
            div { class: "{PANEL} p-4 text-center text-lg leading-snug", "{prompt}" }
            div { class: "flex gap-2 mt-4",
                button {
                    class: "{BTN} flex-1 py-3",
                    onclick: move |_| on_left.call(()),
                    "{left}"
                }
                button {
                    class: "{BTN_PRIMARY} flex-1 py-3",
                    onclick: move |_| on_right.call(()),
                    "{right}"
                }
            }
        }
    }
}
