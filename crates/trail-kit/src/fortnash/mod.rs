//! Fort Nash's data-driven schema — the sibling of the kaintuck schema at the
//! crate root. Fort Nash has its own resources (food, powder, clothing,
//! supplies, the livestock train) and its own flow (the fortnight turn loop, the
//! random-event table, the ridge passes, the Christmas ice crossing), so it gets
//! its own `Scenario`, `Effect`, `EffectTarget`, `Gate`, and menus here — reusing
//! the genuinely game-agnostic [`MiniParams`](crate::MiniParams)/
//! [`MinigameSpec`](crate::MinigameSpec), [`Tier`](crate::Tier), and the
//! `apply_effects` interpreter *pattern*. Keeping Fort Nash's types separate
//! means a change here can never perturb kaintuck's golden trace.

pub mod effect;
pub mod minigame;
pub mod scenario;
pub mod setpiece;

pub use effect::{apply_effects, Effect, EffectCtx, EffectTarget, Outcome};
pub use minigame::{MinigameParams, MinigameSpec};
pub use scenario::{
    Calendar, Checkpoint, Ending, FortParams, HazardArm, HazardTable, RankTier, Scenario,
    ScoringParams, StartParams, TrailParams,
};
pub use setpiece::{Gate, Menus, SetPiece, SetPieceOption};
pub use crate::effect::Tier;

/// Parse a Fort Nash scenario from its RON source. Returns a spanned error on
/// malformed input so the caller can point at the offending line.
pub fn parse_scenario(src: &str) -> Result<Scenario, ron::error::SpannedError> {
    ron::from_str(src)
}
