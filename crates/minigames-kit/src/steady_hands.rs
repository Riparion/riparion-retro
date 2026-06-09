//! SteadyHands — a touch-first precision tracing test.
//!
//! A target reticle wanders the play field on a deterministic, seeded path. The
//! player drags a finger along the field to keep it inside the ring — the
//! cursor tracks the fingertip directly. The ring is sized generously enough
//! that the finger needn't be offset out of the way, and nothing relies on
//! hover, so it works one-thumbed on a touchscreen.
//!
//! Where TimingBar samples a single instant, SteadyHands integrates over a whole
//! run of continuous correction: accuracy is the fraction of the run the cursor
//! was held on-target. It reports that back via [`SteadyHandsResult`] —
//! steady/not, the on-target fraction, and the longest unbroken streak — and
//! leaves scoring to the host.
//!
//! Timing uses a 20 ms tick counter (gloo-timers) rather than a wall clock, and
//! the target's path is a pure function of elapsed time and `seed`, so the
//! component adds no nondeterminism of its own.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;
use retro_kit::theme::SCREEN_CENTERED;

/// Granularity of the tick clock, in milliseconds (~50 fps).
pub const TICK_MS: u32 = 20;

/// The target stays within `[MARGIN, 1 - MARGIN]` on both axes so it never
/// rides the very edge of the field.
const MARGIN: f64 = 0.15;

/// The outcome of one SteadyHands run, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SteadyHandsResult {
    /// Whether the on-target fraction met `pass_threshold`.
    pub steady: bool,
    /// Fraction of the run the cursor was held on target, 0.0..1.0.
    pub accuracy: f64,
    /// Longest unbroken on-target span, as a fraction 0.0..1.0 of the run.
    pub longest_streak: f64,
}

/// Where the target sits at `elapsed_ms`, as `(x, y)` fractions 0.0..1.0 of the
/// field. A seeded sum-of-sines (Lissajous-style) wander: deterministic for a
/// given seed, and bounded by construction so it can never leave the field.
///
/// Both axes keep the full `[MARGIN, 1 - MARGIN]` band — the cursor tracks the
/// fingertip directly, so anywhere on the field is reachable. `drift_speed`
/// multiplies the wander frequency.
fn target_position(elapsed_ms: f64, seed: u64, drift_speed: f64) -> (f64, f64) {
    use std::f64::consts::TAU;
    let mut rng = GameRng::from_seed(seed);
    // Two frequency/phase pairs per axis. Frequencies stay modest so the path is
    // followable; phases (and the second frequency) decorrelate x from y.
    let mut comp = || {
        let f1 = 0.25 + rng.uniform() * 0.35;
        let p1 = rng.uniform() * TAU;
        let f2 = 0.45 + rng.uniform() * 0.55;
        let p2 = rng.uniform() * TAU;
        (f1, p1, f2, p2)
    };
    let (fx1, px1, fx2, px2) = comp();
    let (fy1, py1, fy2, py2) = comp();

    let t = elapsed_ms / 1000.0 * drift_speed;
    // Map a sum of two unit sines (range -2..2 → halved to -1..1) onto a band.
    let band = |raw: f64, lo: f64, hi: f64| lo + (raw * 0.5 + 0.5) * (hi - lo);
    let x = band(
        0.5 * (TAU * fx1 * t + px1).sin() + 0.5 * (TAU * fx2 * t + px2).sin(),
        MARGIN,
        1.0 - MARGIN,
    );
    let y = band(
        0.5 * (TAU * fy1 * t + py1).sin() + 0.5 * (TAU * fy2 * t + py2).sin(),
        MARGIN,
        1.0 - MARGIN,
    );
    (x, y)
}

/// Whether the cursor is within `tolerance` (normalized radius) of the target.
fn on_target(cursor: (f64, f64), target: (f64, f64), tolerance: f64) -> bool {
    let dx = cursor.0 - target.0;
    let dy = cursor.1 - target.1;
    (dx * dx + dy * dy).sqrt() <= tolerance
}

