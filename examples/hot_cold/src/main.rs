//! Standalone demo of the [`HotCold`] minigame.
//!
//! This is its own crate. Serve it with the Dioxus CLI:
//!
//! ```sh
//! dx serve --package minigames-kit-hot-cold
//! # or, from the repo root: just example hot_cold
//! ```
//!
//! It cycles through a few "search the brush for it" situations — an ox that
//! wandered off, a lost son, an unsafe spring among many — each on a larger grid
//! with a tighter probe budget, with a fresh seed per round so the hidden spot
//! moves. The first two use warmer/colder clues; the last reads absolute distance
//! rings. The last result (found/not, probes used, par) is shown so you can see
//! what `on_complete` reports back.
//!
//! It's touch-first: tap a cell to probe it and read the clue. Drive it under a
//! touch-emulated viewport to test for real; a fresh page load (not RSX hot-reload)
//! exercises the actual wasm.

use dioxus::prelude::*;
use minigames_kit::hot_cold::{Feedback, HotCold, HotColdResult};

/// Compiled by `dx serve` from this crate's root `tailwind.css`.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

/// (prompt, cols, rows, feedback, max probes) — increasingly demanding.
const SITUATIONS: [(&str, usize, usize, Feedback, usize); 3] = [
    (
        "An ox wandered off — search the brush before it strays far.",
        5,
        5,
        Feedback::WarmerColder,
        8,
    ),
    (
        "Your son is lost — hunt for him, warmer to colder.",
        6,
        6,
        Feedback::WarmerColder,
        9,
    ),
    (
        "Unsafe water — probe the springs to find the clean one.",
        7,
        7,
        Feedback::DistanceRing,
        7,
    ),
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<HotColdResult>);

    let r = round();
    let (prompt, cols, rows, feedback, max_probes) = SITUATIONS[(r as usize) % SITUATIONS.len()];
    // A fresh, deterministic seed per round → the hidden spot lands somewhere new.
    let seed = 0xC01D_BA17_u64.wrapping_mul(r as u64 + 1).rotate_left(r % 17);

    rsx! {
        // The shared CRT look plus this crate's own compiled Tailwind. No Play CDN,
        // so the utility classes are present from first paint (no JIT jank).
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "HotCold — minigames-kit demo" }

        div {
            class: "crt flex flex-col",
            style: "min-height: 100vh; display: flex; flex-direction: column;",
            HotCold {
                prompt: prompt.to_string(),
                cols,
                rows,
                feedback,
                max_probes,
                seed,
                on_complete: move |res: HotColdResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }
            div {
                class: "text-center text-sm opacity-70 p-4",
                style: "text-align: center; font-size: 0.875rem; opacity: 0.7; padding: 1rem;",
                match last() {
                    Some(res) if res.found => rsx! {
                        "Found it in {res.probes_used} probes (par {res.par}) — round {r}"
                    },
                    Some(res) => rsx! {
                        "Lost the trail — {res.probes_used} probes spent, none found (par {res.par}) — round {r}"
                    },
                    None => rsx! { "Tap a cell to probe. Closer is warmer." },
                }
            }
        }
    }
}
