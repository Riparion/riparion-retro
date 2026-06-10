//! Setting the daughter's broken arm. A marker sweeps the track; tap as it
//! crosses the zone to set the bone in one clean strike. A dead-center strike
//! barely costs time; a poor one loses ground and burns through supplies.
//!
//! This is a thin wrapper over the shared `minigames_kit::timing_bar::TimingBar`
//! component: the sweep is the moment to set the bone. It derives a per-encounter
//! seed from game state (so the zone moves each time while staying deterministic
//! for a given save) and routes the result into `resolve_splint`.

use dioxus::prelude::*;

use minigames_kit::timing_bar::{TimingBar, TimingResult};

use crate::engine::Game;

#[component]
pub fn Splint() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // A per-encounter seed (see `Game::encounter_seed`); the salt keeps the splint's
    // zone distinct from a same-fortnight dose.
    let seed = g.encounter_seed(0x5B0E_5E77_B04E_5E77);
    drop(g);

    rsx! {
        TimingBar {
            prompt: "Your daughter breaks her arm — set the bone clean!".to_string(),
            action: "SET".to_string(),
            tolerance: 0.10,
            period_ms: 1500,
            seed,
            on_strike: move |res: TimingResult| {
                game.write().resolve_splint(res.hit, res.accuracy);
            },
        }
    }
}
