//! Standalone demo of the [`SteadyHands`] minigame.
//!
//! Run it from the crate dir with the Dioxus CLI:
//!
//! ```sh
//! cd crates/minigames-kit
//! dx serve --example steady_hands
//! ```
//!
//! It cycles through a few "hold it steady" situations — fording a river, riding
//! out hail, easing a splint into place — each tightening the tolerance and
//! speeding the drift, with a fresh seed per round so the wander path moves. The
//! last result (steady/not, accuracy, longest streak) is shown so you can see
//! what `on_complete` reports back.
//!
//! It's touch-first: drag a finger across the field and keep the offset `✛`
//! cursor on the ring. Drive it under a touch-emulated viewport to test for
//! real; a fresh page load (not RSX hot-reload) exercises the actual wasm.

use dioxus::prelude::*;
use minigames_kit::steady_hands::{SteadyHands, SteadyHandsResult};

fn main() {
    dioxus::launch(App);
}

/// (prompt, tolerance, duration ms, drift speed) — increasingly demanding.
const SITUATIONS: [(&str, f64, u32, f64); 3] = [
    ("Hold the team on line across the current —", 0.14, 6000, 0.9),
    ("Keep the wagon steady as the hail spooks the oxen —", 0.11, 6000, 1.2),
    ("Ease the splint into place — hold it dead still —", 0.08, 5000, 1.5),
];

#[component]
fn App() -> Element {
    let mut round = use_signal(|| 0u32);
    let mut last = use_signal(|| None::<SteadyHandsResult>);

    let r = round();
    let (prompt, tolerance, duration_ms, drift_speed) = SITUATIONS[(r as usize) % SITUATIONS.len()];
    // A fresh, deterministic seed per round → the wander path lands somewhere new.
    let seed = 0xC0FFEE_u64.wrapping_mul(r as u64 + 1).rotate_left(r % 17);

    rsx! {
        // The shared CRT look. Tailwind's Play CDN supplies the utility classes;
        // the field itself is styled inline, so it renders without a build step.
        document::Stylesheet { href: retro_kit::CRT_CSS }
        document::Script { src: "https://cdn.tailwindcss.com" }
        document::Title { "SteadyHands — minigames-kit demo" }

        div {
            class: "crt flex flex-col",
            style: "min-height: 100vh; display: flex; flex-direction: column;",
            SteadyHands {
                key: "{r}",
                prompt: prompt.to_string(),
                tolerance,
                duration_ms,
                drift_speed,
                seed,
                on_complete: move |res: SteadyHandsResult| {
                    last.set(Some(res));
                    round += 1;
                },
            }
            div {
                class: "text-center text-sm opacity-70 p-4",
                style: "text-align: center; font-size: 0.875rem; opacity: 0.7; padding: 1rem;",
                match last() {
                    Some(res) if res.steady => rsx! {
                        "Held steady — {(res.accuracy * 100.0) as i32}% on target \
                         (best streak {(res.longest_streak * 100.0) as i32}%) — round {r}"
                    },
                    Some(res) => rsx! {
                        "Slipped — only {(res.accuracy * 100.0) as i32}% on target \
                         (best streak {(res.longest_streak * 100.0) as i32}%) — round {r}"
                    },
                    None => rsx! { "Drag across the field and keep the ✛ on the ring." },
                }
            }
        }
    }
}
