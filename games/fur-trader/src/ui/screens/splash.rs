//! Title screen with the local hall of traders.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::storage;
use crate::ui::components::status_bar::ChipStats;
use retro_kit::components::score_board::ScoreBoard;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN_PRIMARY, BTN_WIDE};

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut revealed = use_context::<Signal<usize>>();
    let mut frozen = use_context::<Signal<ChipStats>>();
    let mut on_splash = use_context::<Signal<bool>>();
    let traders = use_hook(storage::high_scores);
    // A save loads in a gameplay mode; a fresh game sits on Splash.
    let resumable = game.peek().mode != Mode::Splash;

    // Starting a new expedition discards any in-progress save on the next persist.
    let begin = move |_| {
        revealed.set(0);
        game.write().start();
        frozen.set(ChipStats::of(&game.peek()));
        on_splash.set(false);
    };
    let (begin_class, begin_label) = if resumable {
        (BTN_WIDE.to_string(), "NEW GAME")
    } else {
        (format!("{BTN_PRIMARY} text-xl"), "BEGIN EXPEDITION")
    };

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-6 p-6 text-center",
            div {
                h1 { class: "splash-title", "FUR TRADER" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "You are the leader of a French fur trading expedition in 1776, "
                    "leaving the Lake Ontario area to sell furs and get supplies for "
                    "the next year. You have a choice of three forts at which you may "
                    "trade — the cost of supplies and the value of your furs depend "
                    "on the fort you choose."
                }
            }
            div { class: "w-full max-w-xs flex flex-col gap-3",
                if resumable {
                    button {
                        class: "{BTN_PRIMARY} text-xl",
                        onclick: move |_| on_splash.set(false),
                        "RESUME EXPEDITION"
                    }
                }
                button {
                    class: "{begin_class}",
                    onclick: begin,
                    "{begin_label}"
                }
            }
            ScoreBoard {
                title: "HALL OF TRADERS".to_string(),
                rows: traders
                    .iter()
                    .take(5)
                    .map(|hs| {
                        (
                            format!("{} · {} yrs", hs.verdict, hs.years),
                            format!("${}", group_thousands(hs.score)),
                        )
                    })
                    .collect::<Vec<_>>(),
            }
            p { class: "text-xs opacity-60 max-w-xs",
                "After FUR TRADER by Dan Bachor, as published in "
                "BASIC Computer Games (1976)."
            }
            p { class: "text-xs opacity-50", "Ported to mobile by Tony Bierman" }
        }
    }
}
