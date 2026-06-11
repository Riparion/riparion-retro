//! The Trace hub: set your pace and your company, then press on a day at a time.

use dioxus::prelude::*;

use crate::engine::state::{Pace, Stand, TRACE_MILES};
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::components::seg_button::SegButton;
use retro_kit::theme::{PANEL, SCREEN_CENTERED};

#[component]
pub fn TraceHub() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let miles = g.state.miles.max(0.0);
    let pace = g.state.pace;
    let grouped = g.state.grouped;
    let here = Stand::current(miles);
    let progress = (miles / TRACE_MILES * 100.0).min(100.0) as i64;
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
