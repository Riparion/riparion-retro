//! Jim Storer's LUNAR (1969, via lunar.bas): one 10-second turn per burn-rate
//! input, integrated with a truncated-series Tsiolkovsky rocket equation.
//! Line numbers in comments refer to lunar.bas.

use super::state::{fmt_lunar_alt, fmt_num, LogLine, LunarState, TurnRow};

/// Empty capsule mass, lb (BASIC `N`).
pub const EMPTY_MASS: f64 = 16_500.0;
/// Lunar gravity, mi/s² (BASIC `G`).
const G: f64 = 1e-3;
/// Exhaust velocity, mi/s (BASIC `Z`).
const Z: f64 = 1.8;
pub const BURN_MAX: i64 = 200;
pub const START_ALTITUDE: f64 = 120.0;

/// What a turn produced: more lines for the log, and an impact speed (MPH)
/// once the capsule is on the moon.
pub struct TurnOutcome {
    pub lines: Vec<LogLine>,
    pub impact_mph: Option<f64>,
}

pub fn fuel(st: &LunarState) -> f64 {
    st.m - EMPTY_MASS
}

pub fn report_row(st: &LunarState, burn: &str) -> TurnRow {
    TurnRow {
        sec: fmt_num(st.l.round()),
        alt: fmt_lunar_alt(st.a),
        vel: format!("{:.0} mph", 3600.0 * st.v),
        fuel: format!("{:.0} lb", fuel(st)),
        burn: burn.to_string(),
        alt_frac: (st.a / START_ALTITUDE).clamp(0.0, 1.0),
        vel_raw: 3600.0 * st.v,
        fuel_raw: fuel(st),
    }
}

/// Subroutine 420: integrate a burn of `s` seconds at rate `k`. Pure — the
/// caller applies the result (subroutine 330 used the *latest* 420 output).
fn integrate(st: &LunarState, s: f64, k: f64) -> (f64, f64) {
    let q = s * k / st.m;
    let j = st.v
        + G * s
        + Z * (-q - q * q / 2.0 - q.powi(3) / 3.0 - q.powi(4) / 4.0 - q.powi(5) / 5.0);
    let i = st.a
        - G * s * s / 2.0
        - st.v * s
        + Z * s * (q / 2.0 + q * q / 6.0 + q.powi(3) / 12.0 + q.powi(4) / 20.0 + q.powi(5) / 30.0);
    (i, j)
}

/// Subroutine 330: commit an integrated sub-step.
fn apply(st: &mut LunarState, t: &mut f64, s: f64, k: f64, i: f64, j: f64) {
    st.l += s;
    *t -= s;
    st.m -= s * k;
    st.a = i;
    st.v = j;
}

/// Lines 340–360: the capsule crossed the surface mid-step — shrink the step
/// until it lands within 5 ms, then impact. The integrate result that
/// triggered entry is discarded (BASIC re-derives S from the un-applied state).
fn touchdown_refine(st: &mut LunarState, t: &mut f64, mut s: f64, k: f64) {
    let mut guard = 0;
    while s >= 5e-3 {
        let arg = st.v * st.v + 2.0 * st.a * (G - Z * k / st.m);
        if arg < 0.0 {
            // BASIC would abort on SQR(<0); treat as "already down".
            break;
        }
        let d = st.v + arg.sqrt();
        s = 2.0 * st.a / d;
        let (i, j) = integrate(st, s, k);
        apply(st, t, s, k, i, j);
        guard += 1;
        if guard > 1_000 || !s.is_finite() {
            break;
        }
    }
}

/// One player turn: burn `burn` lb/s for up to 10 seconds (lines 150–230 and
/// the subroutine graph). Re-clamps the input; the UI never enforces rules.
pub fn step(st: &mut LunarState, burn: i64) -> TurnOutcome {
    let k = burn.clamp(0, BURN_MAX) as f64;
    let mut lines = Vec::new();
    let mut t = 10.0;
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > 10_000 {
            // Unreachable with faithful math; protects the UI from a wedge.
            return impact(st, lines);
        }
        // Line 160: out of fuel → free fall to the surface (lines 240–250).
        if fuel(st) < 1e-3 {
            lines.push(LogLine::Banner(format!("FUEL OUT AT {} SECONDS", fmt_num(st.l))));
            let s = (-st.v + (st.v * st.v + 2.0 * st.a * G).sqrt()) / G;
            st.v += G * s;
            st.l += s;
            return impact(st, lines);
        }
        // Line 170: the 10 seconds are spent — report and await the next burn.
        if t < 1e-3 {
            lines.push(LogLine::Row(report_row(st, &fmt_num(k))));
            return TurnOutcome { lines, impact_mph: None };
        }
        // Lines 180–190: cap the sub-step by remaining fuel.
        let mut s = t;
        if st.m < EMPTY_MASS + s * k {
            s = fuel(st) / k;
        }
        let (i, j) = integrate(st, s, k);
        // Line 200: crossed the surface during this step.
        if i <= 0.0 {
            touchdown_refine(st, &mut t, s, k);
            return impact(st, lines);
        }
        // Lines 210–220: started descending, would end ascending → line 370's
        // shortened step that stops right at the turnaround.
        if st.v > 0.0 && j < 0.0 {
            let w = (1.0 - st.m * G / (Z * k)) / 2.0;
            let s2 = st.m * st.v / (Z * k * (w + (w * w + st.v / Z).sqrt())) + 0.05;
            let (i2, j2) = integrate(st, s2, k);
            if i2 <= 0.0 {
                touchdown_refine(st, &mut t, s2, k);
                return impact(st, lines);
            }
            apply(st, &mut t, s2, k, i2, j2);
            continue; // lines 390–410 all return to 160
        }
        // Line 230.
        apply(st, &mut t, s, k, i, j);
    }
}

