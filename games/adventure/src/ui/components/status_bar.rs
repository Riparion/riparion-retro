//! Sticky header: where you are, whether the lamp is lit, what you're carrying,
//! and how many turns you've taken.

use dioxus::prelude::*;

use retro_kit::components::chip::Chip;

use crate::engine::{modernize, Game};

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();
    let room = modernize(g.room_name());
    let lamp = if g.lamp_is_on() { "ON" } else { "off" };

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "px-3 pt-2 pb-1 font-bold tracking-wide truncate", "{room}" }
            div { class: "grid grid-cols-3 gap-px text-center pb-1 px-1",
                Chip { label: "LAMP".to_string(), value: lamp.to_string(), danger: !g.lamp_is_on() }
                Chip { label: "CARRYING".to_string(), value: g.inventory_count().to_string() }
                Chip { label: "TURNS".to_string(), value: g.turn_count().to_string() }
            }
        }
    }
}
