//! Name your Duke before the first year.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::ui::Overlay;
use retro_kit::theme::{BTN, BTN_PRIMARY, SCREEN_CENTERED};

#[component]
pub fn NewGame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut overlay = use_context::<Signal<Overlay>>();
    let mut name = use_signal(String::new);

    let take_throne = move |_| {
        let n = name.read().trim().to_string();
        let duke = if n.is_empty() || n.len() > 24 {
            "The Duke".to_string()
        } else {
            n
        };
        game.write().begin(duke);
    };

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-4",
            h2 { class: "text-center text-lg tracking-widest", "A NEW LINE BEGINS" }
            p { class: "text-center leading-snug",
                "By what name shall the realm remember its Duke?"
            }
            input {
                class: "crt-input text-center text-lg",
                r#type: "text",
                maxlength: "24",
                placeholder: "The Duke",
                value: "{name}",
                autofocus: true,
                oninput: move |e| name.set(e.value()),
            }
            button {
                class: "{BTN_PRIMARY} py-4 text-lg mt-2",
                onclick: take_throne,
                "BEGIN THE REIGN ▸"
            }
            button {
                class: "{BTN}",
                onclick: move |_| {
                    game.write().mode = Mode::Splash;
                    overlay.set(Overlay::Splash);
                },
                "Back"
            }
        }
    }
}
