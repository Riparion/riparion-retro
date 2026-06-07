//! Lunar Lander's persistence keys, on top of retro-kit's versioned
//! localStorage. High scores are kept per mission so one mode's point scale
//! can't crowd the other off the board.

use retro_kit::storage;

use crate::engine::scoring::HighScore;
use crate::engine::state::MissionKind;
use crate::engine::Game;

const SAVE_KEY: &str = "lunar-lander.save";
const SAVE_VERSION: u32 = 1;
const MAX_SCORES: usize = 10;

fn scores_key(mission: MissionKind) -> &'static str {
    match mission {
        MissionKind::Lunar => "lunar-lander.highscores.lunar",
        MissionKind::Rocket => "lunar-lander.highscores.rocket",
    }
}

/// Resume a saved descent, or start at the splash screen.
pub fn load_or_new() -> Game {
    let mut game: Game = storage::load(SAVE_KEY, SAVE_VERSION).unwrap_or_default();
    // A save written during contact playback is decided but still in
    // Mode::Flight; the only finish_flight caller is the flight screen's
    // paced task, which a reload kills. Heal here or the game soft-locks.
    game.finish_flight();
    game
}

pub fn save(game: &Game) {
    storage::save(SAVE_KEY, SAVE_VERSION, game);
}

pub fn clear_save() {
    storage::delete(SAVE_KEY);
}

pub fn high_scores(mission: MissionKind) -> Vec<HighScore> {
    storage::scores(scores_key(mission))
}

pub fn record_score(score: HighScore) {
    storage::record_score(scores_key(score.mission), score, |s| s.score, MAX_SCORES);
}
