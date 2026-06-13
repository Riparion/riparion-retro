//! Sticky top panel. River phase shows the purse, the boat, and the crew; the
//! Trace phase shows the purse, provisions, health, and miles home.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::Phase;
use crate::engine::Game;
use retro_kit::components::chip::Chip;
use retro_kit::format::{fmt_dollars_compact, fmt_num, group_thousands};

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;

    match g.phase {
        Phase::River => {
            let cargo = s.cargo_units();
            let cap = s.capacity();
            let low_morale = s.morale < 40.0;
            let in_debt = s.debt > 0.0;
            let hull_bad = s.boat_damage() >= 80.0;
            rsx! {
                header { class: "status-bar shrink-0",
                    div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                        span { class: "font-bold tracking-widest truncate", "{s.trader}" }
                        span { class: "opacity-80", "⚓ {s.town_name()}" }
                    }
                    div { class: "px-3 pb-1 text-xs opacity-70",
                        "{group_thousands(s.river_mile() as i64)} / {scenario().river.river_miles as i64} river miles to Natchez"
                    }
                    div { class: "grid grid-cols-5 gap-px text-center pb-1 px-1",
                        Chip { label: "CASH", value: fmt_dollars_compact(s.cash.max(0.0)) }
                        Chip { label: "DEBT", value: fmt_dollars_compact(s.debt.max(0.0)), danger: in_debt }
                        Chip { label: "CARGO", value: format!("{cargo}/{cap}") }
                        Chip { label: "HULL", value: s.hull_label().to_string(), danger: hull_bad }
                        Chip { label: "CREW", value: format!("{} · {}", s.crew, s.morale_label()), danger: low_morale }
                    }
                }
            }
        }
        Phase::Trace => {
            let miles = s.miles.max(0.0);
            let remaining = (scenario().trace.total_miles - miles).max(0.0);
            let low_food = s.provisions < 12.0;
            let unwell = s.health < 45.0;
            rsx! {
                header { class: "status-bar shrink-0",
                    div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                        span { class: "font-bold tracking-widest truncate", "{s.trader}" }
                        span { class: "opacity-80", "Day {s.day}" }
                    }
                    div { class: "flex justify-between items-baseline px-3 pb-1",
                        span { class: "text-lg", "🥾 {group_thousands(miles as i64)} mi" }
                        span { class: if unwell { "chip-danger" } else { "opacity-80" }, "⚕ {s.health_label()}" }
                    }
                    div { class: "px-3 pb-1 text-xs opacity-70", "{group_thousands(remaining as i64)} miles to Nashville" }
                    div { class: "grid grid-cols-3 gap-px text-center pb-1 px-1",
                        Chip { label: "CASH", value: fmt_dollars_compact(s.cash.max(0.0)) }
                        Chip { label: "PROVISIONS", value: fmt_num(s.provisions.max(0.0)), danger: low_food }
                        Chip { label: if s.has_horse { "HORSE" } else { "ON FOOT" }, value: (if s.has_horse { "yes" } else { "—" }).to_string() }
                    }
                }
            }
        }
    }
}
