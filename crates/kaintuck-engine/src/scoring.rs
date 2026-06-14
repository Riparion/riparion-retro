//! Final scoring and the hall of fame. A successful Kaintuck walks into Nashville
//! rich, with his crew paid and his name good; a robbery on the Trace is the
//! thing that hurts most.

use serde::{Deserialize, Serialize};

use super::scenario_data::scenario;

/// Score a finished journey. Arriving home is worth a great deal; arriving with
/// cash, a surviving crew, a good name, and no robbery behind you is worth more.
/// A journey cut short still earns partial credit for the miles covered.
pub fn score(
    won: bool,
    miles_total: f64,
    days: i64,
    leftover: f64,
    crew_survived: i64,
    reputation: f64,
    robbed: bool,
) -> i64 {
    let p = &scenario().scoring;
    if won {
        let crew_bonus = crew_survived as f64 * p.crew_bonus;
        let rep_bonus = (reputation * p.rep_mult).max(p.rep_floor);
        let robbed_pen = if robbed { p.robbed_penalty } else { 0.0 };
        let speed = (p.speed_par_days - days).max(0) as f64 * p.speed_per_day;
        (p.base_win + leftover + crew_bonus + rep_bonus + robbed_pen + speed).floor() as i64
    } else {
        (miles_total / p.loss_miles_div + leftover / p.loss_leftover_div).floor() as i64
    }
}

pub fn rank(score: i64) -> &'static str {
    let p = &scenario().scoring;
    p.ranks
        .iter()
        .find(|t| score >= t.min)
        .map(|t| t.name.as_str())
        .unwrap_or(p.floor_rank.as_str())
}

/// One line in the hall of fame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    pub trader: String,
    pub score: i64,
    pub rank: String,
    pub won: bool,
}

impl HighScore {
    pub fn from_end(trader: &str, end: &super::state::EndGame) -> Self {
        Self {
            trader: trader.to_string(),
            score: end.score,
            rank: end.rank.clone(),
            won: end.won,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arriving_beats_dying() {
        let win = score(true, 2280.0, 28, 400.0, 3, 10.0, false);
        let loss = score(false, 1200.0, 28, 400.0, 3, 10.0, false);
        assert!(win > loss);
    }

    #[test]
    fn robbery_hurts() {
        let clean = score(true, 2280.0, 28, 400.0, 2, 0.0, false);
        let robbed = score(true, 2280.0, 28, 400.0, 2, 0.0, true);
        assert!(clean > robbed);
        assert_eq!(clean - robbed, 300);
    }

    #[test]
    fn faster_and_richer_score_higher() {
        let fast = score(true, 2280.0, 20, 400.0, 2, 0.0, false);
        let slow = score(true, 2280.0, 34, 400.0, 2, 0.0, false);
        assert!(fast > slow);
        let rich = score(true, 2280.0, 28, 800.0, 2, 0.0, false);
        let poor = score(true, 2280.0, 28, 100.0, 2, 0.0, false);
        assert!(rich > poor);
    }

    #[test]
    fn ranks_are_ordered() {
        assert_eq!(rank(6000), "River King");
        assert_eq!(rank(3500), "Master Kaintuck");
        assert_eq!(rank(2000), "Trader");
        assert_eq!(rank(900), "Boatman");
        assert_eq!(rank(1), "Greenhorn");
        assert_eq!(rank(0), "Drowned Rat");
    }
}
