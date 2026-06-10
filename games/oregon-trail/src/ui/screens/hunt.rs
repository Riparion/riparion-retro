//! The hunting gallery. A deer sweeps the top of the range; track it with the
//! D-pad, reload between rounds, and fire to bag it.
//!
//! This is a thin wrapper over the shared `minigames_kit::hunter::Hunter`
//! component: it derives a per-encounter seed from game state (so the setup
//! varies each hunt while staying deterministic for a given save), maps the
//! party's marksmanship self-rating to how many rounds they bring, and routes
//! the result into `resolve_hunt`.

use dioxus::prelude::*;

use minigames_kit::hunter::{Hunter, HunterResult};

use crate::engine::Game;

#[component]
pub fn Hunt() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    // Better self-rating → fewer rounds in the bag (the original's self-handicap:
    // claim to be a crack shot, get less margin for error).
    let ammo = 7usize
        .saturating_sub(g.state.marksman.clamp(1, 5) as usize)
        .max(2);
    // A per-encounter seed (see `Game::encounter_seed`); no salt — the gallery
    // hunt is chosen at the hub, never alongside another minigame.
    let seed = g.encounter_seed(0);
    drop(g);

    rsx! {
        Hunter {
            prompt: "A deer! Track it and fire —".to_string(),
            hunter_icon: "🤠".to_string(),
            hunted_icon: "🦌".to_string(),
            ammo,
            seed,
            on_complete: move |res: HunterResult| {
                game.write().resolve_hunt(res.hit, res.shots_fired);
            },
        }
    }
}
