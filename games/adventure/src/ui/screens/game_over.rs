//! End of the adventure: the final transcript (score/rank already written by the
//! engine) plus a fresh-start button.

use dioxus::prelude::*;

use retro_kit::components::stat_row::StatRow;
use retro_kit::rng::random_seed;
use retro_kit::theme::{BTN_PRIMARY, PANEL, SCREEN_HERO};

use crate::engine::{modernize, Game, Line};
use crate::storage;

#[component]
pub fn GameOver() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut on_splash = use_context::<Signal<bool>>();

    let (score, max) = game.read().final_score();
    // The last engine block is the score/rank summary; show it verbatim.
    let summary = game
        .read()
        .transcript
        .iter()
        .rev()
        .find_map(|l| match l {
            Line::Out(s) => Some(modernize(s.trim())),
            _ => None,
        })
        .unwrap_or_default();

    rsx! {
        div { class: "{SCREEN_HERO} gap-5 p-6",
            h1 { class: "splash-title text-3xl", "THE CAVE FALLS SILENT" }
            div { class: "{PANEL} p-4 w-full max-w-xs text-left leading-snug whitespace-pre-wrap",
                "{summary}"
            }
            div { class: "w-full max-w-xs",
                StatRow { label: "Score".to_string(), value: format!("{score} / {max}") }
            }
            button {
                class: "{BTN_PRIMARY} text-xl max-w-xs",
                onclick: move |_| {
                    *game.write() = Game::started(random_seed());
                    on_splash.set(false);
                },
                "ENTER AGAIN"
            }
            button {
                class: "crt-btn w-full max-w-xs",
                onclick: move |_| {
                    *game.write() = Game::new(random_seed());
                    storage::clear_save();
                    on_splash.set(true);
                },
                "TITLE SCREEN"
            }
        }
    }
}
