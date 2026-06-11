//! Patching a snagged hull in the right order. A thin wrapper over
//! `minigames_kit::sequence`.

use dioxus::prelude::*;

use minigames_kit::sequence::{Sequence as SequenceGame, SequenceResult};

use crate::engine::Game;

#[component]
pub fn Sequence() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let seed = game.read().encounter_seed(0x05E9_E0CE);

    rsx! {
        SequenceGame {
            prompt: "A snag punches a hole in the hull — patch it in order before she fills.".to_string(),
            symbols: vec!["🪵".to_string(), "🧵".to_string(), "🔨".to_string(), "🪣".to_string()],
            length: 4,
            seed,
            on_complete: move |res: SequenceResult| {
                game.write().resolve_sequence(res.correct_prefix, res.length, res.perfect);
            },
        }
    }
}
