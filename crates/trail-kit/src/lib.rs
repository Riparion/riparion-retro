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
pub mod fortnash;
pub mod minigame;
pub mod scenario;
pub mod setpiece;

pub use effect::{apply_effects, Effect, EffectCtx, EffectTarget, Outcome, Tier};
pub use minigame::{MiniParams, MinigameSpec};
pub use scenario::{
    BanterBeat, BanterGate, BanterLine, BanterPhase, BanterPool, GossipFlavor, GossipPhrasing,
    HazardArm, MarketBias, PersonaEpithet, Scenario,
};
pub use setpiece::{Gate, Menus, SetPiece, SetPieceOption};

/// Parse a scenario from its RON source. Returns a spanned error on malformed
/// input so the caller can point at the offending line.
pub fn parse_scenario(src: &str) -> Result<Scenario, ron::error::SpannedError> {
    ron::from_str(src)
}

/// Define a host game's `scenario()` accessor: embed a RON file, parse it once
/// behind a `OnceLock`, and hand back an immutable `&'static` scenario. Both
/// trail games share this boilerplate; they differ only in the RON path, the
/// concrete `Scenario` type, and the parse fn (kaintuck's crate-root one vs Fort
/// Nash's `fortnash::parse_scenario`). A malformed RON is a programming error,
/// so a parse failure panics with the span.
///
/// `$ron` is resolved by `include_str!` relative to the invoking file, exactly
/// as a hand-written loader would.
#[macro_export]
macro_rules! embed_scenario {
    ($ron:literal, $ty:ty, $parse:path) => {
        /// The parsed, immutable scenario. Parsed once on first access.
        pub fn scenario() -> &'static $ty {
            static SCENARIO: ::std::sync::OnceLock<$ty> = ::std::sync::OnceLock::new();
            SCENARIO.get_or_init(|| {
                $parse(include_str!($ron)).expect(concat!($ron, " failed to parse"))
            })
        }
    };
}
