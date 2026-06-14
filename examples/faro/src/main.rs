//! Standalone demo of the [`Faro`] minigame.
//!
//! This is its own crate. Serve it with the Dioxus CLI:
//!
//! ```sh
//! dx serve --package minigames-kit-faro
//! # or, from the repo root: just example faro
//! ```
//!
//! Each session deals a fresh, deterministic deck from a new seed, so the run
//! varies but stays replayable. Stake ranks (copper one to bet it loses), deal
//! the turn, and read the case keeper's remaining counts to grow your stake to
//! the target before the deck runs out. The last result (won/stake/turns) is
//! shown so you can see what `on_complete` reports back to a host game.

use dioxus::prelude::*;
use minigames_kit::faro::{Faro, FaroResult};
use retro_kit::format::fmt_dollars_compact;

/// Compiled by `dx serve` from this crate's root `tailwind.css`.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

/// A few saloon situations to cycle through, like a host game's flavor.
const PROMPTS: [&str; 4] = [
    "Under-the-Hill at Natchez. The bank is open —",
    "A faro bank in the back of the Kentucky tavern —",
    "The dealer shuffles the box. Place your stakes —",
    "Lamplight and cards. Try the tiger —",
];

#[component]
fn App() -> Element {
    let mut session = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<FaroResult>);

    let n = session();
    let prompt = PROMPTS[(n as usize) % PROMPTS.len()].to_string();
    // A fresh, deterministic seed per session → a fresh deal each time.
    let seed = 0xFA_2057_u64.wrapping_mul(n as u64 + 1).rotate_left(n % 17);

    rsx! {
        // The shared CRT look plus this crate's own compiled Tailwind, so the
        // utility classes are present from first paint (no Play-CDN JIT jank).
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "Faro — minigames-kit demo" }

        div { class: "crt flex flex-col", style: "min-height: 100vh;",
            Faro {
                key: "{n}",
                prompt,
                seed,
                starting_stake: 200,
                target_stake: 500,
                min_bet: 25,
                on_complete: move |res: FaroResult| {
                    last.set(Some(res));
                    session += 1;
                },
            }
            div { class: "text-center text-sm opacity-70 p-4",
                match last() {
                    Some(res) if res.won => rsx! { "Beat the bank — left with {fmt_dollars_compact(res.final_stake as f64)} after {res.turns_played} turns." },
                    Some(res) => rsx! { "Walked away with {fmt_dollars_compact(res.final_stake as f64)} after {res.turns_played} turns. Deal again." },
                    None => rsx! { "Reach the target stake before the deck runs out." },
                }
            }
        }
    }
}
