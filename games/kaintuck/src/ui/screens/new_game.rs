//! Name your trader before you reach the Pittsburgh boatyard.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use retro_kit::theme::{BTN, BTN_PRIMARY, SCREEN_CENTERED};

#[component]
pub fn NewGame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut name = use_signal(String::new);

    let ready = use_memo(move || {
        let n = name.read();
        !n.trim().is_empty() && n.trim().len() <= 22
    });

    let set_out = move |_| {
        let n = name.read().trim().to_string();
        if !n.is_empty() && n.len() <= 22 {
            game.write().begin(n);
        }
    };

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-4",
            h2 { class: "text-center text-lg tracking-widest", "PITTSBURGH, c. 1805" }
            p { class: "text-center leading-snug",
                "The flatboat capital of the frontier. What name do you go by, Kaintuck? (Up to 22 characters.)"
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
