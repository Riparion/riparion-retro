//! Final scoring, ranks, and high-score records.

use serde::{Deserialize, Serialize};

use super::state::{EndGame, GameState};

/// `score = floor(net_worth / 100 / TI^1.1)`.
pub fn score(net_worth: f64, months: u32) -> i64 {
    (net_worth / 100.0 / (months as f64).powf(1.1)).floor() as i64
}

pub fn rank(score: i64) -> &'static str {
    match score {
        s if s >= 50_000 => "Ma Tsu",
        s if s >= 8_000 => "Master Taipan",
        s if s >= 1_000 => "Taipan",
        s if s >= 500 => "Compradore",
        _ => "Galley Hand",
    }
}

pub fn end_game(state: &GameState, retired: bool, cause: &str) -> EndGame {
    let net_worth = state.net_worth();
    let score = score(net_worth, state.months);
    EndGame {
        retired,
        cause: cause.to_string(),
        net_worth,
        score,
        rank: rank(score).to_string(),
        years: state.months / 12,
        months: state.months % 12,
        capacity: state.capacity,
        guns: state.guns,
        recorded: false,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    pub firm: String,
    pub score: i64,
    pub rank: String,
    pub years: u32,
    pub months: u32,
    pub retired: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_formula() {
        // 1,000,000 net worth after 1 month: 1e6/100/1 = 10000
        assert_eq!(score(1_000_000.0, 1), 10_000);
        // longer games score less for the same worth
        assert!(score(1_000_000.0, 24) < score(1_000_000.0, 12));
        assert!(score(-5_000.0, 12) < 0);
    }

    #[test]
    fn ranks() {
        assert_eq!(rank(50_000), "Ma Tsu");
        assert_eq!(rank(49_999), "Master Taipan");
        assert_eq!(rank(8_000), "Master Taipan");
        assert_eq!(rank(7_999), "Taipan");
        assert_eq!(rank(1_000), "Taipan");
        assert_eq!(rank(999), "Compradore");
        assert_eq!(rank(500), "Compradore");
        assert_eq!(rank(499), "Galley Hand");
        assert_eq!(rank(-10), "Galley Hand");
    }
}
