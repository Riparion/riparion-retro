//! Sticky top panel: mission name and the vital numbers, in mission units.
//! Reads the *revealed* telemetry row so the chips advance with the paced
//! log rather than spoiling the turn's outcome.

use dioxus::prelude::*;

use crate::engine::state::{LogLine, MissionKind};
use crate::engine::Game;
use crate::ui::components::revealed_row;
use retro_kit::components::chip::Chip;

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let pending = use_context::<Signal<Vec<LogLine>>>();
    let g = game.read();
    let row = revealed_row(&g, &pending.read());
    let mission = g.mission();
    let (subtitle, vel_danger, fuel_danger) = match mission {
        MissionKind::Lunar => (
            "Apollo capsule · manual control",
            row.vel_raw > 1000.0,
            row.fuel_raw < 1500.0,
        ),
        MissionKind::Rocket => (
            "Final approach · 1-second bursts",
            row.vel_raw >= 30.0,
            row.fuel_raw < 30.0,
        ),
    };

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                span { class: "font-bold tracking-widest", "{mission.title()} MISSION" }
                span { class: "opacity-80 text-sm", "{subtitle}" }
            }
            div { class: "grid grid-cols-4 gap-px text-center text-xs pb-1 px-1",
                Chip { label: "TIME", value: format!("{} s", row.sec) }
                Chip { label: "ALT", value: row.alt.clone() }
                Chip { label: "VEL", value: row.vel.clone(), danger: vel_danger }
                Chip { label: "FUEL", value: row.fuel.clone(), danger: fuel_danger }
            }
        }
    }
}
