//! QuickDraw — a flash-a-word reaction test.
//!
//! One target word is shown large; the same word plus decoys appear as buttons.
//! Tap the matching one as fast as you can. The component reports back how long
//! the tap took and whether it hit the right word, via [`QuickDrawResult`] — the
//! host game turns that into whatever score it needs (Oregon Trail folds the
//! reaction time into the original's `B1` marksmanship score, for instance).
//!
//! It is fully parameterized: pass any word set, the index of the target, and a
//! prompt. Button positions are shuffled from a caller-supplied `seed`, so the
//! target never sits in a predictable slot and players can't memorize a layout
//! across encounters. Drawing the seed from the host's game RNG keeps runs
//! deterministic and replayable; a fresh seed per encounter keeps them varied.
//!
//! Timing uses a 20 ms tick counter (gloo-timers) rather than a wall clock, so
//! the component adds no nondeterminism of its own and stays test-friendly.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;
use retro_kit::theme::{BTN_WIDE, SCREEN_CENTERED};

/// Granularity of the reaction stopwatch, in milliseconds.
pub const TICK_MS: u32 = 20;

/// The outcome of one QuickDraw round, handed to the host's `on_fire` callback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuickDrawResult {
    /// Reaction time in seconds, from when the round appeared to the tap.
    pub seconds: f64,
    /// Whether the tapped word was the target.
    pub hit: bool,
}

/// Deterministic Fisher–Yates shuffle of `0..len`, seeded so the same `seed`
/// always yields the same arrangement (and a new seed yields a fresh one).
fn shuffled_order(len: usize, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..len).collect();
    if len < 2 {
        return order;
    }
    let mut rng = GameRng::from_seed(seed);
    for i in (1..len).rev() {
        let j = rng.ri(i as i64 + 1) as usize;
        order.swap(i, j);
    }
    order
}

/// A flash-a-word reaction minigame. Mount it (with a `key` that changes per
/// encounter, so the stopwatch and layout reset) and wire `on_fire`:
///
/// ```rust,ignore
/// QuickDraw {
///     key: "{round}",
///     prompt: "A deer! Take your shot —",
///     words: vec!["BANG".into(), "BLAM".into(), "POW".into(), "WHAM".into()],
///     target,            // index into `words`
///     seed,              // e.g. drawn from the game's RNG
///     on_fire: move |r: QuickDrawResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn QuickDraw(
    /// Line shown above the target word (the situation: "Bandits! Draw —").
    prompt: String,
    /// The full option set. Order here is the canonical order; on-screen
    /// positions are shuffled from `seed`.
    words: Vec<String>,
    /// Index into `words` of the correct word to tap.
    target: usize,
    /// Seed for the position shuffle. Same seed → same layout (deterministic);
    /// vary it per encounter so players can't memorize where the target sits.
    seed: u64,
    /// Optional helper line under the target. Defaults to a tap-fast hint.
    #[props(default = String::from("Tap the word above as fast as you can."))]
    hint: String,
    /// Called once, when the player taps a button.
    on_fire: EventHandler<QuickDrawResult>,
) -> Element {
    let len = words.len();
    // Computed once per mount: stopwatch ticks, whether we've fired, the layout.
    let mut ticks = use_signal(|| 0u32);
    let mut answered = use_signal(|| false);
    let order = use_signal(move || shuffled_order(len, seed));

    // Tick the stopwatch until the player fires.
    use_future(move || async move {
        loop {
            if answered() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            ticks += 1;
        }
    });

    let target_word = words.get(target).cloned().unwrap_or_default();

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-4 items-center",
            p { class: "text-center text-lg opacity-80", "{prompt}" }
            div { class: "splash-title", "{target_word}" }
            p { class: "text-center text-sm opacity-60", "{hint}" }
            div { class: "grid grid-cols-2 gap-2 w-full mt-2",
                for word_idx in order.read().iter().copied() {
                    button {
                        key: "{word_idx}",
                        class: "{BTN_WIDE} py-5 text-lg",
                        onclick: move |_| {
                            if answered() {
                                return;
                            }
                            answered.set(true);
                            let seconds = ticks() as f64 * TICK_MS as f64 / 1000.0;
                            on_fire.call(QuickDrawResult {
                                seconds,
                                hit: word_idx == target,
                            });
                        },
                        "{words[word_idx]}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_is_a_permutation() {
        let mut order = shuffled_order(4, 12345);
        order.sort();
        assert_eq!(order, vec![0, 1, 2, 3]);
    }

    #[test]
    fn same_seed_same_layout_different_seed_differs() {
        assert_eq!(shuffled_order(4, 7), shuffled_order(4, 7));
        // Over a handful of seeds at least one rearranges the target slot.
        let base = shuffled_order(4, 7);
        assert!((0..16).any(|s| shuffled_order(4, s) != base));
    }

    #[test]
    fn degenerate_lengths_are_safe() {
        assert_eq!(shuffled_order(0, 1), Vec::<usize>::new());
        assert_eq!(shuffled_order(1, 1), vec![0]);
    }
}
