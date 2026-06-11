//! Standalone demo of the [`Heave`] minigame.
//!
//! This is its own crate. Serve it with the Dioxus CLI:
//!
//! ```sh
//! dx serve --package minigames-kit-heave
//! # or, from the repo root: just example heave
//! ```
//!
//! It cycles through a few "strain against resistance" situations — prying the
//! clam, forcing a stiff door, wedging the grate — each with more stages and a
//! tighter slip ceiling, and feeds a fresh seed per round so the threshold curve
//! moves. The last result (opened / stages cleared / slips / grip left) is shown
//! so you can see what `on_complete` reports back.

use dioxus::prelude::*;
use minigames_kit::heave::{Heave, HeaveResult};

/// Compiled by `dx serve` from this crate's root `tailwind.css`.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

/// (prompt, stages, slip ceiling) — increasingly demanding.
const SITUATIONS: [(&str, usize, f64); 3] = [
    ("Pry the clam open —", 2, 0.94),
    ("Force the stiff door —", 3, 0.92),
    ("Wedge the grate up —", 4, 0.90),
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<HeaveResult>);

    let r = round();
    let (prompt, stages, slip_load) = SITUATIONS[(r as usize) % SITUATIONS.len()];
    // A fresh, deterministic seed per round → the threshold curve shifts.
    let seed = 0xC1A_u64.wrapping_mul(r as u64 + 1).rotate_left(r % 17);

    rsx! {
        // The shared CRT look plus this crate's own compiled Tailwind. No Play
        // CDN, so the utility classes are present from first paint (no JIT jank).
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "Heave — minigames-kit demo" }

        div { class: "crt flex flex-col", style: "min-height: 100vh;",
            Heave {
                key: "{r}",
                prompt: prompt.to_string(),
                stages,
                slip_load,
                seed,
                on_complete: move |res: HeaveResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }
            div { class: "text-center text-sm opacity-70 p-4",
                match last() {
                    Some(res) if res.opened => rsx! {
                        "It gives! Cleared all {res.stages} stages with {res.slips} slip(s), {(res.grip_left * 100.0) as i32}% grip left — round {r}"
                    },
                    Some(res) => rsx! {
                        "It held fast — {res.stages_cleared}/{res.stages} stages, {res.slips} slip(s), out of grip — round {r}"
                    },
                    None => rsx! { "Press and hold to heave; ease off before the dashed ceiling." },
                }
            }
        }
    }
}
