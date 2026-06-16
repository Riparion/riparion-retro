//! The hull-repair drawer, raised from the HULL chip when the boat's hurt at a
//! landing. A boatwright (boatyard towns) mends her whole for cash; patching her
//! yourself is free anywhere but costs days and risks a mishap. Both engine calls
//! guard on `Mode::Town`, so this only ever fires from a town hub.

use dioxus::prelude::*;

use crate::engine::state::fmt_money;
use crate::engine::Game;
use retro_kit::components::sheet::Sheet;
use retro_kit::theme::BTN;

#[component]
pub fn RepairSheet(open: Signal<bool>) -> Element {
    let mut game = use_context::<Signal<Game>>();
    let mut open = open;
    let g = game.read();
    let has_boatyard = g.is_boatyard();
    let repair_cost = fmt_money(g.repair_cost());
    let hull = g.state.hull_label();
    drop(g);

    rsx! {
        Sheet {
            open: open(),
            on_close: move |_| open.set(false),
            title: format!("── HULL: {hull} ──"),
            div { class: "flex flex-col gap-2",
                if has_boatyard {
                    button {
                        class: "{BTN}",
                        onclick: move |_| {
                            open.set(false);
                            game.write().repair_in_port();
                        },
                        "BOATWRIGHT — REPAIR ({repair_cost})"
                    }
                }
                button {
                    class: "{BTN}",
                    onclick: move |_| {
                        open.set(false);
                        game.write().self_repair();
                    },
                    "PATCH HER YOURSELF"
                }
                p { class: "text-xs opacity-60",
                    "Mending her yourself costs days and risks a mishap while you lie up — but not a dollar."
                }
            }
        }
    }
}
