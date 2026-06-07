//! Core state: the city's ledger. BASIC variable names from the 1978
//! listing appear in the doc comments for auditability against the spec.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Splash,
    Year,
    GameOver,
}

/// The decision the steward awaits within a year. The original's prompt
/// order: buy land; only if you bought none, offer to sell; feed; plant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    BuyLand,
    SellLand,
    Feed,
    Plant,
}

/// One line of the steward's report scroll.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogLine {
    pub text: String,
    /// Calamities (plague, mass starvation) get the danger tint.
    pub alert: bool,
    /// Opens a new report — the UI puts a gap above it.
    pub head: bool,
}

impl LogLine {
    pub fn line(text: impl Into<String>) -> Self {
        Self { text: text.into(), alert: false, head: false }
    }

    pub fn alert(text: impl Into<String>) -> Self {
        Self { text: text.into(), alert: true, head: false }
    }

    pub fn head(text: impl Into<String>) -> Self {
        Self { text: text.into(), alert: false, head: true }
    }
}

/// The city of Sumeria. (Original BASIC variables in parens.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// Year of the current report, 1-based (Z). The term is 10 years; the
    /// report for year 11 shows the last harvest and ends the game.
    pub year: i64,
    /// Population (P).
    pub population: i64,
    /// Bushels of grain in store (S).
    pub store: i64,
    /// Acres the city owns (A).
    pub acres: i64,
    /// Last harvest's yield in bushels per acre (Y).
    pub bushels_per_acre: i64,
    /// Bushels the rats ate last year (E).
    pub rats_ate: i64,
    /// People who came to the city, applied at the next report (I).
    pub immigrants: i64,
    /// People who starved last year (D).
    pub starved: i64,
    /// Bushels handed out this year, pending resolution (Q at line 540).
    pub fed_bushels: i64,
    /// Plague rolled at the end of last year, lands at the next report
    /// (line 542: `Q=INT(10*(2*RND(1)-.3))`, plague iff Q<=0).
    pub plague_pending: bool,
    /// Total starved over the whole term (D1).
    pub total_starved: i64,
    /// Running average percent of the population starved per year (P1).
    pub avg_starved_pct: f64,
    /// This year's land price in bushels per acre (line 310: 17..=26).
    pub land_price: i64,
    pub phase: Phase,
}

impl State {
    /// Lines 95–110: the pre-seeded ledger behind the first report
    /// (95 people + 5 immigrants, 1000 acres, 3000 harvested at 3 per
    /// acre, rats ate 200 leaving 2800 in store).
    pub fn initial() -> Self {
        Self {
            year: 0,
            population: 95,
            store: 2800,
            acres: 1000,
            bushels_per_acre: 3,
            rats_ate: 200,
            immigrants: 5,
            starved: 0,
            fed_bushels: 0,
            plague_pending: false,
            total_starved: 0,
            avg_starved_pct: 0.0,
            land_price: 0,
            phase: Phase::BuyLand,
        }
    }
}
