//! Name your party and rate your shooting before you outfit the train.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::theme::{BTN, BTN_PRIMARY, BTN_WIDE, SCREEN_CENTERED};

const RATINGS: [(u8, &str); 5] = [
    (1, "Dead-eye"),
    (2, "Good shot"),
    (3, "Fair to middlin'"),
    (4, "Need more practice"),
    (5, "Shaky hands"),
];

#[component]
pub fn NewGame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut party = use_signal(String::new);
    let mut marksman = use_signal(|| 0u8);

    let name_ok = use_memo(move || {
        let p = party.read();
        !p.trim().is_empty() && p.trim().len() <= 22
    });
    let ready = use_memo(move || name_ok() && marksman() != 0);

    let head_out = move |_| {
        let name = party.read().trim().to_string();
        if !name.is_empty() && name.len() <= 22 && marksman() != 0 {
            game.write().begin(name, marksman());
        }
    };

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-4",
            h2 { class: "text-center text-lg tracking-widest", "OUTFITTING AT FORT PATRICK HENRY" }
            p { class: "text-center leading-snug",
                "What is your party's name? (Up to 22 characters.)"
            }
            input {
                class: "crt-input text-center text-lg",
                r#type: "text",
                maxlength: "22",
                placeholder: "Party name",
                value: "{party}",
                autofocus: true,
                oninput: move |e| party.set(e.value()),
            }
            p { class: "text-center mt-2", "How good a shot are you with a rifle?" }
            div { class: "flex flex-col gap-2",
                for (n, label) in RATINGS {
                    button {
                        key: "{n}",
                        class: if marksman() == n { "{BTN_WIDE} crt-btn-primary py-3" } else { "{BTN_WIDE} py-3" },
                        onclick: move |_| marksman.set(n),
                        "{n}) {label}"
                    }
                }
            }
            button {
                class: "{BTN_PRIMARY} py-4 text-lg mt-2",
                disabled: !ready(),
                onclick: head_out,
                "OUTFIT THE TRAIN ▸"
            }
            button {
                class: "{BTN}",
                onclick: move |_| game.write().mode = crate::engine::state::Mode::Splash,
                "Back"
            }
        }
    }
}
