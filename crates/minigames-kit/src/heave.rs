//! Heave — a sustained-exertion-against-resistance test.
//!
//! The player presses and holds to pour force into a *load* bar; the load eases
//! back when released. The resistance gives way in *stages* — each stage needs a
//! higher load held briefly to clear it — while a *grip* meter drains the whole
//! time, faster while bearing down. Bear down too hard and the load crosses a
//! *slip* ceiling: you lose your hold (load snaps to zero) and grip takes a hit.
//! The skill is pacing — heave, breathe, heave — not mashing.
//!
//! Where TimingBar is a single instant and SteadyHands is fine *correction*,
//! Heave integrates a run of *effort*: it's the cave's `>STRAIN!<` verb (prying
//! the five-foot clam with the trident, forcing the oil-freed door). It reports
//! back via [`HeaveResult`] — opened/not, how many stages gave, how often you
//! slipped, and the grip left — and leaves scoring to the host.
//!
//! Timing uses a 20 ms tick counter (gloo-timers) rather than a wall clock; the
//! per-tick physics is a pure function and the stage curve is seeded, so the
//! component adds no nondeterminism of its own. Input is press-vs-not only — no
//! coordinate math — so it works one-thumbed on a touchscreen.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use retro_kit::rng::GameRng;
use retro_kit::theme::SCREEN_CENTERED;

/// Granularity of the tick clock, in milliseconds (~50 fps).
pub const TICK_MS: u32 = 20;

/// Grip drains even when not pressing — holding a five-foot clam open is tiring
/// between heaves — at this fraction of the pressed drain. Keeping it nonzero
/// guarantees the run always terminates (a do-nothing player eventually fails),
/// and gives the whole thing a gentle time pressure.
const IDLE_DRAIN_FACTOR: f64 = 0.5;

/// Lowest and highest stage thresholds. The top stays below any sane `slip_load`
/// so the puzzle is always solvable by easing off near the ceiling.
const THRESHOLD_LO: f64 = 0.35;
const THRESHOLD_HI: f64 = 0.85;

/// The outcome of one Heave run, handed to the host's `on_complete`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeaveResult {
    /// Cleared every resistance stage before grip ran out (the clam opened,
    /// the door swung).
    pub opened: bool,
    /// How many stages gave way (== `stages` when `opened`).
    pub stages_cleared: usize,
    /// Total stages this run asked for.
    pub stages: usize,
    /// Over-exertions — times the load snapped past the slip ceiling. A host can
    /// map these to wasted turns / extra grip lost.
    pub slips: usize,
    /// Grip remaining at the end, 0.0..1.0.
    pub grip_left: f64,
}

/// Tunable physics for a run, bundled so the pure step is easy to test.
#[derive(Debug, Clone, Copy)]
struct Cfg {
    stages: usize,
    heave_rate: f64,
    relax_rate: f64,
    grip_drain: f64,
    hold_ticks: u32,
    slip_load: f64,
    slip_penalty: f64,
    seed: u64,
}

/// The load required to start clearing `stage` (of `stages`), 0.0..1.0. Strictly
/// increasing in `stage` and bounded within `(THRESHOLD_LO, THRESHOLD_HI]`, so
/// the sequence is always climbable and the top stage sits below the slip
/// ceiling. Jittered per `seed` (bounded to a quarter-gap, which preserves the
/// strict ordering) so the curve can't be memorized.
fn stage_threshold(stage: usize, stages: usize, seed: u64) -> f64 {
    if stages <= 1 {
        return THRESHOLD_HI;
    }
    let gap = (THRESHOLD_HI - THRESHOLD_LO) / (stages as f64 - 1.0);
    let base = THRESHOLD_LO + gap * stage as f64;
    // Deterministic per-stage jitter: advance the seeded RNG `stage + 1` times so
    // each stage gets its own draw, bounded to ±gap/4 to keep ordering.
    let mut rng = GameRng::from_seed(seed);
    let mut jitter = 0.0;
    for _ in 0..=stage {
        jitter = (rng.uniform() * 2.0 - 1.0) * gap * 0.25;
    }
    (base + jitter).clamp(THRESHOLD_LO * 0.5, THRESHOLD_HI)
}

