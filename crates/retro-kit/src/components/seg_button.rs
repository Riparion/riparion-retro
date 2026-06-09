//! One option in a segmented selector: a full-width button that reads as the
//! emphasized primary when `active`, and as a plain bordered button otherwise.
//! Lay several out in a `flex` or `grid` row and wire each one's `active` to the
//! same piece of state to build a toggle (buy / sell) or a tab strip.

use dioxus::prelude::*;

use crate::theme::{BTN_PRIMARY, BTN_WIDE};

#[component]
pub fn SegButton(label: String, active: bool, onclick: EventHandler<()>) -> Element {
    // Both bases are `w-full`; the trailing `flex-1` makes options share a flex
    // row equally and is harmlessly ignored inside a grid cell.
    let base = if active { BTN_PRIMARY } else { BTN_WIDE };
    rsx! {
        button {
            class: "{base} flex-1",
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}
