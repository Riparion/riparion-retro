//! Eric Peters' ROCKET (via rocket.bas): second-by-second descent from
//! 1000 ft. Each fuel unit cuts the descent by 1 ft/s; gravity adds 5 ft/s².
//! Line numbers in comments refer to rocket.bas.

use super::state::{fmt_num, LogLine, RocketState, TurnRow};

pub const BURN_MAX: i64 = 30;
pub const START_HEIGHT: f64 = 1000.0;
/// Lunar gravity, ft/s².
const GRAVITY: f64 = 5.0;

/// Contact details (lines 670–780).
pub struct Contact {
    /// Signed touchdown velocity, ft/s (down positive).
    pub landing_v: f64,
    pub touchdown_at: f64,
}

pub struct TurnOutcome {
    pub lines: Vec<LogLine>,
    pub contact: Option<Contact>,
}

pub fn report_row(st: &RocketState, burn: &str) -> TurnRow {
    TurnRow {
        sec: fmt_num(st.t),
        alt: format!("{} ft", fmt_num(st.h)),
        vel: format!("{} ft/s", fmt_num(st.v)),
        fuel: fmt_num(st.f),
        burn: burn.to_string(),
    }
}

/// One player second (lines 500–660). When the tank runs dry the descent
/// auto-plays with zero burn until contact, exactly like the original's
/// input-less loop — all of those seconds come back as paced log lines.
pub fn step(st: &mut RocketState, burn: i64) -> TurnOutcome {
    let mut lines = Vec::new();
    // Lines 510–530: negative input coasts; clamp to engine max and the tank.
    let mut b = (burn.max(0) as f64).min(BURN_MAX as f64).min(st.f);
    let mut player_burn = true;
    loop {
        // Lines 540–570.
        let v0 = st.v;
        let v1 = v0 - b + GRAVITY;
        st.f -= b;
        st.h -= 0.5 * (v0 + v1);
        // Line 580: surface contact within this second.
        if st.h <= 0.0 {
            st.h += 0.5 * (v1 + v0); // line 680: undo the step
            // Lines 690–720: a burn of exactly 5 cancels gravity — linear fall.
            #[allow(clippy::float_cmp)] // faithful to BASIC's IF B=5
            let d = if b == 5.0 {
                st.h / v0
            } else {
                (-v0 + (v0 * v0 + st.h * (10.0 - 2.0 * b)).sqrt()) / (GRAVITY - b)
            };
            let landing_v = v0 + (GRAVITY - b) * d; // line 730
            let touchdown_at = st.t + d;
            lines.push(LogLine::Banner("***** CONTACT *****".to_string()));
            lines.push(LogLine::Banner(format!(
                "TOUCHDOWN AT {} SECONDS",
                fmt_num((touchdown_at * 100.0).round() / 100.0),
            )));
            lines.push(LogLine::Banner(format!(
                "LANDING VELOCITY {} FT/S — {} UNITS OF FUEL REMAINING",
                fmt_num((landing_v * 100.0).round() / 100.0),
                fmt_num(st.f),
            )));
            return TurnOutcome { lines, contact: Some(Contact { landing_v, touchdown_at }) };
        }
        // Lines 590–600.
        st.t += 1.0;
        st.v = v1;
        if st.f > 0.0 {
            lines.push(LogLine::Row(report_row(st, &fmt_num(b))));
            return TurnOutcome { lines, contact: None };
        }
        // Lines 615–660: tank dry — banner only if this burn actually fired,
        // then fall with the engine off until contact.
        if player_burn && b > 0.0 {
            lines.push(LogLine::Banner("**** OUT OF FUEL ****".to_string()));
        }
        lines.push(LogLine::Row(report_row(st, &fmt_num(b))));
        b = 0.0;
        player_burn = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_burn_first_second() {
        let mut st = RocketState::new();
        let out = step(&mut st, 0);
        assert!(out.contact.is_none());
        assert_eq!(st.v, 55.0);
        assert_eq!(st.h, 947.5);
        assert_eq!(st.t, 1.0);
        assert_eq!(st.f, 150.0);
    }

    #[test]
    fn full_burn_first_second() {
        let mut st = RocketState::new();
        step(&mut st, 30);
        assert_eq!(st.v, 25.0);
        assert_eq!(st.h, 962.5);
        assert_eq!(st.f, 120.0);
    }

    #[test]
    fn input_clamps() {
        let mut st = RocketState::new();
        step(&mut st, 999); // > engine max
        assert_eq!(st.f, 120.0);
        let mut st = RocketState::new();
        step(&mut st, -5); // negative coasts
        assert_eq!(st.f, 150.0);
        assert_eq!(st.v, 55.0);
        let mut st = RocketState { t: 0.0, h: 1000.0, v: 50.0, f: 10.0 };
        let out = step(&mut st, 30); // > tank
        assert_eq!(st.f, 0.0);
        assert!(out.lines.iter().any(
            |l| matches!(l, LogLine::Banner(b) if b.contains("OUT OF FUEL"))
        ));
    }

    #[test]
    fn hover_burn_is_stable() {
        let mut st = RocketState::new();
        step(&mut st, 5);
        assert_eq!(st.v, 50.0); // thrust cancels gravity exactly
        assert_eq!(st.h, 950.0);
        assert_eq!(st.f, 145.0);
    }

    #[test]
    fn contact_with_hover_burn_uses_linear_solve() {
        // h=10, v=20, b=5: step → h = 10 - 20 = -10 ⇒ contact.
        // Restored h=10, d = h/v = 0.5, landing_v = v (no net acceleration).
        let mut st = RocketState { t: 3.0, h: 10.0, v: 20.0, f: 100.0 };
        let out = step(&mut st, 5);
        let c = out.contact.expect("contact");
        assert_eq!(c.landing_v, 20.0);
        assert_eq!(c.touchdown_at, 3.5);
        assert_eq!(st.f, 95.0);
    }

    #[test]
    fn contact_quadratic_matches_hand_calculation() {
        // h=10, v=20, b=0: v1=25, h→-12.5 ⇒ contact. Restored h=10.
        // d = (-20 + √(400+100))/5, landing_v = 20 + 5d = √500.
        let mut st = RocketState { t: 7.0, h: 10.0, v: 20.0, f: 50.0 };
        let out = step(&mut st, 0);
        let c = out.contact.expect("contact");
        assert!((c.landing_v - 500f64.sqrt()).abs() < 1e-12);
        assert!((c.touchdown_at - (7.0 + (-20.0 + 500f64.sqrt()) / 5.0)).abs() < 1e-12);
    }

    #[test]
    fn perfect_landing_exists() {
        // h=5, v=10, b=15: v1=0, h→0 ⇒ contact; √(v²+h(10-2b)) = √(100-100) = 0.
        let mut st = RocketState { t: 12.0, h: 5.0, v: 10.0, f: 20.0 };
        let out = step(&mut st, 15);
        let c = out.contact.expect("contact");
        assert_eq!(c.landing_v, 0.0);
        assert_eq!(c.touchdown_at, 13.0);
        assert_eq!(st.f, 5.0);
    }

    #[test]
    fn fuel_out_autoplays_to_contact() {
        // Empty the tank high up; the same call must carry through to contact.
        let mut st = RocketState { t: 0.0, h: 500.0, v: 10.0, f: 20.0 };
        let out = step(&mut st, 20);
        assert!(out.contact.is_some(), "auto-advance must reach the surface");
        assert_eq!(st.f, 0.0);
        let banners = out
            .lines
            .iter()
            .filter(|l| matches!(l, LogLine::Banner(b) if b.contains("OUT OF FUEL")))
            .count();
        assert_eq!(banners, 1, "fuel banner exactly once");
        // Time advanced monotonically, one row per simulated second.
        assert!(st.t > 1.0);
    }

    #[test]
    fn fuel_banner_suppressed_when_burn_was_zero() {
        // Line 615: tank already dry and the player coasts — no banner.
        let mut st = RocketState { t: 0.0, h: 400.0, v: 0.0, f: 0.0 };
        let out = step(&mut st, 0);
        assert!(out.contact.is_some());
        assert!(!out
            .lines
            .iter()
            .any(|l| matches!(l, LogLine::Banner(b) if b.contains("OUT OF FUEL"))));
    }
}
