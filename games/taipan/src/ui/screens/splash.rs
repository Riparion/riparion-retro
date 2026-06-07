//! Title screen with the local fleet of past Taipans.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use retro_kit::components::score_board::ScoreBoard;
use retro_kit::theme::BTN_PRIMARY;

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let scores = use_hook(storage::high_scores);

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-6 p-6 text-center",
            div {
                h1 { class: "splash-title", "TAIPAN!" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "A game of trade and treachery on the South China Sea, 1860."
                }
            }
            button {
                class: "{BTN_PRIMARY} max-w-xs text-xl",
                onclick: move |_| game.write().mode = Mode::NewGame,
                "SET SAIL"
            }
            ScoreBoard {
                title: "HALL OF TAIPANS".to_string(),
                rows: scores
                    .iter()
                    .map(|hs| (hs.firm.clone(), format!("{} · {}", hs.score, hs.rank)))
                    .collect::<Vec<_>>(),
            }
            p { class: "text-xs opacity-60", "After the 1982 Apple ][ classic by Art Canfil" }
        }
    }
}
