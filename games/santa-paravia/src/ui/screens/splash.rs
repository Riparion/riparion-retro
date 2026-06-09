//! Title screen with the city's hall of fame.

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
    // of NEW GAME mutates the in-memory game but leaves the save intact.
    let resumable = use_hook(storage::has_save);

    let new_game = move |_| {
        game.write().mode = Mode::NewGame;
        overlay.set(Overlay::Game);
    };
    let (start_class, start_label) = if resumable {
        (BTN_WIDE.to_string(), "NEW GAME")
    } else {
        (format!("{BTN_PRIMARY} text-xl"), "TAKE THE THRONE")
    };

    rsx! {
        div { class: "{SCREEN_HERO} gap-6 p-6",
            div {
                h1 { class: "splash-title", "SANTA PARAVIA" }
                p { class: "opacity-70 tracking-widest text-sm mt-1", "AND FIUMACCIO" }
                p { class: "opacity-80 mt-3 max-w-xs mx-auto leading-snug",
                    "Rule a 15th-century Italian city-state. Trade grain, feed your people, tax wisely, and build — rise from Sir to King before your years run out."
                }
            }
            div { class: "w-full max-w-xs flex flex-col gap-3",
                if resumable {
                    button {
                        class: "{BTN_PRIMARY} text-xl",
                        onclick: move |_| {
                            // Reload the persisted save in case the in-memory game
                            // was clobbered by a backed-out NEW GAME.
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
                title: "HALL OF RULERS".to_string(),
                rows: scores
                    .iter()
                    .map(|hs| {
                        let tag = if hs.won { "♛".to_string() } else { format!("{} yr", hs.years) };
                        (hs.name.clone(), format!("{} · {} · {}", hs.score, hs.title, tag))
                    })
                    .collect::<Vec<_>>(),
            }
            div {
                p { class: "text-xs opacity-60",
                    "After Santa Paravia and Fiumaccio by George Blank, 1978"
                }
                p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }
            }
        }
    }
}
