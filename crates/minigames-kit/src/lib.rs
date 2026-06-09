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
//! - [`hunter`] — track a quarry bouncing across the top row and shoot it; a
//!   single-shot rifle with finite ammo.
//!
//! A host can take only what it needs with
//! `minigames-kit = { default-features = false, features = ["quickdraw"] }`.
//!
//! More to come (each gets its own module, feature, and a runnable example
//! under `examples/`).

#[cfg(feature = "crowd_threading")]
pub mod crowd_threading;
#[cfg(feature = "hunter")]
pub mod hunter;
#[cfg(feature = "quickdraw")]
pub mod quickdraw;
#[cfg(feature = "timing_bar")]
pub mod timing_bar;
