//! Online hall-of-fame glue over [`retro_kit::leaderboard`]: the CMS slug/board
//! this game posts to, and the per-run payload echoed back on board reads.
//!
//! One [`ScorePayload`] serves both the write side (app.rs, on a finished run)
//! and the read side (splash.rs, the board panel), so the JSON field names are a
//! single contract — a renamed field breaks both ends together instead of
//! silently emptying the displayed board.

use serde::{Deserialize, Serialize};

use crate::engine::state::EndGame;

/// The app's slug as published in riparion-cms — the leaderboard namespace.
pub const SLUG: &str = "kaintuck";
/// The default board within the app.
pub const BOARD: &str = "main";

/// The opaque per-run JSON stored with a score and echoed back on board reads.
/// Serialized on submit and deserialized on read through this one type, so the
/// field names stay a single source of truth; `#[serde(default)]` keeps older
/// rows (written before a field existed, or carrying none) readable. `version`
/// lets future balance comparisons filter by game build.
///
/// The deterministic run seed (for replay verification) is intentionally left
/// out for now — capturing it would force a save-format break and is useless
/// without a decision log, both of which belong to a later phase.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScorePayload {
    pub won: bool,
    pub rank: String,
    pub cash: i64,
    pub debt: i64,
    pub crew_survived: i64,
    pub reputation: i64,
    pub robbed: bool,
    pub miles: i64,
    pub days: i64,
    pub version: String,
}

impl ScorePayload {
    pub fn from_end(end: &EndGame) -> Self {
        Self {
            won: end.won,
            rank: end.rank.clone(),
            cash: end.cash,
            debt: end.debt,
            crew_survived: end.crew_survived,
            reputation: end.reputation,
            robbed: end.robbed,
            miles: end.miles,
            days: end.days,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
