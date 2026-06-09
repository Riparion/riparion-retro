//! Dukedom's persistence keys, on top of retro-kit's versioned localStorage.

use retro_kit::rng::random_seed;
use retro_kit::storage;

use crate::engine::scoring::HighScore;
use crate::engine::Game;

const SAVE_KEY: &str = "dukedom.save";
const SCORES_KEY: &str = "dukedom.highscores";
const SAVE_VERSION: u32 = 1;
const MAX_SCORES: usize = 10;

/// Resume a saved reign, or start at the title screen.
pub fn load_or_new() -> Game {
    storage::load(SAVE_KEY, SAVE_VERSION).unwrap_or_else(|| Game::new(random_seed()))
}

/// Whether a resumable reign is persisted — the source of truth for the splash's
/// RESUME button, independent of the in-memory game's current mode.
pub fn has_save() -> bool {
    storage::load::<Game>(SAVE_KEY, SAVE_VERSION).is_some()
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
    storage::record_score(SCORES_KEY, score, |s| s.score, MAX_SCORES);
}
