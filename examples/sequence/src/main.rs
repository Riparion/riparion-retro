//! Standalone demo of the [`Sequence`] minigame.
//!
//! This is its own crate. Serve it with the Dioxus CLI:
//!
//! ```sh
//! dx serve --package minigames-kit-sequence
//! # or, from the repo root: just example sequence
//! ```
//!
//! It cycles through a few "do the steps in order" repair/first-aid situations —
//! re-seating a broken wheel, binding an ox's injured leg, working the snakebite
//! steps — each a longer, faster-flashing run than the last, with a fresh seed
//! per round so the order moves. The last result (correct prefix, length,
//! whether it was flawless) is shown so you can see what `on_complete` reports
//! back.
//!
//! It's touch-first: watch the order flash, then tap the palette back in
//! sequence. Drive it under a touch-emulated viewport to test for real; a fresh
//! page load (not RSX hot-reload) exercises the actual wasm.

use dioxus::prelude::*;
use minigames_kit::sequence::{Sequence, SequenceResult};

/// Compiled by `dx serve` from this crate's root `tailwind.css`.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

/// A demo situation: prompt, the palette of steps, sequence length, per-symbol
/// flash time. Word palettes read as a procedure; the order still flashes (this
/// is working memory for order, not knowledge of the real steps).
struct Situation {
    prompt: &'static str,
    symbols: &'static [&'static str],
    length: usize,
    show_ms: u32,
}

/// Increasingly demanding: longer runs that flash faster.
const SITUATIONS: [Situation; 3] = [
    Situation {
        prompt: "Wagon wheel breaks — re-seat it in order.",
        symbols: &["JACK", "BLOCK", "BOLT", "SEAT"],
        length: 3,
        show_ms: 800,
    },
    Situation {
        prompt: "The ox injures its leg — wrap, pad, bind in sequence.",
        symbols: &["WRAP", "PAD", "BIND", "SPLINT"],
        length: 4,
        show_ms: 700,
    },
    Situation {
        prompt: "Snakebite — work the first-aid steps in order.",
        symbols: &["▲", "●", "■", "◆"],
        length: 5,
        show_ms: 550,
    },
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<SequenceResult>);

    let r = round();
    let sit = &SITUATIONS[(r as usize) % SITUATIONS.len()];
    let symbols: Vec<String> = sit.symbols.iter().map(|s| s.to_string()).collect();
    // A fresh, deterministic seed per round → the order lands somewhere new.
    let seed = 0x5EED_0F0D_u64.wrapping_mul(r as u64 + 1).rotate_left(r % 19);

    rsx! {
        // The shared CRT look plus this crate's own compiled Tailwind. No Play CDN,
        // so the utility classes are present from first paint (no JIT jank).
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "Sequence — minigames-kit demo" }

        div {
            class: "crt flex flex-col",
            style: "min-height: 100vh; display: flex; flex-direction: column;",
            Sequence {
                key: "{r}",
                prompt: sit.prompt.to_string(),
                symbols,
                length: sit.length,
                show_ms: sit.show_ms,
                seed,
                on_complete: move |res: SequenceResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }
            div {
                class: "text-center text-sm opacity-70 p-4",
                style: "text-align: center; font-size: 0.875rem; opacity: 0.7; padding: 1rem;",
                match last() {
                    Some(res) if res.perfect => rsx! {
                        "Flawless — all {res.length} steps in order — round {r}"
                    },
                    Some(res) => rsx! {
                        "Slipped after {res.correct_prefix} of {res.length} steps — round {r}"
                    },
                    None => rsx! { "Watch the order, then tap it back." },
                }
            }
        }
    }
}
