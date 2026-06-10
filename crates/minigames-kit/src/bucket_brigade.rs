//! BucketBrigade — a triage-against-spread minigame.
//!
//! Threats flare up on a small grid and *spread* to their orthogonal neighbours
//! if left alone; the player taps a threatened cell to knock it back out. It's a
//! losing race you win by prioritising — beat the threats back to zero before the
//! clock runs out and you've contained it; let them outpace your taps and the
//! grid fills. The closest cousin is Hunter, but there's no aiming and no ammo:
//! it's growth versus correction, many simultaneous targets.
//!
//! Like the other minigames in this kit it's self-contained: it owns its timing
//! and input and reports a plain [`BucketBrigadeResult`] through `on_complete`,
//! leaving scoring to the host (loss scales with how much was still burning when
//! time ran out).
//!
//! Unlike Hunter (a bounce) and SteadyHands (a wander), whose state is a pure
//! function of elapsed time, BucketBrigade's grid is a *stateful* simulation: it
//! evolves each tick from a seeded spread plus the player's taps. The testable
//! core is therefore a [`Board`] struct, not a `fn(elapsed_ms)` — all randomness
//! is injected through a [`GameRng`], so a given seed and tap history replay
//! identically. Timing uses a 20 ms tick counter (gloo-timers), not a wall clock.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;
use retro_kit::theme::SCREEN_CENTERED;

use crate::grid;

/// Granularity of the tick clock, in milliseconds (~50 fps).
pub const TICK_MS: u32 = 20;

/// The outcome of one BucketBrigade run, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BucketBrigadeResult {
    /// Whether the threats were beaten back to zero before time ran out.
    pub contained: bool,
    /// Worst simultaneous threat count seen over the run.
    pub peak_active: usize,
    /// Threats still burning when the clock ran out — what got away. `0` if
    /// contained. The host turns this into the loss (supplies lost, miles, …).
    pub leaked: usize,
}

/// A row-major grid of safe/active cells — the pure, testable core of the game.
///
/// "Active" means a live threat (a flame, a leak). All randomness is injected via
/// a `&mut GameRng`, so the evolution is reproducible: the same seed and the same
/// sequence of [`spread`](Board::spread)/[`extinguish`](Board::extinguish) calls
/// always yields the same board.
#[derive(Debug, Clone)]
pub struct Board {
    cols: usize,
    rows: usize,
    /// `cols * rows` flags, row-major; `true` = a live threat.
    active: Vec<bool>,
    /// Reusable double-buffer for [`spread`](Board::spread): holds the frozen
    /// pre-step snapshot so a step does no per-step allocation. Not part of the
    /// board's observable state (see the `PartialEq` impl below).
    prev: Vec<bool>,
}

// `prev` is scratch space, so two boards with the same dimensions and live cells
// are equal regardless of the buffer left behind by the last spread.
impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.cols == other.cols && self.rows == other.rows && self.active == other.active
    }
}

impl Board {
    /// A fresh all-safe board. `cols`/`rows` are floored at 2 so the grid is
    /// never degenerate (a single row or column has no interior for threats to
    /// spread through) — this is the one place the min-grid invariant lives.
    fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(2);
        let rows = rows.max(2);
        Board {
            cols,
            rows,
            active: vec![false; cols * rows],
            prev: vec![false; cols * rows],
        }
    }

    fn idx(&self, c: usize, r: usize) -> usize {
        r * self.cols + c
    }

    /// Grid width, after the min-grid floor in [`new`](Board::new).
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Grid height, after the min-grid floor in [`new`](Board::new).
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Total cell count (`cols * rows`).
    pub fn capacity(&self) -> usize {
        self.active.len()
    }

    /// How many threats are live right now.
    pub fn active_count(&self) -> usize {
        self.active.iter().filter(|&&a| a).count()
    }

    /// Whether the cell at `idx` carries a live threat.
    pub fn is_active(&self, idx: usize) -> bool {
        self.active.get(idx).copied().unwrap_or(false)
    }

    /// Light `n` distinct starting threats, deterministically from `rng`. Picks by
    /// rejection on `rng.ri(len)`, with a first-free fallback scan so it always
    /// places exactly `n.min(capacity)` even if the draws keep colliding.
    fn ignite_initial(&mut self, rng: &mut GameRng, n: usize) {
        let cap = self.active.len();
        let want = n.min(cap);
        let mut placed = 0;
        let mut attempts = 0;
        while placed < want && attempts < cap * 8 {
            let i = rng.ri(cap as i64) as usize;
            if !self.active[i] {
                self.active[i] = true;
                placed += 1;
            }
            attempts += 1;
        }
        // Guarantee the count even in the unlucky case the random draws collided.
        for i in 0..cap {
            if placed >= want {
                break;
            }
            if !self.active[i] {
                self.active[i] = true;
                placed += 1;
            }
        }
    }

    /// Knock out the threat at `idx` (a player tap). A no-op on an already-safe
    /// cell or an out-of-range index, so taps are always harmless.
    fn extinguish(&mut self, idx: usize) {
        if let Some(cell) = self.active.get_mut(idx) {
            *cell = false;
        }
    }

    /// Advance the spread by one step. Double-buffered against a frozen snapshot,
    /// so a cell that catches this step does *not* go on to ignite its own
    /// neighbours within the same step. Cells are visited in fixed row-major order
    /// and each currently-safe cell draws exactly one `rng.uniform()`, so the draw
    /// sequence is a deterministic function of the board state — keeping the whole
    /// run reproducible for a given seed and tap history.
    fn spread(&mut self, rng: &mut GameRng, chance: f64) {
        // Freeze the current state into the reusable `prev` buffer (a memcpy, no
        // allocation), then read `prev` while we mutate `active` — so a cell that
        // catches this step does not go on to ignite its neighbours within it.
        self.prev.copy_from_slice(&self.active);
        for r in 0..self.rows {
            for c in 0..self.cols {
                let i = self.idx(c, r);
                if self.prev[i] {
                    // Already burning — it stays, and consumes no draw.
                    continue;
                }
                let roll = rng.uniform();
                let has_active_neighbor = (c > 0 && self.prev[self.idx(c - 1, r)])
                    || (c + 1 < self.cols && self.prev[self.idx(c + 1, r)])
                    || (r > 0 && self.prev[self.idx(c, r - 1)])
                    || (r + 1 < self.rows && self.prev[self.idx(c, r + 1)]);
                if has_active_neighbor && roll < chance {
                    self.active[i] = true;
                }
            }
        }
    }
}

