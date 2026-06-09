//! Outrunning hostile riders. Memorize a line through the rocky breaks, then
//! reproduce it from memory once the country closes up — thread it clean and you
//! lose them; foul it and they run you down into a gunfight.
//!
//! This is a thin wrapper over the shared `minigames_kit::crowd_threading::
//! CrowdThreading` component: the "crowd" is the terrain (rocks, scrub, gullies)
//! you weave between. It derives a per-encounter seed from game state (so the
//! ground varies each escape while staying deterministic for a given save) and
//! routes the result into `resolve_flee`.

use dioxus::prelude::*;

use minigames_kit::crowd_threading::{CrowdThreading, CrowdThreadingResult};

use crate::engine::Game;

#[component]
pub fn Flee() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // A per-encounter seed pulled from existing state — varies with the fortnight
    // and how far the party has traveled, so the ground differs between escapes
    // without touching (and so without perturbing) the game's RNG stream.
    let seed = (g.state.turn as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ g.state.miles.to_bits();
    drop(g);

    rsx! {
        CrowdThreading {
            prompt: "Ride hard — thread the breaks and lose them!".to_string(),
            crowd_size: 10,
            member_icon: "🌲".to_string(),
            player_icon: "🐎".to_string(),
            exit_icon: "🌄".to_string(),
            seed,
            on_complete: move |res: CrowdThreadingResult| {
                game.write().resolve_flee(res.cleared, res.accuracy);
            },
        }
    }
}
