//! Title screen with the trailside hall of fame.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use retro_kit::components::score_board::ScoreBoard;
use retro_kit::theme::{BTN_PRIMARY, BTN_WIDE, SCREEN_HERO};

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
        (format!("{BTN_PRIMARY} text-xl"), "HEAD WEST")
    };

    let version = env!("CARGO_PKG_VERSION");

    rsx! {
        div { class: "{SCREEN_HERO} gap-6 p-6",
            div {
                h1 { class: "splash-title", "THE OREGON TRAIL" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "2,040 miles from Independence to Oregon City. It is 1847. Will your family make it before the snows?"
                }
            }
            div { class: "w-full max-w-xs flex flex-col gap-3",
                if resumable {
                    button {
                        class: "{BTN_PRIMARY} text-xl",
                        onclick: move |_| on_splash.set(false),
                        "RESUME JOURNEY"
                    }
                }
                button {
                    class: "{start_class}",
                    onclick: new_game,
                    "{start_label}"
                }
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
            p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }
            p { class: "text-xs opacity-40", "v{version}" }
        }
    }
}
