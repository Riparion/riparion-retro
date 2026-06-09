//! Start-of-year standings: the weather, this year's prices, and where the
//! city-state stands before you trade.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::chip::Chip;
use retro_kit::components::stat_row::StatRow;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN_PRIMARY, PANEL, SCREEN};

#[component]
pub fn YearReport() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let year = s.year;
    let famine = s.harvest <= 1;

    rsx! {
        div { class: "{SCREEN} gap-3",
            div { class: "text-center",
                h2 { class: "text-lg tracking-widest", "YEAR {year}" }
                p { class: "opacity-70 text-sm", "{s.full_title()} of {s.city}" }
            }

            div {
                class: if famine { "{PANEL} chip-danger p-3 text-center" } else { "{PANEL} p-3 text-center" },
                p { "{s.weather()}" }
                p { class: "text-sm opacity-80 mt-1",
                    "Rats ate {s.rats}% · fields yielded {group_thousands(s.harvest_yield)} steres"
                }
            }

            div { class: "{PANEL} p-3 flex flex-col gap-1 text-sm",
                p { class: "chip-label mb-1 text-center", "── MARKET & GRANARY ──" }
                StatRow { label: "Treasury", value: format!("{} fl", group_thousands(s.treasury)) }
                StatRow { label: "Grain reserve", value: format!("{} st", group_thousands(s.grain)) }
                StatRow { label: "Grain needed", value: format!("{} st", group_thousands(s.grain_demand)) }
                StatRow { label: "Grain price", value: format!("{} fl / 1000 st", s.grain_price) }
                StatRow { label: "Land price", value: format!("{:.2} fl / ha", s.land_price) }
            }

            div { class: "{PANEL} p-3",
                p { class: "chip-label mb-2 text-center", "── YOUR SUBJECTS ──" }
                div { class: "grid grid-cols-3 gap-px text-center",
                    Chip { label: "Serfs", value: group_thousands(s.serfs) }
                    Chip { label: "Soldiers", value: group_thousands(s.soldiers) }
                    Chip { label: "Merchants", value: group_thousands(s.merchants) }
                    Chip { label: "Nobles", value: group_thousands(s.nobles) }
                    Chip { label: "Clergy", value: group_thousands(s.clergy) }
                    Chip { label: "Land ha", value: group_thousands(s.land) }
                }
            }

            button {
                class: "{BTN_PRIMARY} py-4 text-lg mt-1",
                onclick: move |_| game.write().advance_from_report(),
                "TO MARKET ▸"
            }
        }
    }
}
