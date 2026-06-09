//! Standalone demo of the [`Hunter`] minigame.
//!
//! This is its own crate. Serve it with the Dioxus CLI:
//!
//! ```sh
//! dx serve --package minigames-kit-hunter
//! # or, from the repo root: just example hunter
//! ```
//!
//! Cast as a hunt that gets harder each round: a slow duck, then a darting
//! hare, then a bounding buck — each faster across the range and with fewer
//! shells in the bag. Slide the rifle (🔫) left/right to line up the column,
//! FIRE, then RELOAD before the next shot. A fresh seed per round moves the
//! quarry's start and direction. The last result is shown so you can see what
//! `on_complete` reports back.
//!
//! Note: the icons here are emoji (great on iOS/macOS); the component's
//! *defaults* are portable roguelike glyphs (`@`, `d`) for hosts without a
//! color-emoji font.

use dioxus::prelude::*;
use minigames_kit::hunter::{Hunter, HunterResult};

/// Compiled by `dx serve` from this crate's root `tailwind.css`.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

/// (prompt, quarry icon, step ms, ammo) — the hunt gets quicker and leaner.
const SITUATIONS: [(&str, &str, u32, usize); 3] = [
    ("Bag the duck —", "🦆", 420, 6),
    ("Catch the hare —", "🐇", 300, 5),
    ("Drop the buck —", "🦌", 220, 4),
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<HunterResult>);
    // One random base seed per launch (browser entropy), so the quarry's start
    // and direction differ every run rather than replaying from round 0.
    let base_seed = use_signal(retro_kit::rng::random_seed);

    let r = round();
    let (prompt, hunted, step_ms, ammo) = SITUATIONS[(r as usize) % SITUATIONS.len()];
    // A fresh seed per round, anchored to this launch's base.
    let seed = base_seed()
        .wrapping_mul(r as u64 + 1)
        .rotate_left(r % 17);

    // On a clean kill the top line cheers; a miss just shows the next quarry.
    let prompt = match last() {
        Some(res) if res.hit => "Got it!".to_string(),
        _ => prompt.to_string(),
    };

    rsx! {
        // The shared CRT look plus this crate's own compiled Tailwind. No Play
        // CDN, so the utility classes are present from first paint (no JIT jank).
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "Hunter — minigames-kit demo" }

        div {
            class: "crt flex flex-col",
            style: "min-height: 100vh;",
            Hunter {
                key: "{r}",
                prompt,
                hunter_icon: "🔫".to_string(),
                hunted_icon: hunted.to_string(),
                step_ms,
                ammo,
                seed,
                on_complete: move |res: HunterResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }

            div {
                class: "text-center text-sm opacity-70 p-4",
                match last() {
                    Some(res) => {
                        let verdict = if res.hit { "hit" } else { "missed" };
                        rsx! { "Last: {verdict} after {res.shots_fired} shot(s)." }
                    }
                    None => rsx! { "Take your first shot." },
                }
            }
        }
    }
}