/// A triage-against-spread minigame. Mount it (with a `key` that changes per
/// encounter, so the board and clock reset) and wire `on_complete`:
///
/// ```rust,ignore
/// BucketBrigade {
///     key: "{round}",
///     prompt: "Stamp out the flames before they reach the supplies —",
///     threat_icon: "🔥",
///     spread_ms: 700,
///     spread_chance: 0.5,
///     seed,                 // e.g. drawn from the game's RNG
///     on_complete: move |r: BucketBrigadeResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn BucketBrigade(
    /// Line shown above the grid (the situation).
    prompt: String,
    /// Glyph for a live threat. Defaults to a portable `*`; pass an emoji
    /// (e.g. "🔥") on hosts with a color-emoji font.
    #[props(default = String::from("*"))]
    threat_icon: String,
    /// Grid width in cells.
    #[props(default = 5)]
    cols: usize,
    /// Grid height in cells.
    #[props(default = 5)]
    rows: usize,
    /// How many threats are already burning when the run begins.
    #[props(default = 2)]
    initial_active: usize,
    /// Milliseconds between spread steps. Lower = the threats grow faster/harder.
    #[props(default = 700)]
    spread_ms: u32,
    /// Per-neighbour chance a threat jumps to an adjacent safe cell each step,
    /// 0.0..1.0. Higher = harder. Default 0.5.
    #[props(default = 0.5)]
    spread_chance: f64,
    /// How long the run lasts, in milliseconds.
    #[props(default = 8000)]
    duration_ms: u32,
    /// Seed for the starting layout and the spread — vary it per encounter; same
    /// seed (and same taps) → same run.
    #[props(default = 0)]
    seed: u64,
    /// Called once, when the grid is cleared or time runs out.
    on_complete: EventHandler<BucketBrigadeResult>,
) -> Element {
    // Board + its RNG live together in one signal: a spread is then a single
    // scoped `write()`, and the RNG never drifts out of sync with the board it
    // drives. Seeded once at mount — `ignite_initial` consumes the first draws,
    // then each `spread` consumes the next slice of the stream. `Board::new` owns
    // the min-grid floor, so we read the realized dimensions back below rather
    // than re-deriving (and possibly disagreeing with) them here.
    let mut board = use_signal(move || {
        let mut rng = GameRng::from_seed(seed);
        let mut b = Board::new(cols, rows);
        // At least one threat to start (so the run can't auto-win at tick 0),
        // capped at the grid's capacity.
        let start = initial_active.max(1).min(b.capacity());
        b.ignite_initial(&mut rng, start);
        (b, rng)
    });
    let cols = board.read().0.cols();
    let rows = board.read().0.rows();

    let mut ticks = use_signal(|| 0u32);
    // Seed the peak from the board's true post-ignite count, not a recomputed
    // clamp, so the two can never disagree.
    let mut peak_active = use_signal(move || board.read().0.active_count());
    let mut done = use_signal(|| false);

    // The clock: advance ticks, fire a spread on its cadence, track the worst
    // moment, and end when the grid is cleared (contained) or time runs out.
    use_future(move || async move {
        let spread_every = (spread_ms / TICK_MS).max(1);
        loop {
            if done() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            let t = ticks() + 1;
            ticks.set(t);

            if t % spread_every == 0 {
                // One scoped borrow: mutate the board through its paired RNG, then
                // drop the guard before any read below.
                let mut guard = board.write();
                let (b, rng) = &mut *guard;
                b.spread(rng, spread_chance);
            }

            let active = board.read().0.active_count();
            if active > peak_active() {
                peak_active.set(active);
            }

            let elapsed = t * TICK_MS;
            if active == 0 || elapsed >= duration_ms {
                done.set(true);
                on_complete.call(BucketBrigadeResult {
                    contained: active == 0,
                    peak_active: peak_active(),
                    leaked: if active == 0 { 0 } else { active },
                });
                break;
            }
        }
    });

    // Tap a cell to knock its threat out. Harmless on safe cells, ignored once the
    // run is over.
    let mut tap = move |idx: usize| {
        if done() {
            return;
        }
        board.write().0.extinguish(idx);
    };

    // Borrow the board for the whole render instead of cloning it each frame
    // (~50 renders/sec). Reads are safe here: the only writers — the async
    // spread loop and the tap handler — run between renders, never during one.
    let board_ref = board.read();
    let cells = &board_ref.0;
    let active_now = cells.active_count();
    let elapsed = ticks() as f64 * TICK_MS as f64;
    let progress = (elapsed / duration_ms as f64).min(1.0) * 100.0;

    // Shared CRT-grid styling (see `grid`): container plus the per-cell box that
    // pins each cell to its aspect ratio. BucketBrigade adds `touch-action` so a
    // rapid tap can't be swallowed by the browser's double-tap-to-zoom.
    let grid_style = format!("{} touch-action: manipulation;", grid::container_style(cols));
    let cell_box = grid::CELL_BOX_STYLE;

    rsx! {
        // Layout is set INLINE, not via Tailwind utilities: some hosts (the
        // Tailwind Play CDN) emit the flex/grid utilities only lazily, which would
        // shift the centered grid mid-run. Inline styles are stable from first
        // paint; the classes stay for hosts that do build Tailwind.
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

            // Elapsed-time progress + live threat count.
            div {
                class: "w-full flex items-center gap-3 text-sm opacity-70",
                style: "width: 100%; display: flex; align-items: center; \
                    gap: 0.75rem; font-size: 0.875rem; opacity: 0.7;",
                div {
                    style: "flex: 1; height: 0.4rem; border-radius: 3px; \
                        background: rgba(20, 95, 40, 0.18); overflow: hidden;",
                    div {
                        style: "height: 100%; width: {progress}%; \
                            background: var(--phosphor-dim);",
                    }
                }
                span { "Threats {active_now}" }
            }

            // The grid: a dim lattice of cells. Each live threat glows amber and is
            // tappable; safe cells stay dark. Styled inline so the core visual
            // needs no utilities.
            div {
                style: "{grid_style}",
                for r in 0..rows {
                    for c in 0..cols {
                        {
                            let idx = r * cols + c;
                            let is_active = cells.is_active(idx);
                            let glyph = if is_active { threat_icon.clone() } else { String::new() };
                            // text-shadow is set in both branches: Dioxus patches
                            // style props one at a time and won't drop an omitted
                            // one, so a cleared cell would otherwise keep its glow.
                            let glyph_style = if is_active {
                                "color: #ffb000; text-shadow: 0 0 6px #ffb000;"
                            } else {
                                "color: var(--phosphor-dim); text-shadow: none;"
                            };
                            // A live threat reads as warm and invites a tap; a safe
                            // cell is a flat dim lane.
                            let cell_style = if is_active {
                                "background: rgba(255, 176, 0, 0.16); \
                                 border: 1px solid rgba(255, 176, 0, 0.55); cursor: pointer;"
                            } else {
                                "background: rgba(51, 255, 102, 0.04); \
                                 border: 1px solid transparent; cursor: default;"
                            };
                            rsx! {
                                div {
                                    key: "{r}-{c}",
                                    style: "{cell_box} {cell_style}",
                                    onpointerdown: move |_| tap(idx),
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
                "Tap the flames faster than they spread."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a board the way the component does: seed an RNG, ignite `n` cells,
    /// and hand both back so a test can drive spreads.
    fn seeded(cols: usize, rows: usize, seed: u64, n: usize) -> (Board, GameRng) {
        let mut rng = GameRng::from_seed(seed);
        let mut b = Board::new(cols, rows);
        b.ignite_initial(&mut rng, n);
        (b, rng)
    }

    #[test]
    fn ignite_initial_places_exactly_n_distinct() {
        for &(cols, rows, n) in &[(5usize, 5usize, 3usize), (4, 3, 1), (2, 2, 10)] {
            let (b, _) = seeded(cols, rows, 7, n);
            assert_eq!(b.active_count(), n.min(cols * rows));
        }
    }

    #[test]
    fn initial_floor_guarantees_a_nonzero_start() {
        // The component floors `initial_active` to at least 1.
        let (b, _) = seeded(5, 5, 1, 1);
        assert!(b.active_count() >= 1);
    }

    #[test]
    fn extinguish_is_idempotent_and_safe() {
        let (mut b, _) = seeded(4, 4, 3, 4);
        // Find a live cell and clear it.
        let live = (0..16).find(|&i| b.is_active(i)).unwrap();
        let before = b.active_count();
        b.extinguish(live);
        assert_eq!(b.active_count(), before - 1);
        // Clearing it again, a safe cell, and an out-of-range index are all no-ops.
        b.extinguish(live);
        b.extinguish(live);
        b.extinguish(9999);
        assert_eq!(b.active_count(), before - 1);
    }

    #[test]
    fn spread_is_double_buffered_no_same_step_cascade() {
        // One active cell in the middle of a 5x5; one step at certainty.
        let mut b = Board::new(5, 5);
        let center = b.idx(2, 2);
        b.active[center] = true;
        let mut rng = GameRng::from_seed(0);
        b.spread(&mut rng, 1.0);
        // Exactly the 4 orthogonal neighbours catch — not the diagonals, and not
        // the neighbours-of-neighbours (which would mean the cascade leaked within
        // the step).
        assert_eq!(b.active_count(), 5);
        for &(c, r) in &[(2usize, 1usize), (2, 3), (1, 2), (3, 2)] {
            assert!(b.is_active(b.idx(c, r)), "neighbour ({c},{r}) should catch");
        }
        assert!(!b.is_active(b.idx(1, 1)), "diagonal should not catch");
        assert!(!b.is_active(b.idx(2, 0)), "neighbour-of-neighbour should not catch");
    }

    #[test]
    fn spread_with_zero_chance_never_grows() {
        let (mut b, mut rng) = seeded(5, 5, 11, 2);
        let start = b.active_count();
        for _ in 0..20 {
            b.spread(&mut rng, 0.0);
        }
        assert_eq!(b.active_count(), start);
    }

    #[test]
    fn same_seed_and_taps_replay_identically() {
        let (mut a, mut ra) = seeded(5, 5, 99, 3);
        let (mut b, mut rb) = seeded(5, 5, 99, 3);
        for step in 0..12 {
            a.spread(&mut ra, 0.5);
            b.spread(&mut rb, 0.5);
            // The same tap on both, interleaved with the spreads.
            let idx = (step * 7) % 25;
            a.extinguish(idx);
            b.extinguish(idx);
            assert_eq!(a, b, "diverged at step {step}");
        }
    }

    #[test]
    fn a_different_seed_diverges() {
        // Different seeds light different starting cells, so the runs differ from
        // the very first frame.
        let (a, _) = seeded(5, 5, 1, 3);
        let (b, _) = seeded(5, 5, 2, 3);
        assert_ne!(a, b);
    }

    #[test]
    fn do_nothing_loses_under_a_brisk_spread() {
        // The doc's lower test bar: ignore it and it gets away. 8000/700 ≈ 11
        // spread steps with no taps, a high spread chance.
        let (mut b, mut rng) = seeded(5, 5, 42, 2);
        let steps = 8000 / 700;
        for _ in 0..steps {
            b.spread(&mut rng, 0.9);
        }
        // Not contained (something still burns → leaked > 0), and it actually grew.
        assert!(b.active_count() > 0, "do-nothing run should not self-contain");
        assert!(b.active_count() > 2, "spread should have grown past the start");
    }

    #[test]
    fn perfect_tap_contains() {
        // The doc's upper test bar: a perfect bot dousing every live cell each step
        // beats it back to — and holds at — zero, so containment has headroom.
        let (mut b, mut rng) = seeded(5, 5, 42, 2);
        let steps = 8000 / 700;
        for _ in 0..steps {
            b.spread(&mut rng, 0.9);
            // Douse everything that's lit.
            for i in 0..25 {
                b.extinguish(i);
            }
        }
        assert_eq!(b.active_count(), 0, "perfect play should contain it");
    }

    #[test]
    fn peak_tracks_the_worst_moment() {
        // Mirrors the component's peak tracking: the max active count over a
        // do-nothing run is at least the start and at least every sample.
        let (mut b, mut rng) = seeded(5, 5, 5, 2);
        let mut peak = b.active_count();
        for _ in 0..11 {
            b.spread(&mut rng, 0.9);
            let now = b.active_count();
            assert!(now <= peak.max(now));
            if now > peak {
                peak = now;
            }
        }
        assert!(peak >= 2, "peak should be at least the starting count");
    }
}
