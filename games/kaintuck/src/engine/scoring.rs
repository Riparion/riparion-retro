//! Final scoring and the hall of fame. A successful Kaintuck walks into Nashville
//! rich, with his crew paid and his name good; a robbery on the Trace is the
//! thing that hurts most.

use serde::{Deserialize, Serialize};

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
    if won {
        let crew_bonus = crew_survived as f64 * 75.0;
        let rep_bonus = (reputation * 4.0).max(-100.0);
        let robbed_pen = if robbed { -300.0 } else { 0.0 };
        let speed = (35 - days).max(0) as f64 * 20.0;
        (1500.0 + leftover + crew_bonus + rep_bonus + robbed_pen + speed).floor() as i64
    } else {
        (miles_total / 4.0 + leftover / 4.0).floor() as i64
    }
}

pub fn rank(score: i64) -> &'static str {
    match score {
        s if s >= 5000 => "River King",
        s if s >= 3500 => "Master Kaintuck",
        s if s >= 2000 => "Trader",
        s if s >= 900 => "Boatman",
        s if s >= 1 => "Greenhorn",
        _ => "Drowned Rat",
    }
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
