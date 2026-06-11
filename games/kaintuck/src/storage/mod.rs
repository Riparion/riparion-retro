//! Kaintuck's persistence keys, on top of retro-kit's versioned localStorage.

use retro_kit::rng::random_seed;
use retro_kit::storage;

use crate::engine::scoring::HighScore;
use crate::engine::Game;

const SAVE_KEY: &str = "kaintuck.save";
const SCORES_KEY: &str = "kaintuck.highscores";
// Bumped to 2: the `Boat` struct dropped its cached fields and the six
// `pending_*` minigame slots became one `pending_task`, so v1 blobs no longer
// deserialize — an old save falls back to a fresh game rather than mis-loading.
const SAVE_VERSION: u32 = 2;
const MAX_SCORES: usize = 10;

/// Resume a saved journey, or start at the title screen.
pub fn load_or_new() -> Game {
    storage::load(SAVE_KEY, SAVE_VERSION).unwrap_or_else(|| Game::new(random_seed()))
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
