//! Sticky top panel. River phase shows the purse, the boat, and the crew; the
//! Trace phase shows the purse, provisions, health, and miles home.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::{Mode, Phase, NUM_RIVER_TOWNS};
use crate::engine::Game;
use crate::ui::components::notifications::NotificationButton;
use crate::ui::components::repair_sheet::RepairSheet;
use retro_kit::components::chip::Chip;
use retro_kit::format::{fmt_dollars_compact, fmt_num, group_thousands};

#[component]
pub fn StatusBar() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut repair_open = use_signal(|| false);
    // Stable handlers for the chip shortcuts — created once (not per render), so
    // they can be handed to `Chip` as `Some` only when the action applies.
    let to_moneylender = use_callback(move |_: MouseEvent| game.write().mode = Mode::Moneylender);
    let to_trade = use_callback(move |_: MouseEvent| {
        game.write().mode = Mode::Trade { can_buy: true, can_sell: true };
    });
    let to_repair = use_callback(move |_: MouseEvent| repair_open.set(true));
    let g = game.read();
    let s = &g.state;

    match g.phase {
        Phase::River => {
            let cargo = s.cargo_units();
            let cap = s.capacity();
            let low_morale = s.morale < 40.0;
            let in_debt = s.debt > 0.0;
            let hull_bad = s.boat_damage() >= 80.0;
            // The DEBT / CARGO / HULL chips double as shortcuts to the
            // moneylender, the market, and hull repairs — but only at a town
            // hub, the one place those actions apply (and where the engine's
            // repair guards permit them). Elsewhere they stay plain read-outs,
            // so the per-town facts are only looked up when `at_town` holds.
            let at_town = g.mode == Mode::Town;
            let moneylender_here =
                at_town && scenario().river.towns[s.town.min(NUM_RIVER_TOWNS - 1)].moneylender;
            let repair_here = at_town && s.boat_damage() > 0.0;
            let town_name = s.town_name();
            let river_mile = s.river_mile() as i64;
            let cash = fmt_dollars_compact(s.cash.max(0.0));
            let debt = fmt_dollars_compact(s.debt.max(0.0));
            let hull = s.hull_label();
            let crew = format!("{} · {}", s.crew, s.morale_label());
            drop(g);
            rsx! {
                header { class: "status-bar shrink-0",
                    div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                        NotificationButton {}
                        span { class: "opacity-80", "⚓ {town_name}" }
                    }
                    div { class: "px-3 pb-1 text-xs opacity-70",
                        "{group_thousands(river_mile)} / {scenario().river.river_miles as i64} river miles to Natchez"
                    }
                    div { class: "grid grid-cols-5 gap-px text-center pb-1 px-1",
                        Chip { label: "CASH", value: cash }
                        Chip {
                            label: "DEBT",
                            value: debt,
                            danger: in_debt,
                            onclick: moneylender_here.then_some(to_moneylender),
                        }
                        Chip {
                            label: "CARGO",
                            value: format!("{cargo}/{cap}"),
                            onclick: at_town.then_some(to_trade),
                        }
                        Chip {
                            label: "HULL",
                            value: hull.to_string(),
                            danger: hull_bad,
                            onclick: repair_here.then_some(to_repair),
                        }
                        Chip { label: "CREW", value: crew, danger: low_morale }
                    }
                }
                if repair_here {
                    RepairSheet { open: repair_open }
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
                        NotificationButton {}
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
