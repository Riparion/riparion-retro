//! Final scoring and the duchy's hall of fame.
//!
//! The 1980 original kept no score — your reign simply ended, well or badly. We
//! synthesize one from the same ingredients: the *kind* of ending dominates
//! (seizing the crown ≫ honourable retirement ≫ any failure), then how many
//! years you ruled, then the estate you leave behind. So a long reign that
//! eventually fails still outscores a quick collapse, but every win is strictly
//! above every loss.

use serde::{Deserialize, Serialize};

use super::state::{EndGame, Ending, GameState};

/// Rough worth of the standing duchy, in score points.
pub fn estate_value(peasants: i64, land: i64, grain: i64) -> i64 {
    peasants * 4 + land * 2 + grain / 10
}

/// Score a finished reign.
pub fn score(ending: Ending, years: i64, peasants: i64, land: i64, grain: i64) -> i64 {
    let base = match ending {
        Ending::HighKing => 50_000,
        Ending::Retired => 20_000,
        Ending::Deposed | Ending::Collapsed | Ending::Beheaded => 0,
    };
    let longevity = years.max(0) * 50;
    let estate = estate_value(peasants.max(0), land.max(0), grain.max(0));
    (base + longevity + estate).max(0)
}

pub fn rank(score: i64) -> &'static str {
    match score {
        s if s >= 50_000 => "High King",
        s if s >= 20_000 => "Sovereign Duke",
        s if s >= 12_000 => "Grand Duke",
        s if s >= 6_000 => "Duke",
        s if s >= 2_500 => "Marquess",
        s if s >= 1_000 => "Earl",
        s if s >= 300 => "Baron",
        s if s >= 1 => "Knight",
        _ => "Serf",
    }
}

/// Build the end-of-game reckoning from the final state.
pub fn build_end(state: &GameState, ending: Ending, cause: String) -> EndGame {
    let years = state.n_y;
    let score = score(ending, years, state.n_p, state.n_l, state.n_g);
    EndGame {
        ending,
        cause,
        years,
        peasants: state.n_p.max(0),
        land: state.n_l.max(0),
        grain: state.n_g.max(0),
        score,
        rank: rank(score).to_string(),
        recorded: false,
    }
}

/// One line in the hall of fame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    pub duke: String,
    pub score: i64,
    pub rank: String,
    pub years: i64,
    pub ending: String,
    pub won: bool,
}

impl HighScore {
    pub fn from_end(duke: &str, end: &EndGame) -> Self {
        Self {
            duke: duke.to_string(),
            score: end.score,
            rank: end.rank.clone(),
            years: end.years,
            ending: end.ending.label().to_string(),
            won: end.ending.won(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wins_outrank_every_loss() {
        // A glorious 45-year reign that still ends in a loss...
        let long_loss = score(Ending::Deposed, 45, 500, 1500, 9000);
        // ...must still rank below the weakest possible win.
        let weak_win = score(Ending::Retired, 1, 0, 0, 0);
        assert!(weak_win > long_loss);
    }

    #[test]
    fn high_king_beats_retirement() {
        let hk = score(Ending::HighKing, 10, 100, 600, 4000);
        let ret = score(Ending::Retired, 10, 100, 600, 4000);
        assert!(hk > ret);
    }

    #[test]
    fn longer_failed_reign_beats_quick_collapse() {
        let long = score(Ending::Collapsed, 20, 50, 250, 500);
        let quick = score(Ending::Collapsed, 2, 50, 250, 500);
        assert!(long > quick);
    }

    #[test]
    fn ranks_are_ordered() {
        assert_eq!(rank(50_000), "High King");
        assert_eq!(rank(20_000), "Sovereign Duke");
        assert_eq!(rank(0), "Serf");
        assert_eq!(rank(1), "Knight");
    }
}
