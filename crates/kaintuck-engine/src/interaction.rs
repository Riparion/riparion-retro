//! The engine ↔ UI contract: things the game needs to show or ask, and the
//! player's possible answers. The UI renders the queue head; the player's
//! [`Response`] is fed back through [`Game::resolve`](crate::Game::resolve).

use serde::{Deserialize, Serialize};

/// One pending prompt or paced message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Interaction {
    /// Informational line; tap to continue. `cover` is an optional narrative
    /// cover-art key (the slug after the `interaction-` prefix).
    Message { text: String, cover: Option<String> },
    /// A hand wants paying off or he walks. Yes pays `pay`; No risks morale.
    CrewLeaves { pay: f64 },
    /// A ferryman at Duck River. Yes pays the toll to cross dry; No fords it.
    FerryToll { toll: f64 },
}

/// The player's answer to the current interaction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Acknowledge,
    Yes,
    No,
}

/// A market report logged to the notification center rather than popped up:
/// wharf-factor lines, rumor payoffs, next-leg price tips, and trader gossip.
/// `read` flips once the player has opened the center.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub text: String,
    pub read: bool,
}
