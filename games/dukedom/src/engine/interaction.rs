//! The engine ↔ UI contract: paced narration plus the in-stride decisions the
//! year throws at you (the King's demands and the chaos of war), and the handful
//! of answers you can give back.

use serde::{Deserialize, Serialize};

/// One item at the head of the pending queue. Most are one-line narration;
/// a few carry a decision the [`crate::engine::Game`] is waiting on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Interaction {
    /// Informational line; tap to continue (the original's keypress pacing).
    Message(String),
    /// The King demands twice the royal tax to provoke you. Pay (Yes) or refuse
    /// and risk war (No).
    DoubleTax,
    /// The King levies peasants for his estates: supply them (Yes) or pay grain
    /// instead (No).
    KingLevy { peasants: i64, grain: i64 },
    /// A rival Duke threatens war — strike first (Yes) or hold (No)?
    WarAttack,
    /// Hire mercenaries for the coming battle (at `price` HL each, up to `max`).
    WarMercenary { max: i64, price: i64 },
}

/// The player's answer to the head interaction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Response {
    /// Tap-through on a [`Interaction::Message`].
    Acknowledge,
    Yes,
    No,
    /// A numeric answer (e.g. mercenaries hired).
    Amount(i64),
}
