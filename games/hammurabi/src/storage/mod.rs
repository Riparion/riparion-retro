//! Hammurabi's persistence keys, on top of retro-kit's versioned
//! localStorage.

use retro_kit::storage;

use crate::engine::scoring::HighScore;
use crate::engine::Game;

const SAVE_KEY: &str = "hammurabi.save";
const SAVE_VERSION: u32 = 1;
const SCORES_KEY: &str = "hammurabi.highscores";
const MAX_SCORES: usize = 10;

/// Resume a saved reign, or start at the splash screen.
pub fn load_or_new() -> Game {
    let mut game: Game = storage::load(SAVE_KEY, SAVE_VERSION).unwrap_or_default();
    // Hand-edited or corrupted saves are accepted by serde; keep the ledger
    // in ranges the engine arithmetic is safe for.
    game.sanitize();
    // A save written while the final report was still being paced is decided
    // but still in Mode::Year; the only finish_year caller is the year
    // screen's paced task, which a reload kills. Heal here or it soft-locks.
    game.finish_year();
    game
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
