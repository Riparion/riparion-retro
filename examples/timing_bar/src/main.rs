//! Standalone demo of the [`TimingBar`] minigame.
//!
//! This is its own crate. Serve it with the Dioxus CLI:
//!
//! ```sh
//! dx serve --package minigames-kit-timing-bar
//! # or, from the repo root: just example timing_bar
//! ```
//!
//! It cycles through a few physical "rhythm" situations — lockpicking, surgery,
//! a sword strike — each with its own zone width and sweep speed, and feeds a
//! fresh seed per round so the target zone moves and can't be memorized. The
//! last result (hit/miss plus accuracy) is shown so you can see what
//! `on_strike` reports back.

use dioxus::prelude::*;
use minigames_kit::timing_bar::{TimingBar, TimingResult};

/// Compiled by `dx serve` from this crate's root `tailwind.css`.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

/// (prompt, action label, zone half-width, sweep ms) — increasingly demanding.
const SITUATIONS: [(&str, &str, f64, u32); 3] = [
    ("Pick the lock —", "PICK", 0.12, 1700),
    ("Steady the scalpel —", "CUT", 0.08, 1400),
    ("Time your strike —", "STRIKE", 0.05, 1050),
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<TimingResult>);

    let r = round();
    let (prompt, action, tolerance, period_ms) = SITUATIONS[(r as usize) % SITUATIONS.len()];
    // A fresh, deterministic seed per round → the zone lands somewhere new.
    let seed = 0xC0FFEE_u64.wrapping_mul(r as u64 + 1).rotate_left(r % 17);

    rsx! {
        // The shared CRT look plus this crate's own compiled Tailwind. No Play
        // CDN, so the utility classes are present from first paint (no JIT jank).
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "TimingBar — minigames-kit demo" }

        div { class: "crt flex flex-col", style: "min-height: 100vh;",
            TimingBar {
                key: "{r}",
                prompt: prompt.to_string(),
                action: action.to_string(),
                tolerance,
                period_ms,
                seed,
                on_strike: move |res: TimingResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }
            div { class: "text-center text-sm opacity-70 p-4",
                match last() {
                    Some(res) if res.hit => rsx! {
                        "Landed it — {(res.accuracy * 100.0) as i32}% accuracy (off by {res.offset:.3}) — round {r}"
                    },
                    Some(res) => rsx! {
                        "Missed the zone (off by {res.offset:.3}) — round {r}"
                    },
                    None => rsx! { "Tap the button as the bar crosses the lit zone." },
                }
            }
        }
    }
}
