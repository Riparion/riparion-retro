//! Standalone demo of the [`QuickDraw`] minigame.
//!
//! Run it from the crate dir with the Dioxus CLI:
//!
//! ```sh
//! cd crates/minigames-kit
//! dx serve --example quickdraw
//! ```
//!
//! Each round rotates the target word and feeds a fresh seed, so the buttons
//! reshuffle and the target never stays put — the same anti-memorization
//! behavior a host game gets by drawing the seed from its own RNG. The last
//! result (reaction time + hit/miss) is shown so you can see what `on_fire`
//! reports back.

use dioxus::prelude::*;
use minigames_kit::quickdraw::{QuickDraw, QuickDrawResult};

fn main() {
    dioxus::launch(App);
}

/// The four classic Oregon Trail gunfight words make a convenient demo set.
const WORDS: [&str; 4] = ["BANG", "BLAM", "POW", "WHAM"];

/// A few situations to cycle through, so the prompt varies like a real game's.
const PROMPTS: [&str; 4] = [
    "A deer! Take your shot —",
    "Bandits! Draw —",
    "Wolves! Fire —",
    "Riders close in! Fire —",
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<QuickDrawResult>);

    let r = round();
    let target = (r % WORDS.len() as u32) as usize;
    let prompt = PROMPTS[(r as usize) % PROMPTS.len()].to_string();
    // A fresh, deterministic seed per round → a fresh button layout each time.
    let seed = 0x5EED_u64.wrapping_mul(r as u64 + 1).rotate_left(r % 13);
    let words: Vec<String> = WORDS.iter().map(|w| w.to_string()).collect();

    rsx! {
        // The shared CRT look. Tailwind's Play CDN supplies the utility classes
        // the components use; real games compile their own tailwind.css instead.
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Script { src: "https://cdn.tailwindcss.com" }
        document::Title { "QuickDraw — minigames-kit demo" }

        div { class: "crt flex flex-col", style: "min-height: 100vh;",
            QuickDraw {
                key: "{r}",
                prompt,
                words,
                target,
                seed,
                on_fire: move |res: QuickDrawResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }
            div { class: "text-center text-sm opacity-70 p-4",
                match last() {
                    Some(res) if res.hit => rsx! { "Hit! {res.seconds:.2}s — round {r}" },
                    Some(res) => rsx! { "Missed (wrong word) at {res.seconds:.2}s — round {r}" },
                    None => rsx! { "Tap the matching word as fast as you can." },
                }
            }
        }
    }
}
