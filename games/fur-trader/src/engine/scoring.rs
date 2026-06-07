//! End-of-run reckoning. The original kept no score — final savings *is*
//! the score; the verdict says how the run ended.

use serde::{Deserialize, Serialize};

use super::state::State;

/// How the expedition ended. The original only had the Iroquois ambush
/// (line 1284's STOP); Retired and Bankrupt are this port's endings for
/// the otherwise endless year loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Verdict {
    /// Quit while ahead: savings banked, name made.
    Retired,
    /// Killed by an Iroquois war party on the way to Fort New York.
    Killed,
    /// Couldn't outfit another expedition (savings below $105).
    Bankrupt,
}

impl Verdict {
    pub fn label(&self) -> &'static str {
        match self {
            Verdict::Retired => "Retired",
            Verdict::Killed => "Killed",
            Verdict::Bankrupt => "Bankrupt",
        }
    }
}

/// The final reckoning, built once when the run ends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndGame {
    pub verdict: Verdict,
    /// Savings at the end, cents and all.
    pub savings: f64,
    /// Years traded (the year the run ended on; bankruptcy counts only
    /// completed years).
    pub years: i64,
    pub score: i64,
    /// Guards the once-only high-score record in the autosave effect.
    pub recorded: bool,
}

/// Final savings in whole dollars — the only score the original implies.
pub fn score(savings: f64) -> i64 {
    savings.max(0.0).floor() as i64
}

fn end(st: &State, verdict: Verdict, years: i64) -> EndGame {
    EndGame { verdict, savings: st.savings, years, score: score(st.savings), recorded: false }
}

/// Hung up the paddle after completing year N.
pub fn retired(st: &State) -> EndGame {
    end(st, Verdict::Retired, st.year)
}

/// The ambush on the way to Fort New York, in year N.
pub fn killed(st: &State) -> EndGame {
    end(st, Verdict::Killed, st.year)
}

/// Checked before the year counter increments: `st.year` completed years.
pub fn bankrupt(st: &State) -> EndGame {
    end(st, Verdict::Bankrupt, st.year)
}

/// One row of the local trading-post ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighScore {
    pub score: i64,
    pub verdict: String,
    pub savings: i64,
    pub years: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_is_floored_savings_never_negative() {
        assert_eq!(score(600.0), 600);
        assert_eq!(score(1234.99), 1234);
        assert_eq!(score(0.5), 0);
        assert_eq!(score(-40.0), 0);
    }

    #[test]
    fn endings_carry_savings_and_years() {
        let st = State { year: 7, savings: 912.35, ..State::initial() };
        let end = retired(&st);
        assert_eq!(end.verdict, Verdict::Retired);
        assert_eq!(end.years, 7);
        assert_eq!(end.score, 912);
        assert!(!end.recorded);
        assert_eq!(killed(&st).verdict, Verdict::Killed);
        assert_eq!(bankrupt(&st).verdict, Verdict::Bankrupt);
    }
}
