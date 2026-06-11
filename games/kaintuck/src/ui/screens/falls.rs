//! The Falls of the Ohio at Louisville — the river's worst obstacle. Hire a
//! pilot, run them yourself, or wait for high water.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::theme::{PANEL, SCREEN_CENTERED};

#[component]
pub fn Falls() -> Element {
    let mut game = use_context::<Signal<Game>>();

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "{PANEL} p-3 text-center leading-snug",
                "Two miles of rapids drop away below the landing. In low water a flatboat can't pass unguided — many a Kaintuck has wrecked here."
            }
            MenuButton {
                title: "Hire a falls pilot — $8".to_string(),
                hint: "A local takes the oar and threads you down. A sure thing.".to_string(),
                onclick: move |_| game.write().falls_pilot(),
            }
            MenuButton {
                title: "Run the falls yourself".to_string(),
                hint: "Free — but the current is violent. Hold your line.".to_string(),
                primary: true,
                onclick: move |_| game.write().falls_run(),
            }
            MenuButton {
                title: "Wait for high water".to_string(),
                hint: "Safe passage, but it costs days and frays the crew.".to_string(),
                onclick: move |_| game.write().falls_wait(),
            }
        }
    }
}
