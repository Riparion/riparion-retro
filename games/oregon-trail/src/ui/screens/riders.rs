//! Riders ahead — choose your tactics. They may not be what they appear.

use dioxus::prelude::*;

use crate::engine::interaction::Tactic;
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::theme::SCREEN_CENTERED;

const TACTICS: [(Tactic, &str, &str); 4] = [
    (Tactic::Run, "Run for it", "thread the breaks to shake them"),
    (Tactic::Attack, "Attack", "draw and fight"),
    (Tactic::Continue, "Keep going", "hold your course and hope"),
    (Tactic::CircleWagons, "Circle the wagons", "dig in and defend"),
];

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
                "Riders ahead! "
                if looks_hostile {
                    span { class: "chip-danger", "They look hostile." }
                } else {
                    span { "They look friendly." }
                }
            }
            h2 { class: "text-center", "What are your tactics?" }
            for (tactic, label, hint) in TACTICS {
                MenuButton {
                    key: "{label}",
                    title: label.to_string(),
                    hint: hint.to_string(),
                    onclick: move |_| game.write().resolve_tactic(tactic),
                }
            }
        }
    }
}