/// Lines 260+: on the moon. Returns the banner lines; classification text
/// lives in `scoring.rs`.
fn impact(st: &mut LunarState, mut lines: Vec<LogLine>) -> TurnOutcome {
    let w = 3600.0 * st.v;
    lines.push(LogLine::Banner(format!(
        "ON MOON AT {} SECONDS — IMPACT VELOCITY {} MPH",
        fmt_num(st.l),
        fmt_num((w * 100.0).round() / 100.0),
    )));
    TurnOutcome { lines, impact_mph: Some(w) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ballistic_integrate_when_no_burn() {
        // k=0 ⇒ q=0 ⇒ exact free-fall kinematics.
        let st = LunarState::new();
        let (i, j) = integrate(&st, 10.0, 0.0);
        assert!((j - (st.v + G * 10.0)).abs() < 1e-12);
        assert!((i - (st.a - G * 50.0 - st.v * 10.0)).abs() < 1e-12);
    }

    #[test]
    fn integrate_matches_series_by_hand() {
        // s=10, k=200, fresh capsule: q = 2000/33000.
        let st = LunarState::new();
        let q: f64 = 2000.0 / 33_000.0;
        let expect_j = st.v
            + G * 10.0
            + Z * (-q - q * q / 2.0 - q.powi(3) / 3.0 - q.powi(4) / 4.0 - q.powi(5) / 5.0);
        let expect_i = st.a - G * 100.0 / 2.0 - st.v * 10.0
            + Z * 10.0
                * (q / 2.0 + q * q / 6.0 + q.powi(3) / 12.0 + q.powi(4) / 20.0
                    + q.powi(5) / 30.0);
        let (i, j) = integrate(&st, 10.0, 200.0);
        assert!((i - expect_i).abs() < 1e-12);
        assert!((j - expect_j).abs() < 1e-12);
    }

    #[test]
    fn zero_burn_turn_is_pure_free_fall() {
        let mut st = LunarState::new();
        let out = step(&mut st, 0);
        assert!(out.impact_mph.is_none());
        // One 10 s ballistic step: v += G·10, a -= G·50 + v0·10.
        assert!((st.v - 1.01).abs() < 1e-12);
        assert!((st.a - (120.0 - 0.05 - 10.0)).abs() < 1e-12);
        assert!((st.l - 10.0).abs() < 1e-12);
        assert!((st.m - 33_000.0).abs() < 1e-12); // no fuel spent
    }

    #[test]
    fn fuel_cap_limits_substep_then_free_fall() {
        // 100 lb of fuel at 200 lb/s burns for 0.5 s, then line 160 takes over.
        let mut st = LunarState { a: 50.0, v: 0.5, m: EMPTY_MASS + 100.0, l: 0.0 };
        let out = step(&mut st, 200);
        assert!(out.impact_mph.is_some(), "must coast to the surface");
        assert!(fuel(&st).abs() < 1e-9, "all fuel burnt, none negative");
        assert!(out
            .lines
            .iter()
            .any(|l| matches!(l, LogLine::Banner(b) if b.starts_with("FUEL OUT"))));
    }

    #[test]
    fn free_fall_impact_closed_form() {
        // No fuel from the start: v_impact = sqrt(v² + 2AG).
        let mut st = LunarState { a: 1.0, v: 0.1, m: EMPTY_MASS, l: 0.0 };
        let out = step(&mut st, 100);
        let expect = (0.1f64 * 0.1 + 2.0 * 1.0 * G).sqrt();
        assert!((st.v - expect).abs() < 1e-12);
        let mph = out.impact_mph.expect("impact");
        assert!((mph - 3600.0 * expect).abs() < 1e-9);
    }

    #[test]
    fn touchdown_refine_terminates_and_lands() {
        // Hard free-fall into the surface from low altitude.
        let mut st = LunarState { a: 0.5, v: 0.2, m: 33_000.0, l: 0.0 };
        let out = step(&mut st, 0);
        let mph = out.impact_mph.expect("hits the moon within the turn");
        assert!(mph > 60.0, "uncontrolled drop is fatal: {mph}");
        assert!(st.l > 0.0 && st.l.is_finite());
    }

    #[test]
    fn anti_ascent_path_runs_without_panic() {
        // Slow descent + max burn: J goes negative within the first step.
        let mut st = LunarState { a: 10.0, v: 0.05, m: 33_000.0, l: 0.0 };
        let out = step(&mut st, 200);
        assert!(out.impact_mph.is_none());
        assert!(st.v.is_finite() && st.a.is_finite() && st.m.is_finite());
        assert!(st.m < 33_000.0, "fuel was spent");
        assert!((st.l - 10.0).abs() < 1e-9, "full 10 s elapsed");
    }

    #[test]
    fn soft_landing_is_achievable() {
        // Final approach: 5 ft up, drifting down at 1.8 mph, near-hover burn
        // (gravity cancels at m·G/Z ≈ 18.3 lb/s). Touches down within the
        // turn at well under the 10 mph "good landing" threshold.
        let mut st = LunarState { a: 0.001, v: 0.0005, m: 33_000.0, l: 0.0 };
        let out = step(&mut st, 19);
        let mph = out.impact_mph.expect("lands within the turn");
        assert!(mph < 10.0, "controlled approach lands softly, got {mph} mph");
    }
}
