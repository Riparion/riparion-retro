//! Bailing a flooded boat. A thin wrapper over `minigames_kit::bucket_brigade`.

use dioxus::prelude::*;

use minigames_kit::bucket_brigade::{BucketBrigade, BucketBrigadeResult};

use crate::engine::Game;

#[component]
pub fn Brigade() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let seed = game.read().encounter_seed(0x6A11_0000);

    rsx! {
        BucketBrigade {
            prompt: "High water floods the boat — bail her out faster than she fills!".to_string(),
            threat_icon: "💧".to_string(),
            cols: 5,
            rows: 5,
            initial_active: 3,
            spread_ms: 750,
            duration_ms: 8000,
            seed,
            on_complete: move |res: BucketBrigadeResult| {
                game.write().resolve_brigade(res.contained, res.leaked, res.capacity);
            },
        }
    }
}
