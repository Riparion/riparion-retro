//! The marksmanship reaction game. A word flashes; tap the matching one fast.
//! The tap's reaction time becomes the original's `B1` shooting score.
//!
//! This is a thin wrapper over the shared `minigames_kit::quickdraw::QuickDraw`
//! component: it picks the prompt for the situation, derives a shuffle seed from
//! game state (so the buttons rearrange each encounter and can't be memorized,
//! while staying deterministic for a given save), and routes the result into
//! `resolve_shot`.

use dioxus::prelude::*;

use minigames_kit::quickdraw::{QuickDraw, QuickDrawResult};

use crate::engine::interaction::{ShotPurpose, SHOT_WORDS};
use crate::engine::Game;

fn prompt_for(purpose: Option<ShotPurpose>) -> &'static str {
    match purpose {
        Some(ShotPurpose::Bandits) => "Bandits! Draw —",
        Some(ShotPurpose::WildAnimals) => "Wolves! Fire —",
        Some(ShotPurpose::Riders { .. }) => "Riders close in! Fire —",
        None => "Fire —",
    }
}

#[component]
pub fn Shoot() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let target = g.shot_word.min(SHOT_WORDS.len() - 1);
    let purpose = g.shot;
    // A per-encounter seed pulled from existing state — varies with the target
    // and how far the party has traveled, so layouts differ between shots
    // without touching (and so without perturbing) the game's RNG stream.
    let seed = (g.shot_word as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ g.state.miles.to_bits();
    drop(g);

    let words: Vec<String> = SHOT_WORDS.iter().map(|w| w.to_string()).collect();

    rsx! {
        QuickDraw {
            prompt: prompt_for(purpose).to_string(),
            words,
            target,
            seed,
            on_fire: move |res: QuickDrawResult| {
                game.write().resolve_shot(res.seconds, res.hit);
            },
        }
    }
}
