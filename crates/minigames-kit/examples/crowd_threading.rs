//! Standalone demo of the [`CrowdThreading`] minigame.
//!
//! Run it from the crate dir with the Dioxus CLI:
//!
//! ```sh
//! cd crates/minigames-kit
//! dx serve --example crowd_threading
//! ```
//!
//! Cast as a ship slipping a pirate fleet: your sloop (⛵) must thread the
//! gaps between the raiders' colours (🏴‍☠️) to reach safe harbour (🏝️). It
//! cycles through a thickening threat — a patrol, a blockade, a whole fleet —
//! each on its own grid with its own route to memorize, and feeds a fresh seed
//! per round so the route moves and can't be memorized. The reveal time
//! stretches with the threat so a longer route still gets a fair look. The last
//! result is shown so you can see what `on_complete` reports back.
//!
//! Note: the icons here are emoji (great on iOS/macOS); the component's
//! *defaults* are portable roguelike glyphs (`@`, `&`, `>`) for hosts without a
//! color-emoji font.

use dioxus::prelude::*;
use minigames_kit::crowd_threading::{CrowdThreading, CrowdThreadingResult};

fn main() {
    dioxus::launch(App);
}

/// (prompt, crowd size, reveal ms, navigate ms) — the pirate threat thickens.
/// The navigate timer scales with the route so a longer escape stays fair.
const SITUATIONS: [(&str, usize, u32, u32); 3] = [
    ("Slip the pirate patrol —", 4, 2200, 8000),
    ("Run the blockade —", 12, 2800, 11000),
    ("Escape the pirate fleet —", 28, 3600, 14000),
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<CrowdThreadingResult>);
    // One random base seed per launch (browser entropy), so the layouts differ
    // every run rather than replaying the same sequence from round 0.
    let base_seed = use_signal(retro_kit::rng::random_seed);

    let r = round();
    let (prompt, crowd_size, reveal_ms, navigate_ms) = SITUATIONS[(r as usize) % SITUATIONS.len()];
    // A fresh seed per round, anchored to this launch's base → the route lands
    // somewhere new each round, and the whole run differs from the last.
    let seed = base_seed()
        .wrapping_mul(r as u64 + 1)
        .rotate_left(r % 17);

    // On a clean clear, the top line becomes the news; a boarding just shows
    // the next situation plainly.
    let prompt = match last() {
        Some(res) if res.cleared => "Success!".to_string(),
        _ => prompt.to_string(),
    };

    rsx! {
        // The shared CRT look. Tailwind's Play CDN supplies the utility classes;
        // the grid itself is styled inline, so it renders without a build step.
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Script { src: "https://cdn.tailwindcss.com" }
        document::Title { "CrowdThreading — minigames-kit demo" }

        div { class: "crt flex flex-col", style: "min-height: 100vh;",
            CrowdThreading {
                key: "{r}",
                prompt,
                crowd_size,
                member_icon: "🏴‍☠️".to_string(),
                player_icon: "⛵".to_string(),
                exit_icon: "🏝️".to_string(),
                reveal_ms,
                navigate_ms,
                seed,
                on_complete: move |res: CrowdThreadingResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }
        }
    }
}
