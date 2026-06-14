//! The trading house: what survives a journey. A run ends — in Nashville or in a
//! grave — and the money and good name you earned roll into the next Kaintuck who
//! sets out under your house. Pure and host-testable; the localStorage plumbing
//! lives in `crate::storage`.

use serde::{Deserialize, Serialize};

use super::scenario_data::scenario;
use super::state::EndGame;

/// The starting fortune a new run inherits. A brand-new house starts at the
/// scenario's stake; thereafter it's whatever the last run walked away with.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Carryover {
    pub cash: f64,
    pub reputation: f64,
}

impl Carryover {
    /// The clean-slate stake for a house that has never run.
    pub fn fresh() -> Self {
        Self {
            cash: scenario().start.cash,
            reputation: 0.0,
        }
    }
}

/// The persistent record of a house across every run: the fortune carried into
/// the next journey, plus a lifetime tally for the title screen.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Ledger {
    /// The name the last Kaintuck went by, pre-filled when naming the next one.
    /// `#[serde(default)]` so a pre-name ledger blob still loads (no version bump).
    #[serde(default)]
    pub name: String,
    /// Net cash the next run starts with (debt forgiven, never below zero).
    pub cash: f64,
    /// Reputation the next run starts with (−50..50).
    pub reputation: f64,
    /// Journeys begun and finished under this house.
    pub journeys: u32,
    /// Journeys that reached Nashville.
    pub arrivals: u32,
    /// Best single-run score the house has ever posted.
    pub best_score: i64,
    /// Total miles travelled across every run.
    pub lifetime_miles: i64,
}

impl Ledger {
    /// The fortune a new run inherits from this house.
    pub fn carry(&self) -> Carryover {
        Carryover {
            cash: self.cash,
            reputation: self.reputation,
        }
    }

    /// Fold a finished run into the house. Net wealth carries forward with debt
    /// forgiven and floored at zero, so a ruinous run leaves you poor but never
    /// soft-locked — the boatyard credit still covers a skiff. The trader's name
    /// is kept so the next Kaintuck can be named with one tap.
    pub fn record(&mut self, end: &EndGame, trader: &str) {
        self.name = trader.to_string();
        self.cash = (end.cash - end.debt).max(0) as f64;
        self.reputation = end.reputation as f64;
        self.journeys += 1;
        if end.won {
            self.arrivals += 1;
        }
        self.best_score = self.best_score.max(end.score);
        self.lifetime_miles += end.miles;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::state::GameOverCause;

    fn end(won: bool, cash: i64, debt: i64, reputation: i64, score: i64, miles: i64) -> EndGame {
        EndGame {
            won,
            cause: String::new(),
            cause_kind: if won {
                GameOverCause::Victory
            } else {
                GameOverCause::Drowned
            },
            score,
            rank: String::new(),
            cash,
            debt,
            crew_survived: 0,
            reputation,
            robbed: false,
            miles,
            days: 0,
            recorded: false,
        }
    }

    #[test]
    fn record_carries_net_wealth_and_forgives_debt() {
        let mut l = Ledger::default();
        l.record(&end(true, 400, 150, 12, 2000, 2280), "Davy");
        assert_eq!(l.cash, 250.0);
        assert_eq!(l.reputation, 12.0);
        assert_eq!(l.carry().cash, 250.0);
        assert_eq!(l.carry().reputation, 12.0);
        // The trader's name is kept for the next run's naming screen.
        assert_eq!(l.name, "Davy");
    }

    #[test]
    fn cash_floors_at_zero() {
        let mut l = Ledger::default();
        l.record(&end(false, 30, 200, -10, 0, 1200), "Davy");
        assert_eq!(l.cash, 0.0);
    }

    #[test]
    fn counters_and_best_score_accrue() {
        let mut l = Ledger::default();
        l.record(&end(false, 0, 0, 0, 500, 1000), "Davy");
        l.record(&end(true, 600, 0, 5, 2200, 2280), "Davy");
        l.record(&end(true, 100, 0, 0, 1800, 2280), "Davy");
        assert_eq!(l.journeys, 3);
        assert_eq!(l.arrivals, 2);
        assert_eq!(l.best_score, 2200);
        assert_eq!(l.lifetime_miles, 1000 + 2280 + 2280);
    }
}
