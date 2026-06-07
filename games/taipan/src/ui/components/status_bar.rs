//! Sticky top panel: firm, date, location, and the vital numbers.

use dioxus::prelude::*;

use crate::engine::state::fmt_money;
use crate::engine::Game;

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
    let debt_class = if s.debt > 10_000.0 { "chip-danger" } else { "" };
    let hold_class = if hold_free < 0 { "chip-danger" } else { "" };

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
                div { class: "chip",
                    div { class: "chip-label", "CASH" }
                    div { "{fmt_money(s.cash)}" }
                }
                div { class: "chip",
                    div { class: "chip-label", "BANK" }
                    div { "{fmt_money(s.bank)}" }
                }
                div { class: "chip {debt_class}",
                    div { class: "chip-label", "DEBT" }
                    div { "{fmt_money(s.debt)}" }
                }
                div { class: "chip {hold_class}",
                    div { class: "chip-label", "HOLD · GUNS" }
                    div { "{hold_free} · {s.guns}" }
                }
            }
        }
    }
}
