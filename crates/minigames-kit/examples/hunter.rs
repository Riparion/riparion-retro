//! Standalone demo of the [`Hunter`] minigame.
//!
//! Run it from the crate dir with the Dioxus CLI:
//!
//! ```sh
//! cd crates/minigames-kit
//! dx serve --example hunter
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
        // The shared CRT look. Tailwind's Play CDN supplies the utility classes;
        // the range itself is styled inline, so it renders without a build step.
        document::Stylesheet { href: retro_kit::CRT_CSS }
        // DIAGNOSTIC: Tailwind Play CDN disabled. If the first-fire layout shift
        // is gone, the CDN's lazy JIT (injecting utility CSS on the first
        // out-of-grid DOM patch) was the cause and we make the kit fully
        // inline-styled. If it still shifts, the CDN is ruled out.
        // document::Script { src: "https://cdn.tailwindcss.com" }
        document::Title { "Hunter — minigames-kit demo" }

        // display:flex inline, not just via the `flex flex-col` classes: the
        // Tailwind Play CDN JIT-generates utilities from the DOM, so on a fresh
        // load those classes aren't ready and the page stacks as block flow —
        // then snaps to a centered column once the first interaction makes the
        // CDN catch up. Inlining keeps the layout put from the first paint.
        div {
            class: "crt flex flex-col",
            style: "display: flex; flex-direction: column; min-height: 100vh;",
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

            // Padding/size inlined too — same Play CDN reason: without it the
            // footer is unpadded on first paint and gains its `p-4` only once the
            // CDN catches up, resizing this box and nudging the centered game.
            div {
                class: "text-center text-sm opacity-70 p-4",
                style: "text-align: center; font-size: 0.875rem; opacity: 0.7; padding: 1rem;",
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
