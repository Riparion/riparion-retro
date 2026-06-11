//! Keeping to the Trace through a tangle of side trails. A thin wrapper over
//! `minigames_kit::crowd_threading` — memorize the path, then walk it blind.

use dioxus::prelude::*;

use minigames_kit::crowd_threading::{CrowdThreading, CrowdThreadingResult};

use crate::engine::Game;

#[component]
pub fn Crowd() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let seed = game.read().encounter_seed(0x51DE_7411);

    rsx! {
        CrowdThreading {
            prompt: "The Trace forks into a dozen side trails. Fix the way north in your mind.".to_string(),
            crowd_size: 11,
            member_icon: "🌲".to_string(),
            player_icon: "@".to_string(),
            exit_icon: ">".to_string(),
            reveal_ms: 2800,
            navigate_ms: 11_000,
            seed,
            on_complete: move |res: CrowdThreadingResult| {
                game.write().resolve_crowd(res.cleared, res.accuracy);
            },
        }
    }
}
