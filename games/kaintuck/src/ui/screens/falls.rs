//! The Falls of the Ohio at Louisville — the river's worst obstacle. Hire a
//! pilot, run them yourself, or wait for high water. The options come from the
//! scenario, rendered through the generic set-piece menu.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::Game;
use crate::ui::components::set_piece_menu::SetPieceMenu;
use retro_kit::theme::{PANEL, SCREEN_CENTERED};

#[component]
pub fn Falls() -> Element {
    let mut game = use_context::<Signal<Game>>();

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "{PANEL} p-3 text-center leading-snug",
                "Two miles of rapids drop away below the landing. In low water a flatboat can't pass unguided — many a Kaintuck has wrecked here."
            }
            SetPieceMenu {
                options: scenario().menus.falls.options.as_slice(),
                onselect: move |(action, cost): (String, f64)| {
                    game.write().run_set_piece(&action, cost);
                },
            }
        }
    }
}
