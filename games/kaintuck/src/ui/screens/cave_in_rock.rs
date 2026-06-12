//! Cave-in-Rock on the lower Ohio — the river pirates' lair, just above Cairo.
//! A stranger hails you from the bank offering to pilot the shoals; take his
//! offer, hire your own pilot, or run the serpentine channel yourself. The
//! options come from the scenario, rendered through the generic set-piece menu.

use dioxus::prelude::*;

use crate::engine::scenario_data::scenario;
use crate::engine::Game;
use crate::ui::components::set_piece_menu::SetPieceMenu;
use retro_kit::theme::{PANEL, SCREEN_CENTERED};

#[component]
pub fn CaveInRock() -> Element {
    let mut game = use_context::<Signal<Game>>();

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "{PANEL} p-3 text-center leading-snug",
                "The bluffs close in at Cave-in-Rock, the channel narrowing past bars and rock. A man on the bank cups his hands and offers to pilot you through — but this water is the pirates' own, and many a boat that put in here was never heard from again."
            }
            SetPieceMenu {
                options: scenario().menus.caveinrock.options.as_slice(),
                onselect: move |(action, cost): (String, f64)| {
                    game.write().run_set_piece(&action, cost);
                },
            }
        }
    }
}
