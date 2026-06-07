//! Distributing the year's 190 furs: one NumberEntry prompt per pelt, a
//! running tally, then a set-out confirmation. The engine clamps to the
//! unassigned remainder, so the original's ">190 = cheat" path is
//! structurally impossible.

use dioxus::prelude::*;

use crate::engine::state::{FUR_ORDER, FURS_PER_YEAR};
use crate::engine::Game;
use retro_kit::components::number_entry::NumberEntry;
use retro_kit::components::stat_row::StatRow;
use retro_kit::theme::{ACTION_BAR, BTN, BTN_PRIMARY, PANEL};

#[component]
pub fn Allocate() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let st = &g.state;
    let year = st.year;
    let cursor = st.cursor;
    let remaining = g.remaining();
    let current = g.current_fur();

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0",
            div { class: "{PANEL} mx-3 mt-2 p-3 text-sm",
                p { class: "font-bold", "YEAR {year} — OUTFITTING" }
                p { class: "opacity-80 mt-1 leading-snug",
                    "Your {FURS_PER_YEAR} furs are distributed among the following "
                    "kinds of pelts: mink, beaver, ermine and fox."
                }
            }
            div { class: "{PANEL} mx-3 mt-2 p-3 text-sm flex flex-col gap-1",
                for (i, fur) in FUR_ORDER.iter().enumerate() {
                    StatRow {
                        label: fur.name().to_string(),
                        value: if i < cursor {
                            st.furs[*fur as usize].to_string()
                        } else {
                            "—".to_string()
                        },
                    }
                }
                StatRow { label: "UNASSIGNED", value: remaining.to_string() }
            }
            div { class: "flex-1" }
            div { class: "{ACTION_BAR}",
                if let Some(fur) = current {
                    NumberEntry {
                        prompt: format!("How many {} pelts do you have?", fur.name()),
                        max: remaining,
                        confirm: "TAKE",
                        preview: Callback::new(move |n: i64| {
                            format!("{} furs left to assign", remaining - n)
                        }),
                        on_submit: move |n: i64| game.write().submit_fur(n),
                    }
                } else {
                    div { class: "flex flex-col gap-2 p-3",
                        if remaining > 0 {
                            p { class: "text-sm opacity-80 text-center",
                                "{remaining} furs stay home unsold."
                            }
                        }
                        div { class: "flex gap-2",
                            button {
                                class: "{BTN} flex-1 py-3",
                                onclick: move |_| game.write().redo_allocation(),
                                "REDO"
                            }
                            button {
                                class: "{BTN_PRIMARY} flex-1 py-3",
                                onclick: move |_| game.write().finish_allocation(),
                                "CHOOSE A FORT"
                            }
                        }
                    }
                }
            }
        }
    }
}
