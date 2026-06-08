//! The final reckoning: Oregon City, or a marker on the trail.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use retro_kit::components::stat_row::StatRow;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL, SCREEN_HERO};

#[component]
pub fn GameOver() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let Some(end) = g.outcome.clone() else {
        return rsx! {};
    };
    let party = g.state.party.clone();
    drop(g);

    let banner = if end.won {
        "O R E G O N   C I T Y"
    } else {
        "E N D   O F   T H E   T R A I L"
    };

    rsx! {
        div { class: "{SCREEN_HERO} gap-4 p-4",
            h1 {
                class: if end.won { "splash-title" } else { "splash-title chip-danger" },
                "{banner}"
            }
            p { class: "max-w-sm leading-snug", "{end.cause}" }
            div { class: "{PANEL} p-4 w-full max-w-sm text-left text-sm flex flex-col gap-1",
                StatRow { label: "Family", value: party }
                if let Some(arrival) = end.arrival.clone() {
                    StatRow { label: "Arrived", value: arrival }
                }
                StatRow { label: "Miles traveled", value: group_thousands(end.miles) }
                StatRow { label: "Days on the trail", value: end.days.to_string() }
                StatRow { label: "Food", value: end.food.to_string() }
                StatRow { label: "Bullets", value: end.bullets.to_string() }
                StatRow { label: "Clothing", value: end.clothing.to_string() }
                StatRow { label: "Supplies", value: end.misc.to_string() }
                StatRow { label: "Cash", value: format!("${}", end.cash) }
                StatRow { label: "Score", value: end.score.to_string() }
                StatRow { label: "Rating", value: end.rank.clone() }
            }
            div { class: "flex gap-2 w-full max-w-sm",
                button {
                    class: "{BTN} flex-1 py-3",
                    onclick: move |_| game.write().mode = Mode::Splash,
                    "TITLE"
                }
                button {
                    class: "{BTN_PRIMARY} flex-1 py-3",
                    onclick: move |_| game.write().mode = Mode::NewGame,
                    "TRAVEL AGAIN"
                }
            }
        }
    }
}
