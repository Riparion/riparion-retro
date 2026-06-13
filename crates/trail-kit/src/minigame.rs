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
    /// Press-and-hold sustained exertion (heaving a grounded boat off a bar,
    /// hauling on a cordelle). The other props use the component's defaults.
    Heave {
        prompt: String,
        stages: usize,
        heave_rate: f64,
        hold_ticks: u32,
        slip_load: f64,
    },
    /// Probe-and-deduce search (finding firm ground in a swamp, a bandit hunting
    /// your hidden money). `feedback` stays at the component default (warmer/colder).
    HotCold {
        prompt: String,
        cols: usize,
        rows: usize,
        max_probes: usize,
    },
    /// Track-and-shoot with finite ammo (a fight for your life with the Harpes,
    /// hunting game for the pot on the Trace).
    Hunter {
        prompt: String,
        hunted_icon: String,
        cols: usize,
        rows: usize,
        ammo: usize,
        step_ms: u32,
        reload_ms: u32,
    },
}

/// A minigame instance: its id and the parameters to launch it with. Generic
/// over the host game's params enum `P` (kaintuck's [`MiniParams`], Fort Nash's
/// [`MinigameParams`](crate::fortnash::MinigameParams)) so both games share one
/// id↔params spec shape while keeping their own parameter vocabularies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinigameSpec<P> {
    pub id: String,
    pub params: P,
}
