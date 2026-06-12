//! Grand Tower on the Mississippi — the rivermen's initiation. A boat passing
//! the rock for the first time stood a treat or was ducked in the river. The
//! options come from the scenario, rendered through the generic set-piece menu.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::Game;
use crate::ui::components::set_piece_menu::SetPieceMenu;
use retro_kit::theme::{PANEL, SCREEN_CENTERED};

#[component]
pub fn GrandTower() -> Element {
    let mut game = use_context::<Signal<Game>>();

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "{PANEL} p-3 text-center leading-snug",
                "The Grand Tower stands in the river ahead. Like sailors at the equator, rivermen make the first-timer pay his footing — stand a treat, or be shaved and ducked over the side."
            }
            SetPieceMenu {
                options: scenario().menus.grandtower.options.as_slice(),
                onselect: move |(action, cost): (String, f64)| {
                    game.write().run_set_piece(&action, cost);
                },
            }
        }
    }
}
