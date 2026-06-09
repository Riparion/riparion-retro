//! Renders the head of the pending queue: this year's paced narration, tapped
//! through one line at a time like the original's press-ENTER pacing.

use dioxus::prelude::*;

use crate::engine::interaction::{Interaction, Response};
use crate::engine::state::Mode;
use crate::engine::Game;
use retro_kit::components::notice::Notice;

#[component]
pub fn InteractionHost() -> Element {
    let mut game = use_context::<Signal<Game>>();
    // Recover from an inconsistent (Interaction mode, empty queue) state *after*
    // render rather than mutating during it. This never fires in normal play.
    use_effect(move || {
        let g = game.read();
        if g.mode == Mode::Interaction && g.pending.is_empty() {
            drop(g);
            game.write().resolve(Response::Acknowledge);
        }
    });
    let Some(head) = game.read().pending.front().cloned() else {
        return rsx! {};
    };

    match head {
        Interaction::Message(text) => rsx! {
            Notice {
                message: text,
                on_ack: move |_| game.write().resolve(Response::Acknowledge),
            }
        },
    }
}
