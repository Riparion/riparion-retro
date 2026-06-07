//! Touchdown or crater: the final reckoning.

use dioxus::prelude::*;

use crate::engine::state::{fmt_num, Mode};
use crate::engine::Game;
use retro_kit::components::stat_row::StatRow;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL};

#[component]
pub fn GameOver() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let Some(end) = g.outcome.clone() else {
        return rsx! {};
    };
    drop(g);

    let landed = end.quality.landed_ok();
    let banner = if landed { "T O U C H D O W N" } else { "G A M E   O V E R" };

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-4 p-4 text-center",
            h1 { class: if landed { "splash-title text-2xl" } else { "splash-title text-2xl chip-danger" },
                "{banner}"
            }
            p { class: "max-w-sm leading-snug font-bold", "{end.headline}" }
            if !end.detail.is_empty() {
                p { class: "max-w-sm leading-snug text-sm opacity-80", "{end.detail}" }
            }
            div { class: "{PANEL} p-4 w-full max-w-sm text-left text-sm flex flex-col gap-1",
                StatRow { label: "Mission", value: end.mission.title().to_string() }
                StatRow { label: "Outcome", value: end.quality.label().to_string() }
                StatRow { label: "Impact velocity", value: end.impact.clone() }
                StatRow { label: "Flight time", value: format!("{} seconds", fmt_num(end.elapsed.round())) }
                StatRow { label: "Fuel remaining", value: fmt_num(end.fuel_left.floor()) }
                StatRow { label: "Score", value: end.score.to_string() }
            }
            div { class: "flex gap-2 w-full max-w-sm",
                button {
                    class: "{BTN} flex-1 py-3",
                    onclick: move |_| game.write().mode = Mode::Splash,
                    "TITLE"
                }
                button {
                    class: "{BTN_PRIMARY} flex-1 py-3",
                    onclick: move |_| game.write().mode = Mode::MissionSelect,
                    "FLY AGAIN"
                }
            }
        }
    }
}

