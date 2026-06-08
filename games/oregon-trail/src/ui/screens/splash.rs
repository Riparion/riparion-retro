//! Title screen with the trailside hall of fame.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use retro_kit::components::score_board::ScoreBoard;
use retro_kit::theme::{BTN_PRIMARY, SCREEN_HERO};

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let scores = use_hook(storage::high_scores);

    rsx! {
        div { class: "{SCREEN_HERO} gap-6 p-6",
            div {
                h1 { class: "splash-title", "THE OREGON TRAIL" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "2,040 miles from Independence to Oregon City. It is 1847. Will your family make it before the snows?"
                }
            }
            button {
                class: "{BTN_PRIMARY} max-w-xs text-xl",
                onclick: move |_| game.write().mode = Mode::NewGame,
                "HEAD WEST"
            }
            ScoreBoard {
                title: "TRAIL HALL OF FAME".to_string(),
                rows: scores
                    .iter()
                    .map(|hs| {
                        let tag = if hs.won { "✓".to_string() } else { format!("{} mi", hs.miles) };
                        (hs.party.clone(), format!("{} · {} · {}", hs.score, hs.rank, tag))
                    })
                    .collect::<Vec<_>>(),
            }
            p { class: "text-xs opacity-60", "After the 1975 MECC classic by Rawitsch, Heinemann & Dillenberger" }
        }
    }
}
