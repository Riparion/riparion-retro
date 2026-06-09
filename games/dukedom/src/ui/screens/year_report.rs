//! Start-of-year standings: where the duchy stands, and what last year wrought.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::stat_row::StatRow;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN_PRIMARY, PANEL, SCREEN};

const TIER_LABELS: [&str; 6] = ["100%", "80%", "60%", "40%", "20%", "Depl"];

#[component]
pub fn YearReport() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let Some(report) = g.last_report.clone() else {
        return rsx! {};
    };
    let duke = g.state.duke.clone();
    drop(g);

    let ledger = report.ledger_lines();

    rsx! {
        div { class: "{SCREEN} gap-3",
            div { class: "text-center",
                h2 { class: "text-lg tracking-widest", "YEAR {report.year}" }
                p { class: "opacity-70 text-sm", "The reign of {duke}" }
            }
            div { class: "{PANEL} p-3 flex flex-col gap-1 text-sm",
                StatRow { label: "Peasants", value: group_thousands(report.peasants) }
                StatRow { label: "Land (HA)", value: group_thousands(report.land) }
                StatRow { label: "Grain (HL)", value: group_thousands(report.grain) }
            }
            div { class: "{PANEL} p-3",
                p { class: "chip-label mb-2 text-center", "── LAND BY FERTILITY ──" }
                div { class: "grid grid-cols-6 gap-px text-center text-xs",
                    for (i, label) in TIER_LABELS.iter().enumerate() {
                        div { key: "{i}", class: "flex flex-col",
                            span { class: "opacity-60", "{label}" }
                            span { class: "text-base", "{report.tiers[i + 1]}" }
                        }
                    }
                }
            }
            if !ledger.is_empty() {
                div { class: "{PANEL} p-3 flex flex-col gap-1 text-sm",
                    p { class: "chip-label mb-1 text-center", "── LAST YEAR ──" }
                    for (i, (label, value)) in ledger.iter().enumerate() {
                        StatRow {
                            key: "{i}",
                            label: label.to_string(),
                            value: if *value > 0 { format!("+{value}") } else { value.to_string() },
                        }
                    }
                }
            }
            button {
                class: "{BTN_PRIMARY} py-4 text-lg mt-1",
                onclick: move |_| game.write().advance_from_report(),
                "RULE YEAR {report.year} ▸"
            }
        }
    }
}
