//! Reusable minigame components, parameterized so any game in the workspace can
//! drop them in. Each minigame is self-contained: it owns its own timing/input
//! and reports a plain result back through a callback, leaving scoring to the
//! host game's engine.
//!
//! So far:
//! - [`quickdraw`] — a flash-a-word, tap-it-fast reaction test.
//! - [`timing_bar`] — a rhythm-matching sweep: hit the button as the marker
//!   crosses a target zone.
//!
//! More to come (each gets its own module and a runnable example under
//! `examples/`).

pub mod quickdraw;
pub mod timing_bar;
