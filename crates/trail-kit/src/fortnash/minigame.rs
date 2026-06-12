//! Fort Nash's minigame difficulty/presentation parameters as data. Each minigame
//! instance (keyed by an id the host maps its live task to) carries the props the
//! host feeds straight into its minigames-kit component — prompt text, tolerances,
//! durations, board sizes, icons. Field names and types mirror the minigames-kit
//! component props so the host can splat them directly.
//!
//! This is a fort-nash-local cousin of [`crate::minigame::MiniParams`]: Fort Nash
//! needs fields kaintuck's variant doesn't carry (the per-task `show_ms` flash
//! speed, the per-task `spread_chance`) and omits ones it never sets (the crowd
//! reveal/navigate windows, the brigade grid size — both left at the component
//! defaults), so it gets its own concrete enum rather than bending the shared one.

use serde::{Deserialize, Serialize};

pub use crate::minigame::MinigameSpec;

/// One minigame's parameters, by kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MinigameParams {
    /// The steady-hand trace (the frozen-Cumberland ice crossing).
    Steady {
        prompt: String,
        tolerance: f64,
        duration_ms: u32,
        drift_speed: f64,
    },
    /// The timing bar (setting a bone, measuring a dose).
    Timing {
        prompt: String,
        action: String,
        tolerance: f64,
        period_ms: u32,
    },
    /// The route-memory game (fog, the escape, the rugged climb).
    Crowd {
        prompt: String,
        crowd_size: usize,
        member_icon: String,
        player_icon: String,
        exit_icon: String,
    },
    /// The order-memory game (wheel, ox leg, frostbite first-aid).
    Sequence {
        prompt: String,
        symbols: Vec<String>,
        length: usize,
        /// How long each step flashes (ms) — faster is harder.
        show_ms: u32,
    },
    /// The triage-against-spread game (fire, sleet, blizzard).
    Brigade {
        prompt: String,
        threat_icon: String,
        initial_active: usize,
        spread_ms: u32,
        spread_chance: f64,
        duration_ms: u32,
    },
}
