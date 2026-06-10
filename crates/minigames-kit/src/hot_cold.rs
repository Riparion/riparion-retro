//! HotCold — a search/deduction minigame.
//!
//! A target is hidden somewhere on a grid. Each probe (a tap on a cell) reports
//! back how close it was — either "warmer/colder" relative to your last probe, or
//! the cell's absolute distance ring from the target. Find it in as few probes as
//! possible. Unlike the kit's reaction (QuickDraw), memory (CrowdThreading), and
//! tracking (Hunter) games, this tests *inference*: each clue narrows the field, and
//! a binary-search-ish strategy beats blind guessing.
//!
//! Like the other minigames it's self-contained and reports a plain
//! [`HotColdResult`] through `on_complete`, leaving scoring to the host (which maps
//! `probes_used` to time/days lost).
//!
//! It is, however, the kit's first *turn-based* minigame: its cost is **probes**,
//! not wall-time, so it deliberately has **no tick clock**. A run is purely
//! tap-driven — it ends when the target is found or a `max_probes` budget is spent
//! — which keeps it fully deterministic from `seed` plus the player's taps without
//! any timing nondeterminism to reason about. It's touch-first: every interaction
//! is a single tap on a cell, no drag or hover.

use dioxus::prelude::*;

use retro_kit::rng::GameRng;
use retro_kit::theme::SCREEN_CENTERED;

use crate::grid;

/// Smallest grid HotCold will run on. A 1-wide grid has no room to triangulate, so
/// both axes are floored here — the one place the min-grid invariant lives.
const MIN_GRID: usize = 2;

/// What a probe tells the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feedback {
    /// Each probe reports whether it is *closer or farther than the previous probe*.
    /// The first probe has no prior, so it reads as a neutral baseline.
    WarmerColder,
    /// Each probe reports its *absolute* distance ring from the target (0 = found).
    DistanceRing,
}

/// Closer/farther than the previous probe — the [`Feedback::WarmerColder`] verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Temperature {
    /// This probe is nearer the target than the previous one.
    Warmer,
    /// This probe is farther from the target than the previous one.
    Colder,
    /// Same distance as the previous probe.
    Same,
}

/// The outcome of one HotCold run, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotColdResult {
    /// Whether the target was located within the probe budget.
    pub found: bool,
    /// How many probes were spent. The host turns this into the loss (time, days,
    /// illness risk, …) — a sharp searcher pays little, a flailing one a lot.
    pub probes_used: usize,
    /// A sensible benchmark probe count for this layout, so the host can grade
    /// `probes_used` against "good play" without re-deriving the grid difficulty.
    pub par: usize,
}

/// `(col, row)` of a row-major cell index.
fn coords(idx: usize, cols: usize) -> (usize, usize) {
    (idx % cols, idx / cols)
}

/// Chebyshev (king-move) distance between two cells — `max(|Δcol|, |Δrow|)`. This is
/// the metric behind the distance rings: ring `k` is the square shell `k` cells out.
pub fn distance(a: usize, b: usize, cols: usize) -> usize {
    let (ac, ar) = coords(a, cols);
    let (bc, br) = coords(b, cols);
    ac.abs_diff(bc).max(ar.abs_diff(br))
}

/// The distance ring a probe lands in relative to the target. `0` ⇔ the probe *is*
/// the target. Alias for [`distance`], named for how it reads at the call site.
pub fn ring(probe: usize, target: usize, cols: usize) -> usize {
    distance(probe, target, cols)
}

/// Whether `probe` is warmer/colder/same versus `prev` — the warmer-colder verdict,
/// comparing each probe's distance to the target against the previous probe's.
pub fn temperature(probe: usize, prev: usize, target: usize, cols: usize) -> Temperature {
    use std::cmp::Ordering::*;
    match distance(probe, target, cols).cmp(&distance(prev, target, cols)) {
        Less => Temperature::Warmer,
        Greater => Temperature::Colder,
        Equal => Temperature::Same,
    }
}

/// The hidden target cell for a given seed and grid, picked deterministically from a
/// seeded [`GameRng`] — same seed → same spot, distinct seeds spread across the grid.
pub fn target_index(seed: u64, cols: usize, rows: usize) -> usize {
    let cap = (cols * rows).max(1) as i64;
    let mut rng = GameRng::from_seed(seed);
    rng.ri(cap) as usize
}

/// `ceil(log2(n))` — the bisection depth for an `n`-wide axis. `0` for `n <= 1`.
fn ceil_log2(n: usize) -> usize {
    if n <= 1 {
        0
    } else {
        (usize::BITS - (n - 1).leading_zeros()) as usize
    }
}

