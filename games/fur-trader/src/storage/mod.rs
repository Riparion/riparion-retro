//! Fur Trader's persistence keys, on top of retro-kit's versioned
//! localStorage.

use retro_kit::storage;

use crate::engine::scoring::HighScore;
use crate::engine::Game;

const SAVE_KEY: &str = "fur-trader.save";
const SAVE_VERSION: u32 = 1;
const SCORES_KEY: &str = "fur-trader.highscores";
const MAX_SCORES: usize = 10;

/// Resume a saved run, or start at the splash screen.
pub fn load_or_new() -> Game {
    let mut game: Game = storage::load(SAVE_KEY, SAVE_VERSION).unwrap_or_default();
    // Hand-edited or corrupted saves are accepted by serde; keep the books
    // in ranges the engine arithmetic is safe for.
    game.sanitize();
    // A save written while a fatal journal entry was still being paced is
    // decided but not yet in GameOver; the only finish_visit caller is the
    // results screen's paced task, which a reload kills. Heal here or it
    // soft-locks.
    game.finish_visit();
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
