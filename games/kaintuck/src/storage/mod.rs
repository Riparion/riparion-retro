//! Kaintuck's persistence keys, on top of retro-kit's versioned localStorage.

use retro_kit::rng::random_seed;
use retro_kit::storage;

use crate::engine::ledger::Ledger;
use crate::engine::scoring::HighScore;
use crate::engine::Game;

const SAVE_KEY: &str = "kaintuck.save";
const SCORES_KEY: &str = "kaintuck.highscores";
const LEDGER_KEY: &str = "kaintuck.ledger";
const LEDGER_VERSION: u32 = 1;
// Bumped to 2: the `Boat` struct dropped its cached fields and the six
// `pending_*` minigame slots became one `pending_task`, so v1 blobs no longer
// deserialize — an old save falls back to a fresh game rather than mis-loading.
// Bumped to 3: the data-driven refactor dropped the `base_ranks` field from the
// saved `GameState` (it now reads from the embedded scenario), so v2 blobs no
// longer deserialize — an old save falls back to a fresh game.
const SAVE_VERSION: u32 = 3;
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

/// The persistent trading house, if one has ever finished a run.
pub fn ledger() -> Option<Ledger> {
    storage::load(LEDGER_KEY, LEDGER_VERSION)
}

pub fn save_ledger(l: &Ledger) {
    storage::save(LEDGER_KEY, LEDGER_VERSION, l);
}

pub fn clear_ledger() {
    storage::delete(LEDGER_KEY);
}
