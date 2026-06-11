//! Quick-draw against river pirates and the Trace bandits. A thin wrapper over
//! `minigames_kit::quickdraw`.

use dioxus::prelude::*;

use minigames_kit::quickdraw::{QuickDraw, QuickDrawResult};

use crate::engine::tasks::QuickTask;
use crate::engine::Game;

const WORDS: [&str; 4] = ["FIRE", "DRAW", "BANG", "CRACK"];

#[component]
pub fn Quick() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let task = g.pending_quick.unwrap_or(QuickTask::Pirates);
    let seed = g.encounter_seed(0x0BAD_F00D);
    drop(g);

    let prompt = match task {
        QuickTask::Pirates => "River pirates pull alongside — draw!",
        QuickTask::Mason => "Sam Mason's men step out of the brush — draw!",
        QuickTask::Harpe => "The Harpe brothers are on you — DRAW, fast!",
    };
    // The target word the player must tap; vary it per encounter.
    let target = (seed % WORDS.len() as u64) as usize;
    let words = WORDS.iter().map(|w| w.to_string()).collect::<Vec<_>>();

    rsx! {
        QuickDraw {
            prompt: prompt.to_string(),
            words,
            target,
            seed,
            on_fire: move |res: QuickDrawResult| {
                game.write().resolve_quick(res.seconds, res.hit);
            },
        }
    }
}
