//! `trail-kit` — a data-driven scenario engine for the trail games in this
//! workspace. A whole game (content *and* flow) is described by an embedded RON
//! file; this crate provides the schema it deserializes into and the helpers a
//! host game uses to interpret it.
//!
//! The crate is deliberately platform-free (no dioxus, no wasm bits) so it is
//! fully host-testable with `cargo test`. A host game embeds its RON with
//! `include_str!`, parses it once behind a `OnceLock`, and drives its own engine
//! loop off the resulting immutable [`Scenario`].

pub mod effect;
pub mod minigame;
pub mod scenario;
pub mod setpiece;

pub use effect::{apply_effects, Effect, EffectCtx, EffectTarget, Outcome, Tier};
pub use minigame::{MiniParams, MinigameSpec};
pub use scenario::{HazardArm, Scenario};
pub use setpiece::{Gate, Menus, SetPiece, SetPieceOption};

/// Parse a scenario from its RON source. Returns a spanned error on malformed
/// input so the caller can point at the offending line.
pub fn parse_scenario(src: &str) -> Result<Scenario, ron::error::SpannedError> {
    ron::from_str(src)
}
