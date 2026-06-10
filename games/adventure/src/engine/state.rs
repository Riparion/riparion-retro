//! Small state types shared by the engine and the UI: the screen [`Mode`], the
//! transcript [`Line`], and the [`Pending`] yes/no continuation. The big mutable
//! `Game` struct itself lives in `game.rs`.

use serde::{Deserialize, Serialize};

/// Bumped on any breaking change to the serialized `Game`; old saves are then
/// silently discarded (by design — see NOTES.md).
pub const SAVE_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Fresh, un-started game sitting behind the title screen.
    Splash,
    /// The single command loop.
    Playing,
    /// The adventure has ended (won, quit, or permadeath).
    GameOver,
}

/// One line of the scrollback transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Line {
    /// A command the player typed (rendered as `> ...`).
    Echo(String),
    /// Engine output (faithful uppercase; the UI lightly modernizes casing).
    Out(String),
}

/// A yes/no question the engine is waiting on. Mirrors python-adventure's
/// `yesno_callback`, but as a serializable tag we dispatch on when the answer
/// arrives (closures can't be saved).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pending {
    /// "Would you like instructions?" at game start.
    Instructions,
    /// "Do you want me to try to reincarnate you?"
    Resurrect,
    /// A loiter hint; the payload is the index into `data().hints`.
    Hint(usize),
    /// "Attack the dragon with your bare hands?" (casual: any non-yes/no cancels.)
    AttackDragon,
    /// Confirm quitting.
    Quit,
    /// Confirm ending after the SCORE command.
    Score,
    /// Reading the oyster's cryptic clue.
    ReadOyster,
    /// "Do you mean …?" — the payload is the inferred command to run on `yes`.
    /// (casual: any non-yes/no answer cancels and is reprocessed as a command.)
    DidYouMean(Vec<String>),
}
