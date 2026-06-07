//! Sticky top panel: mission name and the vital numbers, in mission units.

use dioxus::prelude::*;

use crate::engine::state::{fmt_lunar_alt, fmt_num, Flight, MissionKind};
use crate::engine::Game;

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();
    let (time, alt, vel, vel_danger, fuel, fuel_danger) = match &g.flight {
        Flight::Lunar(st) => {
            let mph = 3600.0 * st.v;
            (
                format!("{} s", fmt_num(st.l.round())),
                fmt_lunar_alt(st.a),
                format!("{mph:.0} mph"),
                mph > 1000.0,
                format!("{:.0} lb", crate::engine::lunar::fuel(st)),
                crate::engine::lunar::fuel(st) < 1500.0,
            )
        }
        Flight::Rocket(st) => (
            format!("{} s", fmt_num(st.t)),
            format!("{} ft", fmt_num(st.h)),
            format!("{} ft/s", fmt_num(st.v)),
            st.v >= 30.0,
            fmt_num(st.f),
            st.f < 30.0,
        ),
    };
    let mission = g.mission;
    let subtitle = match mission {
        MissionKind::Lunar => "Apollo capsule · manual control",
        MissionKind::Rocket => "Final approach · 1-second bursts",
    };
    let vel_class = if vel_danger { "chip-danger" } else { "" };
    let fuel_class = if fuel_danger { "chip-danger" } else { "" };

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "flex justify-between items-baseline px-3 pt-2 pb-1",
                span { class: "font-bold tracking-widest", "{mission.title()} MISSION" }
                span { class: "opacity-80 text-sm", "{subtitle}" }
            }
            div { class: "grid grid-cols-4 gap-px text-center text-xs pb-1 px-1",
                div { class: "chip",
                    div { class: "chip-label", "TIME" }
                    div { "{time}" }
                }
                div { class: "chip",
                    div { class: "chip-label", "ALT" }
                    div { "{alt}" }
                }
                div { class: "chip {vel_class}",
                    div { class: "chip-label", "VEL" }
                    div { "{vel}" }
                }
                div { class: "chip {fuel_class}",
                    div { class: "chip-label", "FUEL" }
                    div { "{fuel}" }
                }
            }
        }
    }
}
