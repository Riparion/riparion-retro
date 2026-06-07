//! Firm name and the classic opening choice: cash and a debt, or guns.

use dioxus::prelude::*;

use crate::engine::state::StartChoice;
use crate::engine::Game;
use retro_kit::theme::BTN_WIDE;

#[component]
pub fn NewGame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut firm = use_signal(String::new);
    let name_ok = use_memo(move || {
        let f = firm.read();
        !f.trim().is_empty() && f.trim().len() <= 22
    });

    let mut start = move |choice: StartChoice| {
        let name = firm.read().trim().to_string();
        if !name.is_empty() && name.len() <= 22 {
            game.write().start(name, choice);
        }
    };

    rsx! {
        div { class: "flex-1 flex flex-col justify-center gap-4 p-4 max-w-md w-full mx-auto",
            h2 { class: "text-center text-lg tracking-widest", "TAIPAN," }
            p { class: "text-center leading-snug",
                "What will you name your firm? (Up to 22 characters.)"
            }
            input {
                class: "crt-input text-center text-lg",
                r#type: "text",
                maxlength: "22",
                placeholder: "Firm name",
                value: "{firm}",
                autofocus: true,
                oninput: move |e| firm.set(e.value()),
            }
            p { class: "text-center mt-2", "Do you want to start . . ." }
            button {
                class: "{BTN_WIDE} py-4",
                disabled: !name_ok(),
                onclick: move |_| start(StartChoice::Cash),
                div { class: "text-lg", "1) WITH CASH (AND A DEBT)" }
                div { class: "text-xs opacity-70 mt-1", "400 in cash · owing 5,000 to Wu" }
            }
            button {
                class: "{BTN_WIDE} py-4",
                disabled: !name_ok(),
                onclick: move |_| start(StartChoice::Guns),
                div { class: "text-lg", "2) WITH FIVE GUNS AND NO CASH" }
                div { class: "text-xs opacity-70 mt-1", "(But no debt!)" }
            }
        }
    }
}