/// A benchmark "good play" probe count for the layout. An absolute distance reading
/// lets an ideal searcher bisect each axis, so [`Feedback::DistanceRing`] pars at
/// `ceil(log2(cols)) + ceil(log2(rows))`; warmer/colder yields less per probe
/// (a single comparison, not a coordinate), so it pars higher. Monotonic in grid
/// size, and `DistanceRing` par never exceeds `WarmerColder` par — so a host scoring
/// against par always has headroom.
pub fn par(cols: usize, rows: usize, feedback: Feedback) -> usize {
    let base = ceil_log2(cols) + ceil_log2(rows);
    match feedback {
        Feedback::DistanceRing => base.max(2),
        Feedback::WarmerColder => (base * 2).max(3),
    }
}

/// A search/deduction minigame. Mount it once per encounter and wire `on_complete`;
/// changing `seed` (or the grid/feedback/budget) starts a fresh run, so a host can
/// reuse one instance across encounters as well as mount a new one each time:
///
/// ```rust,ignore
/// HotCold {
///     prompt: "Your son wandered into the brush — search for him.",
///     feedback: Feedback::WarmerColder,
///     max_probes: 8,
///     seed,                 // e.g. drawn from the game's RNG
///     on_complete: move |r: HotColdResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn HotCold(
    /// Line shown above the grid (the situation).
    prompt: String,
    /// Grid width in cells.
    #[props(default = 5)]
    cols: usize,
    /// Grid height in cells.
    #[props(default = 5)]
    rows: usize,
    /// What a probe reveals. Defaults to warmer/colder (the classic).
    #[props(default = Feedback::WarmerColder)]
    feedback: Feedback,
    /// How many probes the player gets before the search fails. Floored at 1.
    #[props(default = 8)]
    max_probes: usize,
    /// Seed for the hidden spot — vary it per encounter; same seed → same target.
    #[props(default = 0)]
    seed: u64,
    /// Called once, when the target is found or the probe budget is spent.
    on_complete: EventHandler<HotColdResult>,
) -> Element {
    // Floor the grid once, here, so every read below agrees on the dimensions.
    let cols = cols.max(MIN_GRID);
    let rows = rows.max(MIN_GRID);
    let max_probes = max_probes.max(1);

    // The hidden target and the par benchmark follow the current inputs. They're
    // cheap, so we recompute them per render rather than caching — that way they stay
    // correct even when this instance is reused for a new encounter (see the reset
    // effect below).
    let target = target_index(seed, cols, rows);
    let par_value = par(cols, rows, feedback);

    // The player's probe history, in order. Drives both the rendered feedback and
    // the warmer/colder baseline (each probe is compared to the one before it).
    let mut probes = use_signal(Vec::<usize>::new);
    let mut found = use_signal(|| false);
    let mut done = use_signal(|| false);

    // A run is defined by its seed, grid, feedback mode, and budget; when any of
    // those change, begin a fresh run. We reset here rather than relying on a
    // changing `key` to remount the component, because in this Dioxus version a key
    // change on a lone child doesn't force a remount — so a host that reuses one
    // instance across encounters (e.g. a demo cycling rounds) would otherwise carry
    // the old probe history forward.
    use_effect(use_reactive!(|(seed, cols, rows, feedback, max_probes)| {
        let _ = (seed, cols, rows, feedback, max_probes);
        probes.write().clear();
        found.set(false);
        done.set(false);
    }));

    // Probe a cell. Re-tapping an already-probed cell is a free no-op (it spends no
    // budget); a fresh tap either finds the target or burns one of the probes, and
    // ends the run on a find or once the budget is spent.
    let mut probe = move |idx: usize| {
        if done() || probes.read().contains(&idx) {
            return;
        }
        probes.write().push(idx);
        let used = probes.read().len();
        if idx == target {
            found.set(true);
            done.set(true);
            on_complete.call(HotColdResult {
                found: true,
                probes_used: used,
                par: par_value,
            });
        } else if used >= max_probes {
            done.set(true);
            on_complete.call(HotColdResult {
                found: false,
                probes_used: used,
                par: par_value,
            });
        }
    };

    // Borrow the history for the whole render rather than cloning it each frame.
    let history = probes.read();
    let used = history.len();
    let is_over = done();
    // The farthest possible ring, used to scale the heat colour so "warm" reads
    // brighter than "cold" regardless of grid size.
    let max_ring = (cols.max(rows) - 1).max(1);

    // Shared CRT-grid styling (see `grid`), plus `touch-action` so a quick tap can't
    // be swallowed by the browser's double-tap-to-zoom. Built once — it depends only
    // on `cols`, which never changes after mount.
    let grid_style = use_memo(move || {
        format!("{} touch-action: manipulation;", grid::container_style(cols))
    });
    let grid_style = grid_style();
    let cell_box = grid::CELL_BOX_STYLE;

    let hint = match feedback {
        Feedback::WarmerColder => "Tap a cell to search. ▲ warmer · ▼ colder · = same.",
        Feedback::DistanceRing => "Tap a cell to search. The number is how many cells away.",
    };

    rsx! {
        // Layout is set INLINE, not via Tailwind utilities: some hosts (the Tailwind
        // Play CDN) emit the flex/grid utilities only lazily, which would shift the
        // centered grid mid-run. Inline styles are stable from first paint; the
        // classes stay for hosts that do build Tailwind.
        div {
            class: "{SCREEN_CENTERED} gap-4 items-center",
            style: "flex: 1; display: flex; flex-direction: column; \
                align-items: center; justify-content: center; gap: 1rem; \
                padding: 1rem; max-width: 28rem; width: 100%; margin: 0 auto;",
            p {
                class: "text-center text-lg opacity-80",
                style: "text-align: center; opacity: 0.8; font-size: 1.125rem;",
                "{prompt}"
            }

            // Probe budget readout — the analogue of the timed games' progress bar.
            div {
                class: "w-full flex items-center justify-center gap-3 text-sm opacity-70",
                style: "width: 100%; display: flex; align-items: center; \
                    justify-content: center; gap: 0.75rem; \
                    font-size: 0.875rem; opacity: 0.7;",
                span { "Probes {used} / {max_probes}" }
            }

            // The grid: a dim lattice. Probed cells show their feedback (a warmer/
            // colder arrow or a distance number), the latest probe is ringed, and the
            // target is revealed once the run is over. Styled inline so the core
            // visual needs no utilities.
            div {
                style: "{grid_style}",
                for r in 0..rows {
                    for c in 0..cols {
                        {
                            let idx = r * cols + c;
                            let order = history.iter().position(|&p| p == idx);
                            let is_probed = order.is_some();
                            let is_latest = history.last() == Some(&idx);
                            let is_target = idx == target;
                            // Reveal the target once the run ends (whether found or
                            // not) so the player sees where it was.
                            let reveal_target = is_over && is_target;

                            // The glyph and its colour. Both `color` and `text-shadow`
                            // are set in EVERY branch: Dioxus patches style props one
                            // at a time and won't drop an omitted one, so a re-rendered
                            // cell would otherwise keep a stale glow.
                            let (glyph, glyph_style): (String, String) = if reveal_target {
                                (
                                    "★".to_string(),
                                    "color: #ffd000; text-shadow: 0 0 8px #ffd000;".to_string(),
                                )
                            } else if let Some(k) = order {
                                match feedback {
                                    Feedback::DistanceRing => {
                                        let d = ring(idx, target, cols);
                                        // Warm (small ring) reads bright amber; cold
                                        // (large ring) fades toward dim.
                                        let heat = 1.0 - (d as f64 / max_ring as f64);
                                        let a = 0.45 + 0.55 * heat;
                                        (
                                            format!("{d}"),
                                            format!(
                                                "color: rgba(255, 176, 0, {a:.2}); \
                                                 text-shadow: 0 0 {sh:.0}px rgba(255, 176, 0, {a:.2});",
                                                sh = 4.0 + 4.0 * heat,
                                            ),
                                        )
                                    }
                                    Feedback::WarmerColder => {
                                        // First probe has no prior — a neutral dot.
                                        if k == 0 {
                                            (
                                                "•".to_string(),
                                                "color: var(--phosphor-dim); text-shadow: none;"
                                                    .to_string(),
                                            )
                                        } else {
                                            let prev = history[k - 1];
                                            match temperature(idx, prev, target, cols) {
                                                Temperature::Warmer => (
                                                    "▲".to_string(),
                                                    "color: #ffb000; \
                                                     text-shadow: 0 0 6px #ffb000;"
                                                        .to_string(),
                                                ),
                                                Temperature::Colder => (
                                                    "▼".to_string(),
                                                    "color: rgba(80, 160, 220, 0.85); \
                                                     text-shadow: none;"
                                                        .to_string(),
                                                ),
                                                Temperature::Same => (
                                                    "=".to_string(),
                                                    "color: var(--phosphor-dim); \
                                                     text-shadow: none;"
                                                        .to_string(),
                                                ),
                                            }
                                        }
                                    }
                                }
                            } else {
                                (
                                    String::new(),
                                    "color: var(--phosphor-dim); text-shadow: none;".to_string(),
                                )
                            };

                            // The cell box: probed cells read as lit lanes, the latest
                            // probe is ringed brighter, un-probed cells stay dark and
                            // tappable. `cursor` set in every branch for the same
                            // patch-one-prop-at-a-time reason as above.
                            let cell_style = if reveal_target {
                                "background: rgba(255, 208, 0, 0.18); \
                                 border: 1px solid rgba(255, 208, 0, 0.75); cursor: default;"
                            } else if is_latest {
                                "background: rgba(255, 176, 0, 0.14); \
                                 border: 1px solid rgba(255, 176, 0, 0.65); cursor: default;"
                            } else if is_probed {
                                "background: rgba(255, 176, 0, 0.06); \
                                 border: 1px solid rgba(255, 176, 0, 0.30); cursor: default;"
                            } else if is_over {
                                "background: rgba(51, 255, 102, 0.04); \
                                 border: 1px solid transparent; cursor: default;"
                            } else {
                                "background: rgba(51, 255, 102, 0.06); \
                                 border: 1px solid rgba(51, 255, 102, 0.18); cursor: pointer;"
                            };

                            rsx! {
                                div {
                                    key: "{r}-{c}",
                                    style: "{cell_box} {cell_style}",
                                    onpointerdown: move |_| probe(idx),
                                    span { style: "{glyph_style}", "{glyph}" }
                                }
                            }
                        }
                    }
                }
            }

            p {
                class: "text-center text-sm opacity-60",
                style: "text-align: center; font-size: 0.875rem; opacity: 0.6;",
                "{hint}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn target_is_in_range_for_every_seed() {
        for &(cols, rows) in &[(5usize, 5usize), (6, 4), (3, 7)] {
            for seed in 0..200u64 {
                let t = target_index(seed, cols, rows);
                assert!(t < cols * rows, "target {t} out of range on {cols}x{rows}");
            }
        }
    }

    #[test]
    fn target_is_deterministic_per_seed_and_varies_across_seeds() {
        assert_eq!(target_index(42, 5, 5), target_index(42, 5, 5));
        // Across a spread of seeds the target should land on more than one cell.
        let distinct: std::collections::HashSet<usize> =
            (0..64u64).map(|s| target_index(s, 5, 5)).collect();
        assert!(distinct.len() > 1, "target never moved across seeds");
    }

    #[test]
    fn distance_is_chebyshev_zero_at_self_and_symmetric() {
        let cols = 5;
        // (0,0) to (3,2): max(3,2) = 3.
        assert_eq!(distance(0, 2 * cols + 3, cols), 3);
        // Self-distance is zero; ring 0 only at the target.
        for i in 0..cols * cols {
            assert_eq!(distance(i, i, cols), 0);
        }
        // Symmetric.
        let (a, b) = (1, 3 * cols + 4);
        assert_eq!(distance(a, b, cols), distance(b, a, cols));
        // A diagonal step counts as one ring (king-move), not two.
        assert_eq!(distance(0, cols + 1, cols), 1);
    }

    #[test]
    fn ring_is_zero_only_at_the_target() {
        let (cols, rows, seed) = (6, 5, 9);
        let t = target_index(seed, cols, rows);
        for i in 0..cols * rows {
            assert_eq!(ring(i, t, cols) == 0, i == t);
        }
    }

    #[test]
    fn temperature_reads_closer_farther_and_equal() {
        let cols = 7;
        let target = 3 * cols + 3; // (3,3)
        let far = 0; // (0,0), ring 3
        let near = 2 * cols + 2; // (2,2), ring 1
        let other_near = 4 * cols + 4; // (4,4), ring 1 — same distance as `near`
        assert_eq!(temperature(near, far, target, cols), Temperature::Warmer);
        assert_eq!(temperature(far, near, target, cols), Temperature::Colder);
        assert_eq!(
            temperature(other_near, near, target, cols),
            Temperature::Same
        );
    }

    #[test]
    fn par_is_sane_monotonic_and_leaves_headroom() {
        // At least one probe, always.
        assert!(par(2, 2, Feedback::DistanceRing) >= 1);
        assert!(par(2, 2, Feedback::WarmerColder) >= 1);
        // Ring feedback is never harder-to-par than warmer/colder — host headroom.
        for &(c, r) in &[(2usize, 2usize), (5, 5), (8, 6), (16, 16)] {
            assert!(par(c, r, Feedback::DistanceRing) <= par(c, r, Feedback::WarmerColder));
        }
        // Monotonic non-decreasing as the grid grows.
        for fb in [Feedback::DistanceRing, Feedback::WarmerColder] {
            assert!(par(4, 4, fb) <= par(8, 8, fb));
            assert!(par(8, 8, fb) <= par(16, 16, fb));
        }
    }

    /// A deterministic, feedback-only searcher used as the test bar. It keeps the set
    /// of cells still consistent with every clue and, each turn, probes the unprobed
    /// candidate whose feedback splits that set most evenly (minimax) — the kind of
    /// inference the game rewards. It never reads the target except through feedback,
    /// so the probe counts it achieves are a fair "good play" benchmark. Returns
    /// `(found_cell, probes_used)`.
    fn solve(seed: u64, cols: usize, rows: usize, feedback: Feedback) -> (usize, usize) {
        let target = target_index(seed, cols, rows);
        let cap = cols * rows;
        let mut candidates: Vec<usize> = (0..cap).collect();
        let mut probed = vec![false; cap];
        let mut prev: Option<usize> = None;
        let mut count = 0usize;

        loop {
            // Pick the unprobed candidate that minimizes the worst-case number of
            // candidates left after its feedback. On the very first warmer/colder
            // probe (no prior) every choice is uninformative, so fall back to the
            // candidate nearest the grid centre.
            let pick = candidates
                .iter()
                .copied()
                .filter(|&p| !probed[p])
                .min_by_key(|&p| {
                    let mut groups: HashMap<u8, usize> = HashMap::new();
                    for &cand in &candidates {
                        let key = match feedback {
                            Feedback::DistanceRing => ring(p, cand, cols) as u8,
                            Feedback::WarmerColder => match prev {
                                Some(pp) => match temperature(p, pp, cand, cols) {
                                    Temperature::Warmer => 0,
                                    Temperature::Colder => 1,
                                    Temperature::Same => 2,
                                },
                                None => 0,
                            },
                        };
                        *groups.entry(key).or_insert(0) += 1;
                    }
                    groups.values().copied().max().unwrap_or(0)
                })
                .expect("a candidate is always available (the target stays in the set)");

            probed[pick] = true;
            count += 1;
            if pick == target {
                return (pick, count);
            }

            // Observe the real feedback and keep only consistent candidates.
            match feedback {
                Feedback::DistanceRing => {
                    let obs = ring(pick, target, cols);
                    candidates.retain(|&cand| ring(pick, cand, cols) == obs);
                }
                Feedback::WarmerColder => {
                    if let Some(pp) = prev {
                        let obs = temperature(pick, pp, target, cols);
                        candidates.retain(|&cand| temperature(pick, pp, cand, cols) == obs);
                    }
                }
            }
            // The target is always consistent with its own feedback, so it can never
            // be eliminated — guaranteeing the search converges.
            assert!(candidates.contains(&target));
            prev = Some(pick);

            assert!(count <= cap, "solver failed to converge on {cols}x{rows}");
        }
    }

    #[test]
    fn the_searcher_always_finds_the_target() {
        for &(cols, rows) in &[(5usize, 5usize), (6, 4), (7, 7)] {
            for seed in 0..60u64 {
                for fb in [Feedback::DistanceRing, Feedback::WarmerColder] {
                    let (found, probes) = solve(seed, cols, rows, fb);
                    assert_eq!(found, target_index(seed, cols, rows));
                    assert!(probes <= cols * rows, "took {probes} probes on {cols}x{rows}");
                }
            }
        }
    }

    #[test]
    fn a_sharp_searcher_beats_a_blind_one_within_budget() {
        // The test bar (cf. BucketBrigade's do-nothing-loses / perfect-contains): a
        // smart searcher finds the target inside a tight budget for every seed, while
        // a blind scan that ignores the feedback often blows the same budget — so the
        // host's penalty curve, graded on probes, has real headroom.
        let (cols, rows, budget) = (5usize, 5usize, 12usize);

        for fb in [Feedback::DistanceRing, Feedback::WarmerColder] {
            for seed in 0..80u64 {
                let (_, probes) = solve(seed, cols, rows, fb);
                assert!(
                    probes <= budget,
                    "smart searcher took {probes} (> {budget}) for seed {seed}, {fb:?}"
                );
            }
        }

        // A blind searcher probes a fixed scan order, ignoring clues. With a budget
        // smaller than the grid it must miss whenever the target sits late in that
        // order — which happens for plenty of seeds.
        let mut blind_misses = 0;
        for seed in 0..80u64 {
            let target = target_index(seed, cols, rows);
            // Scan cells 0..budget; a miss is any target beyond that prefix.
            if target >= budget {
                blind_misses += 1;
            }
        }
        assert!(
            blind_misses > 0,
            "a blind scan should miss within budget for some seed"
        );
    }
}
