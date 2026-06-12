//! A war party ahead — choose your tactics. They may not be what they appear.
//! The four tactics come from the scenario's `riders` menu, rendered through the
//! generic `SetPieceMenu`.

use dioxus::prelude::*;

use crate::engine::interaction::Tactic;
use crate::engine::scenario_data::scenario;
use crate::engine::Game;
use crate::ui::components::set_piece_menu::SetPieceMenu;
use retro_kit::theme::SCREEN_CENTERED;

#[component]
pub fn Riders() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let looks_hostile = game
        .read()
        .riders
        .map(|r| r.looks_hostile)
        .unwrap_or(true);

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            div { class: "crt-panel p-4 text-center text-lg leading-snug",
                "Riders on the trail ahead! "
                if looks_hostile {
                    span { class: "chip-danger", "They look hostile." }
                } else {
                    span { "They look friendly." }
                }
            }
            h2 { class: "text-center", "What are your tactics?" }
            SetPieceMenu {
                options: scenario().menus.riders.options.as_slice(),
                onselect: move |action: String| {
                    let tactic = match action.as_str() {
                        "run" => Tactic::Run,
                        "attack" => Tactic::Attack,
                        "keep-going" => Tactic::Continue,
                        "fort-up" => Tactic::CircleWagons,
                        _ => return,
                    };
                    game.write().resolve_tactic(tactic);
                },
            }
        }
    }
}
