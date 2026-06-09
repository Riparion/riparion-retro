//! The engine ↔ UI contract. Santa Paravia's only between-screen events are
//! paced narration lines — births and deaths, the Baron's raid, bankruptcy, a
//! new title — so the queue carries plain messages, tapped through one at a
//! time like the original's press-ENTER pacing.

use serde::{Deserialize, Serialize};

/// One item at the head of the pending queue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Interaction {
    /// Informational line; tap to continue.
    Message(String),
}

/// The player's answer to the head interaction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Response {
    /// Tap-through on a [`Interaction::Message`].
    Acknowledge,
}
