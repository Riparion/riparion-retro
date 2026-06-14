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
pub mod game_flow;
pub mod leaderboard;
pub mod playback;
pub mod rations;
pub mod scoring;
pub mod storage;
pub mod theme;

// The seedable RNG and money/number formatting now live in the pure
// `retro-core` crate (no Dioxus/gloo/web-sys), so headless engines and the
// native server can share them. Re-exported here so existing
// `retro_kit::rng` / `retro_kit::format` paths keep working unchanged.
pub use retro_core::{format, rng};

use dioxus::prelude::*;

/// The shared CRT stylesheet: phosphor palette, scanlines, buttons, inputs,
/// panels, chips, action bar. Game-specific flourishes belong in each game's
/// own css, layered on top of this.
pub const CRT_CSS: Asset = asset!("/assets/crt.css");

/// Read a `<meta name="…">` tag's trimmed `content` from the document. `None`
/// outside a browser or when the tag is absent or empty. Used to pick up
/// server-injected configuration (the leaderboard API base, the multiplayer
/// WebSocket endpoint) without baking a URL into the bundle.
pub fn meta_content(name: &str) -> Option<String> {
    let doc = web_sys::window()?.document()?;
    let el = doc
        .query_selector(&format!("meta[name=\"{name}\"]"))
        .ok()??;
    let content = el.get_attribute("content")?;
    let trimmed = content.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
