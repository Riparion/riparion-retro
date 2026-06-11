//! The pure-Rust Colossal Cave engine: data tables, RNG, and game logic. No
//! dioxus, web-sys, or gloo here — everything is host-testable with `cargo test`.

pub mod data;
pub mod game;
pub mod parse_infer;
pub mod rng;
pub mod state;
pub mod text;

#[cfg(test)]
mod tests;

pub use game::Game;
pub use state::{Line, Mode};
pub use text::{modernize, reflow};
