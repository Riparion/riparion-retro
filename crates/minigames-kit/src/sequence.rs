//! Sequence — a Simon-style order-memory test.
//!
//! A short ordered run of symbols flashes one at a time; then the palette opens
//! and the player taps them back in the same order. It rewards working memory
//! for *order* — not where things sit (that's CrowdThreading) and not reflexes
//! (that's QuickDraw). A wrong tap means you misremembered the procedure, not
//! that your fingers were slow. The host turns the result into whatever it
//! models — spokes re-seated true, a wound dressed in the right order, the
//! snakebite steps done before the venom spreads.
//!
//! It reports back how far the player got via [`SequenceResult`] — the correct
//! prefix entered before the first slip, the full length, and whether it was
//! flawless. Scoring stays in the host game's engine: a clean run, a fumbled
//! one, a botched one.
//!
//! Fully parameterized: the `symbols` palette (shapes, icons, or words), the
//! sequence `length`, and how long each symbol flashes (`show_ms` — lower is
//! harder). The order is a pure function of `seed`, so runs are deterministic
//! and replayable; a fresh seed per encounter keeps the order from being
//! memorized across runs.
//!
//! Timing uses a 20 ms tick counter (gloo-timers) rather than a wall clock, so
//! which symbol is lit is a pure function of elapsed ticks and the component
//! adds no nondeterminism of its own.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;
use retro_kit::theme::{BTN, SCREEN_CENTERED};

/// Granularity of the flash clock, in milliseconds (~50 fps).
pub const TICK_MS: u32 = 20;

/// Fraction of each symbol's slot the glyph stays lit; the remainder is a blank
/// gap so a repeated symbol reads as two distinct flashes rather than one long
/// one.
const LIT_FRACTION: f64 = 0.65;

/// The outcome of one Sequence round, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SequenceResult {
    /// Steps entered correctly before the first slip (equals `length` on a
    /// flawless run).
    pub correct_prefix: usize,
    /// Total steps in the sequence.
    pub length: usize,
    /// Whether the whole sequence was reproduced without a single slip.
    pub perfect: bool,
}

/// Build the symbol order: `length` indices into a palette of `set_size`
/// symbols. Deterministic — same inputs → same order; vary `seed` per encounter
/// for a fresh order. Returns empty when the palette is empty.
pub fn build_sequence(length: usize, set_size: usize, seed: u64) -> Vec<usize> {
    if set_size == 0 {
        return Vec::new();
    }
    let mut rng = GameRng::from_seed(seed);
    (0..length)
        .map(|_| rng.ri(set_size as i64) as usize)
        .collect()
}

/// Which sequence slot is lit at `elapsed_ms` during the flash phase, or `None`
/// during the brief gap between slots and once the whole sequence has played.
/// Each slot owns a `show_ms` window, lit for its first [`LIT_FRACTION`] and
/// dark for the rest so repeats read as two flashes.
fn lit_slot(elapsed_ms: f64, length: usize, show_ms: f64) -> Option<usize> {
    if show_ms <= 0.0 || length == 0 || elapsed_ms < 0.0 {
        return None;
    }
    let slot = (elapsed_ms / show_ms).floor() as usize;
    if slot >= length {
        return None;
    }
    let within = elapsed_ms - slot as f64 * show_ms;
    (within <= show_ms * LIT_FRACTION).then_some(slot)
}

/// How many of `entered` match `target` from the start, stopping at the first
/// mismatch (or the end of the shorter list). Because the input phase only ever
/// records correct taps, this is the player's correct prefix at any moment.
fn correct_prefix(target: &[usize], entered: &[usize]) -> usize {
    target
        .iter()
        .zip(entered)
        .take_while(|(a, b)| a == b)
        .count()
}

/// The kit's default four-symbol palette: distinct phosphor-friendly shapes that
/// render without an emoji font. Hosts can pass their own `symbols` (icons or
/// words) instead.
fn default_symbols() -> Vec<String> {
    ["▲", "●", "■", "◆"].iter().map(|s| s.to_string()).collect()
}

