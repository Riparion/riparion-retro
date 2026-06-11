//! Holding a hand steady under strain. A reticle wanders the field; drag to keep
//! the cursor pinned to it for the duration. How much of the run you hold on
//! target decides the toll — a steady line costs little, a wandering one the full
//! hit, and a badly shaky crossing breaks the ice under the party.
//!
//! This is a thin wrapper over the shared `minigames_kit::steady_hands::SteadyHands`
//! component: the frozen-Cumberland ice crossing is the lone catastrophe on the
//! steady-hand trace, so the prompt and difficulty are fixed; it derives a
//! per-encounter seed from game state (deterministic for a
//! given save, distinct from a same-week splint or dose), and routes the result
//! into `resolve_steady`.

use dioxus::prelude::*;

use minigames_kit::steady_hands::{SteadyHands, SteadyHandsResult};

use crate::engine::Game;

#[component]
pub fn Steady() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // The frozen-Cumberland ice crossing is the lone steady-hand catastrophe; the
    // resolve is guarded on the pending task, so a stray mount just no-ops.
    // A per-encounter seed (see `Game::encounter_seed`); the salt keeps the trace's
    // path distinct from a same-week splint or dose.
    let seed = g.encounter_seed(0x57EA_D70F_57EA_D70F);
    drop(g);

    let (prompt, tolerance, duration_ms, drift_speed): (&str, f64, u32, f64) = (
        "Christmas Day — drive the livestock over the frozen Cumberland. Hold a steady line; a slip cracks the ice.",
        0.12,
        7000,
        1.4,
    );

    rsx! {
        SteadyHands {
            prompt: prompt.to_string(),
            tolerance,
            duration_ms,
            drift_speed,
            seed,
            on_complete: move |res: SteadyHandsResult| {
                game.write().resolve_steady(res.steady, res.accuracy);
            },
        }
    }
}
