//! Sticky top panel: firm, date, location, and the vital numbers.

use dioxus::prelude::*;

use crate::engine::state::fmt_money;
use crate::engine::Game;
use retro_kit::components::chip::Chip;

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let location = if g.voyage.is_some() {
        "At sea".to_string()
    } else {
        s.port_name().to_string()
    };
    let hold_free = s.free_hold();
    let sw = s.seaworthiness();
    let sw_title = s.seaworthiness_title();
    let danger = if sw < 40 { "chip-danger" } else { "" };
    let debt_danger = s.debt > 10_000.0;
    let hold_danger = hold_free < 0;

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                span { class: "font-bold tracking-widest", "{s.firm}" }
                span { class: "opacity-80", "{s.date_string()}" }
            }
            div { class: "flex justify-between items-baseline px-3 pb-1",
                span { class: "text-lg", "⚓ {location}" }
                span { class: "{danger}", "Ship: {sw_title} ({sw}%)" }
            }
            div { class: "grid grid-cols-4 gap-px text-center text-xs pb-1 px-1",
                Chip { label: "CASH", value: fmt_money(s.cash) }
                Chip { label: "BANK", value: fmt_money(s.bank) }
                Chip { label: "DEBT", value: fmt_money(s.debt), danger: debt_danger }
                Chip {
                    label: "HOLD · GUNS",
                    value: format!("{hold_free} · {}", s.guns),
                    danger: hold_danger,
                }
            }
        }
    }
}
