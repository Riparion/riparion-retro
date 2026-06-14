//! Pure, platform-free retro-game primitives shared across the workspace:
//! a seedable, serializable RNG and the money/number display helpers.
//!
//! Unlike `retro-kit` (which pulls in Dioxus and the browser `gloo`/`web-sys`
//! stack), this crate has no UI or platform dependencies, so it compiles to
//! both wasm and native. The headless game engines and the native game server
//! depend on it directly; `retro-kit` re-exports it so existing
//! `retro_kit::rng` / `retro_kit::format` paths keep resolving.

pub mod format;
pub mod hash;
pub mod rng;
