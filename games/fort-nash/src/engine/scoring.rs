//! Final scoring and the trail's hall of fame.
//!
//! We synthesize a score from surviving supplies, distance covered, and how
//! quickly you reached the French Lick, so the local high-score board has
//! something to rank.

use serde::{Deserialize, Serialize};

use super::state::{EndGame, GameState};

/// Total worth of what's left in the packs, in rough dollar-points.
pub fn leftover_value(s: &GameState) -> f64 {
    s.food + s.clothing + s.misc + s.cash + s.bullets / 50.0
}

/// Score a finished journey. Arriving is worth a great deal; arriving early with
/// full packs is worth more. A journey cut short still earns partial credit for
/// the miles covered.
pub fn score(won: bool, miles: f64, days: i64, leftover: f64) -> i64 {
    if won {
        // The whole trek runs ~8 weeks; reward beating the deep winter in.
        let speed_bonus = (70 - days).max(0) as f64 * 8.0;
        (1500.0 + leftover + speed_bonus).floor() as i64
    } else {
        ((miles / 4.0) + leftover / 4.0).floor() as i64
    }
}

pub fn rank(score: i64) -> &'static str {
    match score {
        s if s >= 3500 => "Founder",
        s if s >= 2200 => "Long Hunter",
        s if s >= 1500 => "Settler",
        s if s >= 600 => "Frontiersman",
        s if s >= 1 => "Greenhorn",
        _ => "Tenderfoot",
    }
}

/// Day-of-year (1779 is a common year) and weekday for an arrival `turn` weeks
/// out plus a fraction `f9` of the final week. The party marches out of Fort
/// Patrick Henry on Monday, November 1, 1779.
pub fn arrival_date(turn: u32, f9: f64) -> (String, i64) {
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
    // November 1 is day-of-year 305 (31+28+31+30+31+30+31+31+30+31 + 1).
    let days_out = turn as i64 * 7 + (f9 * 7.0).floor() as i64;
    // November 1, 1779 was a Monday, so day 0 lands on index 0.
    let weekday = WEEKDAYS[(days_out.rem_euclid(7)) as usize];
    // Pin the displayed calendar date to the year (a late arrival can spill past
    // Dec 31); the returned day count is always the true days on the trail.
    let doy = (305 + days_out).min(365);
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
    (format!("{weekday}, {month} {day}, 1779"), days_out)
}

/// One line in the trailside hall of fame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    pub party: String,
    pub score: i64,
    pub rank: String,
    pub miles: i64,
    pub won: bool,
}

impl HighScore {
    pub fn from_end(party: &str, end: &EndGame) -> Self {
        Self {
            party: party.to_string(),
            score: end.score,
            rank: end.rank.clone(),
            miles: end.miles,
            won: end.won,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arriving_beats_dying() {
        let win = score(true, 1600.0, 49, 200.0);
        let loss = score(false, 1000.0, 49, 200.0);
        assert!(win > loss);
    }

    #[test]
    fn faster_arrival_scores_higher() {
        let fast = score(true, 1600.0, 42, 200.0);
        let slow = score(true, 1600.0, 56, 200.0);
        assert!(fast > slow);
    }

    #[test]
    fn more_supplies_score_higher() {
        let rich = score(true, 1600.0, 49, 400.0);
        let poor = score(true, 1600.0, 49, 50.0);
        assert!(rich > poor);
    }

    #[test]
    fn ranks_are_ordered() {
        assert_eq!(rank(5000), "Founder");
        assert_eq!(rank(3500), "Founder");
        assert_eq!(rank(3499), "Long Hunter");
        assert_eq!(rank(2200), "Long Hunter");
        assert_eq!(rank(1500), "Settler");
        assert_eq!(rank(600), "Frontiersman");
        assert_eq!(rank(1), "Greenhorn");
        assert_eq!(rank(0), "Tenderfoot");
    }

    #[test]
    fn arrival_date_leaves_on_a_monday() {
        // Turn 0, no weeks out: the day you leave, November 1.
        let (s, days) = arrival_date(0, 0.0);
        assert_eq!(days, 0);
        assert!(s.starts_with("Monday, November 1"), "{s}");
    }

    #[test]
    fn arrival_date_advances_one_week() {
        let (s, days) = arrival_date(1, 0.0);
        assert_eq!(days, 7);
        assert!(s.contains("November 8"), "{s}");
        assert!(s.starts_with("Monday"), "{s}");
    }
}
