//! Enemy fleet display for sea battles: a grid of lorchas with hull bars.

use dioxus::prelude::*;

use retro_kit::theme::PANEL;

use crate::engine::combat::Battle;

#[component]
pub fn ShipDisplay(battle: ReadSignal<Battle>) -> Element {
    let b = battle.read();
    let off_screen = b.ships - b.on_screen.len() as i64;

    rsx! {
        div { class: "{PANEL} mx-3 p-2",
            div { class: "grid grid-cols-5 gap-2",
                for (i, ship) in b.on_screen.iter().enumerate() {
                    div { key: "{i}", class: "enemy-ship text-center",
                        div { class: "text-xl", "⛵" }
                        div { class: "hull-bar",
                            div {
                                class: "hull-fill",
                                style: format!(
                                    "width: {}%",
                                    (100.0 * (1.0 - ship.damage / ship.health).max(0.0)) as i64
                                ),
                            }
                        }
                    }
                }
            }
            if off_screen > 0 {
                p { class: "text-xs opacity-70 mt-1 text-center", "+{off_screen} more on the horizon" }
            }
            if b.on_screen.is_empty() {
                p { class: "text-center py-2", "The seas are clearing..." }
            }
        }
    }
}
