//! The engine ↔ UI contract: things the game needs to show or ask,
//! and the player's possible answers.

use serde::{Deserialize, Serialize};

/// One pending prompt or paced message. The UI renders the queue head;
/// the player's `Response` is fed back through `Game::resolve`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Interaction {
    /// Informational line; tap to continue (the original's keypress pacing).
    Message(String),
    /// Li Yuen asks for a donation of `amount` at Hong Kong. Yes/No.
    LiYuenDonation { amount: f64 },
    /// Player accepted a donation larger than their cash; Wu offers to cover
    /// the difference (added to debt). Yes/No.
    WuCoverDonation { amount: f64, shortfall: f64 },
    /// McHenry offers repairs at `rate` cash per damage point. Amount(0) declines.
    McHenryRepair { rate: f64 },
    /// "Do you have business with Elder Brother Wu?" Yes opens the Wu screen.
    WuVisitPrompt,
    /// Wu offers `offer` cash now in exchange for `repay` added to debt.
    /// Refusing ends the game.
    WuBailout { offer: f64, repay: f64 },
    /// A larger ship: +50 capacity for `price`. Yes/No.
    ShipOffer { price: f64, damaged: bool },
    /// One more gun for `price` (takes 10 hold units). Yes/No.
    GunOffer { price: f64 },
}

/// The player's answer to the current interaction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Acknowledge,
    Yes,
    No,
    /// A quantity or cash amount (already clamped by the UI; re-clamped by the engine).
    Amount(f64),
}

/// Combat orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Order {
    Fight,
    Run,
    /// Throw `amount` units of commodity `item` overboard, then try to run.
    Throw { item: usize, amount: i64 },
}