/// Folds the per-tick on-target booleans into the final result. Kept separate
/// from the UI so the accumulation is pure and unit-testable.
#[derive(Default, Clone, Copy)]
struct Accumulator {
    total: u32,
    on: u32,
    streak: u32,
    longest: u32,
}

impl Accumulator {
    fn tick(&mut self, on_target: bool) {
        self.total += 1;
        if on_target {
            self.on += 1;
            self.streak += 1;
            if self.streak > self.longest {
                self.longest = self.streak;
            }
        } else {
            self.streak = 0;
        }
    }

    /// Live on-target fraction so far (0.0 before any tick).
    fn accuracy(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.on as f64 / self.total as f64
        }
    }

    fn result(&self, pass_threshold: f64) -> SteadyHandsResult {
        let accuracy = self.accuracy();
        let longest_streak = if self.total == 0 {
            0.0
        } else {
            self.longest as f64 / self.total as f64
        };
        SteadyHandsResult {
            steady: accuracy >= pass_threshold,
            accuracy,
            longest_streak,
        }
    }
}

/// A continuous precision-tracing minigame. Mount it (with a `key` that changes
/// per encounter, so the path and clock reset) and wire `on_complete`:
///
/// ```rust,ignore
/// SteadyHands {
///     key: "{round}",
///     prompt: "Hold the team on line across the current —",
///     tolerance: 0.12,
///     duration_ms: 6000,
///     seed,                 // e.g. drawn from the game's RNG
///     on_complete: move |r: SteadyHandsResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn SteadyHands(
    /// Line shown above the field (the situation).
    prompt: String,
    /// On-target radius in normalized field units. Smaller = harder. Default 0.12.
    #[props(default = 0.12)]
    tolerance: f64,
    /// How long the run lasts, in milliseconds.
    #[props(default = 6000)]
    duration_ms: u32,
    /// Multiplies the wander frequency; higher = the target moves faster/harder.
    #[props(default = 1.0)]
    drift_speed: f64,
    /// On-target fraction needed for `steady`. Default 0.60.
    #[props(default = 0.60)]
    pass_threshold: f64,
    /// Seed for the wander path — vary it per encounter; same seed → same path.
    #[props(default = 0)]
    seed: u64,
    /// Called once, when the run completes.
    on_complete: EventHandler<SteadyHandsResult>,
) -> Element {
    let mut ticks = use_signal(|| 0u32);
    let mut acc = use_signal(Accumulator::default);
    // Fingertip position as a 0..1 fraction of the field; None before first touch.
    let mut finger = use_signal(|| None::<(f64, f64)>);
    let mut pressed = use_signal(|| false);
    // The field's client rect (origin x/y, width, height) and a handle to the
    // field element so we can re-query that rect. We can't trust a mount-time
    // snapshot: the Tailwind Play CDN loads asynchronously and reflows the layout
    // *after* `onmounted` fires (the field shifts down and narrows), which would
    // make every pointer-to-fraction conversion wrong. So we re-capture the rect
    // fresh on each press instead.
    let mut field_rect = use_signal(|| None::<(f64, f64, f64, f64)>);
    let mut field_el = use_signal(|| None::<std::rc::Rc<MountedData>>);
    let mut done = use_signal(|| false);

    // Drive the run: accumulate on-target ticks until the duration elapses.
    use_future(move || async move {
        loop {
            if done() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            let t = ticks() + 1;
            ticks.set(t);
            let elapsed = t as f64 * TICK_MS as f64;
            let target = target_position(elapsed, seed, drift_speed);
            let on = match finger() {
                Some(cursor) => on_target(cursor, target, tolerance),
                None => false,
            };
            acc.write().tick(on);
            if elapsed >= duration_ms as f64 {
                done.set(true);
                on_complete.call(acc().result(pass_threshold));
                break;
            }
        }
    });

    // Translate a pointer event into a clamped 0..1 fraction within the field.
    let to_fraction = move |evt: &Event<PointerData>| -> Option<(f64, f64)> {
        let (ox, oy, w, h) = field_rect()?;
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let c = evt.client_coordinates();
        Some((
            ((c.x - ox) / w).clamp(0.0, 1.0),
            ((c.y - oy) / h).clamp(0.0, 1.0),
        ))
    };

    let elapsed = ticks() as f64 * TICK_MS as f64;
    let (tx, ty) = target_position(elapsed, seed, drift_speed);
    let progress = (elapsed / duration_ms as f64).min(1.0) * 100.0;
    let live_pct = (acc().accuracy() * 100.0) as i32;

    // Geometry as field percentages.
    let target_left = tx * 100.0;
    let target_top = ty * 100.0;
    let cursor = finger().map(|(fx, fy)| (fx * 100.0, fy * 100.0));
    let tol_pct = tolerance * 100.0;

    rsx! {
        // Layout is set INLINE, not via Tailwind utilities. The Play CDN applies
        // its classes lazily (on the first DOM mutation), and `justify-center`
        // flipping in mid-run would reflow the field *under* our pointer-rect
        // measurement — breaking the pointer-to-fraction math. Inline styles are
        // stable from first paint. Classes are kept alongside for hosts that
        // build Tailwind.
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

            // Elapsed-time progress + live on-target readout.
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
                span { "On target {live_pct}%" }
            }

            // The field: a framed lane carrying the drifting target and the
            // cursor. Styled inline so the core visual stands without Tailwind
            // utilities; `touch-action: none` so dragging traces instead of
            // scrolling the page.
            div {
                onmounted: move |evt: Event<MountedData>| async move {
                    // Keep a handle so we can re-query the rect later, and capture
                    // an initial rect now. The layout is inline (stable from first
                    // paint), so this mount-time value is reliable; the pointer-down
                    // refresh below is then just belt-and-suspenders.
                    field_el.set(Some(evt.data()));
                    if let Ok(rect) = evt.get_client_rect().await {
                        field_rect.set(Some((
                            rect.origin.x,
                            rect.origin.y,
                            rect.size.width,
                            rect.size.height,
                        )));
                    }
                },
                onpointerdown: move |evt: Event<PointerData>| async move {
                    pressed.set(true);
                    // Re-capture the field's rect *now* — the layout may have
                    // reflowed since mount — then convert this press to a fraction.
                    if let Some(el) = field_el() {
                        if let Ok(rect) = el.get_client_rect().await {
                            field_rect.set(Some((
                                rect.origin.x,
                                rect.origin.y,
                                rect.size.width,
                                rect.size.height,
                            )));
                        }
                    }
                    if let Some(f) = to_fraction(&evt) {
                        finger.set(Some(f));
                    }
                },
                onpointermove: move |evt: Event<PointerData>| {
                    // No reflow happens mid-drag, so the press-time rect holds.
                    if pressed() {
                        if let Some(f) = to_fraction(&evt) {
                            finger.set(Some(f));
                        }
                    }
                },
                onpointerup: move |_| pressed.set(false),
                onpointercancel: move |_| pressed.set(false),
                style: "position: relative; width: 100%; aspect-ratio: 4 / 3; \
                    border: 1px solid var(--phosphor-dim); border-radius: 4px; \
                    background: rgba(20, 95, 40, 0.12); overflow: hidden; \
                    touch-action: none; cursor: crosshair;",

                // The drifting target: a ring sized to the on-target tolerance.
                div {
                    style: "position: absolute; left: {target_left}%; top: {target_top}%; \
                        width: {tol_pct * 2.0}%; aspect-ratio: 1; \
                        transform: translate(-50%, -50%); border-radius: 50%; \
                        border: 2px solid var(--phosphor); box-shadow: var(--glow); \
                        background: rgba(51, 255, 102, 0.10);",
                    // Center pip.
                    div {
                        style: "position: absolute; left: 50%; top: 50%; \
                            width: 4px; height: 4px; margin: -2px 0 0 -2px; \
                            border-radius: 50%; background: var(--phosphor);",
                    }
                }

                // The controlled cursor, tracking the fingertip.
                match cursor {
                    Some((cx, cy)) => rsx! {
                        div {
                            style: "position: absolute; left: {cx}%; top: {cy}%; \
                                width: 18px; height: 18px; margin: -9px 0 0 -9px; \
                                pointer-events: none; \
                                color: var(--phosphor); font-weight: bold; \
                                line-height: 18px; text-align: center;",
                            "✛"
                        }
                    },
                    None => rsx! {
                        div {
                            style: "position: absolute; inset: 0; \
                                display: flex; align-items: center; justify-content: center; \
                                pointer-events: none; text-align: center; \
                                opacity: 0.6; font-size: 0.9rem;",
                            "Touch the field and drag — keep the ✛ inside the ring."
                        }
                    },
                }
            }

            p {
                class: "text-center text-sm opacity-60",
                style: "text-align: center; font-size: 0.875rem; opacity: 0.6;",
                "Keep your finger inside the ring as it drifts."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_stays_on_the_field_across_seeds_and_time() {
        for seed in 0..64u64 {
            for step in 0..400 {
                let elapsed = step as f64 * 25.0; // 0..10s
                let (x, y) = target_position(elapsed, seed, 1.0);
                assert!(
                    (MARGIN..=1.0 - MARGIN).contains(&x),
                    "target x off-field at seed {seed}, t={elapsed}: {x}"
                );
                assert!(
                    (MARGIN..=1.0 - MARGIN).contains(&y),
                    "target y off-field at seed {seed}, t={elapsed}: {y}"
                );
            }
        }
    }

    #[test]
    fn target_is_deterministic_per_seed() {
        let a = target_position(1234.0, 7, 1.0);
        let b = target_position(1234.0, 7, 1.0);
        assert_eq!(a, b);
        // A different seed should (essentially always) give a different point.
        let c = target_position(1234.0, 8, 1.0);
        assert!(a != c);
    }

    #[test]
    fn on_target_is_a_normalized_distance_check() {
        assert!(on_target((0.5, 0.5), (0.5, 0.5), 0.12)); // dead on
        assert!(on_target((0.5, 0.58), (0.5, 0.5), 0.12)); // within radius
        assert!(!on_target((0.5, 0.7), (0.5, 0.5), 0.12)); // outside radius
        // Distance is Euclidean, not per-axis.
        assert!(!on_target((0.6, 0.6), (0.5, 0.5), 0.12)); // ~0.141 > 0.12
    }

    #[test]
    fn all_on_target_is_perfect_and_steady() {
        let mut acc = Accumulator::default();
        for _ in 0..100 {
            acc.tick(true);
        }
        let r = acc.result(0.60);
        assert!(r.steady);
        assert!((r.accuracy - 1.0).abs() < 1e-9);
        assert!((r.longest_streak - 1.0).abs() < 1e-9);
    }

    #[test]
    fn never_touched_is_zero_and_not_steady() {
        let mut acc = Accumulator::default();
        for _ in 0..100 {
            acc.tick(false);
        }
        let r = acc.result(0.60);
        assert!(!r.steady);
        assert_eq!(r.accuracy, 0.0);
        assert_eq!(r.longest_streak, 0.0);
    }

    #[test]
    fn partial_run_reports_the_expected_fraction() {
        let mut acc = Accumulator::default();
        for _ in 0..70 {
            acc.tick(true);
        }
        for _ in 0..30 {
            acc.tick(false);
        }
        let r = acc.result(0.60);
        assert!(r.steady); // 0.70 >= 0.60
        assert!((r.accuracy - 0.70).abs() < 1e-9);
        // The one unbroken span of 70 on-target ticks out of 100.
        assert!((r.longest_streak - 0.70).abs() < 1e-9);
    }

    #[test]
    fn longest_streak_tracks_the_best_unbroken_span() {
        let mut acc = Accumulator::default();
        // on x3, off, on x5, off, on x2  → longest is 5 of 12.
        for on in [
            true, true, true, false, true, true, true, true, true, false, true, true,
        ] {
            acc.tick(on);
        }
        let r = acc.result(0.60);
        assert!((r.longest_streak - 5.0 / 12.0).abs() < 1e-9);
    }

    #[test]
    fn empty_accumulator_is_well_defined() {
        let r = Accumulator::default().result(0.60);
        assert_eq!(r.accuracy, 0.0);
        assert_eq!(r.longest_streak, 0.0);
        assert!(!r.steady);
    }
}
