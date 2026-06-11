//! Beating back a spreading threat. Threats flare on a small grid and spread if
//! ignored; tap them down faster than they multiply. How much was still spreading
//! when the clock ran out decides the toll — contain it and you pay the floor, let
//! it run and the load is lost.
//!
//! This is a thin wrapper over the shared `minigames_kit::bucket_brigade::BucketBrigade`
//! component, shared across three catastrophes: it reads `pending_brigade` to pick
//! the prompt and per-task difficulty, derives a per-encounter seed from game state
//! (deterministic for a given save, distinct from a same-fortnight splint or dose),
//! normalizes the leaked-cell count into a 0..1 severity, and routes the result
//! into `resolve_brigade`.

use dioxus::prelude::*;

use minigames_kit::bucket_brigade::{BucketBrigade, BucketBrigadeResult};

use crate::engine::{BrigadeTask, Game};

#[component]
pub fn Brigade() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // Default to the wagon fire if somehow mounted without a pending task — the
    // resolve is guarded on `pending_brigade`, so a stray mount just no-ops.
    let task = g.pending_brigade.unwrap_or(BrigadeTask::Fire);
    // A per-encounter layout seed — varies with progress but stays deterministic
    // for a given save and doesn't perturb the game's RNG stream. The salt keeps it
    // distinct from a same-fortnight splint, dose, or steady trace.
    let seed = g.encounter_seed(0xB0CC_E7B0_CCE7_B0CC);
    drop(g);

    // Per-task framing and difficulty; the grid stays at the component's default
    // 5×5 and the loss grading (leaked/capacity) is done in `resolve_brigade`.
    let (prompt, threat_icon, initial_active, spread_ms, spread_chance, duration_ms): (
        &str,
        &str,
        usize,
        u32,
        f64,
        u32,
    ) = match task {
        BrigadeTask::Fire => (
            "Fire in camp — stamp out the flames before they reach the supplies!",
            "🔥",
            3,
            650,
            0.45,
            8000,
        ),
        BrigadeTask::Rains => (
            "Freezing sleet — cover the packs before it soaks the load!",
            "💧",
            3,
            800,
            0.40,
            8000,
        ),
        BrigadeTask::Blizzard => (
            "Blizzard in the pass — keep the fire fed against the wind!",
            "❄️",
            3,
            700,
            0.45,
            8500,
        ),
    };

    rsx! {
        BucketBrigade {
            key: "{seed}",
            prompt: prompt.to_string(),
            threat_icon: threat_icon.to_string(),
            initial_active,
            spread_ms,
            spread_chance,
            duration_ms,
            seed,
            on_complete: move |res: BucketBrigadeResult| {
                // The engine grades the loss from leaked/capacity; capacity comes
                // back in the result so the grid size lives in one place.
                game.write().resolve_brigade(res.contained, res.leaked, res.capacity);
            },
        }
    }
}
