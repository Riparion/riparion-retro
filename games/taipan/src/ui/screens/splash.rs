//! Title screen with the local fleet of past Taipans.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use retro_kit::components::score_board::ScoreBoard;
use retro_kit::theme::{BTN_PRIMARY, BTN_WIDE};

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut on_splash = use_context::<Signal<bool>>();
    let scores = use_hook(storage::high_scores);
    // A save loads in a gameplay mode; a fresh game sits on Splash.
    let resumable = game.peek().mode != Mode::Splash;

    // Starting a new game discards any in-progress save on the next persist.
    let new_game = move |_| {
        game.write().mode = Mode::NewGame;
        on_splash.set(false);
    };
    let (start_class, start_label) = if resumable {
        (BTN_WIDE.to_string(), "NEW GAME")
    } else {
        (format!("{BTN_PRIMARY} text-xl"), "SET SAIL")
    };

    let version = env!("CARGO_PKG_VERSION");

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-6 p-6 text-center",
            div {
                h1 { class: "splash-title", "TAIPAN!" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "A game of trade and treachery on the South China Sea, 1860."
                }
            }
            div { class: "w-full max-w-xs flex flex-col gap-3",
                if resumable {
                    button {
                        class: "{BTN_PRIMARY} text-xl",
                        onclick: move |_| on_splash.set(false),
                        "RESUME VOYAGE"
                    }
                }
                button {
                    class: "{start_class}",
                    onclick: new_game,
                    "{start_label}"
                }
            }
            ScoreBoard {
                title: "HALL OF TAIPANS".to_string(),
                rows: scores
                    .iter()
                    .map(|hs| (hs.firm.clone(), format!("{} · {}", hs.score, hs.rank)))
                    .collect::<Vec<_>>(),
            }
            p { class: "text-xs opacity-60", "After the 1982 Apple ][ classic by Art Canfil" }
            p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }
            p { class: "text-xs opacity-40", "v{version}" }
        }
    }
}
