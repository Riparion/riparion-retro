//! Adventure's persistence keys, on top of retro-kit's versioned localStorage.

use serde::{Deserialize, Serialize};

use retro_kit::rng::random_seed;
use retro_kit::storage;

use crate::engine::state::SAVE_VERSION;
use crate::engine::Game;

const SAVE_KEY: &str = "adventure.save";
const SCORES_KEY: &str = "adventure.highscores";
const MAX_SCORES: usize = 10;

/// One row of the hall of fame. The original game has no player name, so we
/// keep the score, the turns taken, and whether the cave was reached/closed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighScore {
    pub score: i32,
    pub turns: i32,
    pub closed: bool,
}

/// Resume a saved adventure, or sit at the title screen with a fresh seed. A
/// save whose data-derived vectors no longer match the embedded `advent.dat`
/// (data changed without a `SAVE_VERSION` bump) is discarded rather than risk an
/// out-of-bounds panic on resume.
pub fn load_or_new() -> Game {
    storage::load(SAVE_KEY, SAVE_VERSION)
        .filter(Game::is_data_compatible)
        .unwrap_or_else(|| Game::new(random_seed()))
}

pub fn save(game: &Game) {
    storage::save(SAVE_KEY, SAVE_VERSION, game);
}

pub fn clear_save() {
    storage::delete(SAVE_KEY);
}

pub fn high_scores() -> Vec<HighScore> {
    storage::scores(SCORES_KEY)
}

pub fn record_score(score: HighScore) {
    storage::record_score(SCORES_KEY, score, |s| s.score as i64, MAX_SCORES);
}
