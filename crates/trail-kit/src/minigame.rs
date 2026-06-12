//! Minigame difficulty/presentation parameters as data. Each minigame instance
//! (keyed by the same id its [`Outcome`](crate::Outcome) uses) carries the props
//! the host feeds straight into its minigame component — prompt text, tolerances,
//! durations, board sizes, icons. The host UI reads these instead of hardcoding
//! literals per task, so a scenario can retune feel without recompiling.

use serde::{Deserialize, Serialize};

/// One minigame's parameters, by kind. Field names and types mirror the
/// minigames-kit component props so the host can splat them directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MiniParams {
    Steady {
        prompt: String,
        tolerance: f64,
        duration_ms: u32,
        drift_speed: f64,
    },
    Quick {
        prompt: String,
        /// The words to choose among; the host picks the live target per seed.
        words: Vec<String>,
    },
    Timing {
        prompt: String,
        action: String,
        tolerance: f64,
        period_ms: u32,
    },
    Crowd {
        prompt: String,
        crowd_size: usize,
        member_icon: String,
        player_icon: String,
        exit_icon: String,
        reveal_ms: u32,
        navigate_ms: u32,
    },
    Sequence {
        prompt: String,
        symbols: Vec<String>,
        length: usize,
    },
    Brigade {
        prompt: String,
        threat_icon: String,
        cols: usize,
        rows: usize,
        initial_active: usize,
        spread_ms: u32,
        duration_ms: u32,
    },
}

/// A minigame instance: its id and the parameters to launch it with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinigameSpec {
    pub id: String,
    pub params: MiniParams,
}
