//! Title screen with the local hall of rulers.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use crate::ui::components::status_bar::ChipStats;
use retro_kit::components::score_board::ScoreBoard;
use retro_kit::theme::{BTN_PRIMARY, BTN_WIDE};

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut revealed = use_context::<Signal<usize>>();
    let mut frozen = use_context::<Signal<ChipStats>>();
    let mut on_splash = use_context::<Signal<bool>>();
    let rulers = use_hook(storage::high_scores);
    // A save loads in a gameplay mode; a fresh game sits on Splash.
    let resumable = game.peek().mode != Mode::Splash;

    // Starting a fresh reign discards any in-progress save on the next persist.
    let begin = move |_| {
        revealed.set(0);
        game.write().start();
        frozen.set(ChipStats::of(&game.peek()));
        on_splash.set(false);
    };
    let (begin_class, begin_label) = if resumable {
        (BTN_WIDE.to_string(), "NEW REIGN")
    } else {
        (format!("{BTN_PRIMARY} text-xl"), "BEGIN REIGN")
    };

    let version = env!("CARGO_PKG_VERSION");

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-6 p-6 text-center",
            div {
                h1 { class: "splash-title", "HAMURABI" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "Try your hand at governing ancient Sumeria for a ten-year term of office."
                }
            }
            div { class: "w-full max-w-xs flex flex-col gap-3",
                if resumable {
                    button {
                        class: "{BTN_PRIMARY} text-xl",
                        onclick: move |_| on_splash.set(false),
                        "RESUME REIGN"
                    }
                }
                button {
                    class: "{begin_class}",
                    onclick: begin,
                    "{begin_label}"
                }
            }
            ScoreBoard {
                title: "HALL OF RULERS".to_string(),
                rows: rulers
                    .iter()
                    .take(5)
                    .map(|hs| (format!("{} · pop {}", hs.verdict, hs.population), hs.score.to_string()))
                    .collect::<Vec<_>>(),
            }
            p { class: "text-xs opacity-60 max-w-xs",
                "After HAMURABI, as published in BASIC Computer Games (1978), "
                "after Doug Dyment's The Sumer Game (1968). "
                "The missing M is original."
            }
            p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }
            p { class: "text-xs opacity-40", "v{version}" }
        }
    }
}
