//! Reproducing an ordered procedure from memory. A short run of steps flashes
//! one at a time; tap them back in the same order. How much of the order you get
//! right before the first slip decides the toll — a clean run barely costs you,
//! a botched one loses ground and supplies (and, for the frostbite, the medicine
//! that keeps you alive).
//!
//! This is a thin wrapper over the shared `minigames_kit::sequence::Sequence`
//! component, shared across three catastrophes: it reads the pending task to
//! pick the prompt, the themed step palette, and the per-task difficulty, derives
//! a per-encounter seed from game state (deterministic for a given save, distinct
//! from a same-week splint, dose, steady trace, or brigade), and routes the
//! result into `resolve_sequence`.

use dioxus::prelude::*;

use minigames_kit::sequence::{Sequence as SequenceGame, SequenceResult};

use crate::engine::{Game, SequenceTask};

#[component]
pub fn Sequence() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // Default to the wheel if somehow mounted without a pending task — the resolve
    // is guarded on the pending task, so a stray mount just no-ops.
    let task = g.sequence_task().unwrap_or(SequenceTask::Wheel);
    // A per-encounter seed (see `Game::encounter_seed`); the salt keeps the order
    // distinct from a same-week splint, dose, steady trace, or brigade.
    let seed = g.encounter_seed(0x5E9E_9CE0_5E9E_9CE0);
    drop(g);

    // Per-task framing, the step palette (words read as a procedure), how long the
    // sequence runs, and how fast it flashes. The deadliest one (frostbite) is the
    // hardest — a longer, faster run — so a botched order really can cost you.
    let (prompt, steps, length, show_ms): (&str, &[&str], usize, u32) = match task {
        SequenceTask::Wheel => (
            "A cart wheel breaks down — re-hang it in order.",
            &["JACK", "BLOCK", "BOLT", "SEAT"],
            4,
            750,
        ),
        SequenceTask::OxLeg => (
            "A pack animal hurt its leg — dress it in sequence.",
            &["WRAP", "PAD", "BIND", "SPLINT"],
            4,
            700,
        ),
        SequenceTask::Frostbite => (
            "Frostbite sets in! Work the first-aid steps in order.",
            &["WARM", "DRY", "WRAP", "PAD", "BIND"],
            5,
            600,
        ),
    };
    let symbols: Vec<String> = steps.iter().map(|s| s.to_string()).collect();

    rsx! {
        SequenceGame {
            key: "{seed}",
            prompt: prompt.to_string(),
            symbols,
            length,
            show_ms,
            seed,
            on_complete: move |res: SequenceResult| {
                game.write().resolve_sequence(res.correct_prefix, res.length, res.perfect);
            },
        }
    }
}