/// The evolving run state. Kept separate from the UI so the per-tick physics is
/// pure and unit-testable without a DOM.
#[derive(Debug, Clone, Copy)]
struct HeaveState {
    load: f64,
    grip: f64,
    /// Index of the stage currently being worked; `== stages` once opened.
    stage: usize,
    /// Consecutive ticks the current threshold has been held.
    hold: u32,
    slips: usize,
}

impl HeaveState {
    fn new() -> Self {
        HeaveState {
            load: 0.0,
            grip: 1.0,
            stage: 0,
            hold: 0,
            slips: 0,
        }
    }

    fn finished(&self, cfg: &Cfg) -> bool {
        self.stage >= cfg.stages || self.grip <= 0.0
    }

    /// Advance one tick. `pressed` is whether the player is bearing down.
    fn step(&mut self, pressed: bool, cfg: &Cfg) {
        if self.finished(cfg) {
            return;
        }
        if pressed {
            self.load = (self.load + cfg.heave_rate).min(1.0);
            self.grip -= cfg.grip_drain;
        } else {
            self.load = (self.load - cfg.relax_rate).max(0.0);
            self.grip -= cfg.grip_drain * IDLE_DRAIN_FACTOR;
        }
        if self.grip <= 0.0 {
            self.grip = 0.0;
            return; // out of grip — finished() now reports a failed run.
        }
        // Over-exertion: shoving past the ceiling snaps your hold.
        if self.load > cfg.slip_load {
            self.slips += 1;
            self.grip = (self.grip - cfg.slip_penalty).max(0.0);
            self.load = 0.0;
            self.hold = 0;
            return;
        }
        // Hold at or above the current stage's threshold to make it give.
        if self.stage < cfg.stages {
            let thr = stage_threshold(self.stage, cfg.stages, cfg.seed);
            if self.load >= thr {
                self.hold += 1;
                if self.hold >= cfg.hold_ticks {
                    self.stage += 1; // this stage gave way
                    self.hold = 0;
                    self.load = 0.0; // the resistance releases; build again for the next
                }
            } else {
                self.hold = 0; // dipped below — the hold has to be unbroken
            }
        }
    }

    fn result(&self, cfg: &Cfg) -> HeaveResult {
        HeaveResult {
            opened: self.stage >= cfg.stages,
            stages_cleared: self.stage.min(cfg.stages),
            stages: cfg.stages,
            slips: self.slips,
            grip_left: self.grip.max(0.0),
        }
    }
}

