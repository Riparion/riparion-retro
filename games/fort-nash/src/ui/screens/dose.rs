//! Measuring out medicine for an illness. A marker sweeps the track; tap as it
//! crosses the zone to pour an exact dose. The illness's own toll lands either
//! way — but a shaky pour spills extra medical supplies, which can be what tips
//! a thin party into pneumonia.
//!
//! This is a thin wrapper over the shared `minigames_kit::timing_bar::TimingBar`
//! component: the sweep is the moment to stop pouring. The prompt names the
//! severity the engine already rolled (held in `pending_illness`). It derives a
//! per-encounter seed from game state (so the zone moves each time while staying
//! deterministic for a given save) and routes the result into `resolve_dose`.

use dioxus::prelude::*;

use minigames_kit::timing_bar::{TimingBar, TimingResult};

use crate::engine::{Game, Illness};

fn prompt_for(severity: Option<Illness>) -> &'static str {
    match severity {
        Some(Illness::Mild) => "A mild illness — measure the dose!",
        Some(Illness::Bad) => "A bad illness — measure the dose carefully!",
        Some(Illness::Serious) => "A serious illness — every drop counts!",
        None => "Measure the dose!",
    }
}

#[component]
pub fn Dose() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let prompt = prompt_for(g.pending_illness);
    // A serious illness gives a tighter zone — the dose has to be exact.
    let tolerance = if g.pending_illness == Some(Illness::Serious) {
        0.08
    } else {
        0.11
    };
    // A per-encounter seed (see `Game::encounter_seed`); the salt keeps the dose's
    // zone distinct from a same-fortnight splint.
    let seed = g.encounter_seed(0xD05E_D05E_D05E_D05E);
    drop(g);

    rsx! {
        TimingBar {
            prompt: prompt.to_string(),
            action: "POUR".to_string(),
            tolerance,
            period_ms: 1400,
            seed,
            on_strike: move |res: TimingResult| {
                game.write().resolve_dose(res.hit, res.accuracy);
            },
        }
    }
}
