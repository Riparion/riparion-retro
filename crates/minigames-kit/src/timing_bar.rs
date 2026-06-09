//! TimingBar — a rhythm-matching sweep test.
//!
//! A marker sweeps across a track that holds a highlighted target zone; the
//! player taps the action button when the marker is over the zone. Pure timing:
//! trivial to grasp, hard to nail. The host turns the result into whatever it
//! models — a lock springing open, a clean incision, a sword strike landing
//! true.
//!
//! It reports back where the marker was via [`TimingResult`] — position, the
//! target it was aiming at, how far off it landed, whether it was inside the
//! zone, and a 0..1 accuracy (1.0 dead center, 0.0 at the zone's edge). Scoring
//! stays in the host game's engine.
//!
//! Fully parameterized: target position and zone width, sweep speed, and
//! whether the marker bounces back and forth or wraps around. Leave `target`
//! unset and it's randomized from `seed`, so the zone never sits in the same
//! place and players can't memorize it — deterministic for a given seed,
//! varied across encounters.
//!
//! Timing uses a 20 ms tick counter (gloo-timers) rather than a wall clock, so
//! the marker's position is a pure function of elapsed ticks and the component
//! adds no nondeterminism of its own.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;
use retro_kit::theme::{BTN_PRIMARY, SCREEN_CENTERED};

/// Granularity of the sweep clock, in milliseconds (~50 fps).
pub const TICK_MS: u32 = 20;

/// The outcome of one TimingBar strike, handed to the host's `on_strike`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimingResult {
    /// Marker position at the moment of the strike, 0.0 (left) .. 1.0 (right).
    pub position: f64,
    /// Center of the target zone, 0.0 .. 1.0.
    pub target: f64,
    /// `|position - target|`. 0.0 is a dead-center strike.
    pub offset: f64,
    /// Whether the strike landed inside the target zone.
    pub hit: bool,
    /// 1.0 dead center, falling linearly to 0.0 at the zone edge (and outside).
    pub accuracy: f64,
}

/// Where the marker sits at `elapsed_ms`, as a fraction 0.0..1.0 of the track.
/// `bounce` ping-pongs left↔right (period = one crossing each way); otherwise
/// it wraps left→right repeatedly.
fn marker_position(elapsed_ms: f64, period_ms: f64, bounce: bool) -> f64 {
    if period_ms <= 0.0 {
        return 0.0;
    }
    let phase = elapsed_ms / period_ms;
    if bounce {
        let p = phase.rem_euclid(2.0);
        if p <= 1.0 {
            p
        } else {
            2.0 - p
        }
    } else {
        phase.rem_euclid(1.0)
    }
}

/// Pin the target zone center on the track. An explicit `target` is clamped so
/// the whole zone fits; otherwise it's drawn from `seed`.
fn target_position(target: Option<f64>, tolerance: f64, seed: u64) -> f64 {
    let tol = tolerance.clamp(0.01, 0.45);
    match target {
        Some(t) => t.clamp(tol, 1.0 - tol),
        None => {
            let mut rng = GameRng::from_seed(seed);
            tol + rng.uniform() * (1.0 - 2.0 * tol)
        }
    }
}

/// Score a strike: did it land in the zone, how far off, and how accurate.
fn resolve_strike(position: f64, target: f64, tolerance: f64) -> (bool, f64, f64) {
    let tol = tolerance.clamp(0.01, 0.45);
    let offset = (position - target).abs();
    let hit = offset <= tol;
    let accuracy = (1.0 - offset / tol).max(0.0);
    (hit, offset, accuracy)
}

