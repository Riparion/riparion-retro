//! Standalone demo of the [`VingtUn`] minigame.
//!
//! This is its own crate. Serve it with the Dioxus CLI:
//!
//! ```sh
//! dx serve --package minigames-kit-vingt-un
//! # or, from the repo root: just example vingt_un
//! ```
//!
//! Each session deals fresh, deterministic shuffles from a new seed, so the run
//! varies but stays replayable. Lay a stake, draw toward twenty-one against the
//! dealer's fixed hand (a two-card 21 pays 3:2), and grow your purse to the
//! target before the round allowance runs out. The last result (won/stake/rounds)
//! is shown so you can see what `on_complete` reports back to a host game.

use dioxus::prelude::*;
use minigames_kit::vingt_un::{VingtUn, VingtUnResult};
use retro_kit::format::fmt_dollars_compact;

/// Compiled by `dx serve` from this crate's root `tailwind.css`.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

/// A few saloon situations to cycle through, like a host game's flavor.
const PROMPTS: [&str; 4] = [
    "Under-the-Hill at Natchez. The banker deals —",
    "A New Orleans card room, vingt-et-un by lamplight —",
    "The dealer squares the deck. Lay your stake —",
    "Twenty-one for the boatmen's purses. Sit down —",
];

#[component]
fn App() -> Element {
    let mut session = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<VingtUnResult>);

    let n = session();
    let prompt = PROMPTS[(n as usize) % PROMPTS.len()].to_string();
    // A fresh, deterministic seed per session → fresh deals each time.
    let seed = 0x21_2057_u64.wrapping_mul(n as u64 + 1).rotate_left(n % 17);

    rsx! {
        // The shared CRT look plus this crate's own compiled Tailwind, so the
        // utility classes are present from first paint (no Play-CDN JIT jank).
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "Vingt-et-Un — minigames-kit demo" }

        div { class: "crt flex flex-col", style: "min-height: 100vh;",
            VingtUn {
                key: "{n}",
                prompt,
                seed,
                starting_stake: 200,
                target_stake: 500,
                min_bet: 25,
                on_complete: move |res: VingtUnResult| {
                    last.set(Some(res));
                    session += 1;
                },
            }
            div { class: "text-center text-sm opacity-70 p-4",
                match last() {
                    Some(res) if res.won => rsx! { "Beat the bank — left with {fmt_dollars_compact(res.final_stake as f64)} after {res.rounds_played} rounds." },
                    Some(res) => rsx! { "Walked away with {fmt_dollars_compact(res.final_stake as f64)} after {res.rounds_played} rounds. Deal again." },
                    None => rsx! { "Reach the target stake before the round allowance runs out." },
                }
            }
        }
    }
}
