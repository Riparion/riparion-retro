//! Name your trader before you reach the Pittsburgh boatyard.

use dioxus::prelude::*;

use crate::engine::ledger::Carryover;
use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN, BTN_PRIMARY, SCREEN_CENTERED};

#[component]
pub fn NewGame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let house = use_hook(storage::ledger);
    // Pre-fill the name the last Kaintuck went by, so a returning player names the
    // next one with one tap (still editable).
    let remembered = house.as_ref().map(|l| l.name.clone()).unwrap_or_default();
    let mut name = use_signal(|| remembered);

    let ready = use_memo(move || {
        let n = name.read();
        !n.trim().is_empty() && n.trim().len() <= 22
    });

    let set_out = move |_| {
        let n = name.read().trim().to_string();
        if !n.is_empty() && n.len() <= 22 {
            let carry = storage::ledger()
                .map(|l| l.carry())
                .unwrap_or_else(Carryover::fresh);
            game.write().begin_with(n, carry);
        }
    };

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-4",
            h2 { class: "text-center text-lg tracking-widest", "PITTSBURGH, c. 1805" }
            p { class: "text-center leading-snug",
                "The flatboat capital of the frontier. What name do you go by, Kaintuck? (Up to 22 characters.)"
            }
            if let Some(l) = house.as_ref() {
                p { class: "text-center text-sm opacity-80 leading-snug",
                    "You set out with ${group_thousands(l.cash as i64)} and a reputation of {l.reputation as i64} to your house's name."
                }
            }
            input {
                class: "crt-input text-center text-lg",
                r#type: "text",
                maxlength: "22",
                placeholder: "Your name",
                value: "{name}",
                autofocus: true,
                oninput: move |e| name.set(e.value()),
            }
            button {
                class: "{BTN_PRIMARY} py-4 text-lg mt-2",
                disabled: !ready(),
                onclick: set_out,
                "TO THE BOATYARD ▸"
            }
            button {
                class: "{BTN}",
                onclick: move |_| game.write().mode = Mode::Splash,
                "Back"
            }
        }
    }
}
