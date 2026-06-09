//! Groping through heavy fog. Fix the trail in your mind during the glimpse,
//! then hold it from memory once the fog closes — keep your bearings and you
//! lose no time; drift off it and you wander before finding the trail again.
//!
//! This is a thin wrapper over the shared `minigames_kit::crowd_threading::
//! CrowdThreading` component, and the cleanest fit for it: the map "closing" is
//! the fog itself rolling in. The "crowd" is the fog banks you thread between.
//! It derives a per-encounter seed from game state (so the trail varies each
//! time while staying deterministic for a given save) and routes the result
//! into `resolve_fog`.

use dioxus::prelude::*;

use minigames_kit::crowd_threading::{CrowdThreading, CrowdThreadingResult};

use crate::engine::Game;

#[component]
pub fn Fog() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // A per-encounter seed pulled from existing state — varies with the fortnight
    // and mileage so the trail differs between fogs without touching (and so
    // without perturbing) the game's RNG stream. The salt keeps a fog's layout
    // distinct from a same-fortnight flee or climb, which share the base formula.
    let seed = (g.state.turn as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ g.state.miles.to_bits()
        ^ 0xF06F_06F0_F06F_06F0;
    drop(g);

    rsx! {
        CrowdThreading {
            prompt: "Fog rolls in — fix the trail before it closes!".to_string(),
            crowd_size: 10,
            member_icon: "🌫️".to_string(),
            player_icon: "🐂".to_string(),
            exit_icon: "🚩".to_string(),
            seed,
            on_complete: move |res: CrowdThreadingResult| {
                game.write().resolve_fog(res.cleared, res.accuracy);
            },
        }
    }
}
