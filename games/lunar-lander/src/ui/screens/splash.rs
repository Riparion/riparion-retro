//! Title screen with the local flight records.

use dioxus::prelude::*;

use crate::engine::state::{MissionKind, Mode};
use crate::engine::Game;
use crate::storage;
use retro_kit::theme::{BTN_PRIMARY, PANEL};

#[component]
pub fn Splash() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let lunar = use_hook(|| storage::high_scores(MissionKind::Lunar));
    let rocket = use_hook(|| storage::high_scores(MissionKind::Rocket));

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-6 p-6 text-center",
            div {
                h1 { class: "splash-title", "LUNAR LANDER" }
                p { class: "opacity-80 mt-2 max-w-xs mx-auto leading-snug",
                    "The on-board computer has failed. Land the capsule manually."
                }
            }
            button {
                class: "{BTN_PRIMARY} max-w-xs text-xl",
                onclick: move |_| game.write().mode = Mode::MissionSelect,
                "BEGIN MISSION"
            }
            for (title, scores) in [("LUNAR", lunar), ("ROCKET", rocket)] {
                if !scores.is_empty() {
                    div { class: "{PANEL} p-3 w-full max-w-xs text-left",
                        h2 { class: "chip-label mb-2 text-center", "── {title} FLIGHT RECORDS ──" }
                        for (i, hs) in scores.iter().take(5).enumerate() {
                            div { key: "{i}", class: "flex justify-between text-sm gap-2",
                                span { class: "truncate", "{hs.quality} · {hs.impact}" }
                                span { class: "opacity-80 shrink-0", "{hs.score}" }
                            }
                        }
                    }
                }
            }
            p { class: "text-xs opacity-60 max-w-xs",
                "After LUNAR by Jim Storer (1969) and ROCKET by Eric Peters, "
                "as published in BASIC Computer Games (1978)"
            }
        }
    }
}
