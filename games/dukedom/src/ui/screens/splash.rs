//! Title screen with the duchy's hall of fame.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use crate::ui::Overlay;
use retro_kit::components::score_board::ScoreBoard;
use retro_kit::theme::{BTN_PRIMARY, BTN_WIDE, SCREEN_HERO};

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut overlay = use_context::<Signal<Overlay>>();
    let scores = use_hook(storage::high_scores);
    // Truth comes from the persisted save, not the in-memory mode — backing out
    // of NEW REIGN mutates the in-memory game but leaves the save intact.
    let resumable = use_hook(storage::has_save);

    let new_game = move |_| {
        game.write().mode = Mode::NewGame;
        overlay.set(Overlay::Game);
    };
    let (start_class, start_label) = if resumable {
        (BTN_WIDE.to_string(), "NEW REIGN")
    } else {
        (format!("{BTN_PRIMARY} text-xl"), "TAKE THE THRONE")
    };

    rsx! {
        div { class: "{SCREEN_HERO} gap-6 p-6",
            div {
                h1 { class: "splash-title", "DUKEDOM" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "You are a medieval Duke. Feed your peasants, work the land, and outlast plague, war, and the High King's taxes. How long can your line rule?"
                }
            }
            div { class: "w-full max-w-xs flex flex-col gap-3",
                if resumable {
                    button {
                        class: "{BTN_PRIMARY} text-xl",
                        onclick: move |_| {
                            // Reload the persisted save in case the in-memory game
                            // was clobbered by a backed-out NEW REIGN.
                            game.set(storage::load_or_new());
                            overlay.set(Overlay::Game);
                        },
                        "RESUME REIGN"
                    }
                }
                button { class: "{start_class}", onclick: new_game, "{start_label}" }
                button {
                    class: "{BTN_WIDE}",
                    onclick: move |_| overlay.set(Overlay::Help),
                    "HOW TO RULE"
                }
            }
            ScoreBoard {
                title: "HALL OF DUKES".to_string(),
                rows: scores
                    .iter()
                    .map(|hs| {
                        let tag = if hs.won { "♛".to_string() } else { format!("{} yr", hs.years) };
                        (hs.duke.clone(), format!("{} · {} · {}", hs.score, hs.rank, tag))
                    })
                    .collect::<Vec<_>>(),
            }
            p { class: "text-xs opacity-60",
                "After Dukedom by Kaapke, as published in Creative Computing, Feb 1980"
            }
            p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }
        }
    }
}
