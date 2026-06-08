//! Final scoring and the trail's hall of fame.
//!
//! The 1975 original kept no score — it just printed your arrival date and
//! whatever you had left. We synthesize a score from the same ingredients
//! (surviving supplies, distance covered, and how quickly you arrived) so the
//! local high-score board has something to rank.

use serde::{Deserialize, Serialize};

use super::state::{EndGame, GameState};

/// Total worth of what's left in the wagon, in rough dollar-points.
pub fn leftover_value(s: &GameState) -> f64 {
    s.food + s.clothing + s.misc + s.cash + s.bullets / 50.0
}

/// Score a finished journey. Arriving is worth a great deal; arriving early
/// with a full wagon is worth more. A journey cut short still earns partial
/// credit for the miles covered.
pub fn score(won: bool, miles: f64, days: i64, leftover: f64) -> i64 {
    if won {
        let speed_bonus = (250 - days).max(0) as f64 * 3.0;
        (1500.0 + leftover + speed_bonus).floor() as i64
    } else {
        ((miles / 4.0) + leftover / 4.0).floor() as i64
    }
}

pub fn rank(score: i64) -> &'static str {
    match score {
        s if s >= 4000 => "Wagon Master",
        s if s >= 2500 => "Trail Boss",
        s if s >= 1500 => "Pioneer",
        s if s >= 600 => "Settler",
        s if s >= 1 => "Greenhorn",
        _ => "Tenderfoot",
    }
}

/// Day-of-year (1847 is a common year) and weekday for an arrival `turn`
/// fortnights out plus a fraction `f9` of the final fortnight. The trek leaves
/// Independence on Monday, March 29, 1847.
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
    // March 29 is day-of-year 88 (31 + 28 + 29).
    let days_out = turn as i64 * 14 + (f9 * 14.0).floor() as i64;
    let weekday = WEEKDAYS[(days_out.rem_euclid(7)) as usize];
    // Pin the displayed calendar date to the year (a turn-19 arrival can spill
    // past Dec 31); the returned day count is always the true days on the trail.
    let doy = (88 + days_out).min(365);
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
    (format!("{weekday}, {month} {day}, 1847"), days_out)
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
        let win = score(true, 2040.0, 150, 200.0);
        let loss = score(false, 1000.0, 150, 200.0);
        assert!(win > loss);
    }

    #[test]
    fn faster_arrival_scores_higher() {
        let fast = score(true, 2040.0, 120, 200.0);
        let slow = score(true, 2040.0, 200, 200.0);
        assert!(fast > slow);
    }

    #[test]
    fn more_supplies_score_higher() {
        let rich = score(true, 2040.0, 150, 400.0);
        let poor = score(true, 2040.0, 150, 50.0);
        assert!(rich > poor);
    }

    #[test]
    fn ranks_are_ordered() {
        assert_eq!(rank(5000), "Wagon Master");
        assert_eq!(rank(4000), "Wagon Master");
        assert_eq!(rank(3999), "Trail Boss");
        assert_eq!(rank(1500), "Pioneer");
        assert_eq!(rank(600), "Settler");
        assert_eq!(rank(1), "Greenhorn");
        assert_eq!(rank(0), "Tenderfoot");
    }

    #[test]
    fn arrival_date_leaves_on_a_monday() {
        // Turn 0, no fortnights out: the day you leave, March 29.
        let (s, days) = arrival_date(0, 0.0);
        assert_eq!(days, 0);
        assert!(s.starts_with("Monday, March 29"), "{s}");
    }

    #[test]
    fn arrival_date_advances_two_weeks() {
        let (s, days) = arrival_date(1, 0.0);
        assert_eq!(days, 14);
        assert!(s.contains("April 12"), "{s}");
        assert!(s.starts_with("Monday"), "{s}");
    }
}
