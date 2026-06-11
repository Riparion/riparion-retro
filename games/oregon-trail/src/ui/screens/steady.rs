//! Holding a hand steady under strain. A reticle wanders the field; drag to keep
//! the cursor pinned to it for the duration. How much of the run you hold on
//! target decides the toll — a steady hand costs little, a wandering one the full
//! hit (and, for the snakebite, the medicine that keeps you alive).
//!
//! This is a thin wrapper over the shared `minigames_kit::steady_hands::SteadyHands`
//! component: it reads `pending_steady` to pick the prompt and difficulty (the
//! river ford is the lone catastrophe still on the steady-hand trace), derives a
//! per-encounter seed from game state (deterministic for a given save, distinct
//! from a same-fortnight splint or dose), and routes the result into
//! `resolve_steady`.

use dioxus::prelude::*;

use minigames_kit::steady_hands::{SteadyHands, SteadyHandsResult};

use crate::engine::{Game, SteadyTask};

#[component]
pub fn Steady() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // Default to the river ford if somehow mounted without a pending task — the
    // resolve is guarded on `pending_steady`, so a stray mount just no-ops.
    let task = g.pending_steady.unwrap_or(SteadyTask::Ford);
    // A per-encounter seed (see `Game::encounter_seed`); the salt keeps the trace's
    // path distinct from a same-fortnight splint or dose.
    let seed = g.encounter_seed(0x57EA_D70F_57EA_D70F);
    drop(g);

    let (prompt, tolerance, duration_ms, drift_speed): (&str, f64, u32, f64) = match task {
        SteadyTask::Ford => (
            "Fording a river — hold the wagon level against the current.",
            0.12,
            7000,
            1.4,
        ),
    };

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
