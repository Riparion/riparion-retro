//! The final reckoning: a crown seized, or a reign cut short by the years.

use dioxus::prelude::*;

use crate::engine::state::Mode;
use crate::engine::Game;
use crate::ui::Overlay;
use retro_kit::components::stat_row::StatRow;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL, SCREEN_HERO};

#[component]
pub fn GameOver() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut overlay = use_context::<Signal<Overlay>>();
    let g = game.read();
    let Some(end) = g.outcome.clone() else {
        return rsx! {};
    };
    let name = g.state.name.clone();
    drop(g);

    let banner = if end.won {
        end.title.to_uppercase()
    } else {
        "THE END".to_string()
    };

    rsx! {
        div { class: "{SCREEN_HERO} gap-4 p-4",
            h1 {
                class: if end.won { "splash-title" } else { "splash-title chip-danger" },
                "{banner}"
            }
            p { class: "max-w-sm leading-snug", "{end.cause}" }
            div { class: "{PANEL} p-4 w-full max-w-sm text-left text-sm flex flex-col gap-1",
                StatRow { label: "Ruler", value: name }
                StatRow { label: "Final title", value: end.title.clone() }
                StatRow { label: "Years ruled", value: end.years.to_string() }
                StatRow { label: "Treasury", value: format!("{} fl", group_thousands(end.treasury)) }
                StatRow { label: "Land (ha)", value: group_thousands(end.land) }
                StatRow { label: "Serfs", value: group_thousands(end.serfs) }
                StatRow { label: "Score", value: group_thousands(end.score) }
                StatRow { label: "Rank", value: end.rank.clone() }
            }
            div { class: "flex gap-2 w-full max-w-sm",
                button {
                    class: "{BTN} flex-1 py-3",
                    onclick: move |_| {
                        game.write().mode = Mode::Splash;
                        overlay.set(Overlay::Splash);
                    },
                    "TITLE"
                }
                button {
                    class: "{BTN_PRIMARY} flex-1 py-3",
                    onclick: move |_| game.write().mode = Mode::NewGame,
                    "RULE AGAIN"
                }
            }
        }
    }
}