/// An order-memory minigame. Mount it (with a `key` that changes per encounter,
/// so the order and progress reset) and wire `on_complete`:
///
/// ```rust,ignore
/// Sequence {
///     key: "{round}",
///     prompt: "Re-seat the spokes —",
///     symbols: vec!["JACK".into(), "BLOCK".into(), "BOLT".into(), "SEAT".into()],
///     length: 4,        // how many flashes to reproduce
///     show_ms: 700,     // per-symbol flash time; lower = harder
///     seed,             // e.g. drawn from the game's RNG
///     on_complete: move |r: SequenceResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn Sequence(
    /// Line shown above the field (the situation: "Re-seat the spokes —").
    prompt: String,
    /// The palette the sequence is drawn from and the player taps back. Defaults
    /// to four distinct shapes; pass icons or words for a themed procedure.
    #[props(default = default_symbols())]
    symbols: Vec<String>,
    /// How many symbols flash (and must be reproduced). Symbols may repeat.
    #[props(default = 4)]
    length: usize,
    /// How long each symbol flashes, in milliseconds. Lower = a faster, harder
    /// run to commit to memory.
    #[props(default = 700)]
    show_ms: u32,
    /// Seed for the symbol order. Same seed → same order (deterministic); vary it
    /// per encounter so the order can't be memorized across runs.
    #[props(default = 0)]
    seed: u64,
    /// Called once, when the player completes the sequence or makes their first
    /// slip.
    on_complete: EventHandler<SequenceResult>,
) -> Element {
    // Number of distinct palette entries, captured before `symbols` is borrowed
    // by the render below (so the sequence init doesn't move the palette).
    let set_size = symbols.len();

    // Pinned once per mount.
    let seq = use_signal(move || build_sequence(length, set_size, seed));
    let mut flashing = use_signal(|| true);
    let mut ticks = use_signal(|| 0u32);
    let mut entered = use_signal(Vec::<usize>::new);
    let mut done = use_signal(|| false);

    // Drive the flash phase: tick until the whole sequence has played, then open
    // the palette for input.
    use_future(move || async move {
        let total_ms = length as u32 * show_ms;
        loop {
            if !flashing() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            ticks += 1;
            if ticks() * TICK_MS >= total_ms {
                flashing.set(false);
                break;
            }
        }
    });

    let elapsed = ticks() as f64 * TICK_MS as f64;
    // Which palette symbol is lit right now (flash phase only).
    let lit_index = if flashing() {
        lit_slot(elapsed, length, show_ms as f64).and_then(|s| seq.read().get(s).copied())
    } else {
        None
    };
    let lit_symbol = lit_index.and_then(|i| symbols.get(i)).cloned();

    // Size the glyphs to the longest label so single-glyph shapes render big while
    // word labels (e.g. "SPLINT") still fit their box instead of overflowing.
    let max_len = symbols.iter().map(|s| s.chars().count()).max().unwrap_or(1);
    let (stage_px, btn_px) = if max_len <= 2 {
        ("3rem", "1.6rem")
    } else if max_len <= 4 {
        ("2rem", "1.15rem")
    } else {
        ("1.4rem", "0.95rem")
    };

    // Progress pips: symbols shown so far during the flash, then the correct
    // prefix the player has tapped back.
    let filled = if flashing() {
        ((elapsed / show_ms.max(1) as f64).floor() as usize).min(length)
    } else {
        correct_prefix(&seq.read(), &entered.read())
    };

    // Resolve a tapped palette symbol during the input phase. Record the tap,
    // then re-derive the correct prefix: if it didn't grow, this tap was the
    // first slip; if it reached the full length, the order is reproduced clean.
    let mut tap = move |idx: usize| {
        if done() || flashing() {
            return;
        }
        entered.write().push(idx);
        let seq = seq.read();
        let cp = correct_prefix(&seq, &entered.read());
        let n = seq.len();
        if cp < entered.read().len() {
            // The latest tap broke the run — slip.
            done.set(true);
            on_complete.call(SequenceResult {
                correct_prefix: cp,
                length: n,
                perfect: false,
            });
        } else if cp >= n {
            // Every step reproduced in order.
            done.set(true);
            on_complete.call(SequenceResult {
                correct_prefix: n,
                length: n,
                perfect: true,
            });
        }
    };

    rsx! {
        // The layout is set inline as well as via Tailwind classes: some hosts
        // (the Tailwind Play CDN) emit flex/grid utilities only lazily, so the
        // classes can fall back to block flow on a fresh load. Inline styles are
        // stable from first paint; the classes stay for hosts that build them.
        div {
            class: "{SCREEN_CENTERED} gap-6 items-center",
            style: "display: flex; flex-direction: column; align-items: center; \
                justify-content: center; flex: 1; gap: 1.5rem; padding: 1rem; \
                width: 100%; max-width: 28rem; margin: 0 auto;",
            p { class: "text-lg opacity-80", style: "text-align: center;", "{prompt}" }

            // Progress pips — one per step, filling as symbols flash, then as the
            // player taps them back.
            div {
                class: "flex gap-2 flex-wrap justify-center text-lg",
                style: "display: flex; gap: 0.5rem; flex-wrap: wrap; justify-content: center;",
                for i in 0..length {
                    span {
                        key: "{i}",
                        style: if i < filled {
                            "color: var(--phosphor); text-shadow: var(--glow);"
                        } else {
                            "color: var(--phosphor-dim); opacity: 0.4;"
                        },
                        if i < filled { "●" } else { "○" }
                    }
                }
            }

            if flashing() {
                // A single large stage that lights one symbol at a time. The blank
                // gap between slots is rendered as an invisible glyph so the box
                // never resizes as symbols come and go.
                div {
                    style: "display: flex; align-items: center; justify-content: center; \
                        width: 9rem; height: 9rem; border: 1px solid var(--phosphor-dim); \
                        border-radius: 8px; background: rgba(20, 95, 40, 0.10); \
                        font-size: {stage_px}; font-weight: 700; line-height: 1; overflow: hidden;",
                    span {
                        // `opacity` is set explicitly in *both* branches: Dioxus
                        // patches style props individually and won't drop one the
                        // new string omits, so a lit style that left `opacity` out
                        // would inherit the gap's `opacity: 0` and every symbol
                        // after the first would stay invisible.
                        style: if lit_symbol.is_some() {
                            "opacity: 1; color: var(--phosphor); text-shadow: var(--glow);"
                        } else {
                            "opacity: 0;"
                        },
                        // Fall back to a placeholder during the gap so the box keeps
                        // its size; it's invisible via opacity above.
                        "{lit_symbol.clone().unwrap_or_else(|| String::from(\"·\"))}"
                    }
                }
                p { class: "text-sm opacity-60", style: "text-align: center;",
                    "Watch the order — the palette opens in a moment."
                }
            } else {
                p { class: "text-sm opacity-60", style: "text-align: center;",
                    "Tap them back in order. Step {filled.min(length)} of {length}."
                }
                // The palette: every symbol, in a wrapped row. Tap them in the
                // order they flashed. Styled inline so the row lays out from first
                // paint regardless of the host's utility build.
                div {
                    class: "flex gap-3 flex-wrap justify-center",
                    style: "display: flex; gap: 0.75rem; flex-wrap: wrap; justify-content: center;",
                    for (i, sym) in symbols.iter().enumerate() {
                        button {
                            key: "{i}",
                            class: "{BTN}",
                            style: "min-width: 4.5rem; height: 4.5rem; padding: 0 0.9rem; \
                                font-size: {btn_px}; font-weight: 700; white-space: nowrap;",
                            onclick: move |_| tap(i),
                            "{sym}"
                        }
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
    fn sequence_has_right_length_and_in_range_indices() {
        for seed in 0..64 {
            let seq = build_sequence(5, 4, seed);
            assert_eq!(seq.len(), 5);
            assert!(seq.iter().all(|&i| i < 4), "index out of palette at seed {seed}");
        }
    }

    #[test]
    fn same_seed_same_order_different_seed_differs() {
        assert_eq!(build_sequence(6, 4, 9), build_sequence(6, 4, 9));
        let base = build_sequence(6, 4, 9);
        assert!((0..32).any(|s| build_sequence(6, 4, s) != base));
    }

    #[test]
    fn empty_palette_yields_empty_sequence() {
        assert!(build_sequence(5, 0, 1).is_empty());
    }

    #[test]
    fn lit_slot_advances_then_goes_dark() {
        // Slot 0 is lit at the start of its window...
        assert_eq!(lit_slot(0.0, 3, 1000.0), Some(0));
        // ...dark during the gap (past the lit fraction)...
        assert_eq!(lit_slot(900.0, 3, 1000.0), None);
        // ...slot 1 lit at the start of the next window...
        assert_eq!(lit_slot(1000.0, 3, 1000.0), Some(1));
        assert_eq!(lit_slot(2050.0, 3, 1000.0), Some(2));
        // ...and dark once the whole sequence has played.
        assert_eq!(lit_slot(3000.0, 3, 1000.0), None);
        // Degenerate timings never light a slot.
        assert_eq!(lit_slot(0.0, 0, 1000.0), None);
        assert_eq!(lit_slot(0.0, 3, 0.0), None);
    }

    #[test]
    fn correct_prefix_stops_at_the_first_slip() {
        let target = [0usize, 2, 1, 3];
        // Flawless full match.
        assert_eq!(correct_prefix(&target, &[0, 2, 1, 3]), 4);
        // A clean prefix, then a slip.
        assert_eq!(correct_prefix(&target, &[0, 2, 0]), 2);
        // Wrong from the very first tap.
        assert_eq!(correct_prefix(&target, &[1]), 0);
        // A correct but unfinished prefix.
        assert_eq!(correct_prefix(&target, &[0, 2]), 2);
        // Nothing entered yet.
        assert_eq!(correct_prefix(&target, &[]), 0);
    }
}
