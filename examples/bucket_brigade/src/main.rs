//! Standalone demo of the [`BucketBrigade`] minigame.
//!
//! This is its own crate. Serve it with the Dioxus CLI:
//!
//! ```sh
//! dx serve --package minigames-kit-bucket-brigade
//! # or, from the repo root: just example bucket_brigade
//! ```
//!
//! It cycles through a few "beat it back before it spreads" situations — a fire
//! in the wagon, heavy rains leaking through, a blizzard whipping the flames —
//! each spreading a touch faster, with a fresh seed per round so the layout
//! moves. The last result (contained/not, peak, leaked) is shown so you can see
//! what `on_complete` reports back.
//!
//! It's touch-first: tap the flames to knock them out faster than they multiply.
//! Drive it under a touch-emulated viewport to test for real; a fresh page load
//! (not RSX hot-reload) exercises the actual wasm.

use dioxus::prelude::*;
use minigames_kit::bucket_brigade::{BucketBrigade, BucketBrigadeResult};

/// Compiled by `dx serve` from this crate's root `tailwind.css`.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

/// (prompt, threat icon, spread ms, spread chance) — increasingly demanding.
const SITUATIONS: [(&str, &str, u32, f64); 3] = [
    ("Stamp out the flames before they reach the supplies —", "*", 800, 0.45),
    ("Bail and cover the load as the leaks spread —", "~", 700, 0.55),
    ("Beat the flames back as the blizzard whips them on —", "*", 600, 0.65),
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<BucketBrigadeResult>);

    let r = round();
    let (prompt, threat_icon, spread_ms, spread_chance) = SITUATIONS[(r as usize) % SITUATIONS.len()];
    // A fresh, deterministic seed per round → the layout lands somewhere new.
    let seed = 0xBA5E_u64.wrapping_mul(r as u64 + 1).rotate_left(r % 17);

    rsx! {
        // The shared CRT look plus this crate's own compiled Tailwind. No Play
        // CDN, so the utility classes are present from first paint (no JIT jank).
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "BucketBrigade — minigames-kit demo" }

        div {
            class: "crt flex flex-col",
            style: "min-height: 100vh; display: flex; flex-direction: column;",
            BucketBrigade {
                key: "{r}",
                prompt: prompt.to_string(),
                threat_icon: threat_icon.to_string(),
                spread_ms,
                spread_chance,
                seed,
                on_complete: move |res: BucketBrigadeResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }
            div {
                class: "text-center text-sm opacity-70 p-4",
                style: "text-align: center; font-size: 0.875rem; opacity: 0.7; padding: 1rem;",
                match last() {
                    Some(res) if res.contained => rsx! {
                        "Contained it — peak {res.peak_active} burning, none got away — round {r}"
                    },
                    Some(res) => rsx! {
                        "It got away — {res.leaked} still burning (peak {res.peak_active}) — round {r}"
                    },
                    None => rsx! { "Tap the flames faster than they spread." },
                }
            }
        }
    }
}