/// A rhythm-matching sweep minigame. Mount it (with a `key` that changes per
/// encounter, so the marker and target reset) and wire `on_strike`:
///
/// ```rust,ignore
/// TimingBar {
///     key: "{round}",
///     prompt: "Pick the lock —",
///     action: "PICK",
///     tolerance: 0.08,   // a tight zone
///     period_ms: 1400,   // a brisk sweep
///     seed,              // e.g. drawn from the game's RNG
///     on_strike: move |r: TimingResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn TimingBar(
    /// Line shown above the track (the situation: "Strike true —").
    prompt: String,
    /// Label on the action button.
    #[props(default = String::from("STRIKE"))]
    action: String,
    /// Target zone center, 0.0..1.0. Leave unset to randomize from `seed`.
    #[props(default)]
    target: Option<f64>,
    /// Half-width of the target zone, 0.0..0.45. Smaller = harder. Default 0.10
    /// (a zone spanning 20% of the track).
    #[props(default = 0.10)]
    tolerance: f64,
    /// Milliseconds for the marker to cross the track once. Lower = faster/harder.
    #[props(default = 1600)]
    period_ms: u32,
    /// Ping-pong back and forth (true) vs. wrap left→right repeatedly (false).
    #[props(default = true)]
    bounce: bool,
    /// Seed for the randomized target (when `target` is unset) — vary it per
    /// encounter so the zone can't be memorized; same seed → same placement.
    #[props(default = 0)]
    seed: u64,
    /// Called once, when the player taps the action button.
    on_strike: EventHandler<TimingResult>,
) -> Element {
    let mut ticks = use_signal(|| 0u32);
    let mut answered = use_signal(|| false);
    // Pinned once per mount.
    let zone = use_signal(move || target_position(target, tolerance, seed));

    // Drive the sweep until the player strikes.
    use_future(move || async move {
        loop {
            if answered() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            ticks += 1;
        }
    });

    let center = zone();
    let tol = tolerance.clamp(0.01, 0.45);
    let elapsed = ticks() as f64 * TICK_MS as f64;
    let pos = marker_position(elapsed, period_ms as f64, bounce);

    // Geometry as track percentages.
    let zone_left = (center - tol) * 100.0;
    let zone_width = tol * 2.0 * 100.0;
    let marker_left = pos * 100.0;

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-6 items-center",
            p { class: "text-center text-lg opacity-80", "{prompt}" }

            // The track: a framed lane with the highlighted target zone and the
            // sweeping marker. Styled inline so it stands on its own (no tailwind
            // utilities needed for the core visual).
            div {
                style: "position: relative; width: 100%; height: 2.75rem; \
                    border: 1px solid var(--phosphor-dim); border-radius: 4px; \
                    background: rgba(20, 95, 40, 0.12); overflow: hidden;",
                // Target zone.
                div {
                    style: "position: absolute; top: 0; bottom: 0; \
                        left: {zone_left}%; width: {zone_width}%; \
                        background: rgba(51, 255, 102, 0.22); \
                        border-left: 1px solid var(--phosphor-dim); \
                        border-right: 1px solid var(--phosphor-dim);",
                }
                // Sweeping marker.
                div {
                    style: "position: absolute; top: -2px; bottom: -2px; \
                        left: {marker_left}%; width: 3px; margin-left: -1.5px; \
                        background: var(--phosphor); box-shadow: var(--glow);",
                }
            }

            p { class: "text-center text-sm opacity-60",
                "Tap when the bar crosses the lit zone."
            }

            button {
                class: "{BTN_PRIMARY} py-5 text-lg",
                onclick: move |_| {
                    if answered() {
                        return;
                    }
                    answered.set(true);
                    let now = ticks() as f64 * TICK_MS as f64;
                    let position = marker_position(now, period_ms as f64, bounce);
                    let (hit, offset, accuracy) = resolve_strike(position, center, tolerance);
                    on_strike.call(TimingResult { position, target: center, offset, hit, accuracy });
                },
                "{action}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_runs_left_to_right_and_repeats() {
        assert!((marker_position(0.0, 1000.0, false) - 0.0).abs() < 1e-9);
        assert!((marker_position(500.0, 1000.0, false) - 0.5).abs() < 1e-9);
        // Wraps back to the start after a full period.
        assert!((marker_position(1000.0, 1000.0, false) - 0.0).abs() < 1e-9);
        assert!((marker_position(1500.0, 1000.0, false) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn bounce_pings_back_at_the_far_edge() {
        assert!((marker_position(0.0, 1000.0, true) - 0.0).abs() < 1e-9);
        assert!((marker_position(1000.0, 1000.0, true) - 1.0).abs() < 1e-9); // far edge
        assert!((marker_position(1500.0, 1000.0, true) - 0.5).abs() < 1e-9); // heading back
        assert!((marker_position(2000.0, 1000.0, true) - 0.0).abs() < 1e-9); // home again
    }

    #[test]
    fn random_target_fits_on_the_track() {
        for seed in 0..64 {
            let t = target_position(None, 0.10, seed);
            assert!((0.10..=0.90).contains(&t), "zone off-track at seed {seed}: {t}");
        }
    }

    #[test]
    fn explicit_target_is_clamped_to_fit() {
        assert!((target_position(Some(0.0), 0.10, 0) - 0.10).abs() < 1e-9);
        assert!((target_position(Some(1.0), 0.10, 0) - 0.90).abs() < 1e-9);
        assert!((target_position(Some(0.5), 0.10, 0) - 0.50).abs() < 1e-9);
    }

    #[test]
    fn scoring_is_dead_center_to_edge_to_miss() {
        // Dead center: a hit at full accuracy.
        let (hit, off, acc) = resolve_strike(0.5, 0.5, 0.10);
        assert!(hit && off == 0.0 && (acc - 1.0).abs() < 1e-9);
        // Halfway to the edge: still a hit, half accuracy.
        let (hit, _, acc) = resolve_strike(0.55, 0.5, 0.10);
        assert!(hit && (acc - 0.5).abs() < 1e-9);
        // Outside the zone: a miss, zero accuracy.
        let (hit, _, acc) = resolve_strike(0.7, 0.5, 0.10);
        assert!(!hit && acc == 0.0);
    }
}
