//! The in-year header: the ruler's title, the year, and a chip grid of the
//! city-state's vital signs.

use dioxus::prelude::*;

use retro_kit::components::chip::Chip;
use retro_kit::format::group_thousands;

use crate::engine::Game;

#[component]
pub fn StatusBar() -> Element {
    let game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;

    rsx! {
        header { class: "status-bar shrink-0",
            div { class: "flex justify-between px-3 pt-2 pb-1",
                span { class: "font-bold tracking-widest truncate", "{s.full_title()}" }
                span { class: "opacity-80", "Year {s.year}" }
            }
            div { class: "grid grid-cols-4 gap-px text-center pb-1 px-1",
                Chip {
                    label: "FLORINS",
                    value: group_thousands(s.treasury),
                    danger: s.treasury < 0,
                }
                Chip { label: "LAND HA", value: group_thousands(s.land) }
                Chip { label: "SERFS", value: group_thousands(s.serfs) }
                Chip { label: "TITLE", value: s.title().to_string() }
            }
        }
    }
}
