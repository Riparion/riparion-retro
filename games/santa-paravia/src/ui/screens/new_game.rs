//! Found your reign: name yourself, choose your city, gender, and difficulty.

use dioxus::prelude::*;

use crate::engine::state::{Difficulty, Gender, Mode, CITY_NAMES};
use crate::engine::Game;
use crate::ui::Overlay;
use retro_kit::components::seg_button::SegButton;
use retro_kit::theme::{BTN, BTN_PRIMARY, BTN_WIDE, SCREEN};

#[component]
pub fn NewGame() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut overlay = use_context::<Signal<Overlay>>();
    let mut name = use_signal(String::new);
    let mut gender = use_signal(|| Gender::Male);
    let mut city_idx = use_signal(|| 0usize);
    let mut difficulty = use_signal(|| Difficulty::Apprentice);

    let take_throne = move |_| {
        let n = name.read().trim().to_string();
        let ruler = if n.is_empty() || n.len() > 24 {
            "The Ruler".to_string()
        } else {
            n
        };
        let city = CITY_NAMES[city_idx()].to_string();
        game.write().begin(ruler, city, gender(), difficulty());
    };

    let g_sel = gender();
    let d_sel = difficulty();

    rsx! {
        div { class: "{SCREEN} gap-4",
            h2 { class: "text-center text-lg tracking-widest", "FOUND YOUR REIGN" }

            label { class: "chip-label", "Your name" }
            input {
                class: "crt-input text-center text-lg",
                r#type: "text",
                maxlength: "24",
                placeholder: "The Ruler",
                value: "{name}",
                autofocus: true,
                oninput: move |e| name.set(e.value()),
            }

            label { class: "chip-label", "City-state" }
            button {
                class: "{BTN_WIDE}",
                onclick: move |_| city_idx.set((city_idx() + 1) % CITY_NAMES.len()),
                "◂ {CITY_NAMES[city_idx()]} ▸"
            }

            label { class: "chip-label", "Sovereign" }
            div { class: "flex gap-2",
                SegButton { label: "♚ King", active: g_sel == Gender::Male, onclick: move |_| gender.set(Gender::Male) }
                SegButton { label: "♛ Queen", active: g_sel == Gender::Female, onclick: move |_| gender.set(Gender::Female) }
            }

            label { class: "chip-label", "Difficulty" }
            div { class: "grid grid-cols-2 gap-2",
                for diff in Difficulty::ALL {
                    SegButton {
                        key: "{diff.label()}",
                        label: diff.label().to_string(),
                        active: d_sel == diff,
                        onclick: move |_| difficulty.set(diff),
                    }
                }
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
