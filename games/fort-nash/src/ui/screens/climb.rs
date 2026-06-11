//! Crossing the rugged mountains. Memorize a line through the broken rock, then
//! walk it from memory once the country closes up — hold the line and you barely
//! lose a step; foul it and you backtrack through the rocks, losing ground.
//!
//! This is a thin wrapper over the shared `minigames_kit::crowd_threading::
//! CrowdThreading` component: the "crowd" is the boulders and scree you pick a
//! line between. It derives a per-encounter seed from game state (so the ground
//! varies each crossing while staying deterministic for a given save) and routes
//! the result into `resolve_climb`.

use dioxus::prelude::*;

use minigames_kit::crowd_threading::{CrowdThreading, CrowdThreadingResult};

use crate::engine::Game;

#[component]
pub fn Climb() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // A per-encounter seed (see `Game::encounter_seed`); the salt keeps a climb's
    // layout distinct from a same-fortnight flee.
    let seed = g.encounter_seed(0x5C1F_F00D_C0DE_5A17);
    drop(g);

    rsx! {
        CrowdThreading {
            prompt: "Rugged ridges — pick a clean line over the rocks!".to_string(),
            crowd_size: 12,
            member_icon: "🪨".to_string(),
            player_icon: "🐂".to_string(),
            exit_icon: "🏔️".to_string(),
            seed,
            on_complete: move |res: CrowdThreadingResult| {
                game.write().resolve_climb(res.cleared, res.accuracy);
            },
        }
    }
}
