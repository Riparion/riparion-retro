//! The in-year header: the year and a chip grid of the duchy's vital signs.

use dioxus::prelude::*;

use retro_kit::components::chip::Chip;
use retro_kit::format::group_thousands;

use crate::engine::Game;

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let restless = s.u1.max(s.u2) > 55;

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "flex justify-between px-3 pt-2 pb-1",
                span { class: "font-bold tracking-widest truncate", "{s.duke}" }
                span { class: "opacity-80", "Year {s.n_y}" }
            }
            div { class: "grid grid-cols-4 gap-px text-center pb-1 px-1",
                Chip { label: "PEASANTS", value: group_thousands(s.n_p) }
                Chip { label: "LAND HA", value: group_thousands(s.n_l) }
                Chip { label: "GRAIN HL", value: group_thousands(s.n_g) }
                Chip { label: "MOOD", value: s.unrest_label().to_string(), danger: restless }
            }
        }
    }
}
