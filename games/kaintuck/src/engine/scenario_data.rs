//! The embedded Kaintuck scenario. The RON content/flow file is compiled into
//! the binary and parsed once, lazily, into an immutable [`Scenario`] behind a
//! [`OnceLock`]. The engine reads it ad hoc through [`scenario()`] rather than
//! holding a borrow, so [`Game`](super::Game) stays `Serialize`.
//!
//! The `OnceLock` + `include_str!` + parse boilerplate is shared with fort-nash
//! through [`trail_kit::embed_scenario!`]; only the path, type, and parse fn
//! differ.

trail_kit::embed_scenario!("kaintuck.ron", trail_kit::Scenario, trail_kit::parse_scenario);
