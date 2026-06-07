//! Sticky top panel: the city's vital numbers. While a report is still
//! being revealed line by line, the chips show a snapshot frozen before
//! the year resolved, so the header never spoils the paced narrative
//! (plague, starvation, the new year number).

use dioxus::prelude::*;

use crate::engine::{Game, BUSHELS_PER_PERSON, TERM_YEARS};
use retro_kit::components::chip::Chip;
use retro_kit::format::group_thousands;

/// What the chips display. Frozen by the year screen right before a
/// decision is submitted; live state takes over once the reveal catches up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipStats {
    pub year: i64,
    pub population: i64,
    pub acres: i64,
    pub store: i64,
}

impl ChipStats {
    pub fn of(game: &Game) -> Self {
        let st = &game.state;
        Self { year: st.year, population: st.population, acres: st.acres, store: st.store }
    }
}

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let revealed = use_context::<Signal<usize>>();
    let frozen = use_context::<Signal<ChipStats>>();
    let g = game.read();
    let stats = if *revealed.read() < g.log.len() { *frozen.read() } else { ChipStats::of(&g) };
    // The store can't keep everyone alive through the year: warn.
    let grain_danger = stats.store < stats.population * BUSHELS_PER_PERSON;

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                span { class: "font-bold tracking-widest", "HAMURABI" }
                span { class: "opacity-80 text-sm", "Ancient Sumeria" }
            }
            div { class: "grid grid-cols-4 gap-px text-center text-xs pb-1 px-1",
                Chip { label: "YEAR", value: format!("{} / {TERM_YEARS}", stats.year.min(TERM_YEARS)) }
                Chip { label: "PEOPLE", value: group_thousands(stats.population) }
                Chip { label: "ACRES", value: group_thousands(stats.acres) }
                Chip {
                    label: "GRAIN",
                    value: group_thousands(stats.store),
                    danger: grain_danger,
                }
            }
        }
    }
}
