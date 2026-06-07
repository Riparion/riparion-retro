//! Choose which of the two classic descents to fly.

use dioxus::prelude::*;

use crate::engine::state::{MissionKind, Mode};
use crate::engine::Game;
use retro_kit::theme::{BTN, PANEL};

#[component]
pub fn MissionSelect() -> Element {
    let mut game = use_context::<Signal<Game>>();

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-4 p-4",
            h1 { class: "splash-title text-2xl", "SELECT MISSION" }
            button {
                class: "{PANEL} p-4 w-full max-w-sm text-left hover:opacity-90",
                onclick: move |_| game.write().start(MissionKind::Lunar),
                h2 { class: "font-bold tracking-widest mb-1", "LUNAR" }
                p { class: "text-sm opacity-80 leading-snug",
                    "The Apollo capsule, 120 miles up at 3,600 MPH. Set the retro "
                    "rocket burn (0–200 lb/s) every 10 seconds. 16,500 lb of fuel."
                }
            }
            button {
                class: "{PANEL} p-4 w-full max-w-sm text-left hover:opacity-90",
                onclick: move |_| game.write().start(MissionKind::Rocket),
                h2 { class: "font-bold tracking-widest mb-1", "ROCKET" }
                p { class: "text-sm opacity-80 leading-snug",
                    "Final approach: 1,000 feet up, falling 50 ft/s. Burn 0–30 fuel "
                    "units each second; every unit cuts your descent by 1 ft/s."
                }
            }
            button {
                class: "{BTN} w-full max-w-sm",
                onclick: move |_| game.write().mode = Mode::Splash,
                "BACK"
            }
        }
    }
}
