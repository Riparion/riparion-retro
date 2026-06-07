//! Sticky top panel: the company's vital numbers. While a journal entry is
//! still being revealed line by line, the chips show a snapshot frozen
//! before the visit resolved, so the header never spoils the paced
//! narrative (the sale, the ambush, the new balance).

use dioxus::prelude::*;

use crate::engine::state::MIN_EXPEDITION_COST;
use crate::engine::Game;
use retro_kit::components::chip::Chip;
use retro_kit::format::fmt_dollars_compact;

/// What the chips display. Frozen by the fort-confirm screen right before
/// a trip is committed; live state takes over once the reveal catches up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipStats {
    pub year: i64,
    pub savings: f64,
    pub furs: i64,
}

impl ChipStats {
    pub fn of(game: &Game) -> Self {
        let st = &game.state;
        Self { year: st.year, savings: st.savings, furs: st.allocated() }
    }
}

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let revealed = use_context::<Signal<usize>>();
    let frozen = use_context::<Signal<ChipStats>>();
    let g = game.read();
    let stats = if *revealed.read() < g.log.len() { *frozen.read() } else { ChipStats::of(&g) };
    // Below the cheapest expedition the next year is the last.
    let broke = stats.savings < MIN_EXPEDITION_COST;
    let savings = fmt_dollars_compact(stats.savings);

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                span { class: "font-bold tracking-widest", "FUR TRADER" }
                span { class: "opacity-80 text-sm", "Lake Ontario, 1776" }
            }
            div { class: "grid grid-cols-3 gap-px text-center text-xs pb-1 px-1",
                Chip { label: "YEAR", value: stats.year.to_string() }
                Chip { label: "SAVINGS", value: savings, danger: broke }
                Chip { label: "FURS", value: stats.furs.to_string() }
            }
        }
    }
}
