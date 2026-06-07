//! Shared kit for the retro games in this workspace: the green-phosphor CRT
//! aesthetic, touch-first UI components, and platform primitives (seedable
//! RNG, money formatting, versioned localStorage persistence).
//!
//! Game crates link the stylesheet once and build on the pieces:
//!
//! ```rust,ignore
//! document::Stylesheet { href: retro_kit::CRT_CSS }
//! ```

pub mod components;
pub mod format;
pub mod playback;
pub mod rng;
pub mod storage;
pub mod theme;

use dioxus::prelude::*;

/// The shared CRT stylesheet: phosphor palette, scanlines, buttons, inputs,
/// panels, chips, action bar. Game-specific flourishes belong in each game's
/// own css, layered on top of this.
pub const CRT_CSS: Asset = asset!("/assets/crt.css");
