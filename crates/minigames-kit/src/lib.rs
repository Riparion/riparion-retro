//! Reusable minigame components, parameterized so any game in the workspace can
//! drop them in. Each minigame is self-contained: it owns its own timing/input
//! and reports a plain result back through a callback, leaving scoring to the
//! host game's engine.
//!
//! So far (each behind a same-named cargo feature, all on by default):
//! - [`quickdraw`] — a flash-a-word, tap-it-fast reaction test.
//! - [`timing_bar`] — a rhythm-matching sweep: hit the button as the marker
//!   crosses a target zone.
//! - [`crowd_threading`] — memorize a lit route through a crowd, then walk it
//!   from memory once the map closes.
//! - [`faro`] — the Old-West banking card game: stake ranks (copper one to bet
//!   it loses), deal the turn, and read the case keeper to grow your stake to a
//!   target before the deck runs out.
//! - [`hunter`] — track a quarry bouncing across the top row and shoot it; a
//!   single-shot rifle with finite ammo.
//! - [`steady_hands`] — a touch-first precision trace: drag to keep an offset
//!   cursor on a drifting target for the duration; accuracy = time on target.
//! - [`bucket_brigade`] — triage against a spreading threat: tap the flames on a
//!   grid faster than they multiply before the clock runs out.
//! - [`hot_cold`] — a search/deduction hunt: probe a grid to find a hidden target
//!   from warmer/colder (or distance-ring) clues in as few taps as possible.
//! - [`sequence`] — a Simon-style order-memory test: a short run of symbols
//!   flashes, then the palette opens and you tap them back in the same order.
//! - [`heave`] — a sustained-exertion test: press and hold to build force
//!   against staged resistance without crossing the slip ceiling or draining
//!   your grip; pacing, not mashing.
//!
//! A host can take only what it needs with
//! `minigames-kit = { default-features = false, features = ["quickdraw"] }`.
//!
//! More to come (each gets its own module, feature, and a standalone runnable
//! demo crate under `examples/<name>` at the workspace root, served with
//! `dx serve --package minigames-kit-<name>`).

#[cfg(feature = "bucket_brigade")]
pub mod bucket_brigade;
#[cfg(feature = "crowd_threading")]
pub mod crowd_threading;
#[cfg(feature = "faro")]
pub mod faro;
/// Shared inline styling for the grid-based minigames (Hunter, BucketBrigade, HotCold).
#[cfg(any(feature = "hunter", feature = "bucket_brigade", feature = "hot_cold"))]
pub mod grid;
#[cfg(feature = "heave")]
pub mod heave;
#[cfg(feature = "hot_cold")]
pub mod hot_cold;
#[cfg(feature = "hunter")]
pub mod hunter;
#[cfg(feature = "quickdraw")]
pub mod quickdraw;
#[cfg(feature = "sequence")]
pub mod sequence;
#[cfg(feature = "steady_hands")]
pub mod steady_hands;
#[cfg(feature = "timing_bar")]
pub mod timing_bar;
