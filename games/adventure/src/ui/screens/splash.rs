//! Title screen with the hall of fame. Every load opens here (NOTES.md), even
//! when a save can be resumed.

use dioxus::prelude::*;

use retro_kit::components::score_board::ScoreBoard;
use retro_kit::rng::random_seed;
use retro_kit::theme::{BTN_PRIMARY, BTN_WIDE, SCREEN_HERO};

use crate::engine::{Game, Mode};
use crate::storage;

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut on_splash = use_context::<Signal<bool>>();
    let scores = use_hook(storage::high_scores);
    // A save loads in a gameplay mode; a fresh game sits on Splash.
    let resumable = game.peek().mode != Mode::Splash;

    let new_game = move |_| {
        *game.write() = Game::started(random_seed());
        on_splash.set(false);
    };
    let (start_class, start_label) = if resumable {
        (BTN_WIDE.to_string(), "NEW GAME")
    } else {
        (format!("{BTN_PRIMARY} text-xl"), "ENTER THE CAVE")
    };

    let version = env!("CARGO_PKG_VERSION");

    rsx! {
        div { class: "{SCREEN_HERO} gap-6 p-6",
            div {
                h1 { class: "splash-title", "COLOSSAL CAVE" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "Somewhere nearby is Colossal Cave, where others have found fortunes in treasure and gold — though it is rumored that some who enter are never seen again."
                }
            }
            div { class: "w-full max-w-xs flex flex-col gap-3",
                if resumable {
                    button {
                        class: "{BTN_PRIMARY} text-xl",
                        onclick: move |_| on_splash.set(false),
                        "RESUME"
                    }
                }
                button { class: "{start_class}", onclick: new_game, "{start_label}" }
            }
            ScoreBoard {
                title: "EXPLORERS' HALL OF FAME".to_string(),
                rows: scores
                    .iter()
                    .map(|hs| {
                        let tag = if hs.closed { "✓".to_string() } else { format!("{} turns", hs.turns) };
                        ("Adventurer".to_string(), format!("{} pts · {}", hs.score, tag))
                    })
                    .collect::<Vec<_>>(),
            }
            p { class: "text-xs opacity-60",
                "After the 1977 Crowther & Woods classic (350-point version)"
            }
            p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }
            p { class: "text-xs opacity-40", "v{version}" }
        }
    }
}
