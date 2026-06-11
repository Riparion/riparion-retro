//! Scoring helpers shared by the trail-style games: the calendar walk that
//! turns a journey length into an arrival date, and the leftover-supplies
//! valuation. Each game keeps its own `score`/`rank`/`HighScore` (the constants
//! and rank names diverge); only the genuinely-identical math lives here.

const MONTHS: [(&str, i64); 12] = [
    ("January", 31),
    ("February", 28),
    ("March", 31),
    ("April", 30),
    ("May", 31),
    ("June", 30),
    ("July", 31),
    ("August", 31),
    ("September", 30),
    ("October", 31),
    ("November", 30),
    ("December", 31),
];
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Format "Weekday, Month D, YEAR" for a date `days_out` days after the journey
/// began. `epoch_doy` is the 1-based day-of-year of departure (March 29 = 88),
/// `epoch_weekday` its weekday index (0 = Monday). The displayed calendar date
/// is pinned to the year (a late arrival can spill past Dec 31), even though
/// `days_out` itself is unbounded.
pub fn calendar_date(epoch_doy: i64, epoch_weekday: i64, year: i64, days_out: i64) -> String {
    let weekday = WEEKDAYS[((epoch_weekday + days_out).rem_euclid(7)) as usize];
    let doy = (epoch_doy + days_out).min(365);
    let mut month = "December";
    let mut day = 31;
    let mut acc = 0;
    for (name, len) in MONTHS {
        if doy <= acc + len {
            month = name;
            day = doy - acc;
            break;
        }
        acc += len;
    }
    format!("{weekday}, {month} {day}, {year}")
}

/// Arrival date and total days out for a journey of `turn` whole periods of
/// `period_days` plus a fraction `f9` of the final period. See
/// [`calendar_date`] for the epoch parameters.
pub fn arrival_date(
    turn: u32,
    f9: f64,
    period_days: i64,
    epoch_doy: i64,
    epoch_weekday: i64,
    year: i64,
) -> (String, i64) {
    let days_out = turn as i64 * period_days + (f9 * period_days as f64).floor() as i64;
    (
        calendar_date(epoch_doy, epoch_weekday, year, days_out),
        days_out,
    )
}

/// Total worth of what's left in the wagon/packs, in rough dollar-points.
/// Bullets are a count, valued at 50 to the dollar.
pub fn leftover_value(food: f64, clothing: f64, misc: f64, cash: f64, bullets: f64) -> f64 {
    food + clothing + misc + cash + bullets / 50.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oregon_leaves_on_a_monday() {
        // Independence, March 29, 1847 (doy 88, a Monday): turn 0 is departure.
        let (s, days) = arrival_date(0, 0.0, 14, 88, 0, 1847);
        assert_eq!(days, 0);
        assert!(s.starts_with("Monday, March 29, 1847"), "{s}");
    }

    #[test]
    fn oregon_advances_two_weeks() {
        let (s, days) = arrival_date(1, 0.0, 14, 88, 0, 1847);
        assert_eq!(days, 14);
        assert!(s.contains("April 12"), "{s}");
        assert!(s.starts_with("Monday"), "{s}");
    }

    #[test]
    fn fort_nash_leaves_on_a_monday() {
        // Fort Patrick Henry, November 1, 1779 (doy 305, a Monday).
        let (s, days) = arrival_date(0, 0.0, 7, 305, 0, 1779);
        assert_eq!(days, 0);
        assert!(s.starts_with("Monday, November 1, 1779"), "{s}");
    }

    #[test]
    fn fort_nash_advances_one_week() {
        let (s, days) = arrival_date(1, 0.0, 7, 305, 0, 1779);
        assert_eq!(days, 7);
        assert!(s.contains("November 8"), "{s}");
        assert!(s.starts_with("Monday"), "{s}");
    }

    #[test]
    fn late_arrival_pins_to_year_end() {
        let s = calendar_date(305, 0, 1779, 200);
        assert!(s.contains("December 31"), "{s}");
    }

    #[test]
    fn leftover_counts_bullets_at_fifty_to_the_dollar() {
        assert_eq!(leftover_value(10.0, 5.0, 5.0, 80.0, 500.0), 110.0);
    }
}
