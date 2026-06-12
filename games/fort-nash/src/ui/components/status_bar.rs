//! Sticky top panel: the date, the miles behind you, your health, and the
//! five supplies the party hauls down the Wilderness Road.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::Game;
use retro_kit::components::chip::Chip;
use retro_kit::format::{fmt_dollars_compact, fmt_num, group_thousands};

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let miles = s.miles.max(0.0);
    let remaining = (scenario().trail.total_miles - miles).max(0.0);
    let unwell = s.ill || s.injured;
    let low_food = s.food < 30.0;
    let low_ammo = s.bullets < 40.0;

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                span { class: "font-bold tracking-widest truncate", "{s.party}" }
                span { class: "opacity-80", "{s.date_string()}" }
            }
            div { class: "flex justify-between items-baseline px-3 pb-1",
                span { class: "text-lg", "🛞 {group_thousands(miles as i64)} mi" }
                span { class: if unwell { "chip-danger" } else { "opacity-80" },
                    "⚕ {s.health_label()}"
                }
            }
            div { class: "px-3 pb-1 text-xs opacity-70", "{group_thousands(remaining as i64)} miles to the French Lick" }
            div { class: "grid grid-cols-5 gap-px text-center pb-1 px-1",
                Chip { label: "FOOD", value: fmt_num(s.food.max(0.0)), danger: low_food }
                Chip { label: "POWDER", value: fmt_num(s.bullets.max(0.0)), danger: low_ammo }
                Chip { label: "CLOTHES", value: fmt_num(s.clothing.max(0.0)) }
                Chip { label: "SUPPLIES", value: fmt_num(s.misc.max(0.0)) }
                Chip { label: "COIN", value: fmt_dollars_compact(s.cash.max(0.0)) }
            }
        }
    }
}
