//! Final scoring and the city's hall of fame.
//!
//! The 1978 original kept no score — you simply became King, or died trying. We
//! synthesize one from the same ingredients: **winning dominates** (a crown
//! outranks any reign that fell short, no matter how rich), then how long you
//! ruled, then the estate you leave behind.

use serde::{Deserialize, Serialize};

use super::state::{EndGame, GameState};

/// Rough worth of the standing city, in score points.
pub fn estate_value(s: &GameState) -> i64 {
    s.treasury.max(0) / 5
        + s.land / 4
        + s.serfs * 2
        + s.merchants * 30
        + s.nobles * 200
        + s.clergy * 40
        + s.buildings() * 500
}

/// Score a finished reign. Every win is strictly above every loss.
pub fn score(s: &GameState, won: bool) -> i64 {
    let years = (s.year - super::state::START_YEAR).max(0);
    let longevity = years * 200;
    let estate = estate_value(s);
    if won {
        1_000_000 + longevity + estate
    } else {
        (s.title_num.max(0) * 5_000 + longevity + estate).min(999_999)
    }
}

/// A flavour rank for the score, independent of the in-game title.
pub fn rank(score: i64) -> &'static str {
    match score {
        s if s >= 1_000_000 => "Sovereign",
        s if s >= 60_000 => "Statesman",
        s if s >= 30_000 => "Magnate",
        s if s >= 12_000 => "Notable",
        s if s >= 4_000 => "Steward",
        s if s >= 1 => "Courtier",
        _ => "Pauper",
    }
}

/// Build the end-of-game reckoning from the final state.
pub fn build_end(state: &GameState, won: bool, cause: String) -> EndGame {
    let score = score(state, won);
    EndGame {
        won,
        cause,
        title: state.title().to_string(),
        years: (state.year - super::state::START_YEAR).max(0),
        treasury: state.treasury,
        land: state.land.max(0),
        serfs: state.serfs.max(0),
        score,
        rank: rank(score).to_string(),
        recorded: false,
    }
}

/// One line in the hall of fame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    pub name: String,
    pub title: String,
    pub score: i64,
    pub years: i64,
    pub won: bool,
}

impl HighScore {
    pub fn from_end(name: &str, end: &EndGame) -> Self {
        Self {
            name: name.to_string(),
            title: end.title.clone(),
            score: end.score,
            years: end.years,
            won: end.won,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::{Difficulty, Gender, GameState};

    fn st() -> GameState {
        GameState::new(
            "Test".into(),
            "Santa Paravia".into(),
            Gender::Male,
            Difficulty::Apprentice,
        )
    }

    #[test]
    fn wins_outrank_every_loss() {
        // A long, fabulously rich reign that still fell short of the crown...
        let mut rich = st();
        rich.year = 1454;
        rich.treasury = 5_000_000;
        rich.land = 4_000_000;
        rich.serfs = 200_000;
        rich.title_num = 6;
        let big_loss = score(&rich, false);
        // ...still ranks below the weakest possible win.
        let mut bare = st();
        bare.year = 1420;
        bare.title_num = 7;
        let weak_win = score(&bare, true);
        assert!(weak_win > big_loss);
    }

    #[test]
    fn longer_reign_beats_shorter_all_else_equal() {
        let mut a = st();
        a.year = 1440;
        let mut b = st();
        b.year = 1410;
        assert!(score(&a, false) > score(&b, false));
    }

    #[test]
    fn ranks_are_ordered() {
        assert_eq!(rank(1_000_000), "Sovereign");
        assert_eq!(rank(0), "Pauper");
        assert_eq!(rank(1), "Courtier");
    }
}