/// A sustained-exertion minigame. Mount it (with a `key` that changes per
/// encounter, so the curve and meters reset) and wire `on_complete`:
///
/// ```rust,ignore
/// Heave {
///     key: "{round}",
///     prompt: "Pry the clam open —",
///     stages: 3,
///     seed,                 // e.g. derived from game state
///     on_complete: move |r: HeaveResult| { /* score it */ },
/// }
/// ```
#[component]
pub fn Heave(
    /// Line shown above the field (the situation: "Pry the clam open —").
    prompt: String,
    /// Resistance steps to clear. Default 3.
    #[props(default = 3)]
    stages: usize,
    /// Load gained per tick while pressed. Higher = easier. Default 0.020.
    #[props(default = 0.020)]
    heave_rate: f64,
    /// Load lost per tick while released. Default 0.015.
    #[props(default = 0.015)]
    relax_rate: f64,
    /// Grip lost per tick while pressed (and `IDLE_DRAIN_FACTOR`× that when not).
    /// Default 0.006.
    #[props(default = 0.006)]
    grip_drain: f64,
    /// Consecutive ticks a stage threshold must be held to clear it. Default 8.
    #[props(default = 8)]
    hold_ticks: u32,
    /// Over-exertion ceiling — load above this slips your grip. Default 0.92.
    #[props(default = 0.92)]
    slip_load: f64,
    /// Grip lost per slip. Default 0.20.
    #[props(default = 0.20)]
    slip_penalty: f64,
    /// Seed for the stage curve — vary it per encounter; same seed → same curve.
    #[props(default = 0)]
    seed: u64,
    /// Called once, when the run completes (opened or out of grip).
    on_complete: EventHandler<HeaveResult>,
) -> Element {
    let cfg = Cfg {
        stages,
        heave_rate,
        relax_rate,
        grip_drain,
        hold_ticks,
        slip_load,
        slip_penalty,
        seed,
    };

    let mut state = use_signal(HeaveState::new);
    let mut pressed = use_signal(|| false);
    let mut touched = use_signal(|| false);
    let mut done = use_signal(|| false);

    // Drive the run: step the physics each tick until opened or grip runs out.
    use_future(move || async move {
        loop {
            if done() {
                break;
            }
            TimeoutFuture::new(TICK_MS).await;
            let mut s = state();
            s.step(pressed(), &cfg);
            state.set(s);
            if s.finished(&cfg) {
                done.set(true);
                on_complete.call(s.result(&cfg));
                break;
            }
        }
    });

    let s = state();
    let thr = if s.stage < stages {
        stage_threshold(s.stage, stages, seed)
    } else {
        1.0
    };
    // Geometry as field-height percentages (the bar fills from the bottom).
    let load_pct = s.load * 100.0;
    let grip_pct = s.grip * 100.0;
    let thr_pct = thr * 100.0;
    let ceil_pct = slip_load * 100.0;
    let stage_label = s.stage.min(stages);
    // Fill turns brighter once you're in the holding band (≥ threshold), and
    // warns as you approach the slip ceiling.
    let near_ceiling = s.load > slip_load - 0.06;
    let fill_color = if near_ceiling {
        "rgba(255, 170, 51, 0.55)" // amber: easing-off zone
    } else if s.load >= thr {
        "rgba(51, 255, 102, 0.45)" // in the holding band
    } else {
        "rgba(51, 255, 102, 0.22)" // still building
    };

    rsx! {
        // Layout set inline (stable from first paint) rather than via Tailwind
        // utilities, which the Play CDN applies lazily; classes are kept for
        // hosts that compile Tailwind.
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

            // Grip meter + stage readout.
            div {
                class: "w-full flex items-center gap-3 text-sm opacity-70",
                style: "width: 100%; display: flex; align-items: center; \
                    gap: 0.75rem; font-size: 0.875rem; opacity: 0.7;",
                span { "Grip" }
                div {
                    style: "flex: 1; height: 0.4rem; border-radius: 3px; \
                        background: rgba(20, 95, 40, 0.18); overflow: hidden;",
                    div {
                        style: "height: 100%; width: {grip_pct}%; \
                            background: var(--phosphor-dim);",
                    }
                }
                span { "Stage {stage_label}/{stages}" }
            }

            // The field: press and hold anywhere to heave. A vertical load bar
            // fills from the bottom; the current stage threshold and the slip
            // ceiling are drawn as lines so the player can see the holding band.
            // `touch-action: none` so a long press traces effort instead of
            // selecting text / scrolling.
            div {
                onpointerdown: move |_| {
                    if done() {
                        return;
                    }
                    touched.set(true);
                    pressed.set(true);
                },
                onpointerup: move |_| pressed.set(false),
                onpointercancel: move |_| pressed.set(false),
                onpointerleave: move |_| pressed.set(false),
                style: "position: relative; width: 100%; aspect-ratio: 3 / 4; \
                    max-height: 60vh; border: 1px solid var(--phosphor-dim); \
                    border-radius: 4px; background: rgba(20, 95, 40, 0.12); \
                    overflow: hidden; touch-action: none; cursor: pointer; \
                    user-select: none;",

                // Load fill, rising from the bottom.
                div {
                    style: "position: absolute; left: 0; right: 0; bottom: 0; \
                        height: {load_pct}%; background: {fill_color}; \
                        border-top: 2px solid var(--phosphor); \
                        box-shadow: var(--glow); transition: height 40ms linear;",
                }
                // Slip ceiling — stay below this.
                div {
                    style: "position: absolute; left: 0; right: 0; \
                        bottom: {ceil_pct}%; height: 0; \
                        border-top: 1px dashed rgba(255, 120, 90, 0.8);",
                }
                // Current stage threshold — hold the fill at/above here.
                div {
                    style: "position: absolute; left: 0; right: 0; \
                        bottom: {thr_pct}%; height: 0; \
                        border-top: 1px solid var(--phosphor);",
                }

                // Prompt to start, before the first press.
                if !touched() {
                    div {
                        style: "position: absolute; inset: 0; display: flex; \
                            align-items: center; justify-content: center; \
                            pointer-events: none; text-align: center; \
                            opacity: 0.6; font-size: 0.9rem; padding: 1rem;",
                        "Press and hold to heave — get the load above the line and hold it, but stay under the dashed ceiling."
                    }
                }
            }

            p {
                class: "text-center text-sm opacity-60",
                style: "text-align: center; font-size: 0.875rem; opacity: 0.6;",
                "Heave in bursts: hold above the line, ease off before the dashed ceiling."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(stages: usize, seed: u64) -> Cfg {
        Cfg {
            stages,
            heave_rate: 0.020,
            relax_rate: 0.015,
            grip_drain: 0.006,
            hold_ticks: 8,
            slip_load: 0.92,
            slip_penalty: 0.20,
            seed,
        }
    }

    #[test]
    fn thresholds_are_strictly_increasing_and_below_the_ceiling() {
        for seed in 0..64u64 {
            for stages in 2..=6usize {
                let mut prev = -1.0;
                for stage in 0..stages {
                    let t = stage_threshold(stage, stages, seed);
                    assert!(t > prev, "not increasing at seed {seed}, stage {stage}: {t} <= {prev}");
                    assert!((0.0..=THRESHOLD_HI).contains(&t), "off-range at seed {seed}: {t}");
                    prev = t;
                }
                // The top stage must be reachable without slipping.
                let top = stage_threshold(stages - 1, stages, seed);
                assert!(top < 0.92, "top stage {top} not below the slip ceiling at seed {seed}");
            }
        }
    }

    #[test]
    fn thresholds_are_deterministic_per_seed_and_vary_across_seeds() {
        assert_eq!(stage_threshold(1, 3, 7), stage_threshold(1, 3, 7));
        assert_ne!(stage_threshold(1, 3, 7), stage_threshold(1, 3, 8));
    }

    /// Run to completion under a controller that decides `pressed` each tick.
    /// Returns the final result. Bounded so a bug can't hang the test.
    fn run(cfg: &Cfg, mut controller: impl FnMut(&HeaveState) -> bool) -> HeaveResult {
        let mut s = HeaveState::new();
        for _ in 0..100_000 {
            if s.finished(cfg) {
                break;
            }
            let pressed = controller(&s);
            s.step(pressed, cfg);
        }
        assert!(s.finished(cfg), "run did not terminate");
        s.result(cfg)
    }

    #[test]
    fn never_pressing_fails_with_no_progress() {
        let r = run(&cfg(3, 1), |_| false);
        assert!(!r.opened);
        assert_eq!(r.stages_cleared, 0);
        assert_eq!(r.grip_left, 0.0);
        assert_eq!(r.slips, 0);
    }

    #[test]
    fn mashing_slips_and_cannot_open() {
        let r = run(&cfg(3, 1), |_| true);
        assert!(!r.opened, "a masher should not be able to clear the top stage");
        assert!(r.slips > 0, "a masher should over-exert and slip");
    }

    #[test]
    fn pacing_in_the_band_opens_it() {
        // Aim for the middle of the holding band [threshold, slip_load): press
        // when below it, ease off when above. This never crosses the ceiling and
        // keeps an unbroken hold, so every stage gives way.
        let c = cfg(3, 1);
        let r = run(&c, |s| {
            let thr = if s.stage < c.stages {
                stage_threshold(s.stage, c.stages, c.seed)
            } else {
                0.0
            };
            let mid = (thr + c.slip_load) * 0.5;
            s.load < mid
        });
        assert!(r.opened, "a paced player should open it: {r:?}");
        assert_eq!(r.stages_cleared, 3);
        assert_eq!(r.slips, 0);
        assert!(r.grip_left > 0.0);
    }

    #[test]
    fn a_single_stage_run_is_well_defined() {
        let c = cfg(1, 0);
        let r = run(&c, |s| {
            let thr = stage_threshold(0, 1, 0);
            let mid = (thr + c.slip_load) * 0.5;
            s.load < mid
        });
        assert!(r.opened);
        assert_eq!(r.stages_cleared, 1);
        assert_eq!(r.stages, 1);
    }
}
