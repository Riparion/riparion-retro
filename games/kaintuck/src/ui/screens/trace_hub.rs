//! The Trace hub: set your pace and your company, then press on a day at a time.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::state::{fmt_money, Pace, Stand};
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::components::seg_button::SegButton;
use retro_kit::theme::{PANEL, SCREEN_CENTERED};

#[component]
pub fn TraceHub() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();

    // Just set out and on foot with the price in pocket? Ask once whether to buy
    // a mount before the walk; either answer clears the offer and reveals the hub.
    if g.state.horse_offer {
        let price = g.natchez_horse_price().unwrap_or(0.0);
        drop(g);
        return rsx! {
            div { class: "{SCREEN_CENTERED} gap-3",
                div { class: "{PANEL} p-4 flex flex-col gap-3 text-center max-w-md w-full mx-auto",
                    div { class: "chip-label", "── THE LONG WALK ──" }
                    p { class: "text-sm opacity-80",
                        "A horse trader works the head of the Trace. A mount makes faster going — and earns its keep at the Duck River crossing. Buy one before you set off?"
                    }
                    MenuButton {
                        title: format!("Buy a horse — {}", fmt_money(price)),
                        primary: true,
                        onclick: move |_| game.write().answer_trace_horse(true),
                    }
                    MenuButton {
                        title: "No thanks — set off on foot".to_string(),
                        onclick: move |_| game.write().answer_trace_horse(false),
                    }
                }
            }
        };
    }

    let miles = g.state.miles.max(0.0);
    let pace = g.state.pace;
    let grouped = g.state.grouped;
    let here = Stand::current(miles);
    let progress = (miles / scenario().trace.total_miles * 100.0).min(100.0) as i64;
    drop(g);

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "{PANEL} p-3 text-center",
                div { class: "chip-label", "─ ON THE TRACE ─" }
                div { class: "text-lg mt-1", "{here}" }
                div { class: "trail-track mt-2",
                    div { class: "trail-fill", style: "width: {progress}%" }
                    div { class: "trail-wagon", style: "left: {progress}%", "🥾" }
                }
                div { class: "text-xs opacity-60 mt-1", "{progress}% of the way to Nashville" }
            }
            div { class: "{PANEL} p-3 flex flex-col gap-2",
                div { class: "chip-label", "── PACE ──" }
                div { class: "flex gap-2",
                    SegButton {
                        label: "Steady".to_string(),
                        active: pace == Pace::Steady,
                        onclick: move |_| game.write().set_pace(Pace::Steady),
                    }
                    SegButton {
                        label: "Hard".to_string(),
                        active: pace == Pace::Hard,
                        onclick: move |_| game.write().set_pace(Pace::Hard),
                    }
                }
                div { class: "chip-label mt-1", "── COMPANY ──" }
                div { class: "flex gap-2",
                    SegButton {
                        label: "Alone".to_string(),
                        active: !grouped,
                        onclick: move |_| game.write().set_grouped(false),
                    }
                    SegButton {
                        label: "With others".to_string(),
                        active: grouped,
                        onclick: move |_| game.write().set_grouped(true),
                    }
                }
                p { class: "text-xs opacity-60",
                    "Hard pace makes miles but wears you down. Banding together draws off bandits."
                }
            }
            MenuButton {
                title: "🥾 Press on ▸".to_string(),
                primary: true,
                onclick: move |_| game.write().travel_day(),
            }
        }
    }
}
