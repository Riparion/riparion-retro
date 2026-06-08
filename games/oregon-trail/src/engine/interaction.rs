//! The engine ↔ UI contract: paced messages the trail throws at you, and the
//! handful of answers you can give back.

use serde::{Deserialize, Serialize};

/// One pending message; the UI renders the queue head and taps to continue.
/// Oregon Trail's events are narration first, so [`Interaction::Message`] does
/// nearly all the work — decisions are their own screens (Eat, Riders, Shoot).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Interaction {
    /// Informational line; tap to continue (the original's keypress pacing).
    Message(String),
}

/// The player's answer to the current interaction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Acknowledge,
}

/// Your move when riders appear on the trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tactic {
    Run = 1,
    Attack = 2,
    Continue = 3,
    CircleWagons = 4,
}

/// Why we're playing the marksmanship reaction game; determines how the
/// measured response time is spent when the shot resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShotPurpose {
    /// Hunting for food.
    Hunt,
    /// Bandits attacking on the trail.
    Bandits,
    /// Wild animals attacking.
    WildAnimals,
    /// Gunfight with hostile riders. `circle` = you circled the wagons.
    Riders { circle: bool },
}

/// The four words the gunfight flashes; type the right one fast.
pub const SHOT_WORDS: [&str; 4] = ["BANG", "BLAM", "POW", "WHAM"];
