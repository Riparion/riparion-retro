//! The final reckoning: Nashville, or a grave on the Trace (or the bottom of the
//! river).

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
    let trader = g.state.trader.clone();
    drop(g);

    let banner = if end.won {
        "N A S H V I L L E"
    } else {
        "E N D   O F   T H E   R O A D"
    };

    rsx! {
        div { class: "{SCREEN_HERO} gap-4 p-4",
            h1 {
                class: if end.won { "splash-title" } else { "splash-title chip-danger" },
                "{banner}"
            }
            p { class: "max-w-sm leading-snug", "{end.cause}" }
            div { class: "{PANEL} p-4 w-full max-w-sm text-left text-sm flex flex-col gap-1",
                StatRow { label: "Kaintuck".to_string(), value: trader }
                StatRow { label: "Miles traveled".to_string(), value: group_thousands(end.miles) }
                StatRow { label: "Days out".to_string(), value: end.days.to_string() }
                StatRow { label: "Cash".to_string(), value: format!("${}", end.cash) }
                if end.debt > 0 {
                    StatRow { label: "Debt".to_string(), value: format!("-${}", end.debt) }
                }
                StatRow { label: "Crew home".to_string(), value: end.crew_survived.to_string() }
                StatRow { label: "Reputation".to_string(), value: end.reputation.to_string() }
                StatRow { label: "Robbed".to_string(), value: (if end.robbed { "yes" } else { "no" }).to_string() }
                StatRow { label: "Score".to_string(), value: end.score.to_string() }
                StatRow { label: "Standing".to_string(), value: end.rank.clone() }
                StatRow {
                    label: "Carried to next run".to_string(),
                    value: format!("${}", group_thousands((end.cash - end.debt).max(0))),
                }
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
                    "RUN IT AGAIN"
                }
            }
        }
    }
}
