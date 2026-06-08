//! Renders the head of the pending-message queue: the trail's narration,
//! paced one tap at a time (the original's keypress pacing).

use dioxus::prelude::*;

use crate::engine::interaction::{Interaction, Response};
use crate::engine::Game;
use retro_kit::theme::{BTN_PRIMARY, PANEL, SCREEN_CENTERED};

#[component]
pub fn InteractionHost() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let Some(head) = game.read().pending.front().cloned() else {
        // Defensive: an empty queue in Interaction mode just moves along.
        game.write().resolve(Response::Acknowledge);
        return rsx! {};
    };

    match head {
        Interaction::Message(text) => rsx! {
            div { class: "{SCREEN_CENTERED}",
                div { class: "{PANEL} p-4 text-center text-lg leading-snug", "{text}" }
                button {
                    class: "{BTN_PRIMARY} mt-4 py-3",
                    onclick: move |_| game.write().resolve(Response::Acknowledge),
                    "▸ ▸ ▸"
                }
            }
        },
    }
}
