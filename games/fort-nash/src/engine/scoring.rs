//! Final scoring and the trail's hall of fame.
//!
//! We synthesize a score from surviving supplies, distance covered, and how
//! quickly you reached the French Lick, so the local high-score board has
//! something to rank.

use serde::{Deserialize, Serialize};

use super::state::{EndGame, GameState};

/// Total worth of what's left in the packs, in rough dollar-points.
pub fn leftover_value(s: &GameState) -> f64 {
    retro_kit::scoring::leftover_value(s.food, s.clothing, s.misc, s.cash, s.bullets)
}

/// Score a finished journey. Arriving is worth a great deal; arriving early with
/// full packs is worth more. A journey cut short still earns partial credit for
/// the miles covered. The weights live in the scenario's `scoring` table.
pub fn score(won: bool, miles: f64, days: i64, leftover: f64) -> i64 {
    let s = &super::scenario_data::scenario().scoring;
    if won {
        // The whole trek runs ~8 weeks; reward beating the deep winter in.
        let speed_bonus = (s.speed_par_days - days).max(0) as f64 * s.speed_per_day;
        (s.base_win + leftover + speed_bonus).floor() as i64
    } else {
        ((miles / s.loss_miles_div) + leftover / s.loss_leftover_div).floor() as i64
    }
}

pub fn rank(score: i64) -> &'static str {
    let s = &super::scenario_data::scenario().scoring;
    s.ranks
        .iter()
        .find(|t| score >= t.min)
        .map(|t| t.name.as_str())
        .unwrap_or(&s.floor_rank)
}

/// Arrival date and days-on-the-trail for a journey `turn` weeks out plus a
/// fraction `f9` of the final week. The calendar epoch (the party marches out of
/// Fort Patrick Henry on Monday, November 1, 1779 — day-of-year 305) is read from
/// the scenario.
pub fn arrival_date(turn: u32, f9: f64) -> (String, i64) {
    let c = &super::scenario_data::scenario().calendar;
    retro_kit::scoring::arrival_date(turn, f9, c.period_days, c.epoch_doy, c.epoch_weekday, c.year)
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
